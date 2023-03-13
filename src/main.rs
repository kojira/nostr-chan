mod config;
mod db;
mod gpt;
use chrono::Utc;
use dotenv::dotenv;
use nostr_sdk::prelude::*;
use rand::Rng;
use std::fs::File;
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

#[tokio::main]
async fn main() -> Result<()> {
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
            if event.kind == Kind::TextNote {
                if event.content.len() > 0 && (event.created_at.as_i64() > last_post_time) {
                    if let Some(lang) = detect(&event.content) {
                        match lang.lang() {
                            Lang::Jpn => {
                                println!("{:?}", event);
                                let mut post = false;
                                if event.created_at.as_i64()
                                    > (last_post_time + config.bot.reaction_freq)
                                {
                                    post = true;
                                }
                                let random_number = rand::thread_rng().gen_range(0..100);
                                println!("random_number:{:?}", random_number);
                                if random_number <= 10 || post {
                                    let person = db::get_random_person(&conn).unwrap();
                                    let follower =
                                        is_follower(&event.pubkey.to_string(), &person.secretkey)
                                            .await?;
                                    println!("follower:{}", follower);
                                    if follower {
                                        let reply = gpt::get_reply(&person.prompt, &event.content)
                                            .await
                                            .unwrap();
                                        println!("publish_text_note...{}", reply);
                                        if reply.len() > 0 {
                                            let bot_keys = Keys::from_sk_str(&person.secretkey)?;
                                            let client_temp = Client::new(&bot_keys);
                                            for item in config.relay_servers.iter() {
                                                client_temp.add_relay(item, None).await?;
                                            }
                                            client_temp.connect().await;
                                            let mut tags: Vec<Tag> = vec![];
                                            tags.push(Tag::Event(
                                                event.id,
                                                None,
                                                Some(Marker::Reply),
                                            ));
                                            tags.push(Tag::PubKey(event.pubkey, None));
                                            let event_id = client_temp
                                                .publish_text_note(format!("{}", reply), &tags)
                                                .await?;
                                            println!("publish_text_note! eventId:{}", event_id);
                                            last_post_time = Utc::now().timestamp();
                                            client_temp.shutdown().await?;
                                        }
                                        println!("publish_text_note!");
                                    }
                                } else {
                                    println!("hazure!");
                                }
                            }
                            _ => (),
                        }
                    } else {
                        println!("Language detection failed.");
                    }
                }
            } else {
                println!("{:?}", event);
            }
        }
    }

    Ok(())
}
