use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use chrono::Utc;

/// ダッシュボードの状態
#[derive(Clone)]
pub struct DashboardState {
    #[allow(dead_code)]
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

/// Bot管理用のデータ型
#[derive(Debug, Serialize, Deserialize)]
pub struct BotData {
    pub pubkey: String,
    pub secretkey: String,
    pub prompt: String,
    pub content: String,
    pub status: i32,
}

#[derive(Debug, Deserialize)]
pub struct BotRequest {
    pub secretkey: String,
    pub prompt: String,
    pub content: String,
}

