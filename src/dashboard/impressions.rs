use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use crate::database as db;

/// ユーザー印象のレスポンス
#[derive(Debug, Serialize)]
pub struct ImpressionResponse {
    pub id: i64,
    pub bot_pubkey: String,
    pub user_pubkey: String,
    pub impression: String,
    pub created_at: i64,
    pub user_name: Option<String>,
    pub user_picture: Option<String>,
}

impl ImpressionResponse {
    fn from_record(record: db::UserImpressionRecord, conn: &rusqlite::Connection) -> Self {
        // eventsテーブルからkind 0イベントを取得
        let kind0_json = conn.query_row(
            "SELECT content FROM events WHERE pubkey = ? AND kind = 0 ORDER BY created_at DESC LIMIT 1",
            rusqlite::params![&record.user_pubkey],
            |row| row.get::<_, String>(0)
        ).ok();
        
        let user_name = kind0_json.as_ref().and_then(|json| {
            serde_json::from_str::<serde_json::Value>(json).ok()
                .and_then(|v| v.get("name")
                    .or_else(|| v.get("display_name"))
                    .and_then(|n| n.as_str().map(|s| s.to_string())))
        });
        let user_picture = kind0_json.as_ref().and_then(|json| {
            serde_json::from_str::<serde_json::Value>(json).ok()
                .and_then(|v| v.get("picture").and_then(|p| p.as_str().map(|s| s.to_string())))
        });
        
        ImpressionResponse {
            id: record.id,
            bot_pubkey: record.bot_pubkey,
            user_pubkey: record.user_pubkey,
            impression: record.impression,
            created_at: record.created_at,
            user_name,
            user_picture,
        }
    }
}

/// ユーザー印象一覧のレスポンス
#[derive(Debug, Serialize)]
pub struct ImpressionsListResponse {
    pub impressions: Vec<ImpressionResponse>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
}

/// ページネーションクエリ
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_per_page")]
    pub per_page: usize,
}

fn default_page() -> usize { 1 }
fn default_per_page() -> usize { 20 }

/// Bot別のユーザー印象一覧を取得
pub async fn get_bot_impressions_handler(
    Path(bot_pubkey): Path<String>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<ImpressionsListResponse>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let offset = (pagination.page - 1) * pagination.per_page;
    
    let impressions = db::get_all_user_impressions(&conn, &bot_pubkey, pagination.per_page, offset)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|record| ImpressionResponse::from_record(record, &conn))
        .collect();
    
    let total = db::count_users_with_impressions(&conn, &bot_pubkey)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(ImpressionsListResponse {
        impressions,
        total,
        page: pagination.page,
        per_page: pagination.per_page,
    }))
}

/// 特定ユーザーの印象履歴を取得
pub async fn get_user_impression_history_handler(
    Path((bot_pubkey, user_pubkey)): Path<(String, String)>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<Vec<ImpressionResponse>>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let limit = pagination.per_page;
    
    let history = db::get_user_impression_history(&conn, &bot_pubkey, &user_pubkey, limit)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|record| ImpressionResponse::from_record(record, &conn))
        .collect();
    
    Ok(Json(history))
}

/// 特定ユーザーの最新印象を取得
pub async fn get_user_latest_impression_handler(
    Path((bot_pubkey, user_pubkey)): Path<(String, String)>,
) -> Result<Json<Option<String>>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let impression = db::get_user_impression(&conn, &bot_pubkey, &user_pubkey)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(impression))
}

/// 印象の更新リクエスト
#[derive(Debug, Deserialize)]
pub struct UpdateImpressionRequest {
    pub impression: String,
}

/// 印象を手動更新（ダッシュボードから）
pub async fn update_user_impression_handler(
    Path((bot_pubkey, user_pubkey)): Path<(String, String)>,
    Json(payload): Json<UpdateImpressionRequest>,
) -> Result<StatusCode, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // 印象の長さチェック（500文字制限）
    if payload.impression.len() > 500 {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    db::save_user_impression(&conn, &bot_pubkey, &user_pubkey, &payload.impression)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(StatusCode::OK)
}

