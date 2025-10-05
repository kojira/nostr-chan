use crate::config::AppConfig;
use dotenv::dotenv;
use std::error::Error;
use std::fs::File;
use std::time::Duration;
use std::env;
use tokio::time::timeout;
use openai_api_rs::v1::api::OpenAIClient;
use openai_api_rs::v1::chat_completion::{self, ChatCompletionRequest, Reasoning, ReasoningEffort};


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

pub async fn get_reply<'a>(personality: &'a str, user_text: &'a str, has_mention: bool) -> Result<String, Box<dyn Error>> {
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
    if !has_mention {
        prompt = format!("{prompt_temp}次の行の文章はSNSでの投稿です。あなたがたまたま見かけたものであなた宛の文章ではないのでその点に注意して回答してください。")
    } else {
        prompt = prompt_temp
    }

    match call_gpt(&prompt, &user_text.to_string()).await {
        Ok(reply) => {
            println!("Reply: {}", reply);
            Ok(reply)
        },
        Err(e) => {
            println!("Error: {}", e);
            Ok("".to_string())
        },
    }
}
