use crate::config;
use crate::db;
use crate::gpt;
use crate::util;
use nostr_sdk::prelude::*;

// 占いコマンド
pub async fn fortune(config: config::AppConfig, person: db::Person, event: Event) -> Result<()> {
    let user_text = format!(
        "今日のわたしの運勢を占って。結果はランダムで決めて、その結果に従って占いの内容を運の良さは★マークを５段階でラッキーアイテム、ラッキーカラーとかも教えて。\n{}",
        event.content
    );
    let reply = gpt::get_reply(&person.prompt, &user_text, true, None).await?;
    util::reply_to(&config, event, person, &reply).await?;
    Ok(())
}
