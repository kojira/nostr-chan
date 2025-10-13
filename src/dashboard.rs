use axum::{
    extract::State,
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;

/// ダッシュボードの状態
#[derive(Clone)]
pub struct DashboardState {
    pub stats: Arc<RwLock<Stats>>,
    pub config_path: String,
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
pub async fn start_dashboard(port: u16, config_path: String) -> Result<(), Box<dyn std::error::Error>> {
    let state = DashboardState {
        stats: Arc::new(RwLock::new(Stats::default())),
        config_path,
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
    let stats = state.stats.read().await;
    Json(stats.clone())
}

/// 統計を更新するヘルパー関数
pub async fn update_stats<F>(state: &DashboardState, updater: F)
where
    F: FnOnce(&mut Stats),
{
    let mut stats = state.stats.write().await;
    updater(&mut *stats);
}

