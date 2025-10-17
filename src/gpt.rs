use crate::config::AppConfig;
use crate::TimelinePost;
use crate::database as db;
use dotenv::dotenv;
use std::error::Error;
use std::fs::File;
use std::time::Duration;
use std::env;
use tokio::time::timeout;
use openai_api_rs::v1::api::OpenAIClient;
use openai_api_rs::v1::chat_completion::{self, ChatCompletionRequest};
use chrono::{Local, TimeZone};
use tiktoken_rs::o200k_base;
use serde::{Deserialize, Serialize};

/// GPTの応答（返信＋印象）
#[derive(Debug, Serialize, Deserialize)]
pub struct GptResponseWithImpression {
    pub reply: String,
    pub impression: String,
}

/// トークン数を計算
fn count_tokens(text: &str) -> usize {
    let bpe = o200k_base().expect("[Token] tiktoken (o200k_base) 初期化に失敗しました");
    let tokens = bpe.encode_with_special_tokens(text);
    tokens.len()
}

#[allow(dead_code)]
pub async fn call_gpt(prompt: &str, user_text: &str) -> Result<String, Box<dyn Error>> {
    call_gpt_with_category(prompt, user_text, "unknown", "general").await
}

pub async fn call_gpt_with_category(prompt: &str, user_text: &str, bot_pubkey: &str, category: &str) -> Result<String, Box<dyn Error>> {
    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY_SECS: u64 = 3;
    
    dotenv().ok();
    let api_key = env::var("OPEN_AI_API_KEY").expect("OPEN_AI_API_KEY is not set");
    
    // タイムアウト設定を取得
    let file = File::open("../config.yml")?;
    let config: AppConfig = serde_yaml::from_reader(file)?;
    let timeout_secs = config.get_u64_setting("gpt_timeout");
    
    // トークン数を計算
    let prompt_tokens = count_tokens(prompt);
    let user_tokens = count_tokens(user_text);
    let total_prompt_tokens = prompt_tokens + user_tokens;
    
    let req = ChatCompletionRequest::new(
        "gpt-5-nano".to_string(),
        vec![
            chat_completion::ChatCompletionMessage {
                role: chat_completion::MessageRole::system,
                content: chat_completion::Content::Text(String::from(prompt)),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
            chat_completion::ChatCompletionMessage {
                role: chat_completion::MessageRole::user,
                content: chat_completion::Content::Text(String::from(user_text)),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
        ]
    );
    
    let mut last_error: Option<String> = None;
    
    for attempt in 1..=MAX_RETRIES {
        let mut client = OpenAIClient::builder()
            .with_api_key(api_key.clone())
            .build()?;
        
        let chat_completion_future = async {
            client.chat_completion(req.clone()).await
        };

        // タイムアウトを設定
        match timeout(Duration::from_secs(timeout_secs), chat_completion_future).await {
            Ok(result) => match result {
                Ok(response) => {
                    // 正常なレスポンスの処理
                    match &response.choices[0].message.content {
                        Some(content) => {
                            if attempt > 1 {
                                println!("[GPT] リトライ成功 (試行 {}/{})", attempt, MAX_RETRIES);
                            }
                            
                            // 完了トークン数を計算
                            let completion_tokens = count_tokens(content);
                            
                            // プロンプト全体を作成（システムプロンプト + ユーザー入力）
                            let full_prompt = format!("{}\n\nユーザー入力:\n{}", prompt, user_text);
                            
                            // トークン使用量を記録
                            println!("[Token] 記録開始: bot_pubkey={}, category={}", bot_pubkey, category);
                            if let Ok(conn) = db::connect() {
                                if let Err(e) = db::record_token_usage(&conn, bot_pubkey, category, total_prompt_tokens, completion_tokens, &full_prompt, content) {
                                    eprintln!("[Token] 記録エラー: {:?}", e);
                                }
                            } else {
                                eprintln!("[Token] DB接続エラー");
                            }
                            
                            return Ok(content.to_string());
                        },
                        None => {
                            last_error = Some("No content found in response".to_string());
                        }
                    }            
                },
                Err(e) => {
                    last_error = Some(format!("{}", e));
                }
            },
            Err(_) => {
                last_error = Some(format!("Timeout after {} seconds", timeout_secs));
            }
        }
        
        // 最後の試行でなければリトライ
        if attempt < MAX_RETRIES {
            eprintln!("[GPT] エラー発生 (試行 {}/{}): {:?} - {}秒後にリトライ", 
                      attempt, MAX_RETRIES, last_error, RETRY_DELAY_SECS);
            tokio::time::sleep(Duration::from_secs(RETRY_DELAY_SECS)).await;
        }
    }
    
    // 全てのリトライが失敗
    Err(last_error.unwrap_or_else(|| "Unknown error after retries".to_string()).into())
}

/// GPT呼び出し（JSON mode、印象付き返信用）
/// 注：openai_api_rsがJSON modeをネイティブサポートしていないため、プロンプトでJSON形式を強制
pub async fn call_gpt_with_json_mode(prompt: &str, user_text: &str, bot_pubkey: &str, category: &str) -> Result<String, Box<dyn Error>> {
    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY_SECS: u64 = 3;
    
    dotenv().ok();
    let api_key = env::var("OPEN_AI_API_KEY").expect("OPEN_AI_API_KEY is not set");
    
    // タイムアウト設定を取得
    let file = File::open("../config.yml")?;
    let config: AppConfig = serde_yaml::from_reader(file)?;
    let timeout_secs = config.get_u64_setting("gpt_timeout");
    
    // トークン数を計算
    let prompt_tokens = count_tokens(prompt);
    let user_tokens = count_tokens(user_text);
    let total_prompt_tokens = prompt_tokens + user_tokens;
    
    // JSON形式を強制するためのシステムプロンプトを追加
    let json_enforced_prompt = format!(
        "{}\n\n重要: 必ずJSON形式で返答してください。他の形式やテキストは一切含めないでください。",
        prompt
    );
    
    let req = ChatCompletionRequest::new(
        "gpt-5-nano".to_string(),
        vec![
            chat_completion::ChatCompletionMessage {
                role: chat_completion::MessageRole::system,
                content: chat_completion::Content::Text(json_enforced_prompt),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
            chat_completion::ChatCompletionMessage {
                role: chat_completion::MessageRole::user,
                content: chat_completion::Content::Text(String::from(user_text)),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
        ]
    );
    
    let mut last_error: Option<String> = None;
    
    for attempt in 1..=MAX_RETRIES {
        let mut client = OpenAIClient::builder()
            .with_api_key(api_key.clone())
            .build()?;
        
        let chat_completion_future = async {
            client.chat_completion(req.clone()).await
        };

        // タイムアウトを設定
        match timeout(Duration::from_secs(timeout_secs), chat_completion_future).await {
            Ok(result) => match result {
                Ok(response) => {
                    // 正常なレスポンスの処理
                    match &response.choices[0].message.content {
                        Some(content) => {
                            if attempt > 1 {
                                println!("[GPT JSON] リトライ成功 (試行 {}/{})", attempt, MAX_RETRIES);
                            }
                            
                            // 完了トークン数を計算
                            let completion_tokens = count_tokens(content);
                            
                            // プロンプト全体を作成（システムプロンプト + ユーザー入力）
                            let full_prompt = format!("{}\n\nユーザー入力:\n{}", prompt, user_text);
                            
                            // トークン使用量を記録
                            println!("[Token] 記録開始: bot_pubkey={}, category={}", bot_pubkey, category);
                            if let Ok(conn) = db::connect() {
                                if let Err(e) = db::record_token_usage(&conn, bot_pubkey, category, total_prompt_tokens, completion_tokens, &full_prompt, content) {
                                    eprintln!("[Token] 記録エラー: {:?}", e);
                                }
                            } else {
                                eprintln!("[Token] DB接続エラー");
                            }
                            
                            return Ok(content.to_string());
                        },
                        None => {
                            last_error = Some("No content found in response".to_string());
                        }
                    }            
                },
                Err(e) => {
                    last_error = Some(format!("{}", e));
                }
            },
            Err(_) => {
                last_error = Some(format!("Timeout after {} seconds", timeout_secs));
            }
        }
        
        // 最後の試行でなければリトライ
        if attempt < MAX_RETRIES {
            eprintln!("[GPT JSON] エラー発生 (試行 {}/{}): {:?} - {}秒後にリトライ", 
                      attempt, MAX_RETRIES, last_error, RETRY_DELAY_SECS);
            tokio::time::sleep(Duration::from_secs(RETRY_DELAY_SECS)).await;
        }
    }
    
    // 全てのリトライが失敗
    Err(last_error.unwrap_or_else(|| "Unknown error after retries".to_string()).into())
}

/// 新しいインターフェース: 会話コンテキスト文字列を受け取る
#[allow(dead_code)]
pub async fn get_reply_with_context<'a>(
    bot_pubkey: &'a str,
    personality: &'a str,
    user_text: &'a str,
    has_mention: bool,
    context: Option<String>,
) -> Result<String, Box<dyn Error>> {
    dotenv().ok();
    
    // 回答長設定を取得
    let file = File::open("../config.yml")?;
    let config: AppConfig = serde_yaml::from_reader(file)?;
    let answer_length = config.get_i32_setting("gpt_answer_length");

    let start_delimiter = "<<";
    let end_delimiter = ">>";
    let mut extracted_prompt = "";
    let mut modified_personality = String::new();

    if let Some(start_index) = personality.find(start_delimiter) {
        if let Some(end_index) = personality.find(end_delimiter) {
            extracted_prompt = &personality[start_index + start_delimiter.len()..end_index];
            modified_personality = personality.replacen(
                &format!("{}{}{}", start_delimiter, extracted_prompt, end_delimiter),
                "",
                1,
            );
        }
    }

    let prompt;
    let prompt_temp;

    if modified_personality.len() > 0 && extracted_prompt.len() > 0 {
        prompt_temp = format!("これはあなたの人格です。'{personality}'\n{extracted_prompt}");
    } else {
        prompt_temp = format!("これはあなたの人格です。'{personality}'\nこの人格を演じて次の行の文章に対して{answer_length}文字程度で返信してください。ユーザーから文字数指定があった場合はそちらを優先してください。");
    }
    
    // コンテキストがある場合
    let user_input = if let Some(ctx) = context {
        if has_mention {
            // メンション: 会話履歴付き
            prompt = prompt_temp;
            ctx
        } else {
            // エアリプ: タイムライン付き
            prompt = format!("{prompt_temp}\n\n以下は最近のタイムラインです。この流れを見て、あなたが気になった投稿に自然に反応してください。あなた宛ではないので、独り言のように自然に反応してください。");
            ctx
        }
    } else {
        // コンテキストなし
        prompt = if has_mention {
            prompt_temp
        } else {
            format!("{prompt_temp}次の行の文章はSNSでの投稿です。あなたがたまたま見かけたものであなた宛の文章ではないのでその点に注意して回答してください。")
        };
        user_text.to_string()
    };

    // カテゴリを決定
    let category = if has_mention {
        "reply" // メンションへの返信
    } else {
        "air_reply" // エアリプ
    };
    
    match call_gpt_with_category(&prompt, &user_input, bot_pubkey, category).await {
        Ok(reply) => {
            println!("Reply: {}", reply);
            Ok(reply)
        },
        Err(e) => {
            eprintln!("Error calling GPT API: {:?}", e);
            eprintln!("Error details: {}", e);
            Ok("".to_string())
        },
    }
}

/// 旧インターフェース: 互換性のため残す
pub async fn get_reply<'a>(
    bot_pubkey: &'a str,
    personality: &'a str, 
    user_text: &'a str, 
    _has_mention: bool,
    timeline: Option<Vec<TimelinePost>>,
) -> Result<String, Box<dyn Error>> {
    dotenv().ok();
    
    // 回答長設定を取得
    let file = File::open("../config.yml")?;
    let config: AppConfig = serde_yaml::from_reader(file)?;
    let answer_length = config.get_i32_setting("gpt_answer_length");

    let start_delimiter = "<<";
    let end_delimiter = ">>";
    let mut extracted_prompt = "";
    let mut modified_personality = String::new();

    if let Some(start_index) = personality.find(start_delimiter) {
        if let Some(end_index) = personality.find(end_delimiter) {
            extracted_prompt = &personality[start_index + start_delimiter.len()..end_index];
            modified_personality = personality.replacen(
                &format!("{}{}{}", start_delimiter, extracted_prompt, end_delimiter),
                "",
                1,
            );
        }
    }

    let prompt;
    let prompt_temp;

    if modified_personality.len() > 0 && extracted_prompt.len() > 0 {
        prompt_temp = format!("これはあなたの人格です。'{personality}'\n{extracted_prompt}");
    } else {
        prompt_temp = format!("これはあなたの人格です。'{personality}'\nこの人格を演じて次の行の文章に対して{answer_length}文字程度で返信してください。ユーザーから文字数指定があった場合はそちらを優先してください。");
    }
    
    // タイムラインがある場合（エアリプ）
    // カテゴリを先に決定（moveの前に）
    let category = if timeline.is_some() {
        "air_reply"
    } else {
        "reply"
    };
    
    let user_input = if let Some(timeline_posts) = timeline {
        if !timeline_posts.is_empty() {
            // 既存のタイムラインをフォーマット
            let timeline_lines: Vec<String> = timeline_posts.iter()
                .enumerate()
                .map(|(i, post)| {
                    // 日本時間に変換
                    let dt = Local.timestamp_opt(post.timestamp, 0).unwrap();
                    let time_str = dt.format("%m/%d %H:%M").to_string();
                    
                    // 名前を取得（なければpubkeyの先頭8文字）
                    let display_name = post.name.clone().unwrap_or_else(|| {
                        if post.pubkey.len() > 8 {
                            format!("{}...", &post.pubkey[..8])
                        } else {
                            post.pubkey.clone()
                        }
                    });
                    
                    format!("{}. [{}] {}: {}", i + 1, time_str, display_name, post.content)
                })
                .collect();
            
            let timeline_text = timeline_lines.join("\n");
            
            prompt = format!("{prompt_temp}\n\n以下は最近のタイムラインです。この流れを見て、あなたが気になった投稿に自然に反応してください。あなた宛ではないので、独り言のように自然に反応してください。");
            
            // タイムラインのみをuser_inputに含める
            let user_input_text = format!("【タイムライン】\n{}", timeline_text);
            
            // デバッグ: エアリプ時のLLM入力内容をログ出力
            println!("=== Air-reply LLM Input ===");
            println!("Prompt:\n{}", prompt);
            println!("User input:\n{}", user_input_text);
            println!("===========================");
            
            user_input_text
        } else {
            prompt = format!("{prompt_temp}次の行の文章はSNSでの投稿です。あなたがたまたま見かけたものであなた宛の文章ではないのでその点に注意して回答してください。");
            user_text.to_string()
        }
    } else {
        // メンションの場合
        prompt = prompt_temp;
        user_text.to_string()
    };

    match call_gpt_with_category(&prompt, &user_input, bot_pubkey, category).await {
        Ok(reply) => {
            println!("Reply: {}", reply);
            Ok(reply)
        },
        Err(e) => {
            eprintln!("Error calling GPT API: {:?}", e);
            eprintln!("Error details: {}", e);
            Ok("".to_string())
        },
    }
}

/// ユーザーへの印象を含む返信を生成（メンション返信のみ）
pub async fn get_reply_with_impression<'a>(
    bot_pubkey: &'a str,
    user_pubkey: &'a str,
    personality: &'a str,
    user_text: &'a str,
    context: Option<String>,
) -> Result<GptResponseWithImpression, Box<dyn Error>> {
    dotenv().ok();
    
    // 設定を取得
    let file = File::open("../config.yml")?;
    let config: AppConfig = serde_yaml::from_reader(file)?;
    let answer_length = config.get_i32_setting("gpt_answer_length");
    let max_impression_length = config.get_usize_setting("max_impression_length");
    
    // DB接続
    let conn = db::connect()?;
    
    // 既存の印象を取得
    let existing_impression = db::get_user_impression(&conn, bot_pubkey, user_pubkey)?;
    let impression_context = if let Some(imp) = &existing_impression {
        format!("\n\n【このユーザーについてのあなたの印象】\n{}", imp)
    } else {
        String::new()
    };

    let start_delimiter = "<<";
    let end_delimiter = ">>";
    let mut extracted_prompt = "";
    let mut modified_personality = String::new();

    if let Some(start_index) = personality.find(start_delimiter) {
        if let Some(end_index) = personality.find(end_delimiter) {
            extracted_prompt = &personality[start_index + start_delimiter.len()..end_index];
            modified_personality = personality.replacen(
                &format!("{}{}{}", start_delimiter, extracted_prompt, end_delimiter),
                "",
                1,
            );
        }
    }

    let prompt_temp;
    if modified_personality.len() > 0 && extracted_prompt.len() > 0 {
        prompt_temp = format!("これはあなたの人格です。'{modified_personality}'\n{extracted_prompt}");
    } else {
        prompt_temp = format!("これはあなたの人格です。'{personality}'\nこの人格を演じて次の行の文章に対して{answer_length}文字程度で返信してください。ユーザーから文字数指定があった場合はそちらを優先してください。");
    }
    
    // 印象生成の指示を追加
    let prompt = format!(
        "{}{}\n\n返信と同時に、このユーザーへの印象を{}文字以内で更新してください。印象には会話の内容、ユーザーの性格や特徴、興味関心などを記録してください。\n\n以下のJSON形式で返答してください：\n{{\n  \"reply\": \"ユーザーへの返信文\",\n  \"impression\": \"このユーザーへの印象（{}文字以内）\"\n}}",
        prompt_temp,
        impression_context,
        max_impression_length,
        max_impression_length
    );
    
    let user_input = if let Some(ctx) = context {
        ctx
    } else {
        user_text.to_string()
    };

    // GPTを呼び出し（JSON mode使用）
    let category = "reply";
    match call_gpt_with_json_mode(&prompt, &user_input, bot_pubkey, category).await {
        Ok(response_text) => {
            // JSON形式のパース
            match serde_json::from_str::<GptResponseWithImpression>(&response_text) {
                Ok(parsed) => {
                    println!("Reply: {}", parsed.reply);
                    println!("Impression: {}", parsed.impression);
                    
                    // 印象をDBに保存
                    if let Err(e) = db::save_user_impression(&conn, bot_pubkey, user_pubkey, &parsed.impression) {
                        eprintln!("[Impression] 保存エラー: {}", e);
                    } else {
                        println!("[Impression] 保存成功");
                    }
                    
                    Ok(parsed)
                },
                Err(e) => {
                    eprintln!("[JSON Parse] エラー: {}", e);
                    eprintln!("[JSON Parse] 元の応答: {}", response_text);
                    
                    // JSONパースに失敗した場合はフォールバック
                    Ok(GptResponseWithImpression {
                        reply: response_text,
                        impression: String::new(),
                    })
                }
            }
        },
        Err(e) => {
            eprintln!("Error calling GPT API: {:?}", e);
            Err(e)
        }
    }
}
