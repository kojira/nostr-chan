use crate::config::AppConfig;
use chat_gpt_rs::prelude::*;
use dotenv::dotenv;
use std::fs::File;
use std::time::Duration;
use std::{env, thread};
use tokio::time::timeout;

pub async fn get_reply<'a>(personality: &'a str, user_text: &'a str, has_mention: bool) -> Result<String> {
    dotenv().ok();
    let file = File::open("../config.yml").unwrap();
    let config: AppConfig = serde_yaml::from_reader(file).unwrap();
    let answer_length = config.gpt.answer_length;
    let api_key = env::var("OPEN_AI_API_KEY").expect("OPEN_AI_API_KEY is not set");

    let token = Token::new(&api_key);
    let api = Api::new(token);

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
        prompt_temp = format!("これはあなたの人格です。'{personality}'\nこの人格を演じて次の行の文章に対して{answer_length}文字程度で返信してください。");
    }
    if !has_mention {
        prompt = format!("{prompt_temp}次の行の文章はSNSでの投稿です。あなたがたまたま見かけたものであなた宛の文章ではないのでその点に注意して解凍してください。")
    } else {
        prompt = prompt_temp
    }

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

use regex::Regex;

fn split_text(text: &str, max_length: usize) -> Vec<String> {
    let re = Regex::new(r"[^。\n]*[。\n]").unwrap();
    let sentences: Vec<&str> = re.find_iter(text).map(|m| m.as_str()).collect();

    let mut result = Vec::new();
    let mut current = String::new();
    for sentence in sentences {
        if current.len() + sentence.len() > max_length {
            result.push(current.trim().to_string());
            current.clear();
        }
        current += sentence;
    }
    if !current.is_empty() {
        result.push(current.trim().to_string());
    }
    result
}

pub async fn get_summary<'a>(text: &'a str) -> Result<String> {
    dotenv().ok();
    let api_key = env::var("OPEN_AI_API_KEY").expect("OPEN_AI_API_KEY is not set");

    let token = Token::new(&api_key);
    let api = Api::new(token);

    let prompt = format!(
        "あなたは優秀な新聞記者のお嬢様です。次の文章を読んで要約しお嬢様のような口調で日本語で10行にまとめてください。行頭には必ず'・'を入れて行末には必ず改行を入れてください。"
    );

    let mut summary = String::from("");
    let split_texts = split_text(text, 2048);
    for _text in split_texts {
        loop {
            let request = Request {
                model: Model::Gpt35Turbo,
                messages: vec![
                    Message {
                        role: "system".to_string(),
                        content: prompt.clone(),
                    },
                    Message {
                        role: "user".to_string(),
                        content: _text.to_string(),
                    },
                ],
                presence_penalty: Some(-0.5),
                frequency_penalty: Some(0.0),
                top_p: Some(0.9),

                ..Default::default()
            };
            let result = timeout(Duration::from_secs(30), api.chat(request)).await;
            match result {
                Ok(response) => {
                    // 非同期処理が完了した場合の処理
                    let _summary = response.unwrap().choices[0].message.content.clone();
                    summary = format!("{}{}", summary, _summary);
                    println!("summary:{}:{}", summary.len(), summary);
                    break;
                }
                Err(_) => {
                    eprintln!("**********Timeout occurred while calling api.chat");
                    thread::sleep(Duration::from_secs(3));
                }
            }
        }
    }
    summary = summary.replace("。・", "\n・");
    Ok(summary)
}
