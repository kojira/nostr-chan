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
use std::sync::Arc;
use tokio::sync::RwLock;
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
    
    // ダッシュボード用のBot情報を共有
    let bot_info = Arc::new(RwLock::new(dashboard::BotInfo {
        online: true,
        last_reply_timestamp: Utc::now().timestamp(),
        connected_relays: vec![],
    }));
    
    // ダッシュボードサーバーを起動（バックグラウンド）
    let dashboard_port = config.dashboard.port;
    let dashboard_db_path = "../nostrchan.db".to_string();
    let bot_info_clone = Arc::clone(&bot_info);
    tokio::spawn(async move {
        if let Err(e) = dashboard::start_dashboard(dashboard_port, dashboard_db_path, bot_info_clone).await {
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
    
    // ダッシュボードに接続リレー情報を更新
    {
        let mut info = bot_info.write().await;
        info.connected_relays = config.relay_servers.read.clone();
    }

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
    
    // 起動時に処理中だったイベントをpendingに戻す
    match db::reset_processing_events(&conn) {
        Ok(count) => {
            if count > 0 {
                println!("Reset {} processing events to pending", count);
            }
        }
        Err(e) => eprintln!("Failed to reset processing events: {}", e),
    }
    
    // イベント処理ワーカーを起動
    let config_worker = config.clone();
    let bot_info_worker = Arc::clone(&bot_info);
    tokio::spawn(async move {
        println!("Starting event queue worker...");
        loop {
            // 0.5秒ごとにキューをチェック
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            
            // キューから次のイベントを取得
            let conn_worker = match db::connect() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[Worker] DB接続エラー: {}", e);
                    continue;
                }
            };
            
            let queue_item = match db::dequeue_event(&conn_worker) {
                Ok(Some(item)) => item,
                Ok(None) => continue, // キューが空
                Err(e) => {
                    eprintln!("[Worker] キュー取得エラー: {}", e);
                    continue;
                }
            };
            
            let (queue_id, event_json) = queue_item;
            
            // JSONからEventを復元
            let event: nostr_sdk::Event = match serde_json::from_str(&event_json) {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("[Worker] イベント復元エラー: {}", e);
                    let _ = db::complete_queue_event(&conn_worker, queue_id);
                    continue;
                }
            };
            
            // TODO: イベント処理を実行（次のコミットで実装）
            println!("[Worker] Processing event: {}", event.id);
            
            // 処理完了: キューから削除
            if let Err(e) = db::complete_queue_event(&conn_worker, queue_id) {
                eprintln!("[Worker] キュー削除エラー: {}", e);
            }
        }
    });
    
    let mut notifications = client.notifications();
    println!("Listening for events...");
    
    while let Ok(notification) = notifications.recv().await {
        if let RelayPoolNotification::Event{relay_url: _, subscription_id: _, event} = notification {
            // ブラックリストチェック
            if config.bot.blacklist.iter().any(|s| s == &event.pubkey.to_string()) {
                continue;
            }
            // NIP-36(コンテンツ警告)をスキップ
            let kind = event.kind;
            if kind == Kind::TextNote || kind == Kind::ChannelMessage {
                let mut detect_nip36 = false;
                for tag in event.tags.clone().into_iter() {
                    if let Some(TagStandard::ContentWarning { reason: _ }) = tag.as_standardized() {
                        detect_nip36 = true;
                        break;
                    }
                }
                if detect_nip36 {
                    continue;
                }
                
                // コマンド処理（即座に実行）
                let persons = match db::get_all_persons(&conn) {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("Failed to get persons: {}", e);
                        continue;
                    }
                };
                let handled = commands::command_handler(&config, &conn, &persons, &event).await?;
                
                if handled {
                    // コマンドとして処理済み: キューに入れない
                    continue;
                }
                
                // イベントをキューに追加（永続化）
                let event_json = match serde_json::to_string(&*event) {
                    Ok(json) => json,
                    Err(e) => {
                        eprintln!("Failed to serialize event: {}", e);
                        continue;
                    }
                };
                
                match db::enqueue_event(&conn, &event_json) {
                    Ok(queue_id) => {
                        println!("Enqueued event {} (queue_id: {})", event.id, queue_id);
                    }
                    Err(e) => {
                        eprintln!("Failed to enqueue event: {}", e);
                    }
                }
            } else {
                // Kind::TextNoteでもKind::ChannelMessageでもない
                println!("Skipping event kind: {:?}", event.kind);
            }
        }
    }

    Ok(())
}
