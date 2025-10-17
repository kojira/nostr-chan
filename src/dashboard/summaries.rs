use axum::{
    extract::{State, Path, Query},
    response::Json,
    http::StatusCode,
};
use super::types::DashboardState;
use crate::database as db;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct SummaryData {
    pub id: i64,
    pub bot_pubkey: String,
    pub summary: String,
    pub user_input: String,
    pub participants: Option<Vec<String>>,
    pub from_timestamp: i64,
    pub to_timestamp: i64,
    pub created_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct SummaryQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub search: Option<String>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSummaryRequest {
    pub summary: String,
    pub user_input: String,
}

/// Bot要約一覧取得
pub async fn list_summaries_handler(
    State(_state): State<DashboardState>,
    Path(pubkey): Path<String>,
    Query(query): Query<SummaryQuery>,
) -> Result<Json<Vec<SummaryData>>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);
    let sort_by = query.sort_by.unwrap_or_else(|| "created_at".to_string());
    let sort_order = query.sort_order.unwrap_or_else(|| "DESC".to_string());
    
    // ソートカラムのバリデーション
    let sort_column = match sort_by.as_str() {
        "created_at" => "created_at",
        "from_timestamp" => "from_timestamp",
        "to_timestamp" => "to_timestamp",
        "user_input" => "user_input",
        _ => "created_at",
    };
    
    // ソート順のバリデーション
    let order = match sort_order.to_uppercase().as_str() {
        "ASC" => "ASC",
        "DESC" => "DESC",
        _ => "DESC",
    };
    
    // WHERE句の構築
    let mut where_clause = format!("bot_pubkey = '{}'", pubkey);
    
    // 検索フィルタを追加
    if let Some(search) = &query.search {
        if !search.is_empty() {
            let escaped_search = search.replace("'", "''");
            where_clause.push_str(&format!(
                " AND (summary LIKE '%{}%' OR user_input LIKE '%{}%')",
                escaped_search, escaped_search
            ));
        }
    }
    
    // SQLクエリの構築
    let query_str = format!(
        "SELECT id, bot_pubkey, summary, user_input, participants_json, from_timestamp, to_timestamp, created_at
         FROM conversation_summaries
         WHERE {}
         ORDER BY {} {}
         LIMIT {} OFFSET {}",
        where_clause, sort_column, order, limit, offset
    );
    
    let mut stmt = conn.prepare(&query_str)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let summaries = stmt.query_map([], |row| {
        let id: i64 = row.get(0)?;
        let bot_pubkey: String = row.get(1)?;
        let summary: String = row.get(2)?;
        let user_input: String = row.get(3)?;
        let participants_json: Option<String> = row.get(4)?;
        let from_timestamp: i64 = row.get(5)?;
        let to_timestamp: i64 = row.get(6)?;
        let created_at: i64 = row.get(7)?;
        
        let participants = if let Some(json_str) = participants_json {
            serde_json::from_str(&json_str).ok()
        } else {
            None
        };
        
        Ok(SummaryData {
            id,
            bot_pubkey,
            summary,
            user_input,
            participants,
            from_timestamp,
            to_timestamp,
            created_at,
        })
    })
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(summaries))
}

/// 要約更新
pub async fn update_summary_handler(
    State(_state): State<DashboardState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateSummaryRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    conn.execute(
        "UPDATE conversation_summaries SET summary = ?, user_input = ? WHERE id = ?",
        rusqlite::params![req.summary, req.user_input, id],
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    println!("📝 要約ID{}を更新しました", id);
    
    Ok(Json(serde_json::json!({ "success": true })))
}

/// 要約削除
pub async fn delete_summary_handler(
    State(_state): State<DashboardState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    conn.execute(
        "DELETE FROM conversation_summaries WHERE id = ?",
        rusqlite::params![id],
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    println!("🗑️ 要約ID{}を削除しました", id);
    
    Ok(Json(serde_json::json!({ "success": true })))
}

#[derive(Debug, Deserialize)]
pub struct DeleteBulkRequest {
    pub search: Option<String>,
}

/// Bot要約一括削除（全件またはフィルタ後）
pub async fn delete_summaries_bulk_handler(
    State(_state): State<DashboardState>,
    Path(pubkey): Path<String>,
    Json(req): Json<DeleteBulkRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // WHERE句の構築
    let mut where_clause = format!("bot_pubkey = '{}'", pubkey);
    
    // 検索フィルタを追加
    if let Some(search) = &req.search {
        if !search.is_empty() {
            let escaped_search = search.replace("'", "''");
            where_clause.push_str(&format!(
                " AND (summary LIKE '%{}%' OR user_input LIKE '%{}%')",
                escaped_search, escaped_search
            ));
        }
    }
    
    // SQLクエリの構築
    let delete_query = format!(
        "DELETE FROM conversation_summaries WHERE {}",
        where_clause
    );
    
    let deleted_count = conn.execute(&delete_query, [])
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if req.search.is_some() && req.search.as_ref().unwrap().is_empty() == false {
        println!("🗑️ Bot {} のフィルタ後要約 {}件を削除しました", pubkey, deleted_count);
    } else {
        println!("🗑️ Bot {} の全要約 {}件を削除しました", pubkey, deleted_count);
    }
    
    Ok(Json(serde_json::json!({ 
        "success": true,
        "deleted_count": deleted_count
    })))
}

