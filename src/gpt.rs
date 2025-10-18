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
use openai_api_rs::v1::chat_completion::{self, chat_completion::ChatCompletionRequest};
use chrono::{Local, TimeZone};
use tiktoken_rs::o200k_base;
use serde::{Deserialize, Serialize};

/// GPTの応答（返信＋印象）
#[derive(Debug, Serialize, Deserialize)]
pub struct GptResponseWithImpression {
    pub reply: String,
    pub impression: String,
}

/// GPTの応答（返信＋印象＋心境）
#[derive(Debug, Serialize, Deserialize)]
pub struct GptResponseWithMentalDiary {
    pub reply: String,
    pub user_attributes: db::UserAttributes,
    pub mental_diary: db::MentalDiary,
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
    
    let mut req = ChatCompletionRequest::new(
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
    
    // JSON modeを有効化（serde_json::Valueとして指定）
    req.response_format = Some(serde_json::json!({
        "type": "json_object"
    }));
    
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

/// エアリプ時の心境付き返信を生成（印象なし、心境のみ）
pub async fn get_air_reply_with_mental_diary<'a>(
    bot_pubkey: &'a str,
    personality: &'a str,
    user_text: &'a str,
    has_mention: bool,
    context: Option<String>,
) -> Result<String, Box<dyn Error>> {
    // エアリプ用の追加指示
    let air_reply_instruction = if !has_mention {
        "\n\n以下は最近のタイムラインです。この流れを見て、あなたが気になった投稿に自然に反応してください。\
         あなた宛ではないので、独り言のように自然に反応してください。"
    } else {
        ""
    };
    
    let response = call_gpt_with_mental_diary_internal(
        bot_pubkey,
        None, // user_pubkey なし（エアリプなので印象不要）
        personality,
        user_text,
        context,
        Some(air_reply_instruction),
        "air_reply",
        None, // user_name なし（エアリプなので不要）
    ).await?;
    
    Ok(response.reply)
}

/// 心境・印象付きプロンプトを構築する共通関数
async fn build_mental_diary_prompt<'a>(
    bot_pubkey: &'a str,
    user_pubkey: Option<&'a str>, // Noneの場合は印象を含めない
    personality: &'a str,
    _user_text: &'a str,
    additional_instruction: Option<&'a str>,
    user_name: Option<&'a str>,
) -> Result<(String, rusqlite::Connection), Box<dyn Error>> {
    dotenv().ok();
    
    // 設定を取得
    let file = File::open("../config.yml")?;
    let config: AppConfig = serde_yaml::from_reader(file)?;
    let answer_length = config.get_i32_setting("gpt_answer_length");
    let _max_impression_length = config.get_usize_setting("max_impression_length");
    
    // DB接続
    let conn = db::connect()?;
    
    // 既存のユーザー属性を取得（user_pubkeyがある場合のみ）
    let user_attributes_context = if let Some(upk) = user_pubkey {
        if let Some(attrs) = db::get_user_attributes(&conn, bot_pubkey, upk)? {
            let yaml_str = attrs.to_yaml_string();
            if !yaml_str.is_empty() && yaml_str != "{}" {
                format!("\n\n【このユーザーについて分かっていること】\n{}", yaml_str)
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    
    // 既存の心境を取得
    let mental_state_context = if let Some(mental) = db::get_bot_mental_state(&conn, bot_pubkey)? {
        let yaml_str = mental.to_yaml_string();
        if !yaml_str.is_empty() {
            format!("\n\n【あなたの現在の心境】\n{}", yaml_str)
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    // パーソナリティのパース
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

    // ユーザー名セクション
    let user_name_section = if let Some(name) = user_name {
        format!("\n# 対話相手\n話しかけてきた相手の名前は「{}」です。\n", name)
    } else {
        String::new()
    };
    
    // ベースプロンプトの構築
    let base_prompt = if modified_personality.len() > 0 && extracted_prompt.len() > 0 {
        format!(
            "これはあなたの人格です。'{modified_personality}'{user_name_section}\n{extracted_prompt}"
        )
    } else {
        format!(
            "これはあなたの人格です。'{personality}'{user_name_section}\n\
             この人格を演じて次の行の文章に対して{answer_length}文字程度で返信してください。\n\
             ユーザーから文字数指定があった場合はそちらを優先してください。"
        )
    };
    
    let additional_inst = additional_instruction.unwrap_or("");
    
    // mental_diaryのJSON構造（共通）
    let mental_diary_json = "\
      \"mental_diary\": {\n    \
        \"mood\": \"現在の気分\",\n    \
        \"favorite_people\": [\"好きな人1\", \"好きな人2\"],\n    \
        \"disliked_people\": [],\n    \
        \"trusted_people\": [],\n    \
        \"current_interests\": [\"興味1\", \"興味2\"],\n    \
        \"want_to_learn\": [],\n    \
        \"bored_with\": [],\n    \
        \"short_term_goals\": \"短期目標\",\n    \
        \"long_term_goals\": \"長期目標\",\n    \
        \"concerns\": \"悩み\",\n    \
        \"recent_happy_events\": \"嬉しかったこと\",\n    \
        \"recent_sad_events\": \"悲しかったこと\",\n    \
        \"recent_surprises\": \"驚いたこと\",\n    \
        \"self_changes\": \"自分の変化\",\n    \
        \"personality_state\": \"人格の状態\"\n  \
      }";
    
    // ユーザー属性のJSON構造（客観的な特徴のみ）
    let user_attributes_json = "\
      \"user_attributes\": {\n    \
        \"nickname\": \"わからない\",\n    \
        \"age\": \"30歳くらい？\",\n    \
        \"gender\": \"男性かな？\",\n    \
        \"personality\": \"せっかちで行動的\",\n    \
        \"likes\": [\"技術\", \"アニメ\"],\n    \
        \"dislikes\": [\"早起き\", \"待つこと\"],\n    \
        \"family\": \"わからない\",\n    \
        \"catchphrase\": \"〜だぜ\",\n    \
        \"current_boom\": \"Nostr開発\",\n    \
        \"occupation\": \"エンジニアっぽい\",\n    \
        \"country\": \"日本\",\n    \
        \"hobbies\": [\"プログラミング\", \"深夜作業\"],\n    \
        \"values\": \"効率と自由を重視している様子\",\n    \
        \"recent_events\": \"新しいプロジェクトを始めた\",\n    \
        \"conversation_style\": \"カジュアルで短文、絵文字少なめ\",\n    \
        \"nostr_experience\": \"開発者レベルで詳しい\",\n    \
        \"frequent_topics\": [\"技術\", \"Nostr\", \"開発\"],\n    \
        \"impression\": \"技術志向の夜型エンジニア\"\n  \
      }";
    
    // JSON出力形式とフィールド説明（ユーザー属性の有無で分岐）
    let (json_fields, field_descriptions) = if user_pubkey.is_some() {
        (
            format!(
                "  \"reply\": \"ユーザーへの返信文\",\n  \
                   {user_attributes_json},\n  \
                   {mental_diary_json}"
            ),
            "- **reply**: ユーザーへの返信\n\
             - **user_attributes**: **このユーザーについて**のメモ（感想や印象も含めて自由に記録してOK）\n\
               * **推測元**: 会話履歴や最新の発言から積極的に推測してください\n\
               * **重要**: あなた自身の趣味・目標・活動ではなく、ユーザーの特徴を記録してください\n\
               * 記録すべきもの: ユーザーの口調、年齢、性別、職業、趣味、話題、性格、印象、感想など\n\
               * 例: 口調が若々しい→age: \"10代-20代\", 技術的な話題が多い→occupation: \"エンジニア系\", frequent_topics: [\"技術\", \"プログラミング\"]\n\
               * 例: 「〜だぜ」「〜だな」→gender: \"男性\", catchphrase: \"〜だぜ\"\n\
               * 例: アニメの話題→hobbies: [\"アニメ\"], current_boom: \"〇〇（アニメ名）\"\n\
               * personality: ユーザーの性格（「温かい」「配慮深い」「せっかち」「応援好き」など）\n\
               * impression: ユーザーへの印象や感想（「応援してくれる人」「技術好きで気さくな人」など）\n\
               * **重要**: nullや空配列は禁止です。不確実な場合は「わからない」「男性かな？」「30歳くらい？」「エンジニアっぽい」のように推測や不確実性を含めた自然な表現で記入してください\n\
               * 完全に情報がない項目は「わからない」または「不明」と記入してください\n\
             - **mental_diary**: あなた自身の心境を日記のように記録".to_string()
        )
    } else {
        (
            format!(
                "  \"reply\": \"返信文\",\n  \
                   {mental_diary_json}"
            ),
            "- **reply**: 返信文\n\
             - **mental_diary**: あなた自身の心境を日記のように記録".to_string()
        )
    };
    
    // システムプロンプト全体の構築（完全共通化）
    let system_prompt = format!(
        "# あなたの役割\n\
         {base_prompt}{additional_inst}\
         {user_attributes_context}\
         {mental_state_context}\n\n\
         # 出力形式\n\
         重要: あなたは必ずJSON形式で応答してください。他の形式は一切使用しないでください。\n\n\
         ```json\n\
         {{\n\
         {json_fields}\n\
         }}\n\
         ```\n\n\
         {field_descriptions}"
    );
    
    Ok((system_prompt, conn))
}

/// ユーザーへの印象と心境を含む返信を生成（メンション返信のみ）
pub async fn get_reply_with_mental_diary<'a>(
    bot_pubkey: &'a str,
    user_pubkey: &'a str,
    personality: &'a str,
    user_text: &'a str,
    context: Option<String>,
    user_name: Option<&'a str>,
) -> Result<GptResponseWithMentalDiary, Box<dyn Error>> {
    call_gpt_with_mental_diary_internal(
        bot_pubkey,
        Some(user_pubkey), // user_pubkey あり（メンション返信なので印象必要）
        personality,
        user_text,
        context,
        None, // 追加指示なし
        "reply",
        user_name,
    ).await
}

/// 心境・印象付き返信の内部共通関数
async fn call_gpt_with_mental_diary_internal<'a>(
    bot_pubkey: &'a str,
    user_pubkey: Option<&'a str>,
    personality: &'a str,
    user_text: &'a str,
    context: Option<String>,
    additional_instruction: Option<&'a str>,
    category: &'a str,
    user_name: Option<&'a str>,
) -> Result<GptResponseWithMentalDiary, Box<dyn Error>> {
    // 共通のプロンプト構築関数を使用
    let (system_prompt, conn) = build_mental_diary_prompt(
        bot_pubkey,
        user_pubkey,
        personality,
        user_text,
        additional_instruction,
        user_name,
    ).await?;
    
    let user_input = if let Some(ctx) = context {
        ctx
    } else {
        user_text.to_string()
    };

    // GPTを呼び出し（JSON mode使用）
    match call_gpt_with_json_mode(&system_prompt, &user_input, bot_pubkey, category).await {
        Ok(response_text) => {
            // user_pubkeyがある場合は印象あり、ない場合は印象なし
            if user_pubkey.is_some() {
                // ユーザー属性ありのパース
                match serde_json::from_str::<GptResponseWithMentalDiary>(&response_text) {
                    Ok(parsed) => {
                        // ユーザー属性をJSONとしてDBに保存
                        match parsed.user_attributes.to_json() {
                            Ok(json_str) => {
                                if let Err(e) = db::save_user_impression(&conn, bot_pubkey, user_pubkey.unwrap(), &json_str) {
                                    eprintln!("[UserAttributes] 保存エラー: {}", e);
                                }
                            }
                            Err(e) => {
                                eprintln!("[UserAttributes] JSON変換エラー: {}", e);
                            }
                        }
                        
                        // 心境をDBに保存
                        if let Err(e) = db::save_bot_mental_state(&conn, bot_pubkey, &parsed.mental_diary) {
                            eprintln!("[MentalDiary] 保存エラー: {}", e);
                        }
                        
                        Ok(parsed)
                    },
                    Err(e) => {
                        eprintln!("[JSON Parse] エラー: {}", e);
                        eprintln!("[JSON Parse] 元の応答: {}", response_text);
                        Err(format!("JSONパースエラー: {} (応答: {})", e, response_text).into())
                    }
                }
            } else {
                // 印象なしのパース（エアリプ用）
                #[derive(Debug, serde::Deserialize)]
                struct AirReplyResponse {
                    reply: String,
                    mental_diary: db::MentalDiary,
                }
                
                match serde_json::from_str::<AirReplyResponse>(&response_text) {
                    Ok(parsed) => {
                        // 心境をDBに保存
                        if let Err(e) = db::save_bot_mental_state(&conn, bot_pubkey, &parsed.mental_diary) {
                            eprintln!("[MentalDiary] 保存エラー: {}", e);
                        }
                        
                        // 印象なしのレスポンスをユーザー属性ありの形式に変換
                        Ok(GptResponseWithMentalDiary {
                            reply: parsed.reply,
                            user_attributes: db::UserAttributes::empty(),
                            mental_diary: parsed.mental_diary,
                        })
                    },
                    Err(e) => {
                        eprintln!("[JSON Parse] エラー: {}", e);
                        eprintln!("[JSON Parse] 元の応答: {}", response_text);
                        Err(format!("JSONパースエラー: {} (応答: {})", e, response_text).into())
                    }
                }
            }
        },
        Err(e) => {
            eprintln!("[GPT API] エラー: {:?}", e);
            Err(e)
        }
    }
}
