use crate::config;
use crate::config::AppConfig;
use crate::db;
use lightning_invoice::Bolt11Invoice;
use nostr_sdk::prelude::*;
use rand::Rng;
use std::fs::File;
use std::str::FromStr;
use std::thread;
use std::time::Duration;
use serde_json::Value;

pub async fn is_follower(user_pubkey: &str, bot_secret_key: &str) -> Result<bool> {
  let file = File::open("../config.yml")?;
  let config: config::AppConfig = serde_yaml::from_reader(file)?;
  let my_keys = Keys::parse(&bot_secret_key)?;
  let bot_pubkey = my_keys.public_key();
  let bot_pubkey_str = bot_pubkey.to_string();
  
  // Check cache first
  let conn = db::connect()?;
  let ttl = config.bot.follower_cache_ttl;
  if let Some((cached_result, remaining)) = db::get_follower_cache(&conn, user_pubkey, &bot_pubkey_str, ttl)? {
    println!("Follower cache hit: remaining {}s ({}h {}m)", 
      remaining, remaining / 3600, (remaining % 3600) / 60);
    return Ok(cached_result);
  }
  
  println!("Follower cache miss, fetching from relay...");
  
  // Cache miss, fetch from relay
  let detect = fetch_follower_status(user_pubkey, bot_secret_key).await?;
  
  // Save to cache
  db::set_follower_cache(&conn, user_pubkey, &bot_pubkey_str, detect)?;
  
  Ok(detect)
}

pub async fn fetch_follower_status(user_pubkey: &str, bot_secret_key: &str) -> Result<bool> {
  let file = File::open("../config.yml")?;
  let config: config::AppConfig = serde_yaml::from_reader(file)?;
  let my_keys = Keys::parse(&bot_secret_key)?;
  let bot_pubkey = my_keys.public_key();
  
  let client = Client::new(my_keys);
  for item in config.relay_servers.read.iter() {
      client.add_relay(item.clone()).await?;
  }
  client.connect().await;
  let publickey = PublicKey::from_hex(user_pubkey).unwrap();

  let filter = Filter::new()
    .authors([publickey].to_vec())
    .kinds([nostr_sdk::Kind::ContactList].to_vec())
    .limit(1);

  let events = client
    .fetch_events(filter, Duration::from_secs(30))
    .await?;

  let detect = events.first().map_or(false, |first_event: &Event| {
    first_event.tags.iter().any(|tag| {
      match tag.as_standardized() {
        Some(TagStandard::PublicKey { public_key, .. }) => {
          public_key == &bot_pubkey
        }
        _ => false
      }
    })
  });

  client.shutdown().await;
  
  Ok(detect)
}

pub async fn get_kind0(target_pubkey: &str, bot_secret_key: &str) -> Result<Event> {
  let file = File::open("../config.yml")?;
  let config: config::AppConfig = serde_yaml::from_reader(file)?;
  let my_keys = Keys::parse(&bot_secret_key)?;
  let client = Client::new(my_keys);
  for item in config.relay_servers.read.iter() {
      client.add_relay(item.clone()).await?;
  }
  client.connect().await;
  let public_key = PublicKey::from_hex(target_pubkey).unwrap();
  let subscription = Filter::new()
      .authors([public_key])
      .kinds([nostr_sdk::Kind::Metadata].to_vec())
      .limit(1);

  let _ = client.subscribe(subscription, None).await;
  println!("subscribe");

  let mut events = vec![];
  let mut count = 0;
  let mut notifications = client.notifications();
  while let Ok(notification) = notifications.recv().await {
    if let RelayPoolNotification::Event { relay_url: _, subscription_id: _, event } = notification {
      if event.kind == Kind::Metadata {
        println!("event {:?}", event);
        events.push(event);
        break;
      }
    }
    count += 1;
    println!("count:{:?}", count);
    if events.len() >= (config.relay_servers.read.len() / 2) || count >= 3 {
      break;
    }
  }
  client.shutdown().await;
  events.sort_by_key(|event| std::cmp::Reverse(event.created_at));

  Ok(*events.first().unwrap().clone())
}

pub async fn send_kind0(bot_secret_key: &str, meta_json: &str) -> Result<()> {
  let file = File::open("../config.yml")?;
  let config: config::AppConfig = serde_yaml::from_reader(file)?;
  let my_keys = Keys::parse(&bot_secret_key)?;
  let client = Client::new(my_keys);
  for item in config.relay_servers.write.iter() {
    client.add_relay(item.clone()).await?;
  }
  client.connect().await;
  let metadata = Metadata::from_json(meta_json).unwrap();
  client.set_metadata(&metadata).await?;
  thread::sleep(Duration::from_secs(10));
  client.shutdown().await;

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
      let tag_vec = _tag.clone().to_vec();
      if tag_vec.len() > 1 {
        if tag_vec[0].len() == 1 {
          if tag_vec[0].starts_with('p') {
            for _person in &persons {
              if tag_vec[1].to_string() == _person.pubkey.to_string() {
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

pub async fn send_to(config: &config::AppConfig, event: Event, person: db::Person, text: &str) -> Result<()> {
  let bot_keys = Keys::parse(&person.secretkey)?;
  let client_temp = Client::new(bot_keys.clone());
  for item in config.relay_servers.write.iter() {
    client_temp.add_relay(item.clone()).await.unwrap();
  }
  client_temp.connect().await;
  let kind = event.kind;
  if kind == Kind::TextNote {
    let event_builder = EventBuilder::text_note(text);
    let event = event_builder.sign(&bot_keys).await?;
    let event_id = client_temp.send_event(&event).await?;
    println!("publish_text_note! eventId:{:?}", event_id);
  } else if kind == Kind::ChannelMessage {
    let tags_vec: Vec<Tag> = event.tags.iter().cloned().collect();
    if let Some((event_id, relay_url)) = extract_root_tag_info(&tags_vec) {
      let _id = event_id.clone();
      let relay_url_obj = RelayUrl::parse(&relay_url).unwrap();
      let event_builder = EventBuilder::channel_msg(EventId::parse(&event_id).unwrap(), relay_url_obj, text);
      let event = event_builder.sign(&bot_keys).await?;
      client_temp.send_event(&event).await?;
      println!("eventId:{} relay_url:{} text:{}", _id, relay_url, text);
    }
  }
  client_temp.shutdown().await;
  Ok(())
}

fn extract_root_tag_info(tags: &[Tag]) -> Option<(String, String)> {
  tags.iter().find_map(|tag| {
      match tag.as_standardized() {
          Some(TagStandard::Event { event_id, relay_url: Some(relay_url), marker: Some(Marker::Root), .. }) => {
              Some((event_id.to_string(), relay_url.to_string()))
          }
          _ => None
      }
  })
}

pub async fn reply_to(
  config: &config::AppConfig,
  event: Event,
  person: db::Person,
  text: &str,
) -> Result<Event> {
  let bot_keys = Keys::parse(&person.secretkey)?;
  let client_temp = Client::new(bot_keys.clone());
  for item in config.relay_servers.write.iter() {
    client_temp.add_relay(item.clone()).await.unwrap();
  }
  client_temp.connect().await;
  let kind = event.kind;
  let mut event_copy: Option<Event> = None;

  if kind == Kind::TextNote {
    let event_builder = EventBuilder::text_note(text)
      .tags([Tag::event(event.id), Tag::public_key(event.pubkey)]);
    let event = event_builder.sign(&bot_keys).await?;
    event_copy = Some(event.clone());
    let send_result = client_temp.send_event(&event).await?;
    println!("publish_text_note! relay responses: {:?}", send_result);
    println!("Event ID: {}", event.id);
  } else if kind == Kind::ChannelMessage {
    let tags_vec: Vec<Tag> = event.tags.iter().cloned().collect();
    if let Some((event_id, relay_url)) = extract_root_tag_info(&tags_vec) {
      let _id = event_id.clone();
      let relay_url_obj = RelayUrl::parse(&relay_url).unwrap();
      let event_builder = EventBuilder::channel_msg(EventId::parse(&event_id).unwrap(), relay_url_obj, text);
      let event = event_builder.sign(&bot_keys).await?;
      event_copy = Some(event.clone());
      client_temp.send_event(&event).await?;
      println!("eventId:{} relay_url:{} text:{}", _id, relay_url, text);
      let result = event_copy.clone().unwrap();
      println!("publish_public_message! eventId:{}, text:{}", result.id.to_hex(), result.content);
    }
  }

  client_temp.shutdown().await;
  let event_copy = event_copy.ok_or("Failed to create event")?;
  Ok(event_copy)
}

#[allow(dead_code)]
pub async fn get_zap_received(target_pubkey: &str) -> Result<Vec<Event>> {
  let file = File::open("../config.yml")?;
  let config: config::AppConfig = serde_yaml::from_reader(file)?;
  let client = Client::default();
  // 日本リレーだけだと少ないのでwriteのリレーから取得する
  for item in config.relay_servers.write.iter() {
    client.add_relay(item.clone()).await?;
  }
  client.connect().await;
  let now = Timestamp::now();

  let one_year_ago = now.as_u64() - 60 * 60 * 24 * 30 * 12; // 約1年前

  let mut all_events: Vec<Event> = Vec::new();
  for month_offset in 0..12 {
    println!("month_offset:{:?}", month_offset);
    // sinceは1年前からスタートし、毎回30日分進める
    let since = Timestamp::from(one_year_ago + 60 * 60 * 24 * 30 * month_offset);
    // untilはsinceから30日後
    let until = Timestamp::from(since.as_u64() + 60 * 60 * 24 * 30);

    // Filterのインスタンスを生成
    let filter = Filter::new()
      .custom_tag(SingleLetterTag::from_char('p')?, target_pubkey.to_string())
      .kinds([nostr_sdk::Kind::ZapReceipt].to_vec())
      .until(until)
      .since(since);

    match client
      .fetch_events(filter, Duration::from_secs(30))
      .await
    {
      Ok(events) => {
        println!("counts:{:?}", events.len());
        all_events.extend(events);
      }
      Err(e) => {
        println!("Error fetching events: {:?}", e);
      }
    }
  }

  client.shutdown().await;
  println!("{all_events:#?}");
  let results = all_events;

  Ok(results)
}

#[allow(dead_code)]
pub fn decode_bolt11_invoice(invoice_str: &str) -> Result<Bolt11Invoice, String> {
  Bolt11Invoice::from_str(invoice_str).map_err(|e| e.to_string())
}

#[allow(dead_code)]
pub fn format_with_commas(num: u64) -> String {
    let num_str = num.to_string();
    let mut result = String::new();
    let chars_rev: Vec<_> = num_str.chars().rev().collect();
    for (i, c) in chars_rev.iter().enumerate() {
        if i % 3 == 0 && i != 0 {
            result.push(',');
        }
        result.push(*c);
    }
    result.chars().rev().collect()
}

// 名前取得関数（キャッシュ優先、なければリレーから取得）
pub async fn get_user_name(pubkey: &str) -> Result<String> {
    let file = File::open("../config.yml")?;
    let config: config::AppConfig = serde_yaml::from_reader(file)?;
    let conn = db::connect()?;
    
    // キャッシュをチェック（24時間有効）
    let ttl = 86400; // 24 hours
    if let Some(cached_name) = db::get_kind0_cache(&conn, pubkey, ttl)? {
        return Ok(cached_name);
    }
    
    // キャッシュになければリレーから取得
    let public_key = PublicKey::from_hex(pubkey)?;
    let keys = Keys::generate();
    let client = Client::new(keys);
    
    for relay in config.relay_servers.read.iter() {
        client.add_relay(relay.clone()).await?;
    }
    client.connect().await;
    
    let filter = Filter::new()
        .author(public_key)
        .kind(Kind::Metadata)
        .limit(1);
    
    let events = client.fetch_events(filter, Duration::from_secs(10)).await?;
    client.shutdown().await;
    
    if let Some(event) = events.first() {
        if let Ok(metadata) = serde_json::from_str::<Value>(&event.content) {
            let name = metadata["display_name"]
                .as_str()
                .or_else(|| metadata["name"].as_str())
                .unwrap_or(pubkey)
                .to_string();
            
            // キャッシュに保存
            db::set_kind0_cache(&conn, pubkey, &name)?;
            return Ok(name);
        }
    }
    
    // 取得できなかった場合はpubkeyの先頭8文字
    let short_pubkey = if pubkey.len() > 8 {
        &pubkey[..8]
    } else {
        pubkey
    };
    Ok(format!("{}...", short_pubkey))
}

// Gemini CLIでWeb検索を実行
pub async fn gemini_search(query: &str) -> Result<String, String> {
    use std::process::Command;
    use tokio::time::{timeout, Duration};
    
    println!("Executing gemini search: {}", query);
    
    // 30秒のタイムアウトを設定
    let search_future = tokio::task::spawn_blocking({
        let query = query.to_string();
        move || {
            Command::new("gemini")
                .arg("search")
                .arg(&query)
                .output()
        }
    });
    
    let output = match timeout(Duration::from_secs(30), search_future).await {
        Ok(Ok(Ok(output))) => output,
        Ok(Ok(Err(e))) => return Err(format!("Failed to execute gemini command: {}", e)),
        Ok(Err(e)) => return Err(format!("Task join error: {}", e)),
        Err(_) => return Err("Gemini search timed out after 30 seconds".to_string()),
    };
    
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        eprintln!("Gemini search failed: {}", error);
        return Err(format!("Gemini search failed: {}", error));
    }
    
    let result = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(result)
}
