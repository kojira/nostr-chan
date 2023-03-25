use crate::config::AppConfig;
use chat_gpt_rs::prelude::*;
use dotenv::dotenv;
use std::env;
use std::fs::File;
use std::time::Duration;
use tokio::time::timeout;

pub async fn get_reply<'a>(personality: &'a str, user_text: &'a str) -> Result<String> {
    dotenv().ok();
    let file = File::open("../config.yml").unwrap();
    let config: AppConfig = serde_yaml::from_reader(file).unwrap();
    let answer_length = config.gpt.answer_length;
    let api_key = env::var("OPEN_AI_API_KEY").expect("OPEN_AI_API_KEY is not set");

    let token = Token::new(&api_key);
    let api = Api::new(token);

    let prompt = format!("これはあなたの人格です。'{personality}'\nこの人格を演じて次の文章に対して{answer_length}文字程度で返信してください。");

    let request = Request {
        model: Model::Gpt35Turbo,
        messages: vec![
            Message {
                role: "system".to_string(),
                content: prompt,
            },
            Message {
                role: "user".to_string(),
                content: user_text.to_string(),
            },
        ],
        presence_penalty: Some(-0.5),
        frequency_penalty: Some(0.0),
        top_p: Some(0.9),

        ..Default::default()
    };
    // let response = api.chat(request).await?;
    let reply;
    let result = timeout(Duration::from_secs(30), api.chat(request)).await;
    match result {
        Ok(response) => {
            // 非同期処理が完了した場合の処理
            reply = response.unwrap().choices[0].message.content.clone();
            println!("{:?}", reply);
            Ok(reply)
        }
        Err(_) => {
            eprintln!("**********Timeout occurred while calling api.chat");
            reply = "".to_string();
            Ok(reply)
        }
    }
}
