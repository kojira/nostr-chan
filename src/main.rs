mod config;
mod database;
mod gpt;
mod commands;
mod util;
mod embedding;
mod conversation;
mod dashboard;
mod init;
mod event_processor;
use database as db;
use chrono::Utc;
use dotenv::dotenv;
use nostr_sdk::prelude::*;
use std::{fs::File, str::FromStr};
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;

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
    
    // データベース初期化（テーブル作成とマイグレーション）
    db::initialize_db(&conn)?;
    
    // システム設定をconfig.ymlの値で初期化（DBに値がない場合のみ）
    init::initialize_system_settings(&conn, &config)?;

    
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

    // TextNote、ChannelMessage、Metadata (kind 0) をsubscribe
    let subscription = Filter::new()
        .kinds([nostr_sdk::Kind::TextNote, nostr_sdk::Kind::ChannelMessage, nostr_sdk::Kind::Metadata].to_vec())
        .since(Timestamp::now());

    let _ = client.subscribe(subscription, None).await;
    println!("subscribe (TextNote, ChannelMessage, Metadata)");
    
    // DBから既存のタイムラインを読み込み（起動時のみ）
    println!("Loading timeline from DB...");
    let timeline_size = config.get_usize_setting("timeline_size");
    let timeline_posts = db::get_latest_timeline_posts(&conn, timeline_size).unwrap_or_else(|e| {
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
    
    // イベント処理ワーカーを起動（別スレッドで実行）
    let config_for_worker = config.clone();
    let bot_info_for_worker = Arc::clone(&bot_info);
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
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
                
                // イベント処理を実行
                match event_processor::process_event(config_for_worker.clone(), Arc::clone(&bot_info_for_worker), event).await {
                    Ok(_) => {
                        // 処理成功: キューから削除
                        if let Err(e) = db::complete_queue_event(&conn_worker, queue_id) {
                            eprintln!("[Worker] キュー削除エラー: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("[Worker] イベント処理エラー: {} - キューから削除", e);
                        // エラーでも削除（無限ループ防止）
                        let _ = db::complete_queue_event(&conn_worker, queue_id);
                    }
                }
            }
        })
    });
    
    let mut notifications = client.notifications();
    println!("Listening for events...");
    
    while let Ok(notification) = notifications.recv().await {
        if let RelayPoolNotification::Event{relay_url: _, subscription_id: _, event} = notification {
            // ブラックリストチェック（DB優先）
            let blacklist = config.get_blacklist();
            if blacklist.iter().any(|s| s == &event.pubkey.to_string()) {
                continue;
            }
            
            let kind = event.kind;
            
            // kind 0 (Metadata) の処理
            if kind == Kind::Metadata {
                // eventsテーブルに保存（upsert処理で最新のみ保持）
                if let Err(e) = db::insert_event(&conn, &event, None) {
                    eprintln!("[Kind0] DB保存エラー: {}", e);
                }
                continue; // kind 0はキューに入れない
            }
            
            // NIP-36(コンテンツ警告)をスキップ
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

