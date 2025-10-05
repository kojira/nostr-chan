mod config;
mod db;
mod gpt;
mod commands;
mod util;
use chrono::Utc;
use dotenv::dotenv;
use nostr_sdk::prelude::*;
use std::{fs::File, str::FromStr};
use std::env;
use whatlang::{detect, Lang};


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    println!("start");
    let file = File::open("../config.yml")?;
    let config: config::AppConfig = serde_yaml::from_reader(file)?;
    let conn = db::connect()?;

    let secret_key = env::var("BOT_SECRETKEY").expect("BOT_SECRETKEY is not set");

    let my_keys = Keys::from_str(&secret_key)?;

    // Create new client
    let client = Client::new(my_keys);
    for item in config.relay_servers.read.iter() {
        client.add_relay(item.clone()).await?;
    }
    println!("add_relay");

    // Connect to relays
    client.connect().await;
    println!("client.connect");

    let subscription = Filter::new()
        .kinds([nostr_sdk::Kind::TextNote, nostr_sdk::Kind::ChannelMessage].to_vec())
        .since(Timestamp::now());

    let _ = client.subscribe(subscription, None).await;
    println!("subscribe");
    let mut last_post_time = Utc::now().timestamp() - config.bot.reaction_freq;
    let mut notifications = client.notifications();
    while let Ok(notification) = notifications.recv().await {
        if let RelayPoolNotification::Event{relay_url: _, subscription_id: _, event} = notification {
            let result = config
                .bot
                .blacklist
                .iter()
                .any(|s| s == &event.pubkey.to_string());
            if result {
                continue;
            }
            let kind = event.kind;
            if kind == Kind::TextNote || kind == Kind::ChannelMessage {
                let mut detect_nip36 = false;
                for tag in event.tags.clone().into_iter() {
                    match tag.as_standardized() {
                        Some(TagStandard::ContentWarning { reason: _ }) => {
                            // skip NIP-36
                            detect_nip36 = true;
                            break;
                        },
                        _ => ()
                    }
                }
                if detect_nip36 {
                    continue;
                }
                let persons = db::get_all_persons(&conn).unwrap();
                let _handled = commands::command_handler(&config, &conn, &persons, &event).await?;
                let mut japanese: bool = false;
                if let Some(lang) = detect(&event.content) {
                    match lang.lang() {
                        Lang::Jpn => japanese = true,
                        _ => (),
                    }
                } else {
                    // println!("Language detection failed.");
                }
                if event.content.len() > 0
                    && (event.created_at.as_u64() as i64 > last_post_time)
                {
                    let (mut post, person_op) = util::judge_post(&config, persons, &event).unwrap();
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
                    if !japanese && !has_mention {
                        continue;
                    }

                    if event.created_at.as_u64() as i64 > (last_post_time + config.bot.reaction_freq) || has_mention {
                        post = true;
                    }
                    if post {
                        let follower =
                            util::is_follower(&event.pubkey.to_string(), &person.secretkey).await?;
                        println!("follower:{}", follower);
                        if follower {
                            // GPT処理を別タスクで非同期実行
                            let config_clone = config.clone();
                            let event_clone = *event;
                            let person_clone = person.clone();
                            let prompt_clone = person.prompt.clone();
                            let content_clone = event_clone.content.clone();
                            
                            tokio::spawn(async move {
                                let reply = match gpt::get_reply(&prompt_clone, &content_clone, has_mention).await {
                                    Ok(reply) => reply,
                                    Err(e) => {
                                        eprintln!("Error: {}", e);
                                        return;
                                    }
                                };

                                if reply.len() > 0 {
                                    println!("publish_text_note...{}", reply);
                                    if has_mention {
                                        if let Err(e) = util::reply_to(&config_clone, event_clone, person_clone.clone(), &reply).await {
                                            eprintln!("Failed to reply: {}", e);
                                        }
                                    } else if event_clone.kind == Kind::TextNote {
                                        if let Err(e) = util::send_to(&config_clone, event_clone, person_clone, &reply).await {
                                            eprintln!("Failed to send: {}", e);
                                        }
                                    }
                                }
                            });
                            
                            // last_post_timeは即座に更新（連続投稿防止）
                            last_post_time = Utc::now().timestamp();
                        }
                    } else {
                        println!("hazure!");
                    }
                }
            } else {
                println!("{:?}", event);
            }
        }
    }

    Ok(())
}
