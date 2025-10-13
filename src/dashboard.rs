use axum::{
    extract::{State, Path},
    response::{Html, IntoResponse, Json},
    routing::{get, post, put, delete},
    Router,
    http::StatusCode,
};
use tower_http::services::ServeDir;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use chrono::Utc;
use crate::db;

/// ダッシュボードの状態
#[derive(Clone)]
pub struct DashboardState {
    pub db_path: String,
    pub start_time: Arc<Instant>,
    pub bot_info: Arc<RwLock<BotInfo>>,
}

/// Bot実行情報
#[derive(Debug, Clone)]
pub struct BotInfo {
    pub online: bool,
    pub last_reply_timestamp: i64,
    pub connected_relays: Vec<String>,
}

/// 統計情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub bot_status: BotStatus,
    pub reply_stats: ReplyStats,
    pub conversation_stats: ConversationStats,
    pub rag_stats: RagStats,
    pub error_log: Vec<ErrorEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotStatus {
    pub online: bool,
    pub uptime_seconds: u64,
    pub last_reply_timestamp: i64,
    pub connected_relays: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplyStats {
    pub today: u32,
    pub this_week: u32,
    pub this_month: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationStats {
    pub unique_users: u32,
    pub rate_limited_users: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagStats {
    pub vectorized_events: u32,
    pub total_events: u32,
    pub pending_vectorization: u32,
    pub total_searches: u32,
    pub average_similarity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEntry {
    pub timestamp: i64,
    pub error_type: String,
    pub message: String,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            bot_status: BotStatus {
                online: true,
                uptime_seconds: 0,
                last_reply_timestamp: Utc::now().timestamp(),
                connected_relays: vec![],
            },
            reply_stats: ReplyStats {
                today: 0,
                this_week: 0,
                this_month: 0,
                total: 0,
            },
            conversation_stats: ConversationStats {
                unique_users: 0,
                rate_limited_users: 0,
            },
            rag_stats: RagStats {
                vectorized_events: 0,
                total_events: 0,
                pending_vectorization: 0,
                total_searches: 0,
                average_similarity: 0.0,
            },
            error_log: vec![],
        }
    }
}

/// ダッシュボードサーバーを起動
pub async fn start_dashboard(
    port: u16,
    db_path: String,
    bot_info: Arc<RwLock<BotInfo>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = DashboardState {
        db_path,
        start_time: Arc::new(Instant::now()),
        bot_info,
    };

    // APIルート
    let api_router = Router::new()
        .route("/api/stats", get(stats_handler))
        .route("/api/bots", get(list_bots_handler))
        .route("/api/bots", post(create_bot_handler))
        .route("/api/bots/generate-key", get(generate_key_handler))
        .route("/api/bots/:pubkey", put(update_bot_handler))
        .route("/api/bots/:pubkey", delete(delete_bot_handler))
        .route("/api/bots/:pubkey/toggle", post(toggle_bot_handler))
        .route("/api/bots/:pubkey/kind0", get(fetch_kind0_handler))
        .route("/api/bots/:pubkey/post", post(post_as_bot_handler))
        .route("/api/global-pause", get(get_global_pause_handler))
        .route("/api/global-pause", post(set_global_pause_handler))
        .route("/api/analytics/daily-replies", get(daily_replies_handler))
        .with_state(state);

    // 静的ファイル配信 + APIルート
    // プロジェクトルートからの絶対パス（CARGO_MANIFEST_DIRを使用）
    let project_root = std::env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|_| ".".to_string());
    let dashboard_path = std::path::PathBuf::from(project_root).join("dashboard");
    
    println!("📁 Dashboard path: {:?}", dashboard_path);
    println!("📁 Assets path: {:?}", dashboard_path.join("assets"));
    
    // ファイル存在確認
    let assets_path = dashboard_path.join("assets");
    if assets_path.exists() {
        println!("✅ Assets directory exists");
        if let Ok(entries) = std::fs::read_dir(&assets_path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    println!("  📄 {}", entry.file_name().to_string_lossy());
                }
            }
        }
    } else {
        println!("❌ Assets directory NOT found!");
    }
    
    let app = Router::new()
        .route("/", get(index_handler))
        .merge(api_router)
        .nest_service("/assets", ServeDir::new(assets_path));

    let addr = format!("127.0.0.1:{}", port);
    println!("📊 Dashboard starting on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// トップページ
async fn index_handler() -> Html<&'static str> {
    Html(include_str!("../dashboard/index.html"))
}

/// 統計API
async fn stats_handler(State(state): State<DashboardState>) -> impl IntoResponse {
    // DBから統計データを取得
    let conn = match rusqlite::Connection::open(&state.db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[Dashboard] DB接続エラー: {}", e);
            return Json(Stats::default());
        }
    };
    
    let db_stats = match db::get_dashboard_stats(&conn) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[Dashboard] 統計取得エラー: {}", e);
            return Json(Stats::default());
        }
    };
    
    // Bot情報を取得
    let bot_info = state.bot_info.read().await;
    let uptime = state.start_time.elapsed().as_secs();
    
    let stats = Stats {
        bot_status: BotStatus {
            online: bot_info.online,
            uptime_seconds: uptime,
            last_reply_timestamp: bot_info.last_reply_timestamp,
            connected_relays: bot_info.connected_relays.clone(),
        },
        reply_stats: ReplyStats {
            today: db_stats.replies_today,
            this_week: db_stats.replies_week,
            this_month: db_stats.replies_month,
            total: db_stats.replies_total,
        },
        conversation_stats: ConversationStats {
            unique_users: db_stats.unique_users,
            rate_limited_users: db_stats.rate_limited_users,
        },
        rag_stats: RagStats {
            vectorized_events: db_stats.vectorized_events,
            total_events: db_stats.total_events,
            pending_vectorization: db_stats.pending_vectorization,
            total_searches: db_stats.total_searches,
            average_similarity: db_stats.average_similarity as f32,
        },
        error_log: vec![], // エラーログは将来の拡張用に空配列を返す
    };
    
    Json(stats)
}

/// Bot情報（API用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotData {
    pub pubkey: String,
    pub secretkey: String,
    pub prompt: String,
    pub content: String,
    pub status: i32, // 0: active, 1: inactive
}

/// Bot作成/更新リクエスト
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotRequest {
    pub secretkey: String,
    pub prompt: String,
    pub content: String,
}

/// Bot一覧取得
async fn list_bots_handler(State(_state): State<DashboardState>) -> Result<Json<Vec<BotData>>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let persons = db::get_all_persons(&conn).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let bots: Vec<BotData> = persons.into_iter().map(|p| BotData {
        pubkey: p.pubkey,
        secretkey: p.secretkey,
        prompt: p.prompt,
        content: p.content,
        status: p.status,
    }).collect();
    
    Ok(Json(bots))
}

/// Bot作成
async fn create_bot_handler(
    State(_state): State<DashboardState>,
    Json(req): Json<BotRequest>,
) -> Result<Json<BotData>, StatusCode> {
    use nostr_sdk::prelude::*;
    
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // secretkeyからpubkeyを取得
    let keys = Keys::parse(&req.secretkey).map_err(|_| StatusCode::BAD_REQUEST)?;
    let pubkey = keys.public_key().to_string();
    
    // DBに追加
    db::add_person(&conn, &pubkey, &req.secretkey, &req.prompt, &req.content)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // 誕生投稿を非同期で送信
    let secretkey = req.secretkey.clone();
    let content = req.content.clone();
    tokio::spawn(async move {
        if let Err(e) = post_birth_announcement(&secretkey, &content).await {
            eprintln!("誕生投稿エラー: {}", e);
        }
    });
    
    Ok(Json(BotData {
        pubkey,
        secretkey: req.secretkey,
        prompt: req.prompt,
        content: req.content,
        status: 0,
    }))
}

/// 誕生投稿
async fn post_birth_announcement(secretkey: &str, content_json: &str) -> Result<(), Box<dyn std::error::Error>> {
    use nostr_sdk::prelude::*;
    
    // Botの名前を取得
    let bot_name = if !content_json.is_empty() {
        match serde_json::from_str::<serde_json::Value>(content_json) {
            Ok(json) => {
                json["display_name"].as_str()
                    .or_else(|| json["name"].as_str())
                    .unwrap_or("新しいBot")
                    .to_string()
            }
            Err(_) => "新しいBot".to_string()
        }
    } else {
        "新しいBot".to_string()
    };
    
    let keys = Keys::parse(secretkey)?;
    let client = Client::new(keys);
    
    // config.ymlから設定を読み込む
    let config_path = "../config.yml";
    let file = std::fs::File::open(config_path)?;
    let config: crate::config::AppConfig = serde_yaml::from_reader(file)?;
    
    // リレーに接続
    for relay in &config.relay_servers.write {
        let _ = client.add_relay(relay).await;
    }
    
    client.connect().await;
    
    // 誕生メッセージを投稿（adminコマンドと同じ文面）
    let message = format!("{}です。コンゴトモヨロシク！", bot_name);
    
    let builder = EventBuilder::text_note(message);
    client.send_event_builder(builder).await?;
    
    println!("✨ {}の誕生投稿を送信しました", bot_name);
    
    Ok(())
}

/// Bot更新
async fn update_bot_handler(
    State(_state): State<DashboardState>,
    Path(pubkey): Path<String>,
    Json(req): Json<BotRequest>,
) -> Result<Json<BotData>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // 既存のbotを取得
    let persons = db::get_all_persons(&conn).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let existing = persons.iter().find(|p| p.pubkey == pubkey).ok_or(StatusCode::NOT_FOUND)?;
    
    // 更新
    db::update_person(&conn, &pubkey, &req.secretkey, &req.prompt, &req.content)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(BotData {
        pubkey,
        secretkey: req.secretkey,
        prompt: req.prompt,
        content: req.content,
        status: existing.status,
    }))
}

/// Bot削除
async fn delete_bot_handler(
    State(_state): State<DashboardState>,
    Path(pubkey): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    db::delete_person(&conn, &pubkey)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(StatusCode::NO_CONTENT)
}

/// Bot有効/無効切り替え
async fn toggle_bot_handler(
    State(_state): State<DashboardState>,
    Path(pubkey): Path<String>,
) -> Result<Json<BotData>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // 既存のbotを取得
    let persons = db::get_all_persons(&conn).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let existing = persons.iter().find(|p| p.pubkey == pubkey).ok_or(StatusCode::NOT_FOUND)?;
    
    // statusを切り替え
    let new_status = if existing.status == 0 { 1 } else { 0 };
    db::update_person_status(&conn, &pubkey, new_status)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(BotData {
        pubkey: existing.pubkey.clone(),
        secretkey: existing.secretkey.clone(),
        prompt: existing.prompt.clone(),
        content: existing.content.clone(),
        status: new_status,
    }))
}

/// グローバル一時停止状態の取得
async fn get_global_pause_handler(
    State(_state): State<DashboardState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let paused = db::is_global_pause(&conn).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(serde_json::json!({ "paused": paused })))
}

/// グローバル一時停止状態の設定
async fn set_global_pause_handler(
    State(_state): State<DashboardState>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let paused = req["paused"].as_bool().ok_or(StatusCode::BAD_REQUEST)?;
    
    let value = if paused { "true" } else { "false" };
    db::set_system_setting(&conn, "global_pause", value).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    println!("🔔 グローバル一時停止: {}", if paused { "有効" } else { "無効" });
    
    Ok(Json(serde_json::json!({ "paused": paused })))
}

/// Bot毎の日別返信数を取得（過去30日分）
async fn daily_replies_handler(
    State(_state): State<DashboardState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db::connect().map_err(|e| {
        eprintln!("DB接続エラー: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let results = db::get_bot_daily_reply_counts(&conn, 30).map_err(|e| {
        eprintln!("日別返信数取得エラー: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    // Bot別にグループ化してJSON形式に変換
    use std::collections::HashMap;
    let mut bot_data: HashMap<String, Vec<(String, i64)>> = HashMap::new();
    
    for (bot_pubkey, date, count) in results {
        bot_data
            .entry(bot_pubkey)
            .or_insert_with(Vec::new)
            .push((date, count));
    }
    
    Ok(Json(serde_json::json!({ "data": bot_data })))
}

/// ランダムな秘密鍵を生成
async fn generate_key_handler(
    State(_state): State<DashboardState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use nostr_sdk::prelude::*;
    
    // 新しいKeysを生成
    let keys = Keys::generate();
    let secret_key = keys.secret_key().to_secret_hex();
    
    Ok(Json(serde_json::json!({ 
        "secretkey": secret_key 
    })))
}

/// Botとして投稿
#[derive(Debug, Deserialize)]
struct PostRequest {
    content: String,
}

async fn post_as_bot_handler(
    State(_state): State<DashboardState>,
    Path(pubkey): Path<String>,
    Json(req): Json<PostRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use nostr_sdk::prelude::*;
    
    // DBから対象BotのSecretKeyを取得
    let conn = db::connect().map_err(|e| {
        eprintln!("DB接続エラー: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let persons = db::get_all_persons(&conn).map_err(|e| {
        eprintln!("Bot情報取得エラー: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let bot = persons.iter().find(|p| p.pubkey == pubkey)
        .ok_or_else(|| {
            eprintln!("Botが見つかりません: {}", pubkey);
            StatusCode::NOT_FOUND
        })?;
    
    // Keysを生成
    let keys = Keys::parse(&bot.secretkey).map_err(|e| {
        eprintln!("秘密鍵のパースエラー: {}", e);
        StatusCode::BAD_REQUEST
    })?;
    
    // Clientを作成してリレーに接続
    let client = Client::new(keys);
    
    // config.ymlから設定を読み込む
    let config_path = "../config.yml";
    let file = std::fs::File::open(config_path).map_err(|e| {
        eprintln!("設定ファイルオープンエラー: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let config: crate::config::AppConfig = serde_yaml::from_reader(file).map_err(|e| {
        eprintln!("設定ファイルパースエラー: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    // リレーに接続
    for relay in &config.relay_servers.write {
        if let Err(e) = client.add_relay(relay).await {
            eprintln!("リレー追加エラー ({}): {}", relay, e);
        }
    }
    
    client.connect().await;
    
    // 投稿を送信
    let builder = EventBuilder::text_note(&req.content);
    let event_id = client.send_event_builder(builder)
        .await
        .map_err(|e| {
            eprintln!("投稿送信エラー: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    println!("📝 {}として投稿しました: {}", pubkey, req.content);
    
    Ok(Json(serde_json::json!({ 
        "success": true,
        "event_id": event_id.to_string()
    })))
}

/// リレーからKind 0を取得
async fn fetch_kind0_handler(
    State(_state): State<DashboardState>,
    Path(pubkey): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use nostr_sdk::prelude::*;
    
    // DBから対象BotのSecretKeyを取得
    let conn = db::connect().map_err(|e| {
        eprintln!("DB接続エラー: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let persons = db::get_all_persons(&conn).map_err(|e| {
        eprintln!("Bot情報取得エラー: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let bot = persons.iter().find(|p| p.pubkey == pubkey)
        .ok_or_else(|| {
            eprintln!("Botが見つかりません: {}", pubkey);
            StatusCode::NOT_FOUND
        })?;
    
    // Keysを生成
    let keys = Keys::parse(&bot.secretkey).map_err(|e| {
        eprintln!("秘密鍵のパースエラー: {}", e);
        StatusCode::BAD_REQUEST
    })?;
    
    // Clientを作成してリレーに接続
    let client = Client::new(keys);
    
    // config.ymlから設定を読み込む
    let config_path = "../config.yml";
    let file = std::fs::File::open(config_path).map_err(|e| {
        eprintln!("設定ファイルオープンエラー: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let config: crate::config::AppConfig = serde_yaml::from_reader(file).map_err(|e| {
        eprintln!("設定ファイルパースエラー: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    // リレーに接続
    for relay in &config.relay_servers.read {
        if let Err(e) = client.add_relay(relay).await {
            eprintln!("リレー追加エラー ({}): {}", relay, e);
        }
    }
    
    client.connect().await;
    
    // 自分のKind 0を取得
    let signer = client.signer().await
        .map_err(|e| {
            eprintln!("Signer取得エラー: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    let pubkey = signer.get_public_key().await
        .map_err(|e| {
            eprintln!("公開鍵取得エラー: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    let filter = Filter::new()
        .kind(Kind::Metadata)
        .author(pubkey);
    
    let events = client.fetch_events(filter, std::time::Duration::from_secs(10))
        .await
        .map_err(|e| {
            eprintln!("Kind 0取得エラー: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // 最新のKind 0を取得
    let latest_event = events.iter()
        .max_by_key(|e| e.created_at);
    
    if let Some(event) = latest_event {
        Ok(Json(serde_json::json!({ 
            "content": event.content.clone() 
        })))
    } else {
        Ok(Json(serde_json::json!({ 
            "content": "" 
        })))
    }
}

