use crate::config;
use crate::config::AppConfig;
use crate::db;
use nostr_sdk::prelude::*;
use std::fs::File;
use std::str::FromStr;
use std::thread;
use std::time::Duration;
use rand::Rng;


pub async fn is_follower(user_pubkey: &str, bot_secret_key: &str) -> Result<bool> {
  let file = File::open("../config.yml")?;
  let config: config::AppConfig = serde_yaml::from_reader(file)?;
  let my_keys = Keys::from_sk_str(&bot_secret_key)?;
  let bot_pubkey = my_keys.public_key();
  let client = Client::new(&my_keys);
  for item in config.relay_servers.read.iter() {
      client.add_relay(item.clone()).await?;
  }
  client.connect().await;
  let pubkey = XOnlyPublicKey::from_str(user_pubkey).unwrap();
  let subscription = Filter::new()
      .authors([pubkey].to_vec())
      .kinds([nostr_sdk::Kind::ContactList].to_vec())
      .limit(1);

  client.subscribe(vec![subscription]).await;
  println!("subscribe:{}",pubkey.to_string());

  let mut events = vec![];
  let mut count = 0;
  let mut notifications = client.notifications();
  while let Ok(notification) = notifications.recv().await {
      if let RelayPoolNotification::Event{relay_url, event} = notification {
          if event.kind == Kind::ContactList {
              // println!("event {:?}", event);
              events.push(event);
              break;
          }
      }
      count += 1;
      println!("count:{:?}", count);
      if events.len() >= (config.relay_servers.read.len() / 2) ||
          count >= 3
      {
          break;
      }
  }
  let mut detect = false;
  events.sort_by_key(|event| std::cmp::Reverse(event.created_at));
  if let Some(first_event) = events.first() {
      for _tag in first_event.tags.iter() {
          if _tag.as_vec().len() > 1 {
              if _tag.as_vec()[0].len() == 1 {
                  if _tag.as_vec()[0].starts_with('p') {
                      if _tag.as_vec()[1].to_string() == bot_pubkey.to_string() {
                          detect = true;
                      }
                  }
              }
          }
      }
  }
  client.shutdown().await?;
  Ok(detect)
}

pub async fn get_kind0(target_pubkey: &str, bot_secret_key: &str) -> Result<Event> {
  let file = File::open("../config.yml")?;
  let config: config::AppConfig = serde_yaml::from_reader(file)?;
  let my_keys = Keys::from_sk_str(&bot_secret_key)?;
  let client = Client::new(&my_keys);
  for item in config.relay_servers.read.iter() {
      client.add_relay(item.clone()).await?;
  }
  client.connect().await;
  let pubkey = XOnlyPublicKey::from_str(target_pubkey).unwrap();
  let subscription = Filter::new()
      .authors([pubkey].to_vec())
      .kinds([nostr_sdk::Kind::Metadata].to_vec())
      .limit(1);

  client.subscribe(vec![subscription]).await;
  println!("subscribe");

  let mut events = vec![];
  let mut count = 0;
  let mut notifications = client.notifications();
  while let Ok(notification) = notifications.recv().await {
      if let RelayPoolNotification::Event{relay_url, event} = notification {
          if event.kind == Kind::Metadata {
              println!("event {:?}", event);
              events.push(event);
              break;
          }
      }
      count += 1;
      println!("count:{:?}", count);
      if events.len() >= (config.relay_servers.read.len() / 2) ||
          count >= 3
      {
          break;
      }
  }
  client.shutdown().await?;
  events.sort_by_key(|event| std::cmp::Reverse(event.created_at));

  Ok(events.first().unwrap().clone())
}

pub async fn send_kind0(bot_secret_key: &str, meta_json: &str) -> Result<()> {
  let file = File::open("../config.yml")?;
  let config: config::AppConfig = serde_yaml::from_reader(file)?;
  let my_keys = Keys::from_sk_str(&bot_secret_key)?;
  let client = Client::new(&my_keys);
  for item in config.relay_servers.write.iter() {
      client.add_relay(item.clone()).await?;
  }
  client.connect().await;
  let metadata = Metadata::from_json(meta_json).unwrap();
  client.set_metadata(&metadata).await?;
  thread::sleep(Duration::from_secs(10));
  client.shutdown().await?;

  Ok(())
}

pub fn extract_mention(persons: Vec<db::Person>, event: &Event) -> Result<Option<db::Person>> {
  let mut person: Option<db::Person> = None;
  for _person in &persons {
      let content: Value = serde_json::from_str(&_person.content)?;
      let words: Vec<String> = event
          .content
          .split_whitespace()
          .map(|word| word.to_string())
          .collect();
      let name = &content["name"].to_string().replace('"', "");
      let display_name = &content["display_name"].to_string().replace('"', "");

      if words.len() > 0
          && (&words[0] == name
              || &words[0] == display_name
              || event.content.contains(display_name))
      {
          println!("name:{} display_name:{}", name, display_name);
          person = Some(_person.clone());
          break;
      }
  }

  if person.is_none() {
      for _tag in event.tags.iter() {
          if _tag.as_vec().len() > 1 {
              if _tag.as_vec()[0].len() == 1 {
                  if _tag.as_vec()[0].starts_with('p') {
                      for _person in &persons {
                          if _tag.as_vec()[1].to_string() == _person.pubkey.to_string() {
                              person = Some(_person.clone());
                              break;
                          }
                      }
                  }
              }
          }
          if person.is_some() {
              break;
          }
      }
  }
  Ok(person)
}

pub fn judge_post(
  config: &AppConfig,
  persons: Vec<db::Person>,
  event: &Event,
) -> Result<(bool, Option<db::Person>)> {
  let mut post = false;
  println!("{:?}", event);
  let random_number = rand::thread_rng().gen_range(0..100);
  let person = extract_mention(persons, &event).unwrap();
  let mut base_percent = config.bot.reaction_percent;
  if person.is_some() {
      base_percent += 10;
  }
  println!(
      "random_number:{:?} base_percent:{:?}",
      random_number, base_percent
  );
  if random_number <= base_percent {
      post = true;
  }
  Ok((post, person))
}

pub async fn send_to(
  config: &config::AppConfig,
  person: db::Person,
  text: &str,
) -> Result<()> {
  let bot_keys = Keys::from_sk_str(&person.secretkey)?;
  let client_temp = Client::new(&bot_keys);
  for item in config.relay_servers.write.iter() {
      client_temp.add_relay(item.clone()).await.unwrap();
  }
  client_temp.connect().await;
  let tags: Vec<Tag> = vec![];
  let event_id = client_temp
      .publish_text_note(format!("{}", text), tags)
      .await?;
  println!("publish_text_note! eventId:{}", event_id);
  thread::sleep(Duration::from_secs(10));
  client_temp.shutdown().await?;
  Ok(())
}

pub async fn reply_to(
  config: &config::AppConfig,
  event: Event,
  person: db::Person,
  text: &str,
) -> Result<()> {
  let bot_keys = Keys::from_sk_str(&person.secretkey)?;
  let client_temp = Client::new(&bot_keys);
  for item in config.relay_servers.write.iter() {
      client_temp.add_relay(item.clone()).await.unwrap();
  }
  client_temp.connect().await;

  let event = EventBuilder::new_text_note(
      text,
      [Tag::event(event.id), Tag::public_key(event.pubkey)],
  )
  .to_event(&bot_keys)
  .unwrap();
  client_temp.send_event(event).await?;

  println!("publish_text_note!");
  thread::sleep(Duration::from_secs(10));
  client_temp.shutdown().await?;
  Ok(())
}
