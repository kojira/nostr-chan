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

// タイムライン投稿の構造体
#[derive(Clone, Debug)]
pub struct TimelinePost {
    pub pubkey: String,
    pub name: Option<String>,
    pub content: String,
    pub timestamp: i64,
}


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
    
    // DBから既存のタイムラインを読み込み（起動時のみ）
    println!("Loading timeline from DB...");
    let timeline_posts = db::get_latest_timeline_posts(&conn, config.bot.timeline_size).unwrap_or_else(|e| {
        eprintln!("Failed to load timeline: {}", e);
        Vec::new()
    });
    println!("Loaded {} timeline posts", timeline_posts.len());
    
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
                let handled = commands::command_handler(&config, &conn, &persons, &event).await?;
                let mut japanese: bool = false;
                if let Some(lang) = detect(&event.content) {
                    match lang.lang() {
                        Lang::Jpn => {
                            japanese = true;
                            // 名前を非同期で取得（エラーは無視）
                            let name = util::get_user_name(&event.pubkey.to_string()).await.ok();
                            
                            // 日本語投稿をDBに保存
                            let timeline_post = TimelinePost {
                                pubkey: event.pubkey.to_string(),
                                name: name.clone(),
                                content: event.content.clone(),
                                timestamp: event.created_at.as_u64() as i64,
                            };
                            
                            if let Err(e) = db::add_timeline_post(
                                &conn,
                                &timeline_post.pubkey,
                                timeline_post.name.as_deref(),
                                &timeline_post.content,
                                timeline_post.timestamp
                            ) {
                                eprintln!("Failed to save timeline post: {}", e);
                            }
                            
                            // 古い投稿を削除
                            let _ = db::cleanup_old_timeline_posts(&conn, config.bot.timeline_max_storage);
                        },
                        _ => (),
                    }
                } else {
                    // println!("Language detection failed.");
                }
                if !handled
                    && event.content.len() > 0
                    && (event.created_at.as_u64() as i64 > last_post_time)
                {
                    // メンション判定のみ先に行う
                    let person_op = util::extract_mention(persons.clone(), &event).unwrap();
                    let has_mention = person_op.is_some();
                    
                    // エアリプは日本語のみ、メンションは言語不問
                    if !has_mention && !japanese {
                        continue;
                    }
                    
                    // 日本語チェック後に確率判定
                    let (mut post, _) = util::judge_post(&config, persons, &event).unwrap();
                    println!("post:{}", post);
                    
                    let person: db::Person;
                    if person_op.is_none() {
                        person = db::get_random_person(&conn).unwrap();
                    } else {
                        person = person_op.unwrap();
                    }

                    if event.created_at.as_u64() as i64 > (last_post_time + config.bot.reaction_freq) || has_mention {
                        post = true;
                    }
                    
                    // エアリプの場合は日本語チェックを再確認
                    if post && !has_mention && !japanese {
                        continue;
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
                            
                            // エアリプの場合はDBからタイムラインを読み込む
                            let timeline_clone = if !has_mention {
                                db::get_latest_timeline_posts(&conn, config.bot.timeline_size).ok()
                            } else {
                                None
                            };
                            
                            // bot自身の発言をバッファに追加するためのチャネル
                            let (tx, mut rx) = tokio::sync::mpsc::channel::<TimelinePost>(1);
                            
                            tokio::spawn(async move {
                                let reply = match gpt::get_reply(&prompt_clone, &content_clone, has_mention, timeline_clone).await {
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
                                        if let Err(e) = util::send_to(&config_clone, event_clone, person_clone.clone(), &reply).await {
                                            eprintln!("Failed to send: {}", e);
                                        }
                                    }
                                    
                                    // bot自身の発言をタイムラインに追加
                                    let bot_post = TimelinePost {
                                        pubkey: person_clone.pubkey.clone(),
                                        name: Some("Bot".to_string()), // または person_clone から取得
                                        content: reply,
                                        timestamp: Utc::now().timestamp(),
                                    };
                                    let _ = tx.send(bot_post).await;
                                }
                            });
                            
                            // bot発言を受信してDBに保存
                            if let Ok(bot_post) = rx.try_recv() {
                                if let Err(e) = db::add_timeline_post(
                                    &conn,
                                    &bot_post.pubkey,
                                    bot_post.name.as_deref(),
                                    &bot_post.content,
                                    bot_post.timestamp
                                ) {
                                    eprintln!("Failed to save bot timeline post: {}", e);
                                }
                                
                                // 古い投稿を削除
                                let _ = db::cleanup_old_timeline_posts(&conn, config.bot.timeline_max_storage);
                            }
                            
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
