use axum::{
    extract::{State, Query},
    response::Json,
    http::StatusCode,
};
use super::types::{DashboardState, Stats};
use crate::database as db;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

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

/// トークン使用量の統計を取得
pub async fn token_usage_stats_handler(
    State(_state): State<DashboardState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db::connect().map_err(|e| {
        eprintln!("DB接続エラー: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let daily_usage = if let (Some(from), Some(to)) = (params.get("from"), params.get("to")) {
        // 日付範囲指定
        db::get_daily_token_usage_with_range(&conn, from, to).map_err(|e| {
            eprintln!("トークン使用量取得エラー: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    } else {
        // 日数指定（デフォルト7日）
        let days = params.get("days")
            .and_then(|d| d.parse::<i64>().ok())
            .unwrap_or(7)
            .clamp(1, 365);
        
        db::get_daily_token_usage(&conn, days).map_err(|e| {
            eprintln!("トークン使用量取得エラー: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    };
    
    Ok(Json(serde_json::json!({ "data": daily_usage })))
}

#[derive(Deserialize)]
pub struct TokenDetailsQuery {
    limit: Option<usize>,
    offset: Option<usize>,
}

#[derive(Serialize)]
pub struct TokenDetail {
    id: i64,
    bot_pubkey: String,
    category_name: String,
    category_display_name: String,
    prompt_tokens: i64,
    completion_tokens: i64,
    total_tokens: i64,
    prompt_text: String,
    completion_text: String,
    created_at: i64,
}

/// トークン使用量の詳細を取得
pub async fn token_details_handler(
    State(_state): State<DashboardState>,
    Query(query): Query<TokenDetailsQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db::connect().map_err(|e| {
        eprintln!("DB接続エラー: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);
    
    // トークン使用量の詳細を取得
    let mut stmt = conn.prepare(
        "SELECT tu.id, tu.bot_pubkey, tc.name, tc.display_name, 
                tu.prompt_tokens, tu.completion_tokens, tu.total_tokens,
                tu.prompt_text, tu.completion_text, tu.created_at
         FROM token_usage tu
         INNER JOIN token_categories tc ON tu.category_id = tc.id
         ORDER BY tu.created_at DESC
         LIMIT ? OFFSET ?"
    ).map_err(|e| {
        eprintln!("クエリ準備エラー: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let details = stmt.query_map(rusqlite::params![limit, offset], |row| {
        Ok(TokenDetail {
            id: row.get(0)?,
            bot_pubkey: row.get(1)?,
            category_name: row.get(2)?,
            category_display_name: row.get(3)?,
            prompt_tokens: row.get(4)?,
            completion_tokens: row.get(5)?,
            total_tokens: row.get(6)?,
            prompt_text: row.get(7)?,
            completion_text: row.get(8)?,
            created_at: row.get(9)?,
        })
    }).map_err(|e| {
        eprintln!("クエリ実行エラー: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let mut results = Vec::new();
    for detail in details {
        results.push(detail.map_err(|e| {
            eprintln!("行取得エラー: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?);
    }
    
    // 全件数を取得
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM token_usage",
        [],
        |row| row.get(0)
    ).map_err(|e| {
        eprintln!("件数取得エラー: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    Ok(Json(serde_json::json!({
        "data": results,
        "total": total,
        "limit": limit,
        "offset": offset
    })))
}

