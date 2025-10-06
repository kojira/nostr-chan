use crate::config::AppConfig;
use crate::TimelinePost;
use dotenv::dotenv;
use std::error::Error;
use std::fs::File;
use std::time::Duration;
use std::env;
use tokio::time::timeout;
use openai_api_rs::v1::api::OpenAIClient;
use openai_api_rs::v1::chat_completion::{self, ChatCompletionRequest};
use chrono::{Local, TimeZone};


pub async fn call_gpt(prompt: &str, user_text: &str) -> Result<String, Box<dyn Error>> {
    dotenv().ok();
    let api_key = env::var("OPEN_AI_API_KEY").expect("OPEN_AI_API_KEY is not set");
    let mut client = OpenAIClient::builder()
        .with_api_key(api_key)
        .build()?;
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

    let chat_completion_future = async {
        client.chat_completion(req).await
    };

    // タイムアウトを設定
    match timeout(Duration::from_secs(30), chat_completion_future).await {
        Ok(result) => match result {
            Ok(response) => {
                // 正常なレスポンスの処理
                match &response.choices[0].message.content {
                    Some(content) => Ok(content.to_string()),
                    None => Err("No content found in response".into()), // 適切なエラーメッセージを返す
                }            
            },
            Err(e) => Err(e.into()), // APIErrorをBox<dyn Error>に変換
        },
        Err(_) => Err("Timeout after 30 seconds".into()),
    }
}

pub async fn get_reply<'a>(
    personality: &'a str, 
    user_text: &'a str, 
    _has_mention: bool,
    timeline: Option<Vec<TimelinePost>>
) -> Result<String, Box<dyn Error>> {
    dotenv().ok();
    let file = File::open("../config.yml").unwrap();
    let config: AppConfig = serde_yaml::from_reader(file).unwrap();
    let answer_length = config.gpt.answer_length;

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
    let user_input = if let Some(timeline_posts) = timeline {
        if !timeline_posts.is_empty() {
            let timeline_text = timeline_posts.iter()
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
                .collect::<Vec<String>>()
                .join("\n");
            
            prompt = format!("{prompt_temp}\n\n以下は最近のタイムラインです。この流れ全体を見て、自然に反応してください。あなた宛ではないので、独り言のように自然に反応してください。");
            
            // タイムラインと最新投稿をuser_inputに含める
            let user_input_text = format!("【タイムライン】\n{}\n\n最新の投稿: {}", timeline_text, user_text);
            
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

    match call_gpt(&prompt, &user_input).await {
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
