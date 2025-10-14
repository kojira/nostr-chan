use axum::{
    extract::{State, Path},
    response::Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use super::types::DashboardState;
use crate::db;

/// フォロワーキャッシュ一覧
#[derive(Debug, Serialize)]
pub struct FollowerCacheEntry {
    pub user_pubkey: String,
    pub user_name: Option<String>,
    pub bot_pubkey: String,
    pub bot_name: Option<String>,
    pub is_follower: bool,
    pub cached_at: i64,
}

pub async fn list_follower_cache_handler(
    State(_state): State<DashboardState>,
) -> Result<Json<Vec<FollowerCacheEntry>>, StatusCode> {
    let conn = db::connect().map_err(|e| {
        eprintln!("DB接続エラー: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let caches = db::get_all_follower_cache(&conn).map_err(|e| {
        eprintln!("フォロワーキャッシュ取得エラー: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    // Bot情報を取得
    let persons = db::get_all_persons(&conn).map_err(|e| {
        eprintln!("Bot情報取得エラー: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let entries = caches.into_iter().map(|(user_pubkey, bot_pubkey, is_follower, cached_at)| {
        // ユーザー名を取得（kind0_cacheから）
        let user_name = db::get_kind0_cache(&conn, &user_pubkey, i64::MAX)
            .ok()
            .flatten();
        
        // Bot名を取得（Persons.contentから）
        let bot_name = persons.iter()
            .find(|p| p.pubkey == bot_pubkey)
            .and_then(|p| {
                serde_json::from_str::<serde_json::Value>(&p.content)
                    .ok()
                    .and_then(|json| {
                        json["display_name"].as_str()
                            .or_else(|| json["name"].as_str())
                            .map(|s| s.to_string())
                    })
            });
        
        FollowerCacheEntry {
            user_pubkey,
            user_name,
            bot_pubkey,
            bot_name,
            is_follower,
            cached_at,
        }
    }).collect();
    
    Ok(Json(entries))
}

/// フォロワーキャッシュ全削除
pub async fn clear_follower_cache_handler(
    State(_state): State<DashboardState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let deleted = db::clear_follower_cache(&conn).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    println!("🗑️ フォロワーキャッシュを全削除しました ({}件)", deleted);
    Ok(Json(serde_json::json!({ "deleted": deleted })))
}

/// フォロワーキャッシュ更新
#[derive(Debug, Deserialize)]
pub struct UpdateFollowerCacheRequest {
    pub is_follower: bool,
}

pub async fn update_follower_cache_handler(
    State(_state): State<DashboardState>,
    Path((user_pubkey, bot_pubkey)): Path<(String, String)>,
    Json(req): Json<UpdateFollowerCacheRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    db::update_follower_cache(&conn, &user_pubkey, &bot_pubkey, req.is_follower)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "success": true })))
}

/// フォロワーキャッシュ削除（単一）
pub async fn delete_follower_cache_handler(
    State(_state): State<DashboardState>,
    Path((user_pubkey, bot_pubkey)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let deleted = db::delete_user_follower_cache(&conn, &user_pubkey, &bot_pubkey)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "deleted": deleted })))
}

