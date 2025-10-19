use crate::config;
use crate::database as db;
use crate::gpt;
use crate::util;
use nostr_sdk::prelude::*;
use std::collections::HashMap;
use serde_json::Value;

// zapランキングコマンド
pub async fn zap_ranking(config: config::AppConfig, person: db::Person, event: Event) -> Result<()> {
    println!("zap_ranking");
    let pubkey = &event.pubkey.to_string();
    let text = &format!("「現在から過去1年分のzapを集計します。しばらくお待ち下さい。」をあなたらしく言い換えてください。元の文章に含まれる内容が欠落しないようにしてください。「」内に入る文字だけを返信してください。カギカッコは不要です。");
    let reply = gpt::get_reply(&person.pubkey, &person.prompt, text, true, None, &config).await.unwrap();
    let root_event: Event;
    if reply.len() > 0 {
        root_event = util::reply_to(&config, event.clone(), person.clone(), &reply).await?;
    } else {
        return Ok(());
    }
    
    let receive_zap_events = util::get_zap_received(pubkey).await?;
    let mut all_zap: u64 = 0;
    let mut zap_by_pubkey: HashMap<String, (u64, u64)> = HashMap::new(); // pubkeyごとの集計を保持するHashMap

    for zap_event in &receive_zap_events {
        let mut bolt11_string = "".to_string();
        let mut pubkey_str = "".to_string();
        for tag in zap_event.tags.iter() {
            if let Some(standardized) = tag.as_standardized() {
                match standardized {
                    TagStandard::Bolt11(bolt11) => {
                        bolt11_string = bolt11.to_string();
                    }
                    TagStandard::Description(description) => {
                        if let Ok(content_json) = serde_json::from_str::<Value>(&description) {
                            if let Some(pk) = content_json.get("pubkey").and_then(|k| k.as_str()) {
                                // descriptionの中のhex pubkeyをnpub形式に変換
                                if let Ok(key) = PublicKey::parse(pk) {
                                    pubkey_str = key.to_bech32().unwrap();
                                }
                            }
                        }
                    }
                    _ => {} // 他のタグは無視
                }
            }
        }
        let bolt11 = util::decode_bolt11_invoice(&bolt11_string);
        if let Ok(bolt11) = bolt11 {
            if let Some(raw_amount) = bolt11.amount_milli_satoshis() {
                all_zap += raw_amount;
                // pubkeyに基づいてraw_amountを集計
                let entry = zap_by_pubkey.entry(pubkey_str.to_string()).or_insert((0, 0));
                entry.0 += raw_amount; // zap合計を更新
                entry.1 += 1; // 回数
            }
        }
    }
    
    println!("Total raw amount: {:?}", all_zap);
    
    // HashMapからVecへ変換
    let mut zap_vec: Vec<(String, (u64, u64))> = zap_by_pubkey.into_iter().collect();
    // zap合計で降順ソート
    zap_vec.sort_by(|a, b| b.1.0.cmp(&a.1.0));
    
    let sender_ranking = zap_vec
        .iter()
        .take(10)
        .enumerate()
        .map(|(index, (pubkey, (zap, count)))| {
            format!("{}: {} Zap: {}, Count: {}", index + 1, pubkey, util::format_with_commas(zap / 1000), count)
        })
        .collect::<Vec<_>>()
        .join("\n");
    
    println!("Top 10 pubkeys by zap:");

    util::reply_to(
        &config,
        root_event,
        person.clone(),
        &format!(
            "受取りzap総額:{} satoshi\n投げてくれた人Top10:\n{}",
            util::format_with_commas(all_zap / 1000),
            sender_ranking
        ),
    )
    .await?;
    Ok(())
}
