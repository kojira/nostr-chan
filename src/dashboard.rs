use axum::{
    extract::State,
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
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
            rate_limited_users: 0, // TODO: 実装
        },
        rag_stats: RagStats {
            vectorized_events: db_stats.vectorized_events,
            total_events: db_stats.total_events,
            pending_vectorization: db_stats.pending_vectorization,
            total_searches: 0, // TODO: 実装
            average_similarity: 0.0, // TODO: 実装
        },
        error_log: vec![], // TODO: 実装
    };
    
    Json(stats)
}

