use axum::{
    extract::State,
    response::Json,
    http::StatusCode,
};
use super::types::{DashboardState, Stats};
use crate::db;
use std::collections::HashMap;

/// 統計情報を取得
pub async fn stats_handler(
    State(state): State<DashboardState>,
) -> Result<Json<Stats>, StatusCode> {
    let conn = db::connect().map_err(|e| {
        eprintln!("DB接続エラー: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    // DBから統計情報を取得
    let db_stats = db::get_dashboard_stats(&conn).map_err(|e| {
        eprintln!("統計情報取得エラー: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    // bot_infoから実行時情報を取得
    let bot_info = state.bot_info.read().await;
    let uptime = state.start_time.elapsed().as_secs();
    
    let stats = Stats {
        bot_status: super::types::BotStatus {
            online: bot_info.online,
            uptime_seconds: uptime,
            last_reply_timestamp: bot_info.last_reply_timestamp,
            connected_relays: bot_info.connected_relays.clone(),
        },
        reply_stats: super::types::ReplyStats {
            today: db_stats.replies_today,
            this_week: db_stats.replies_week,
            this_month: db_stats.replies_month,
            total: db_stats.replies_total,
        },
        conversation_stats: super::types::ConversationStats {
            unique_users: db_stats.unique_users,
            rate_limited_users: db_stats.rate_limited_users,
        },
        rag_stats: super::types::RagStats {
            vectorized_events: db_stats.vectorized_events,
            total_events: db_stats.total_events,
            pending_vectorization: db_stats.pending_vectorization,
            total_searches: db_stats.total_searches,
            average_similarity: db_stats.average_similarity as f32,
        },
        error_log: vec![],
    };
    
    Ok(Json(stats))
}

/// Bot毎の日別返信数を取得（過去30日分）
pub async fn daily_replies_handler(
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
    let mut bot_data: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
    
    for (bot_pubkey, date, count) in results {
        bot_data
            .entry(bot_pubkey)
            .or_insert_with(Vec::new)
            .push(serde_json::json!({
                "date": date,
                "count": count
            }));
    }
    
    Ok(Json(serde_json::json!({ "data": bot_data })))
}

