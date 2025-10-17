use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use crate::database as db;

/// Bot心境のレスポンス
#[derive(Debug, Serialize)]
pub struct MentalDiaryResponse {
    pub id: i64,
    pub bot_pubkey: String,
    pub mental_state_json: String,
    pub created_at: i64,
}

impl From<db::BotMentalStateRecord> for MentalDiaryResponse {
    fn from(record: db::BotMentalStateRecord) -> Self {
        MentalDiaryResponse {
            id: record.id,
            bot_pubkey: record.bot_pubkey,
            mental_state_json: record.mental_state_json,
            created_at: record.created_at,
        }
    }
}

/// Bot心境履歴のレスポンス
#[derive(Debug, Serialize)]
pub struct MentalDiaryListResponse {
    pub mental_diaries: Vec<MentalDiaryResponse>,
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

/// Botの心境履歴を取得
pub async fn get_bot_mental_diary_history_handler(
    Path(bot_pubkey): Path<String>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<MentalDiaryListResponse>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let offset = (pagination.page - 1) * pagination.per_page;
    
    let mental_diaries = db::get_bot_mental_state_history(&conn, &bot_pubkey, pagination.per_page, offset)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(MentalDiaryResponse::from)
        .collect();
    
    let total = db::count_bot_mental_state_history(&conn, &bot_pubkey)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(MentalDiaryListResponse {
        mental_diaries,
        total,
        page: pagination.page,
        per_page: pagination.per_page,
    }))
}

/// Botの最新の心境を取得
pub async fn get_bot_latest_mental_diary_handler(
    Path(bot_pubkey): Path<String>,
) -> Result<Json<Option<String>>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mental_diary = db::get_bot_mental_state(&conn, &bot_pubkey)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if let Some(diary) = mental_diary {
        Ok(Json(Some(serde_json::to_string(&diary).unwrap_or_default())))
    } else {
        Ok(Json(None))
    }
}

/// Botの心境を手動で更新
#[derive(Debug, Deserialize)]
pub struct UpdateMentalDiaryRequest {
    pub mental_diary_json: String,
}

pub async fn update_bot_mental_diary_handler(
    Path(bot_pubkey): Path<String>,
    Json(req): Json<UpdateMentalDiaryRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // JSONとしてパース可能かチェック
    let mental_diary: db::MentalDiary = serde_json::from_str(&req.mental_diary_json)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    // DBに保存
    db::save_bot_mental_state(&conn, &bot_pubkey, &mental_diary)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    println!("📔 Bot心境を手動更新: {}", bot_pubkey);
    
    Ok(Json(serde_json::json!({ "success": true })))
}

