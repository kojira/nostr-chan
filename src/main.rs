mod config;
mod db;
mod db_mysql;
mod gpt;
use chrono::{DateTime, Utc};
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
            count >= 10
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

async fn get_kind0(target_pubkey: &str, bot_secret_key: &str) -> Result<Event> {
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
            count >= 10
        {
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

fn extract_mention(persons: Vec<db::Person>, event: &Event) -> Result<Option<db::Person>> {
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

async fn fortune(config: &config::AppConfig, person: &db::Person, event: &Event) -> Result<()> {
    let text = &format!("今日のわたしの運勢を占って。結果はランダムで決めて、その結果に従って占いの内容を運の良さは★マークを５段階でラッキーアイテム、ラッキーカラーとかも教えて。\n{}",event.content);
    let reply = gpt::get_reply(&person.prompt, text, true).await.unwrap();
    if reply.len() > 0 {
        reply_to(config, event.clone(), person.clone(), &reply).await?;
    }
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
                let lines: Vec<String> =
                    event.content.lines().map(|line| line.to_string()).collect();
                if lines[0].contains("new") {
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
                } else if lines[0].contains("get kind 0") {
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
                        &format!("リレーからkind 0を取得してデータベース情報を更新しました"),
                    )
                    .await?;
                } else if lines[0].contains("update kind 0") {
                    println!("update kind 0");
                    db::update_person_content(conn, &person.pubkey, &lines[1])?;
                    send_kind0(&person.secretkey.to_string(), &lines[1]).await?;
                    reply_to(
                        &config,
                        event.clone(),
                        person,
                        &format!("データベースのkind 0を更新してブロードキャストしました"),
                    )
                    .await?;
                } else if lines[0].contains("broadcast kind 0") {
                    println!("broadcast kind 0");
                    send_kind0(&person.secretkey.to_string(), &person.content.to_string()).await?;
                    reply_to(
                        &config,
                        event.clone(),
                        person,
                        &format!("データベースのkind 0の情報をブロードキャストしました"),
                    )
                    .await?;
                } else if lines[0].contains("summary") {
                    let from = &lines[1];
                    let to = &lines[2];
                    let pool = db_mysql::connect().unwrap();
                    let from_timestamp = db_mysql::to_unix_timestamp(&from).unwrap() - 9 * 60 * 60;
                    let from_datetime = DateTime::<Utc>::from_utc(
                        chrono::NaiveDateTime::from_timestamp_opt(from_timestamp, 0).unwrap(),
                        Utc,
                    );
                    let from_datetime_str = from_datetime.format("%Y-%m-%d %H:%M:%S").to_string();
                    let to_timestamp = db_mysql::to_unix_timestamp(&to).unwrap() - 9 * 60 * 60;
                    let to_datetime = DateTime::<Utc>::from_utc(
                        chrono::NaiveDateTime::from_timestamp_opt(to_timestamp, 0).unwrap(),
                        Utc,
                    );
                    let to_datetime_str = to_datetime.format("%Y-%m-%d %H:%M:%S").to_string();
                    let events = db_mysql::select_events(
                        &pool,
                        Kind::TextNote,
                        &from_datetime_str,
                        &to_datetime_str,
                    );

                    if events.len() > 0 {
                        let event_len = events.len();
                        reply_to(
                            &config,
                            event.clone(),
                            person.clone(),
                            &format!("{from}〜{to}の{event_len}件の投稿のうち、日本語の投稿の要約を開始しますわ。しばらくお待ち遊ばせ。"),
                        )
                        .await?;
                    }

                    let mut summary = String::from("");
                    let mut event_count = 0;
                    for event in events {
                        let mut japanese: bool = false;
                        if let Some(lang) = detect(&event.content) {
                            match lang.lang() {
                                Lang::Jpn => japanese = true,
                                _ => (),
                            }
                        }
                        if japanese
                            && !event.content.starts_with("lnbc")
                            && !event.content.contains("#まとめ除外")
                            && event.content.len() < 400
                        {
                            summary = format!("{}{}\n", summary, event.content);
                            event_count += 1;
                        }
                    }
                    while summary.len() > 1500 {
                        summary = gpt::get_summary(&summary).await?;
                    }
                    print!("summary:{}", summary);
                    reply_to(
                        &config,
                        event.clone(),
                        person,
                        &format!(
                            "{from}〜{to}の日本語投稿{event_count}件の要約ですわ。\n{summary}\n#まとめ除外"
                        ),
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

async fn send_to(
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

async fn reply_to(
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
    for item in config.relay_servers.read.iter() {
        client.add_relay(item.clone()).await?;
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
        if let RelayPoolNotification::Event{relay_url, event} = notification {
            let result = config
                .bot
                .blacklist
                .iter()
                .any(|s| s == &event.pubkey.to_string());
            if result {
                continue;
            }
            if event.kind == Kind::TextNote {
                let mut detectNip36 = false;
                for tag in event.tags.clone().into_iter() {
                    match tag {
                        Tag::ContentWarning { reason: _ } => {
                            // skip NIP-36
                            detectNip36 = true;
                            break;
                        },
                        _ => ()
                    }
                }
                if detectNip36 {
                    continue;
                }
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
                        let has_mention;
                        if person_op.is_none() {
                            person = db::get_random_person(&conn).unwrap();
                            has_mention = false;
                        } else {
                            person = person_op.unwrap();
                            has_mention = true;
                        }
                        if event.created_at.as_i64() > (last_post_time + config.bot.reaction_freq) || has_mention {
                            post = true;
                        }
                        if post {
                            let follower =
                                is_follower(&event.pubkey.to_string(), &person.secretkey).await?;
                            println!("follower:{}", follower);
                            if follower {
                                let reply =
                                    match gpt::get_reply(&person.prompt, &event.content, has_mention).await {
                                        Ok(reply) => reply,
                                        Err(e) => {
                                            eprintln!("Error: {}", e);
                                            continue;
                                        }
                                    };

                                if reply.len() > 0 {
                                    println!("publish_text_note...{}", reply);
                                    send_to(&config, person, &reply).await?;
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
