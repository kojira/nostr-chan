mod config;
mod db;
mod gpt;
mod commands;
mod util;
mod embedding;
mod conversation;
mod dashboard;
use chrono::{Utc, TimeZone};
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
    
    // ダッシュボードサーバーを起動（バックグラウンド）
    let dashboard_config_path = "../config.yml".to_string();
    tokio::spawn(async move {
        if let Err(e) = dashboard::start_dashboard(3000, dashboard_config_path).await {
            eprintln!("ダッシュボードエラー: {}", e);
        }
    });
    
    // Embeddingサービスを初期化
    println!("Embeddingサービスを初期化中...");
    if let Err(e) = embedding::EmbeddingService::initialize_global() {
        eprintln!("警告: Embeddingサービスの初期化に失敗しました: {}", e);
        eprintln!("ベクトル化機能は利用できません");
    } else {
        println!("Embeddingサービスの初期化完了");
    }

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
    
    // バックグラウンドでembedding生成タスクを開始
    let conn_bg = db::connect()?;
    tokio::spawn(async move {
        loop {
            // 10秒ごとにembedding未設定のイベントを処理
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            
            match db::get_events_without_embedding(&conn_bg, 10) {
                Ok(events) => {
                    for event in events {
                        // contentが空の場合はスキップし、空のベクトルを保存（再試行を防ぐ）
                        if event.content.trim().is_empty() {
                            let empty_vec = vec![0.0f32; 384]; // multilingual-e5-smallの次元数
                            if let Err(e) = db::update_event_embedding(&conn_bg, &event.event_id, &empty_vec) {
                                eprintln!("[Embedding] 空コンテンツのDB更新エラー: {}", e);
                            }
                            continue;
                        }
                        
                        match embedding::generate_embedding_global(&event.content) {
                            Ok(embedding_vec) => {
                                if let Err(e) = db::update_event_embedding(&conn_bg, &event.event_id, &embedding_vec) {
                                    eprintln!("[Embedding] DB更新エラー: {}", e);
                                }
                            }
                            Err(e) => {
                                eprintln!("[Embedding] ベクトル化エラー (event_id: {}): {}", event.event_id, e);
                                // エラーの場合も空のベクトルを保存して再試行を防ぐ
                                let empty_vec = vec![0.0f32; 384];
                                if let Err(e) = db::update_event_embedding(&conn_bg, &event.event_id, &empty_vec) {
                                    eprintln!("[Embedding] エラー時のDB更新エラー: {}", e);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[Embedding] イベント取得エラー: {}", e);
                }
            }
        }
    });
    
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
                // 言語判定
                let mut japanese: bool = false;
                if let Some(lang) = detect(&event.content) {
                    match lang.lang() {
                        Lang::Jpn => {
                            japanese = true;
                        },
                        _ => (),
                    }
                }
                
                // イベントをeventsテーブルに保存（kind 1のみ）
                if kind == Kind::TextNote {
                    let event_type = if japanese { Some("air_reply") } else { None };
                    match db::insert_event(&conn, &event, japanese, event_type) {
                        Ok(event_ref_id) => {
                            // kind 0情報を非同期で取得して更新
                            let conn_clone = db::connect().unwrap();
                            let event_id = event.id.to_string();
                            let pubkey = event.pubkey.to_string();
                            tokio::spawn(async move {
                                if let Ok(name) = util::get_user_name(&pubkey).await {
                                    let _ = db::update_event_kind0(&conn_clone, &event_id, Some(&name), None);
                                }
                            });
                            
                            // 日本語の場合、後でエアリプ用に使える（event_ref_idは後で使用）
                            let _ = event_ref_id;
                        }
                        Err(e) => {
                            // 重複エラーは無視（UNIQUE制約）
                            if !e.to_string().contains("UNIQUE constraint failed") {
                                eprintln!("Failed to save event: {}", e);
                            }
                        }
                    }
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
                    let (mut post, _) = util::judge_post(&config, persons.clone(), &event).unwrap();
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
                            // メンション時: 会話回数制限チェック
                            if has_mention {
                                let conversation_count = db::get_conversation_count_with_user(
                                    &conn,
                                    &person.pubkey,
                                    &event.pubkey.to_string(),
                                    config.bot.conversation_limit_minutes,
                                )?;
                                
                                if conversation_count >= config.bot.conversation_limit_count {
                                    println!(
                                        "会話回数制限: {}分間で{}回 (制限: {}回)",
                                        config.bot.conversation_limit_minutes,
                                        conversation_count,
                                        config.bot.conversation_limit_count
                                    );
                                    continue;
                                }
                            }
                            
                            // メンション時: 会話ログに記録
                            let conversation_log_id = if has_mention {
                                // イベントが既にeventsテーブルに保存されているか確認
                                let event_record = db::get_event_by_event_id(&conn, &event.id.to_string())?;
                                let event_ref_id = if let Some(record) = event_record {
                                    record.id
                                } else {
                                    // 保存されていない場合は保存（メンション・リプライ）
                                    let event_type = Some("mention");
                                    db::insert_event(&conn, &event, japanese, event_type)?
                                };
                                
                                // event_jsonからメタ情報を抽出
                                let event_json = serde_json::to_string(&*event)?;
                                let mentioned_pubkeys = db::extract_mentioned_pubkeys(&event_json).ok();
                                let thread_root_id = db::extract_thread_root_id(&event_json).ok().flatten();
                                
                                // bot同士の会話を検出
                                let all_bot_pubkeys: Vec<String> = persons.iter().map(|p| p.pubkey.clone()).collect();
                                let is_bot_conversation = if let Some(ref pks) = mentioned_pubkeys {
                                    db::detect_bot_conversation(pks, &all_bot_pubkeys)
                                } else {
                                    false
                                };
                                
                                // 会話ログに記録
                                match db::insert_conversation_log(
                                    &conn,
                                    &person.pubkey,
                                    event_ref_id,
                                    thread_root_id.as_deref(),
                                    mentioned_pubkeys.as_ref().map(|v| v.as_slice()),
                                    false, // is_bot_message: ユーザーの発言
                                    is_bot_conversation,
                                ) {
                                    Ok(log_id) => Some(log_id),
                                    Err(e) => {
                                        eprintln!("会話ログ記録エラー: {}", e);
                                        None
                                    }
                                }
                            } else {
                                None
                            };
                            
                            // GPT処理を別タスクで非同期実行
                            let config_clone = config.clone();
                            let event_clone = *event;
                            let person_clone = person.clone();
                            let mut prompt_clone = person.prompt.clone();
                            let content_clone = event_clone.content.clone();
                            let conn_clone = db::connect().unwrap();
                            let has_conversation_log = conversation_log_id.is_some();
                            
                            // ユーザーのpubkeyを取得
                            let user_pubkey_clone = event_clone.pubkey.to_string();
                            
                            // メンション時はユーザー名を取得してプロンプトに追加
                            if has_mention {
                                if let Ok(user_name) = util::get_user_name(&user_pubkey_clone).await {
                                    // pubkeyの短縮形でない場合のみ追加
                                    if !user_name.ends_with("...") {
                                        prompt_clone = format!("{}。話しかけてきた相手の名前は「{}」です。", prompt_clone, user_name);
                                    }
                                }
                            }
                            
                            // 会話コンテキストを準備
                            let context_clone = if has_conversation_log {
                                // メンション: 会話コンテキストを使用（同じユーザーとの会話履歴のみ）
                                match conversation::prepare_context_for_reply(
                                    &conn_clone,
                                    &person_clone.pubkey,
                                    &user_pubkey_clone,
                                    &content_clone,
                                    50, // 最大50件の会話履歴
                                ).await {
                                    Ok(ctx) => Some(ctx),
                                    Err(e) => {
                                        eprintln!("[Conversation] コンテキスト準備エラー（要約タイムアウト等）: {} - コンテキストなしで返信します", e);
                                        None
                                    }
                                }
                            } else {
                                // エアリプ: 日本語タイムラインを使用
                                match conversation::build_japanese_timeline_for_air_reply(&conn_clone, config.bot.timeline_size) {
                                    Ok(events) => {
                                        if events.is_empty() {
                                            None
                                        } else {
                                            let timeline_lines: Vec<String> = events.iter()
                                                .enumerate()
                                                .map(|(i, event)| {
                                                    let dt = chrono::Local.timestamp_opt(event.created_at, 0).single().unwrap();
                                                    let time_str = dt.format("%m/%d %H:%M").to_string();
                                                    let display_name = event.kind0_name.clone().unwrap_or_else(|| {
                                                        if event.pubkey.len() > 8 {
                                                            format!("{}...", &event.pubkey[..8])
                                                        } else {
                                                            event.pubkey.clone()
                                                        }
                                                    });
                                                    format!("{}. [{}] {}: {}", i + 1, time_str, display_name, event.content)
                                                })
                                                .collect();
                                            Some(format!("【タイムライン】\n{}", timeline_lines.join("\n")))
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("タイムライン取得エラー: {}", e);
                                        None
                                    }
                                }
                            };
                            
                            // bot自身の発言をバッファに追加するためのチャネル
                            let (tx, mut rx) = tokio::sync::mpsc::channel::<TimelinePost>(1);
                            
                            tokio::spawn(async move {
                                // GPT応答（会話コンテキスト付き）
                                let reply = match gpt::get_reply_with_context(&prompt_clone, &content_clone, has_mention, context_clone).await {
                                    Ok(reply) => reply,
                                    Err(e) => {
                                        eprintln!("Error: {}", e);
                                        return;
                                    }
                                };

                                if reply.len() > 0 {
                                    println!("publish_text_note...{}", reply);
                                    
                                    // 返信を送信し、送信したイベントを取得
                                    let sent_event: Option<Event> = if has_mention {
                                        match util::reply_to(&config_clone, event_clone, person_clone.clone(), &reply).await {
                                            Ok(evt) => Some(evt),
                                            Err(e) => {
                                                eprintln!("Failed to reply: {}", e);
                                                None
                                            }
                                        }
                                    } else if event_clone.kind == Kind::TextNote {
                                        // send_toは Result<()>を返すので、まず送信してから手動でイベント作成
                                        let send_ok = util::send_to(&config_clone, event_clone, person_clone.clone(), &reply).await.is_ok();
                                        if send_ok {
                                            // 送信成功: イベントを手動で作成（kind 1, text note）
                                            if let Ok(bot_keys) = Keys::parse(&person_clone.secretkey) {
                                                let event_builder = EventBuilder::text_note(&reply);
                                                event_builder.sign(&bot_keys).await.ok()
                                            } else {
                                                None
                                            }
                                        } else {
                                            eprintln!("Failed to send");
                                            None
                                        }
                                    } else {
                                        None
                                    };
                                    
                                    // bot自身の発言をeventsテーブルとconversation_logsに記録
                                    if let Some(bot_event) = sent_event {
                                        // eventsテーブルに保存
                                        let event_type = if has_conversation_log { Some("bot_reply") } else { Some("bot_post") };
                                        match db::insert_event(&conn_clone, &bot_event, true, event_type) {
                                            Ok(event_ref_id) => {
                                                // メンション会話の場合は conversation_logs にも記録
                                                if has_conversation_log {
                                                    let event_json = serde_json::to_string(&bot_event).unwrap_or_default();
                                                    let thread_root_id = db::extract_thread_root_id(&event_json).ok().flatten();
                                                    
                                                    if let Err(e) = db::insert_conversation_log(
                                                        &conn_clone,
                                                        &person_clone.pubkey,
                                                        event_ref_id,
                                                        thread_root_id.as_deref(),
                                                        None, // mentioned_pubkeys: botの発言なので不要
                                                        true, // is_bot_message: botの発言
                                                        false, // is_bot_conversation: ここでは常にfalse
                                                    ) {
                                                        eprintln!("bot発言の会話ログ記録エラー: {}", e);
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                if !e.to_string().contains("UNIQUE constraint failed") {
                                                    eprintln!("bot発言の保存エラー: {}", e);
                                                }
                                            }
                                        }
                                    }
                                    
                                    // bot自身の発言をバッファに追加（旧システム互換用）
                                    let bot_post = TimelinePost {
                                        pubkey: person_clone.pubkey.clone(),
                                        name: Some("Bot".to_string()),
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
                                
                                // 古い投稿を削除（timeline_sizeと同じ数だけ保持）
                                let _ = db::cleanup_old_timeline_posts(&conn, config.bot.timeline_size);
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
