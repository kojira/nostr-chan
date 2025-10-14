mod config;
mod db;
mod gpt;
mod commands;
mod util;
mod embedding;
mod conversation;
mod dashboard;
mod init;
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

    let subscription = Filter::new()
        .kinds([nostr_sdk::Kind::TextNote, nostr_sdk::Kind::ChannelMessage].to_vec())
        .since(Timestamp::now());

    let _ = client.subscribe(subscription, None).await;
    println!("subscribe");
    
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
                match process_event(config_for_worker.clone(), Arc::clone(&bot_info_for_worker), event).await {
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

/// キューから取り出したイベントを処理する
async fn process_event(
    config: config::AppConfig,
    bot_info: Arc<RwLock<dashboard::BotInfo>>,
    event: Event,
) -> Result<(), Box<dyn std::error::Error>> {
    use whatlang::{detect, Lang};
    use chrono::TimeZone;
    
    // DB接続
    let conn = db::connect()?;
    
    // 言語判定
    let japanese = if let Some(lang) = detect(&event.content) {
        matches!(lang.lang(), Lang::Jpn)
    } else {
        false
    };
    
    // イベントをeventsテーブルに保存
    if event.kind == Kind::TextNote {
        let event_type = if japanese { Some("air_reply") } else { None };
        match db::insert_event(&conn, &event, japanese, event_type) {
            Ok(event_ref_id) => {
                // kind 0情報を非同期で取得して更新
                let conn_clone = db::connect()?;
                let event_id = event.id.to_string();
                let pubkey = event.pubkey.to_string();
                tokio::spawn(async move {
                    if let Ok(name) = util::get_user_name(&pubkey).await {
                        let _ = db::update_event_kind0(&conn_clone, &event_id, Some(&name), None);
                    }
                });
                let _ = event_ref_id;
            }
            Err(e) => {
                if !e.to_string().contains("UNIQUE constraint failed") {
                    eprintln!("[Worker] Failed to save event: {}", e);
                }
            }
        }
    }
    
    // 処理判定
    if event.content.is_empty() {
        return Ok(());
    }
    
    // personsを取得
    let persons = db::get_all_persons(&conn)?;
    
    // 有効なBotのみをフィルタ（status == 0）
    let active_persons: Vec<db::Person> = persons.iter()
        .filter(|p| p.status == 0)
        .cloned()
        .collect();
    
    if active_persons.is_empty() {
        return Ok(());
    }
    
    // メンション判定（有効なBotのみ）
    let person_op = util::extract_mention(active_persons.clone(), &event)?;
    let has_mention = person_op.is_some();
    
    // エアリプは日本語のみ
    if !has_mention && !japanese {
        return Ok(());
    }
    
    // 確率判定（有効なBotのみ）
    let (mut should_post, _) = util::judge_post(&config, active_persons.clone(), &event)?;
    
    // Personを決定
    let person = if let Some(p) = person_op {
        p
    } else {
        // ランダム選択も有効なBotから
        if active_persons.is_empty() {
            return Ok(());
        }
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        active_persons.choose(&mut rng).unwrap().clone()
    };
    
    // Botのステータスチェック（無効化されていたらスキップ）
    if person.status != 0 {
        println!("🚫 Bot無効化中のため、返信をスキップ: {} ({})", person.pubkey, event.id);
        return Ok(());
    }
    
    // メンションの場合は必ず返信
    if has_mention {
        should_post = true;
    }
    
    if !should_post {
        return Ok(());
    }
    
    // グローバル一時停止チェック（返信処理の直前）
    if db::is_global_pause(&conn)? {
        println!("⏸️ グローバル一時停止中のため、返信をスキップ: {}", event.id);
        return Ok(());
    }
    
    // フォロワーチェック
    let is_follower = util::is_follower(&event.pubkey.to_string(), &person.secretkey).await?;
    if !is_follower {
        println!("[Worker] Not a follower, skipping");
        return Ok(());
    }
    
    // 会話回数制限チェック（メンション時のみ）
    if has_mention {
        let conversation_count = db::get_conversation_count_with_user(
            &conn,
            &person.pubkey,
            &event.pubkey.to_string(),
            config.bot.conversation_limit_minutes,
        )?;
        
        if conversation_count >= config.bot.conversation_limit_count {
            println!(
                "[Worker] 会話回数制限: {}分間で{}回 (制限: {}回)",
                config.bot.conversation_limit_minutes,
                conversation_count,
                config.bot.conversation_limit_count
            );
            return Ok(());
        }
    }
    
    // 会話ログに記録（メンション時のみ）
    let conversation_log_id = if has_mention {
        let event_record = db::get_event_by_event_id(&conn, &event.id.to_string())?;
        let event_ref_id = if let Some(record) = event_record {
            record.id
        } else {
            let event_type = Some("mention");
            db::insert_event(&conn, &event, japanese, event_type)?
        };
        
        let event_json = serde_json::to_string(&event)?;
        let mentioned_pubkeys = db::extract_mentioned_pubkeys(&event_json).ok();
        let thread_root_id = db::extract_thread_root_id(&event_json).ok().flatten();
        
        let all_bot_pubkeys: Vec<String> = active_persons.iter().map(|p| p.pubkey.clone()).collect();
        let is_bot_conversation = if let Some(ref pks) = mentioned_pubkeys {
            db::detect_bot_conversation(pks, &all_bot_pubkeys)
        } else {
            false
        };
        
        match db::insert_conversation_log(
            &conn,
            &person.pubkey,
            event_ref_id,
            thread_root_id.as_deref(),
            mentioned_pubkeys.as_ref().map(|v| v.as_slice()),
            false,
            is_bot_conversation,
        ) {
            Ok(log_id) => Some(log_id),
            Err(e) => {
                eprintln!("[Worker] 会話ログ記録エラー: {}", e);
                None
            }
        }
    } else {
        None
    };
    
    let has_conversation_log = conversation_log_id.is_some();
    
    // プロンプト準備
    let mut prompt = person.prompt.clone();
    let user_pubkey = event.pubkey.to_string();
    
    if has_mention {
        if let Ok(user_name) = util::get_user_name(&user_pubkey).await {
            if !user_name.ends_with("...") {
                prompt = format!("{}。話しかけてきた相手の名前は「{}」です。", prompt, user_name);
            }
        }
    }
    
    // 会話コンテキストを準備
    let context = if has_conversation_log {
        match conversation::prepare_context_for_reply(
            &conn,
            &person.pubkey,
            &user_pubkey,
            &event.content,
            50,
        ).await {
            Ok(ctx) => {
                if ctx.is_empty() {
                    None
                } else {
                    Some(ctx)
                }
            },
            Err(e) => {
                eprintln!("[Worker] コンテキスト準備エラー: {}", e);
                None
            }
        }
    } else {
        match conversation::build_japanese_timeline_for_air_reply(&conn, config.bot.timeline_size) {
            Ok(events) => {
                if events.is_empty() {
                    None
                } else {
                    let timeline_lines: Vec<String> = events.iter()
                        .enumerate()
                        .map(|(i, ev)| {
                            let dt = chrono::Local.timestamp_opt(ev.created_at, 0).single().unwrap();
                            let time_str = dt.format("%m/%d %H:%M").to_string();
                            let display_name = ev.kind0_name.clone().unwrap_or_else(|| {
                                if ev.pubkey.len() > 8 {
                                    format!("{}...", &ev.pubkey[..8])
                                } else {
                                    ev.pubkey.clone()
                                }
                            });
                            format!("{}. [{}] {}: {}", i + 1, time_str, display_name, ev.content)
                        })
                        .collect();
                    Some(format!("【タイムライン】\n{}", timeline_lines.join("\n")))
                }
            }
            Err(e) => {
                eprintln!("[Worker] タイムライン取得エラー: {}", e);
                None
            }
        }
    };
    
    // GPT応答生成
    let reply = gpt::get_reply_with_context(&prompt, &event.content, has_mention, context).await?;
    
    if reply.is_empty() {
        return Ok(());
    }
    
    println!("[Worker] Replying: {}", reply);
    
    // 返信送信
    let sent_event = if has_mention {
        match util::reply_to(&config, event.clone(), person.clone(), &reply).await {
            Ok(evt) => {
                // ダッシュボードに最終返信時刻を更新
                {
                    let mut info = bot_info.write().await;
                    info.last_reply_timestamp = Utc::now().timestamp();
                }
                Some(evt)
            }
            Err(e) => {
                eprintln!("[Worker] Failed to reply: {}", e);
                None
            }
        }
    } else if event.kind == Kind::TextNote {
        let send_ok = util::send_to(&config, event.clone(), person.clone(), &reply).await.is_ok();
        if send_ok {
            if let Ok(bot_keys) = Keys::parse(&person.secretkey) {
                let event_builder = EventBuilder::text_note(&reply);
                event_builder.sign(&bot_keys).await.ok()
            } else {
                None
            }
        } else {
            eprintln!("[Worker] Failed to send");
            None
        }
    } else {
        None
    };
    
    // bot自身の発言を記録
    if let Some(bot_event) = sent_event {
        let event_type = if has_conversation_log { Some("bot_reply") } else { Some("bot_post") };
        match db::insert_event(&conn, &bot_event, true, event_type) {
            Ok(event_ref_id) => {
                if has_conversation_log {
                    let event_json = serde_json::to_string(&bot_event).unwrap_or_default();
                    let thread_root_id = db::extract_thread_root_id(&event_json).ok().flatten();
                    
                    if let Err(e) = db::insert_conversation_log(
                        &conn,
                        &person.pubkey,
                        event_ref_id,
                        thread_root_id.as_deref(),
                        None,
                        true,
                        false,
                    ) {
                        eprintln!("[Worker] bot発言の会話ログ記録エラー: {}", e);
                    }
                }
            }
            Err(e) => {
                if !e.to_string().contains("UNIQUE constraint failed") {
                    eprintln!("[Worker] bot発言の保存エラー: {}", e);
                }
            }
        }
        
        // タイムラインに追加
        if let Err(e) = db::add_timeline_post(
            &conn,
            &person.pubkey,
            Some("Bot"),
            &reply,
            Utc::now().timestamp()
        ) {
            eprintln!("[Worker] Failed to save bot timeline post: {}", e);
        }
        
        let _ = db::cleanup_old_timeline_posts(&conn, config.bot.timeline_size);
    }
    
    Ok(())
}
