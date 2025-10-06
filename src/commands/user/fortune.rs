use crate::config;
use crate::db;
use crate::gpt;
use crate::util;
use nostr_sdk::prelude::*;

// 占いコマンド
pub async fn fortune(config: config::AppConfig, person: db::Person, event: Event) -> Result<()> {
    let prompt = "あなたは占い師です。今日の運勢を占ってください。";
    let user_text = "今日の運勢を教えて";
    let reply = gpt::get_reply(prompt, user_text, true, None).await?;
    util::reply_to(&config, event, person, &reply).await?;
    Ok(())
}
