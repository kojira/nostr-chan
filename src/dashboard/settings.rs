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

// ============================================================
// Bot動作設定
// ============================================================

/// Bot動作設定の取得
pub async fn get_bot_behavior_settings_handler(
    State(_state): State<DashboardState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let reaction_percent = get_setting_i64(&conn, "reaction_percent", 50)?;
    let reaction_freq = get_setting_i64(&conn, "reaction_freq", 600)?;
    let timeline_size = get_setting_i64(&conn, "timeline_size", 50)?;
    
    Ok(Json(serde_json::json!({
        "reaction_percent": reaction_percent,
        "reaction_freq": reaction_freq,
        "timeline_size": timeline_size
    })))
}

/// Bot動作設定の保存
pub async fn set_bot_behavior_settings_handler(
    State(_state): State<DashboardState>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if let Some(reaction_percent) = req["reaction_percent"].as_i64() {
        if reaction_percent < 0 || reaction_percent > 100 {
            return Err(StatusCode::BAD_REQUEST);
        }
        db::set_system_setting(&conn, "reaction_percent", &reaction_percent.to_string())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        println!("🎲 リアクション確率: {}%", reaction_percent);
    }
    
    if let Some(reaction_freq) = req["reaction_freq"].as_i64() {
        if reaction_freq < 1 {
            return Err(StatusCode::BAD_REQUEST);
        }
        db::set_system_setting(&conn, "reaction_freq", &reaction_freq.to_string())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        println!("⏱️ リアクション頻度: {}秒", reaction_freq);
    }
    
    if let Some(timeline_size) = req["timeline_size"].as_i64() {
        if timeline_size < 1 || timeline_size > 1000 {
            return Err(StatusCode::BAD_REQUEST);
        }
        db::set_system_setting(&conn, "timeline_size", &timeline_size.to_string())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        println!("📜 タイムラインサイズ: {}", timeline_size);
    }
    
    Ok(Json(serde_json::json!({ "success": true })))
}

// ============================================================
// 会話制限設定
// ============================================================

/// 会話制限設定の取得
pub async fn get_conversation_limit_settings_handler(
    State(_state): State<DashboardState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let count = get_setting_i64(&conn, "conversation_limit_count", 5)?;
    let minutes = get_setting_i64(&conn, "conversation_limit_minutes", 3)?;
    
    Ok(Json(serde_json::json!({
        "count": count,
        "minutes": minutes
    })))
}

/// 会話制限設定の保存
pub async fn set_conversation_limit_settings_handler(
    State(_state): State<DashboardState>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if let Some(count) = req["count"].as_i64() {
        if count < 1 || count > 100 {
            return Err(StatusCode::BAD_REQUEST);
        }
        db::set_system_setting(&conn, "conversation_limit_count", &count.to_string())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        println!("💬 会話制限回数: {}回", count);
    }
    
    if let Some(minutes) = req["minutes"].as_i64() {
        if minutes < 1 || minutes > 1440 {
            return Err(StatusCode::BAD_REQUEST);
        }
        db::set_system_setting(&conn, "conversation_limit_minutes", &minutes.to_string())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        println!("⏰ 会話制限時間: {}分", minutes);
    }
    
    Ok(Json(serde_json::json!({ "success": true })))
}

// ============================================================
// RAG設定
// ============================================================

/// RAG設定の取得
pub async fn get_rag_settings_handler(
    State(_state): State<DashboardState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let threshold = match db::get_system_setting(&conn, "rag_similarity_threshold")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? {
        Some(value) => value.parse::<f64>().unwrap_or(0.9),
        None => 0.9,
    };
    
    Ok(Json(serde_json::json!({
        "similarity_threshold": threshold
    })))
}

/// RAG設定の保存
pub async fn set_rag_settings_handler(
    State(_state): State<DashboardState>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if let Some(threshold) = req["similarity_threshold"].as_f64() {
        if threshold < 0.0 || threshold > 1.0 {
            return Err(StatusCode::BAD_REQUEST);
        }
        db::set_system_setting(&conn, "rag_similarity_threshold", &threshold.to_string())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        println!("🔍 RAG類似度閾値: {}", threshold);
    }
    
    Ok(Json(serde_json::json!({ "success": true })))
}

// ============================================================
// GPT設定
// ============================================================

/// GPT設定の取得
pub async fn get_gpt_settings_handler(
    State(_state): State<DashboardState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let answer_length = get_setting_i64(&conn, "gpt_answer_length", 100)?;
    let timeout = get_setting_i64(&conn, "gpt_timeout", 60)?;
    
    Ok(Json(serde_json::json!({
        "answer_length": answer_length,
        "timeout": timeout
    })))
}

/// GPT設定の保存
pub async fn set_gpt_settings_handler(
    State(_state): State<DashboardState>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if let Some(answer_length) = req["answer_length"].as_i64() {
        if answer_length < 10 || answer_length > 1000 {
            return Err(StatusCode::BAD_REQUEST);
        }
        db::set_system_setting(&conn, "gpt_answer_length", &answer_length.to_string())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        println!("📝 GPT回答長: {}文字", answer_length);
    }
    
    if let Some(timeout) = req["timeout"].as_i64() {
        if timeout < 10 || timeout > 300 {
            return Err(StatusCode::BAD_REQUEST);
        }
        db::set_system_setting(&conn, "gpt_timeout", &timeout.to_string())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        println!("⏱️ GPTタイムアウト: {}秒", timeout);
    }
    
    Ok(Json(serde_json::json!({ "success": true })))
}

// ============================================================
// リレー設定
// ============================================================

/// リレー設定の取得
pub async fn get_relay_settings_handler(
    State(_state): State<DashboardState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let write_relays = db::get_system_setting(&conn, "relay_write")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .unwrap_or_default();
    let read_relays = db::get_system_setting(&conn, "relay_read")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .unwrap_or_default();
    let search_relays = db::get_system_setting(&conn, "relay_search")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .unwrap_or_default();
    
    Ok(Json(serde_json::json!({
        "write": write_relays.split(',').filter(|s| !s.is_empty()).collect::<Vec<_>>(),
        "read": read_relays.split(',').filter(|s| !s.is_empty()).collect::<Vec<_>>(),
        "search": search_relays.split(',').filter(|s| !s.is_empty()).collect::<Vec<_>>(),
    })))
}

/// リレー設定の保存
pub async fn set_relay_settings_handler(
    State(_state): State<DashboardState>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if let Some(write) = req["write"].as_array() {
        let write_relays: Vec<String> = write.iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect();
        db::set_system_setting(&conn, "relay_write", &write_relays.join(","))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        println!("📡 書き込みリレー更新: {}", write_relays.join(", "));
    }
    
    if let Some(read) = req["read"].as_array() {
        let read_relays: Vec<String> = read.iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect();
        db::set_system_setting(&conn, "relay_read", &read_relays.join(","))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        println!("📡 読み込みリレー更新: {}", read_relays.join(", "));
    }
    
    if let Some(search) = req["search"].as_array() {
        let search_relays: Vec<String> = search.iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect();
        db::set_system_setting(&conn, "relay_search", &search_relays.join(","))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        println!("📡 検索リレー更新: {}", search_relays.join(", "));
    }
    
    Ok(Json(serde_json::json!({ "success": true })))
}

// ============================================================
// ブラックリスト設定
// ============================================================

/// ブラックリスト設定の取得
pub async fn get_blacklist_settings_handler(
    State(_state): State<DashboardState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let blacklist = db::get_system_setting(&conn, "blacklist")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .unwrap_or_default();
    
    Ok(Json(serde_json::json!({
        "blacklist": blacklist.split(',').filter(|s| !s.is_empty()).collect::<Vec<_>>(),
    })))
}

/// ブラックリスト設定の保存
pub async fn set_blacklist_settings_handler(
    State(_state): State<DashboardState>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if let Some(blacklist) = req["blacklist"].as_array() {
        let blacklist_pubkeys: Vec<String> = blacklist.iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect();
        db::set_system_setting(&conn, "blacklist", &blacklist_pubkeys.join(","))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        println!("🚫 ブラックリスト更新: {}件", blacklist_pubkeys.len());
    }
    
    Ok(Json(serde_json::json!({ "success": true })))
}

// ============================================================
// ヘルパー関数
// ============================================================

fn get_setting_i64(conn: &rusqlite::Connection, key: &str, default: i64) -> Result<i64, StatusCode> {
    match db::get_system_setting(conn, key).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? {
        Some(value) => Ok(value.parse::<i64>().unwrap_or(default)),
        None => Ok(default),
    }
}

