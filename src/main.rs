mod config;
mod db;
mod gpt;
use chrono::Utc;
use config::AppConfig;
use dotenv::dotenv;
use nostr_sdk::prelude::*;
use rand::Rng;
use rusqlite::Connection;
use serde_json::Value;
use std::fs::File;
use std::thread;
use std::time::Duration;
use std::{env, str::FromStr};
use whatlang::{detect, Lang};

async fn is_follower(user_pubkey: &str, bot_secret_key: &str) -> Result<bool> {
    let file = File::open("../config.yml")?;
    let config: config::AppConfig = serde_yaml::from_reader(file)?;
    let my_keys = Keys::from_sk_str(&bot_secret_key)?;
    let bot_pubkey = my_keys.public_key();
    let client = Client::new(&my_keys);
    for item in config.relay_servers.iter() {
        client.add_relay(item, None).await?;
    }
    client.connect().await;
    let pubkey = XOnlyPublicKey::from_str(user_pubkey).unwrap();
    let subscription = Filter::new()
        .authors([pubkey].to_vec())
        .kinds([nostr_sdk::Kind::ContactList].to_vec());

    client.subscribe(vec![subscription]).await;
    println!("subscribe");

    let mut events = vec![];
    let mut count = 0;
    let mut notifications = client.notifications();
    while let Ok(notification) = notifications.recv().await {
        if let RelayPoolNotification::Event(_url, event) = notification {
            if event.kind == Kind::ContactList {
                events.push(event);
                break;
            }
            println!("event {:?}", event);
        }
        count += 1;
        println!("count:{:?}", count);
        if count > 10 {
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

async fn get_kind0(target_pubkey: &str, bot_secret_key: &str) -> Result<Event> {
    let file = File::open("../config.yml")?;
    let config: config::AppConfig = serde_yaml::from_reader(file)?;
    let my_keys = Keys::from_sk_str(&bot_secret_key)?;
    let client = Client::new(&my_keys);
    for item in config.relay_servers.iter() {
        client.add_relay(item, None).await?;
    }
    client.connect().await;
    let pubkey = XOnlyPublicKey::from_str(target_pubkey).unwrap();
    let subscription = Filter::new()
        .authors([pubkey].to_vec())
        .kinds([nostr_sdk::Kind::Metadata].to_vec());

    client.subscribe(vec![subscription]).await;
    println!("subscribe");

    let mut events = vec![];
    let mut count = 0;
    let mut notifications = client.notifications();
    while let Ok(notification) = notifications.recv().await {
        if let RelayPoolNotification::Event(_url, event) = notification {
            if event.kind == Kind::Metadata {
                events.push(event);
                break;
            }
            println!("event {:?}", event);
        }
        count += 1;
        println!("count:{:?}", count);
        if count > 5 {
            break;
        }
    }
    client.shutdown().await?;
    events.sort_by_key(|event| std::cmp::Reverse(event.created_at));

    Ok(events.first().unwrap().clone())
}

async fn send_kind0(bot_secret_key: &str, meta_json: &str) -> Result<()> {
    let file = File::open("../config.yml")?;
    let config: config::AppConfig = serde_yaml::from_reader(file)?;
    let my_keys = Keys::from_sk_str(&bot_secret_key)?;
    let client = Client::new(&my_keys);
    for item in config.relay_servers.iter() {
        client.add_relay(item, None).await?;
    }
    client.connect().await;
    let metadata = Metadata::from_json(meta_json).unwrap();
    client.set_metadata(metadata).await?;
    thread::sleep(Duration::from_secs(10));
    client.shutdown().await?;

    Ok(())
}

fn extract_mention(persons: Vec<db::Person>, event: &Event) -> Result<Option<db::Person>> {
    let mut person: Option<db::Person> = None;
    for _person in &persons {
        let content: Value = serde_json::from_str(&_person.content)?;
        let name = &content["name"].to_string().replace('"', "");
        let display_name = &content["display_name"].to_string().replace('"', "");

        if event.content.contains(name) || event.content.contains(display_name) {
            person = Some(_person.clone());
            break;
        }
    }

    if person.is_some() {
        for _tag in event.tags.iter() {
            if _tag.as_vec().len() > 1 {
                if _tag.as_vec()[0].len() == 1 {
                    if _tag.as_vec()[0].starts_with('p') {
                        for _person in &persons {
                            if _tag.as_vec()[1].to_string() == _person.pubkey.to_string() {
                                person = Some(_person.clone());
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(person)
}

async fn fortune(config: &config::AppConfig, person: &db::Person, event: &Event) -> Result<()> {
    let text = "今日のわたしの運勢を占って。結果はランダムで決めて、その結果に従って占いの内容を運の良さは★マークを５段階でラッキーアイテム、ラッキーカラーとかも教えて";
    let reply = gpt::get_reply(&person.prompt, text).await.unwrap();
    reply_to(config, event.clone(), person.clone(), &reply).await?;
    Ok(())
}

async fn command_handler(
    config: &config::AppConfig,
    conn: &Connection,
    persons: &Vec<db::Person>,
    event: &Event,
) -> Result<bool> {
    let admin_pubkeys = &config.bot.admin_pubkeys;
    let mut handled: bool = false;
    let persons_ = persons.clone();
    let person_op = extract_mention(persons_, &event).unwrap();
    if person_op.is_some() {
        let person = person_op.unwrap();
        if event.content.contains("占って") {
            fortune(config, &person, event).await?;
            handled = true;
        } else if event.content.contains("silent") {
        } else {
            let is_admin = admin_pubkeys.iter().any(|s| *s == event.pubkey.to_string());
            if is_admin {
                if event.content.contains("new") {
                    let lines: Vec<String> =
                        event.content.lines().map(|line| line.to_string()).collect();
                    let keys = Keys::generate();
                    let prompt = &lines[1];
                    let content = &lines[2];
                    db::insert_person(conn, &keys, &prompt, &content)?;
                    let new_person = db::get_person(conn, &keys.public_key().to_string()).unwrap();
                    send_kind0(&new_person.secretkey.to_string(), content).await?;
                    let content: Value = serde_json::from_str(content)?;
                    let display_name = &content["display_name"].to_string()
                        [1..content["display_name"].to_string().len() - 1];
                    reply_to(
                        &config,
                        event.clone(),
                        new_person,
                        &format!("{}です。コンゴトモヨロシク！", display_name),
                    )
                    .await?;
                } else if event.content.contains("get kind 0") {
                    println!("get kind 0");
                    let _meta_event = get_kind0(&person.pubkey, &person.secretkey).await?;
                    db::update_person_content(
                        conn,
                        &person.pubkey,
                        &_meta_event.content.to_string(),
                    )?;
                    reply_to(
                        &config,
                        event.clone(),
                        person,
                        &format!("リレーからkindo 0を取得してデータベース情報を更新しました"),
                    )
                    .await?;
                } else if event.content.contains("update kind 0") {
                    println!("update kind 0");
                    let lines: Vec<String> =
                        event.content.lines().map(|line| line.to_string()).collect();
                    db::update_person_content(conn, &person.pubkey, &lines[1])?;
                    send_kind0(&person.secretkey.to_string(), &lines[1]).await?;
                    reply_to(
                        &config,
                        event.clone(),
                        person,
                        &format!("データベースのkind 0を更新してブロードキャストしました"),
                    )
                    .await?;
                } else if event.content.contains("broadcast kind 0") {
                    println!("broadcast kind 0");
                    send_kind0(&person.secretkey.to_string(), &person.content.to_string()).await?;
                    reply_to(
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

fn judge_post(
    config: &AppConfig,
    persons: Vec<db::Person>,
    event: &Event,
) -> Result<(bool, Option<db::Person>)> {
    let mut post = false;
    println!("{:?}", event);
    let random_number = rand::thread_rng().gen_range(0..100);
    println!("random_number:{:?}", random_number);
    let person = extract_mention(persons, &event).unwrap();
    let mut base_percent = config.bot.reaction_percent;
    if person.is_some() {
        base_percent += 10;
    }
    if random_number <= base_percent {
        post = true;
    }
    Ok((post, person))
}

async fn reply_to(
    config: &config::AppConfig,
    event: Event,
    person: db::Person,
    text: &str,
) -> Result<()> {
    let bot_keys = Keys::from_sk_str(&person.secretkey)?;
    let client_temp = Client::new(&bot_keys);
    for item in config.relay_servers.iter() {
        client_temp.add_relay(item, None).await?;
    }
    client_temp.connect().await;
    let mut tags: Vec<Tag> = vec![];
    tags.push(Tag::Event(event.id, None, Some(Marker::Reply)));
    tags.push(Tag::PubKey(event.pubkey, None));
    let event_id = client_temp
        .publish_text_note(format!("{}", text), &tags)
        .await?;
    println!("publish_text_note! eventId:{}", event_id);
    thread::sleep(Duration::from_secs(10));
    client_temp.shutdown().await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    println!("start");
    let file = File::open("../config.yml")?;
    let config: config::AppConfig = serde_yaml::from_reader(file)?;
    let conn = db::connect()?;

    let secret_key = env::var("BOT_SECRETKEY").expect("BOT_SECRETKEY is not set");

    let my_keys = Keys::from_sk_str(&secret_key)?;

    // Create new client
    let client = Client::new(&my_keys);
    for item in config.relay_servers.iter() {
        client.add_relay(item, None).await?;
    }
    println!("add_relay");

    // Connect to relays
    client.connect().await;
    println!("client.connect");

    let subscription = Filter::new()
        .kinds([nostr_sdk::Kind::TextNote].to_vec())
        .since(Timestamp::now());

    client.subscribe(vec![subscription]).await;
    println!("subscribe");
    let mut last_post_time = Utc::now().timestamp() - config.bot.reaction_freq;
    let mut notifications = client.notifications();
    while let Ok(notification) = notifications.recv().await {
        if let RelayPoolNotification::Event(_url, event) = notification {
            let result = config
                .bot
                .blacklist
                .iter()
                .any(|s| s == &event.pubkey.to_string());
            if result {
                continue;
            }
            if event.kind == Kind::TextNote {
                let persons = db::get_all_persons(&conn).unwrap();
                let handled = command_handler(&config, &conn, &persons, &event).await?;
                let mut japanese: bool = false;
                if let Some(lang) = detect(&event.content) {
                    match lang.lang() {
                        Lang::Jpn => japanese = true,
                        _ => (),
                    }
                } else {
                    // println!("Language detection failed.");
                }
                if japanese {
                    if !handled
                        && event.content.len() > 0
                        && (event.created_at.as_i64() > last_post_time)
                    {
                        let (mut post, person_op) = judge_post(&config, persons, &event).unwrap();
                        println!("post:{}", post);
                        let person: db::Person;
                        if event.created_at.as_i64() > (last_post_time + config.bot.reaction_freq) {
                            post = true;
                        }
                        if post {
                            if person_op.is_none() {
                                person = db::get_random_person(&conn).unwrap();
                            } else {
                                person = person_op.unwrap();
                            }
                            let follower =
                                is_follower(&event.pubkey.to_string(), &person.secretkey).await?;
                            println!("follower:{}", follower);
                            if follower {
                                let reply = gpt::get_reply(&person.prompt, &event.content)
                                    .await
                                    .unwrap();
                                println!("publish_text_note...{}", reply);
                                if reply.len() > 0 {
                                    reply_to(&config, event, person, &reply).await?;
                                    last_post_time = Utc::now().timestamp();
                                }
                            }
                        } else {
                            println!("hazure!");
                        }
                    }
                }
            } else {
                println!("{:?}", event);
            }
        }
    }

    Ok(())
}
