use std::collections::HashMap;
use std::str::FromStr;

use crate::config;
use crate::db;
use crate::gpt;
use crate::util;
use crate::util::write_events_to_csv;
use nostr_sdk::prelude::*;
use rusqlite::Connection;

pub async fn command_handler(
  config: &config::AppConfig,
  conn: &Connection,
  persons: &Vec<db::Person>,
  event: &Event,
) -> Result<bool> {
  let admin_pubkeys = &config.bot.admin_pubkeys;
  let mut handled: bool = false;
  let persons_ = persons.clone();
  let person_op = util::extract_mention(persons_, &event).unwrap();
  if person_op.is_some() {
    let person = person_op.unwrap();
    if event.content.contains("占って") {
      fortune(config, &person, event).await?;
      handled = true;
    } else if event.content.contains("silent") {
    } else if event.content.contains("zap ranking") {
      zap_ranking(config, &person, event).await?;
      handled = true;
    } else {
      let is_admin = admin_pubkeys.iter().any(|s| *s == event.pubkey.to_string());
      if is_admin {
        let lines: Vec<String> =
          event.content.lines().map(|line| line.to_string()).collect();
        if lines[0].contains("new") {
          admin_new(config, conn, event, lines).await?;
          handled = true;
        } else if lines[0].contains("get kind 0") {
          admin_get_kind0(config, conn, &person, event).await?;
          handled = true;
        } else if lines[0].contains("update kind 0") {
          admin_update_kind0(config, conn, &person, event, lines).await?;
          handled = true;
        } else if lines[0].contains("broadcast kind 0") {
          admin_broadcast_kind0(config, &person, event).await?;
          handled = true;
        }
      }
    }
  }
  Ok(handled)
}

async fn fortune(config: &config::AppConfig, person: &db::Person, event: &Event) -> Result<()> {
  let text = &format!("今日のわたしの運勢を占って。結果はランダムで決めて、その結果に従って占いの内容を運の良さは★マークを５段階でラッキーアイテム、ラッキーカラーとかも教えて。\n{}",event.content);
  let reply = gpt::get_reply(&person.prompt, text, true).await.unwrap();
  if reply.len() > 0 {
      util::reply_to(config, event.clone(), person.clone(), &reply).await?;
  }
  Ok(())
}

async fn zap_ranking(config: &config::AppConfig, person: &db::Person, event: &Event) -> Result<()> {
  println!("zap_ranking");
  let pubkey = &event.pubkey.to_string();
  let text = &format!("「現在から過去1年分のzapを集計します。しばらくお待ち下さい。」をあなたらしく言い換えてください。元の文章に含まれる内容が欠落しないようにしてください。「」内に入る文字だけを返信してください。カギカッコは不要です。");
  let reply = gpt::get_reply(&person.prompt, text, true).await.unwrap();
  let root_event: Event;
  if reply.len() > 0 {
    root_event = util::reply_to(config, event.clone(), person.clone(), &reply).await?;
  } else {
    return Ok(());
  }
  let receive_zap_events = util::get_zap_received(pubkey).await?;
  let mut all_zap: u64 = 0;
  let mut zap_by_pubkey: HashMap<String, (u64, u64)> = HashMap::new(); // pubkeyごとの集計を保持するHashMap

  for zap_event in &receive_zap_events {
    let mut bolt11_string = "".to_string();
    let mut pubkey_str = "".to_string();
    for tag in &zap_event.tags {
      match tag {
        Tag::Bolt11(_bolt11_string) => {
          bolt11_string = _bolt11_string.to_string();
        }
        Tag::Description(description) => {
          if let Ok(content_json) = serde_json::from_str::<Value>(description) {
            if let Some(pubkey) = content_json.get("pubkey").and_then(|k| k.as_str()) {
              // descriptionの中のhex pubkeyをnpub形式に変換
              if let Ok(key) = PublicKey::parse(pubkey) {
                pubkey_str = key.to_bech32().unwrap();
              }
            }
          }
        }
        _ => {} // 他のタグは無視
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
  // let _ = util::write_events_to_csv(receive_zap_events).await;
  // HashMapからVecへ変換
  let mut zap_vec: Vec<(String, (u64, u64))> = zap_by_pubkey.into_iter().collect();
  // zap合計で降順ソート
  zap_vec.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));
  // let _ = util::write_to_csv(&zap_vec);
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
  let root_event2 = root_event.clone();

  util::reply_to_by_event_id_pubkey(
      &config,
      root_event2.id,
      event.pubkey,
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

async fn admin_new(
  config: &config::AppConfig,
  conn: &Connection,
  event: &Event,
  lines: Vec<String>,
) -> Result<()> {
  let keys = Keys::generate();
  let prompt = &lines[1];
  let content = &lines[2];
  db::insert_person(conn, &keys, &prompt, &content)?;
  let new_person = db::get_person(conn, &keys.public_key().to_string()).unwrap();
  util::send_kind0(&new_person.secretkey.to_string(), content).await?;
  let content: Value = serde_json::from_str(content)?;
  let display_name =
    &content["display_name"].to_string()[1..content["display_name"].to_string().len() - 1];
  util::reply_to(
      &config,
      event.clone(),
      new_person,
      &format!("{}です。コンゴトモヨロシク！", display_name),
  )
  .await?;
  Ok(())
}

async fn admin_get_kind0(
  config: &config::AppConfig,
  conn: &Connection,
  person: &db::Person,
  event: &Event,
) -> Result<()> {
  println!("get kind 0");
  let _meta_event = util::get_kind0(&person.pubkey, &person.secretkey).await?;
  db::update_person_content(conn, &person.pubkey, &_meta_event.content.to_string())?;
  util::reply_to(
    &config,
    event.clone(),
    person.clone(),
    &format!("リレーからkind 0を取得してデータベース情報を更新しました"),
  )
  .await?;
  Ok(())
}

async fn admin_update_kind0(
  config: &config::AppConfig,
  conn: &Connection,
  person: &db::Person,
  event: &Event,
  lines: Vec<String>,
) -> Result<()> {
  println!("update kind 0");
  db::update_person_content(conn, &person.pubkey, &lines[1])?;
  util::send_kind0(&person.secretkey.to_string(), &lines[1]).await?;
  util::reply_to(
      &config,
      event.clone(),
      person.clone(),
      &format!("データベースのkind 0を更新してブロードキャストしました"),
  )
  .await?;
  Ok(())
}

async fn admin_broadcast_kind0(
  config: &config::AppConfig,
  person: &db::Person,
  event: &Event,
) -> Result<()> {
  println!("broadcast kind 0");
  util::send_kind0(&person.secretkey.to_string(), &person.content.to_string()).await?;
  util::reply_to(
      &config,
      event.clone(),
      person.clone(),
      &format!("データベースのkind 0の情報をブロードキャストしました"),
  )
  .await?;
  Ok(())
}
