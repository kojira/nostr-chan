use std::collections::HashMap;
use std::future::Future;

use crate::config;
use crate::db;
use crate::gpt;
use crate::util;
use nostr_sdk::prelude::*;
use rusqlite::Connection;

// ヘルパー関数: コマンドを非同期実行
fn spawn_command<F>(future: F, error_msg: String)
where
  F: Future<Output = Result<()>> + Send + 'static,
{
  tokio::spawn(async move {
    if let Err(e) = future.await {
      eprintln!("{}: {}", error_msg, e);
    }
  });
}

// コマンド定義
struct UserCommand {
  name: &'static str,
  patterns: Vec<&'static str>,
  description: &'static str,
  handler: fn(config::AppConfig, db::Person, Event) -> std::pin::Pin<Box<dyn Future<Output = Result<()>> + Send>>,
}

struct AdminCommand {
  name: &'static str,
  pattern: &'static str,
  description: &'static str,
  handler: fn(config::AppConfig, db::Person, Event, Vec<String>) -> std::pin::Pin<Box<dyn Future<Output = Result<()>> + Send>>,
}

struct AdminCommandSimple {
  name: &'static str,
  pattern: &'static str,
  description: &'static str,
  handler: fn(config::AppConfig, Event, Vec<String>) -> std::pin::Pin<Box<dyn Future<Output = Result<()>> + Send>>,
}

// ユーザーコマンドテーブル
fn get_user_commands() -> Vec<UserCommand> {
  vec![
    UserCommand {
      name: "fortune",
      patterns: vec!["占って"],
      description: "今日の運勢を占います",
      handler: |c, p, e| Box::pin(fortune(c, p, e)),
    },
    UserCommand {
      name: "zap_ranking",
      patterns: vec!["zap ranking"],
      description: "過去1年分のzapランキングを表示します",
      handler: |c, p, e| Box::pin(zap_ranking(c, p, e)),
    },
    UserCommand {
      name: "update_follower",
      patterns: vec!["update follower", "フォロワー更新"],
      description: "自分のフォロワーキャッシュを更新します",
      handler: |c, p, e| Box::pin(update_my_follower_cache(c, p, e)),
    },
    UserCommand {
      name: "help",
      patterns: vec!["help", "ヘルプ"],
      description: "利用可能なコマンド一覧を表示します",
      handler: |c, p, e| Box::pin(show_help(c, p, e)),
    },
  ]
}

// 管理者コマンドテーブル（person必要）
fn get_admin_commands() -> Vec<AdminCommand> {
  vec![
    AdminCommand {
      name: "get_kind0",
      pattern: "get kind 0",
      description: "リレーからkind 0を取得してDBを更新",
      handler: |c, p, e, _| Box::pin(admin_get_kind0(c, p, e)),
    },
    AdminCommand {
      name: "update_kind0",
      pattern: "update kind 0",
      description: "DBのkind 0を更新してブロードキャスト",
      handler: |c, p, e, l| Box::pin(admin_update_kind0(c, p, e, l)),
    },
    AdminCommand {
      name: "broadcast_kind0",
      pattern: "broadcast kind 0",
      description: "DBのkind 0をブロードキャスト",
      handler: |c, p, e, _| Box::pin(admin_broadcast_kind0(c, p, e)),
    },
    AdminCommand {
      name: "clear_follower_cache",
      pattern: "clear follower cache",
      description: "全フォロワーキャッシュをクリア",
      handler: |c, p, e, _| Box::pin(admin_clear_follower_cache(c, p, e)),
    },
  ]
}

// 管理者コマンドテーブル（person不要）
fn get_admin_commands_simple() -> Vec<AdminCommandSimple> {
  vec![
    AdminCommandSimple {
      name: "new",
      pattern: "new",
      description: "新しいボットを作成",
      handler: |c, e, l| Box::pin(admin_new(c, e, l)),
    },
  ]
}

pub async fn command_handler(
  config: &config::AppConfig,
  _conn: &Connection,
  persons: &Vec<db::Person>,
  event: &Event,
) -> Result<bool> {
  let admin_pubkeys = &config.bot.admin_pubkeys;
  let persons_ = persons.clone();
  let person_op = util::extract_mention(persons_, &event).unwrap();
  
  if person_op.is_none() {
    return Ok(false);
  }
  
  let person = person_op.unwrap();
  
  // silent コマンドは何もしない
  if event.content.contains("silent") {
    return Ok(false);
  }
  
  // ユーザーコマンドをチェック
  for cmd in get_user_commands() {
    if cmd.patterns.iter().any(|p| event.content.contains(p)) {
      spawn_command(
        (cmd.handler)(config.clone(), person.clone(), event.clone()),
        format!("{} error", cmd.name)
      );
      return Ok(true);
    }
  }
  
  // 管理者コマンドをチェック
  let is_admin = admin_pubkeys.iter().any(|s| *s == event.pubkey.to_string());
  if !is_admin {
    return Ok(false);
  }
  
  let lines: Vec<String> = event.content.lines().map(|line| line.to_string()).collect();
  if lines.is_empty() {
    return Ok(false);
  }
  
  // 管理者コマンド（person不要）
  for cmd in get_admin_commands_simple() {
    if lines[0].contains(cmd.pattern) {
      spawn_command(
        (cmd.handler)(config.clone(), event.clone(), lines.clone()),
        format!("{} error", cmd.name)
      );
      return Ok(true);
    }
  }
  
  // 管理者コマンド（person必要）
  for cmd in get_admin_commands() {
    if lines[0].contains(cmd.pattern) {
      spawn_command(
        (cmd.handler)(config.clone(), person.clone(), event.clone(), lines.clone()),
        format!("{} error", cmd.name)
      );
      return Ok(true);
    }
  }
  
  Ok(false)
}

async fn show_help(config: config::AppConfig, person: db::Person, event: Event) -> Result<()> {
  let mut help_text = String::from("【利用可能なコマンド】\n\n");
  
  // ユーザーコマンド
  help_text.push_str("■ ユーザーコマンド\n");
  for cmd in get_user_commands() {
    let patterns = cmd.patterns.join(" / ");
    help_text.push_str(&format!("・{}\n  {}\n\n", patterns, cmd.description));
  }
  
  // 管理者コマンド（管理者のみ表示）
  let admin_pubkeys = &config.bot.admin_pubkeys;
  let is_admin = admin_pubkeys.iter().any(|s| *s == event.pubkey.to_string());
  
  if is_admin {
    help_text.push_str("\n■ 管理者コマンド\n");
    
    for cmd in get_admin_commands_simple() {
      help_text.push_str(&format!("・{}\n  {}\n\n", cmd.pattern, cmd.description));
    }
    
    for cmd in get_admin_commands() {
      help_text.push_str(&format!("・{}\n  {}\n\n", cmd.pattern, cmd.description));
    }
  }
  
  util::reply_to(&config, event, person, &help_text).await?;
  Ok(())
}

async fn fortune(config: config::AppConfig, person: db::Person, event: Event) -> Result<()> {
  let text = &format!("今日のわたしの運勢を占って。結果はランダムで決めて、その結果に従って占いの内容を運の良さは★マークを５段階でラッキーアイテム、ラッキーカラーとかも教えて。\n{}",event.content);
  let reply = gpt::get_reply(&person.prompt, text, true, None).await.unwrap();
  if reply.len() > 0 {
      util::reply_to(&config, event.clone(), person.clone(), &reply).await?;
  }
  Ok(())
}

async fn zap_ranking(config: config::AppConfig, person: db::Person, event: Event) -> Result<()> {
  println!("zap_ranking");
  let pubkey = &event.pubkey.to_string();
  let text = &format!("「現在から過去1年分のzapを集計します。しばらくお待ち下さい。」をあなたらしく言い換えてください。元の文章に含まれる内容が欠落しないようにしてください。「」内に入る文字だけを返信してください。カギカッコは不要です。");
  let reply = gpt::get_reply(&person.prompt, text, true, None).await.unwrap();
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
      let tag_vec = tag.clone().to_vec();
      if tag_vec.len() >= 2 && tag_vec[0] == "bolt11" {
        bolt11_string = tag_vec[1].to_string();
      } else if tag_vec.len() >= 2 && tag_vec[0] == "description" {
        let description = &tag_vec[1];
        if let Ok(content_json) = serde_json::from_str::<Value>(description) {
          if let Some(pubkey) = content_json.get("pubkey").and_then(|k| k.as_str()) {
            // descriptionの中のhex pubkeyをnpub形式に変換
            if let Ok(key) = PublicKey::parse(pubkey) {
              pubkey_str = key.to_bech32().unwrap();
            }
          }
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
        format!("{}: nostr:{} Zap: {}, Count: {}", index + 1, pubkey, util::format_with_commas(zap / 1000), count)
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
  config: config::AppConfig,
  event: Event,
  lines: Vec<String>,
) -> Result<()> {
  let conn = db::connect()?;
  let keys = Keys::generate();
  let prompt = &lines[1];
  let content = &lines[2];
  db::insert_person(&conn, &keys, &prompt, &content)?;
  let new_person = db::get_person(&conn, &keys.public_key().to_string()).unwrap();
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
  config: config::AppConfig,
  person: db::Person,
  event: Event,
) -> Result<()> {
  println!("get kind 0");
  let _meta_event = util::get_kind0(&person.pubkey, &person.secretkey).await?;
  let conn = db::connect()?;
  db::update_person_content(&conn, &person.pubkey, &_meta_event.content.to_string())?;
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
  config: config::AppConfig,
  person: db::Person,
  event: Event,
  lines: Vec<String>,
) -> Result<()> {
  println!("update kind 0");
  let conn = db::connect()?;
  db::update_person_content(&conn, &person.pubkey, &lines[1])?;
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
  config: config::AppConfig,
  person: db::Person,
  event: Event,
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

async fn admin_clear_follower_cache(
  config: config::AppConfig,
  person: db::Person,
  event: Event,
) -> Result<()> {
  println!("clear follower cache");
  let conn = db::connect()?;
  let deleted_count = db::clear_follower_cache(&conn)?;
  util::reply_to(
      &config,
      event.clone(),
      person.clone(),
      &format!("フォロワーキャッシュをクリアしました（{}件削除）", deleted_count),
  )
  .await?;
  Ok(())
}

async fn update_my_follower_cache(
  config: config::AppConfig,
  person: db::Person,
  event: Event,
) -> Result<()> {
  println!("update my follower cache for user: {}", event.pubkey);
  let user_pubkey = event.pubkey.to_string();
  let bot_pubkey = person.pubkey.clone();
  let secret_key = person.secretkey.clone();
  
  // Delete user's cache
  let conn = db::connect()?;
  let _deleted = db::delete_user_follower_cache(&conn, &user_pubkey, &bot_pubkey)?;
  drop(conn); // Release connection before async operation
  
  // Fetch fresh data from relay
  let is_follower = util::fetch_follower_status(&user_pubkey, &secret_key).await?;
  
  // Save new cache
  let conn = db::connect()?;
  db::set_follower_cache(&conn, &user_pubkey, &bot_pubkey, is_follower)?;
  
  let status_text = if is_follower { "フォロー中" } else { "未フォロー" };
  util::reply_to(
      &config,
      event.clone(),
      person.clone(),
      &format!("フォロワー情報を更新しました！現在のステータス: {}", status_text),
  )
  .await?;
  Ok(())
}
