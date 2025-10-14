use axum::{
    extract::State,
    response::Json,
    http::StatusCode,
};
use super::types::DashboardState;
use crate::db;

/// グローバル一時停止状態の取得
pub async fn get_global_pause_handler(
    State(_state): State<DashboardState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let paused = db::is_global_pause(&conn).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "paused": paused })))
}

/// グローバル一時停止の設定
pub async fn set_global_pause_handler(
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

/// フォロワーキャッシュ有効時間の取得
pub async fn get_follower_cache_ttl_handler(
    State(_state): State<DashboardState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // デフォルトは24時間（秒単位）
    let ttl_seconds = match db::get_system_setting(&conn, "follower_cache_ttl")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? {
        Some(value) => value.parse::<i64>().unwrap_or(86400),
        None => 86400,
    };
    
    Ok(Json(serde_json::json!({ "ttl_seconds": ttl_seconds })))
}

/// フォロワーキャッシュ有効時間の設定
pub async fn set_follower_cache_ttl_handler(
    State(_state): State<DashboardState>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let ttl_seconds = req["ttl_seconds"].as_i64().ok_or(StatusCode::BAD_REQUEST)?;
    
    // 最小1分、最大7日間
    if ttl_seconds < 60 || ttl_seconds > 604800 {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    db::set_system_setting(&conn, "follower_cache_ttl", &ttl_seconds.to_string())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    println!("⏰ フォロワーキャッシュ有効時間: {}秒 ({}時間)", ttl_seconds, ttl_seconds / 3600);
    
    Ok(Json(serde_json::json!({ "ttl_seconds": ttl_seconds })))
}

