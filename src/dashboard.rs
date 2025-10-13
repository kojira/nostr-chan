use axum::{
    extract::{State, Path},
    response::{Html, IntoResponse, Json},
    routing::{get, post, put, delete},
    Router,
    http::StatusCode,
};
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
    pub active_conversations: u32,
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
                active_conversations: 0,
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

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/api/stats", get(stats_handler))
        .route("/api/bots", get(list_bots_handler))
        .route("/api/bots", post(create_bot_handler))
        .route("/api/bots/:pubkey", put(update_bot_handler))
        .route("/api/bots/:pubkey", delete(delete_bot_handler))
        .route("/api/bots/:pubkey/toggle", post(toggle_bot_handler))
        .with_state(state);

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
            active_conversations: db_stats.active_conversations,
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
    pub status: i32, // 0: active, 1: inactive
}

/// Bot作成/更新リクエスト
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotRequest {
    pub secretkey: String,
    pub prompt: String,
}

/// Bot一覧取得
async fn list_bots_handler(State(_state): State<DashboardState>) -> Result<Json<Vec<BotData>>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let persons = db::get_all_persons(&conn).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let bots: Vec<BotData> = persons.into_iter().map(|p| BotData {
        pubkey: p.pubkey,
        secretkey: p.secretkey,
        prompt: p.prompt,
        status: p.status,
    }).collect();
    
    Ok(Json(bots))
}

/// Bot作成
async fn create_bot_handler(
    State(_state): State<DashboardState>,
    Json(req): Json<BotRequest>,
) -> Result<Json<BotData>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // secretkeyからpubkeyを取得
    use nostr_sdk::Keys;
    let keys = Keys::parse(&req.secretkey).map_err(|_| StatusCode::BAD_REQUEST)?;
    let pubkey = keys.public_key().to_string();
    
    // DBに追加
    db::add_person(&conn, &pubkey, &req.secretkey, &req.prompt)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(BotData {
        pubkey,
        secretkey: req.secretkey,
        prompt: req.prompt,
        status: 0,
    }))
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
    db::update_person(&conn, &pubkey, &req.secretkey, &req.prompt)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(BotData {
        pubkey,
        secretkey: req.secretkey,
        prompt: req.prompt,
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
        status: new_status,
    }))
}

