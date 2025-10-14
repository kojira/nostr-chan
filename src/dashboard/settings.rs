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

