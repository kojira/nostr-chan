use crate::config;
use crate::db;
use nostr_sdk::prelude::*;
use crate::gpt;
use crate::util;
use rusqlite::Connection;


async fn fortune(config: &config::AppConfig, person: &db::Person, event: &Event) -> Result<()> {
  let text = &format!("今日のわたしの運勢を占って。結果はランダムで決めて、その結果に従って占いの内容を運の良さは★マークを５段階でラッキーアイテム、ラッキーカラーとかも教えて。\n{}",event.content);
  let reply = gpt::get_reply(&person.prompt, text, true).await.unwrap();
  if reply.len() > 0 {
      util::reply_to(config, event.clone(), person.clone(), &reply).await?;
  }
  Ok(())
}

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
      } else {
          let is_admin = admin_pubkeys.iter().any(|s| *s == event.pubkey.to_string());
          if is_admin {
              let lines: Vec<String> =
                  event.content.lines().map(|line| line.to_string()).collect();
              if lines[0].contains("new") {
                  let keys = Keys::generate();
                  let prompt = &lines[1];
                  let content = &lines[2];
                  db::insert_person(conn, &keys, &prompt, &content)?;
                  let new_person = db::get_person(conn, &keys.public_key().to_string()).unwrap();
                  util::send_kind0(&new_person.secretkey.to_string(), content).await?;
                  let content: Value = serde_json::from_str(content)?;
                  let display_name = &content["display_name"].to_string()
                      [1..content["display_name"].to_string().len() - 1];
                  util::reply_to(
                      &config,
                      event.clone(),
                      new_person,
                      &format!("{}です。コンゴトモヨロシク！", display_name),
                  )
                  .await?;
              } else if lines[0].contains("get kind 0") {
                  println!("get kind 0");
                  let _meta_event = util::get_kind0(&person.pubkey, &person.secretkey).await?;
                  db::update_person_content(
                      conn,
                      &person.pubkey,
                      &_meta_event.content.to_string(),
                  )?;
                  util::reply_to(
                      &config,
                      event.clone(),
                      person,
                      &format!("リレーからkind 0を取得してデータベース情報を更新しました"),
                  )
                  .await?;
              } else if lines[0].contains("update kind 0") {
                  println!("update kind 0");
                  db::update_person_content(conn, &person.pubkey, &lines[1])?;
                  util::send_kind0(&person.secretkey.to_string(), &lines[1]).await?;
                  util::reply_to(
                      &config,
                      event.clone(),
                      person,
                      &format!("データベースのkind 0を更新してブロードキャストしました"),
                  )
                  .await?;
              } else if lines[0].contains("broadcast kind 0") {
                  println!("broadcast kind 0");
                  util::send_kind0(&person.secretkey.to_string(), &person.content.to_string()).await?;
                  util::reply_to(
                      &config,
                      event.clone(),
                      person,
                      &format!("データベースのkind 0の情報をブロードキャストしました"),
                  )
                  .await?;
              }
          }
      }
  }
  Ok(handled)
}
