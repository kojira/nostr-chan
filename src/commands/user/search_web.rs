use crate::config;
use crate::db;
use crate::gpt;
use crate::util;
use nostr_sdk::prelude::*;

// Web検索コマンド（Gemini CLI使用）
pub async fn search_web(config: config::AppConfig, person: db::Person, event: Event) -> Result<()> {
    let content_clone = event.content.clone();
    
    // メンション部分を除去（nostr:npub1..., nostr:note1..., @npub1... など）
    let cleaned_content = regex::Regex::new(r"(nostr:npub1\w+|nostr:note1\w+|@npub1\w+|npub1\w+)")
        .unwrap()
        .replace_all(&content_clone, "")
        .trim()
        .to_string();
    
    // 一次回答を生成（検索前に投稿）
    let user_input = format!(
        "# 質問内容\n{}\n\n# 指示\n上記の質問内容について「これから調べるので待ってて欲しい」という文章を50文字程度であなたらしく作成してください。あくまでこれから調べることに対する一次回答で、質問の回答ではないことに注意。返答のみを出力してください。",
        cleaned_content
    );
    let initial_reply = match gpt::call_gpt(&person.prompt, &user_input).await {
        Ok(reply) => reply,
        Err(e) => {
            eprintln!("Failed to generate initial reply: {}", e);
            "調べてみるね！".to_string()
        }
    };
    
    // 一次回答を投稿
    println!("Initial reply generated: {}", initial_reply);
    let initial_event = match util::reply_to(&config, event.clone(), person.clone(), &initial_reply).await {
        Ok(event) => {
            println!("✓ Initial reply posted successfully: {:?}", event.id);
            Some(event)
        },
        Err(e) => {
            eprintln!("✗ Failed to post initial reply: {}", e);
            eprintln!("Error details: {:?}", e);
            None
        }
    };
    
    if initial_event.is_none() {
        eprintln!("WARNING: Initial reply was not posted!");
    }
    
    // botの名前を取得
    let bot_name = if let Ok(content_json) = serde_json::from_str::<serde_json::Value>(&person.content) {
        content_json.get("display_name")
            .and_then(|v| v.as_str())
            .or_else(|| content_json.get("name").and_then(|v| v.as_str()))
            .unwrap_or("")
            .to_string()
    } else {
        String::new()
    };
    
    // LLMで検索ワードを生成（文脈を理解させる）
    let extract_prompt = if !bot_name.is_empty() {
        format!(
            "あなたはWeb検索の専門家です。以下の文章から、効率の良い検索キーワードを提案してください。\n\
            ・調べたい対象の名詞やトピックのみを抽出（3〜5単語程度）\n\
            ・「調べて」「説明して」などの動詞や、説明方法の指示は含めない\n\
            ・文中の「{}」はbotへの呼びかけなので、検索対象に含まれない限りキーワードに含めない\n\
            ・検索キーワードのみを返し、説明や前置きは不要",
            bot_name
        )
    } else {
        "あなたはWeb検索の専門家です。以下の文章から、効率の良い検索キーワードを提案してください。\n\
        ・調べたい対象の名詞やトピックのみを抽出（3〜5単語程度）\n\
        ・「調べて」「説明して」などの動詞や、説明方法の指示は含めない\n\
        ・検索キーワードのみを返し、説明や前置きは不要".to_string()
    };
    
    let search_keyword = match gpt::call_gpt(&extract_prompt, &cleaned_content).await {
        Ok(keyword) => keyword.trim().to_string(),
        Err(e) => {
            eprintln!("Failed to extract search keyword: {}", e);
            // フォールバック: 「調べて」以降のテキストを使用（メンション除去済み）
            if let Some(pos) = cleaned_content.find("調べて") {
                let after = &cleaned_content[pos + "調べて".len()..];
                after.trim().to_string()
            } else {
                "".to_string()
            }
        }
    };
    
    if search_keyword.is_empty() {
        util::reply_to(&config, event, person, "調べる内容を教えてください。").await?;
        return Ok(());
    }
    
    println!("Gemini search query: {}", search_keyword);
    match util::gemini_search(&search_keyword).await {
        Ok(search_result) => {
            // 検索結果を一次回答を踏まえて要約
            let summary_prompt = format!(
                "{}\n\nユーザーからの質問:「{}」\nあなたの一次回答:「{}」\n\n以下の検索結果を読んで、一次回答に続く形で{}文字程度であなたらしく要約して返答してください。返答のみを出力してください。説明や前置きは不要です。",
                person.prompt,
                cleaned_content,
                initial_reply,
                config.gpt.search_answer_length
            );
            let final_reply = match gpt::get_reply(&summary_prompt, &search_result, true, None).await {
                Ok(summary) => summary,
                Err(e) => {
                    eprintln!("Failed to summarize search result: {}", e);
                    // フォールバック: 検索結果をそのまま返す（文字数制限）
                    let max_len = config.gpt.search_answer_length as usize;
                    if search_result.len() > max_len {
                        format!("{}...", &search_result[..max_len])
                    } else {
                        search_result
                    }
                }
            };
            
            // 最終回答を一次回答へのリプライとして投稿
            if let Some(initial_evt) = initial_event {
                util::reply_to(
                    &config,
                    initial_evt,
                    person.clone(),
                    &final_reply,
                ).await?;
            }
        }
        Err(e) => {
            eprintln!("Gemini search error: {}", e);
            let error_reply = format!("検索に失敗しました: {}", e);
            // エラーも一次回答へのリプライとして投稿
            if let Some(initial_evt) = initial_event {
                util::reply_to(&config, initial_evt, person, &error_reply).await?;
            } else {
                util::reply_to(&config, event, person, &error_reply).await?;
            }
        }
    }
    
    Ok(())
}
