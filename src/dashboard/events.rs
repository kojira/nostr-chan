use axum::{
    extract::{State, Query},
    response::Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use super::types::DashboardState;
use crate::db;

#[derive(Debug, Deserialize)]
pub struct EventsQuery {
    pub page: Option<usize>,
    pub page_size: Option<usize>,
    pub search: Option<String>,
    pub has_embedding: Option<bool>,
    pub is_japanese: Option<bool>,
    pub event_type: Option<String>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VectorizedEvent {
    pub id: i64,
    pub event_id: String,
    pub pubkey: String,
    pub kind: i32,
    pub content: String,
    pub created_at: i64,
    pub received_at: i64,
    pub kind0_name: Option<String>,
    pub is_japanese: bool,
    pub has_embedding: bool,
    pub event_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EventsResponse {
    pub events: Vec<VectorizedEvent>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

/// ベクトル化されたイベント一覧を取得
pub async fn list_events_handler(
    State(_state): State<DashboardState>,
    Query(query): Query<EventsQuery>,
) -> (StatusCode, Json<EventsResponse>) {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(50).clamp(10, 100);
    let offset = (page - 1) * page_size;
    
    let search = query.search.clone();
    let has_embedding = query.has_embedding;
    let is_japanese = query.is_japanese;
    let event_type = query.event_type.clone();
    let sort_by = query.sort_by.clone().unwrap_or_else(|| "created_at".to_string());
    let sort_order = query.sort_order.clone().unwrap_or_else(|| "desc".to_string());
    
    // データベース操作をspawn_blockingで実行
    let result = tokio::task::spawn_blocking(move || {
        let conn = db::connect().ok()?;
        
        // WHERE句の構築
        let mut where_clauses = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        
        if let Some(search_text) = &search {
            where_clauses.push("(content LIKE ? OR kind0_name LIKE ? OR event_id LIKE ? OR pubkey LIKE ?)");
            let search_pattern = format!("%{}%", search_text);
            params.push(Box::new(search_pattern.clone()));
            params.push(Box::new(search_pattern.clone()));
            params.push(Box::new(search_pattern.clone()));
            params.push(Box::new(search_pattern));
        }
        
        if let Some(has_emb) = has_embedding {
            if has_emb {
                where_clauses.push("embedding IS NOT NULL");
            } else {
                where_clauses.push("embedding IS NULL");
            }
        }
        
        if let Some(is_jp) = is_japanese {
            where_clauses.push("is_japanese = ?");
            params.push(Box::new(if is_jp { 1 } else { 0 }));
        }
        
        if let Some(ev_type) = &event_type {
            if !ev_type.is_empty() {
                where_clauses.push("event_type = ?");
                params.push(Box::new(ev_type.clone()));
            }
        }
        
        let where_clause = if where_clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_clauses.join(" AND "))
        };
        
        // ソート順の検証
        let sort_column = match sort_by.as_str() {
            "created_at" => "created_at",
            "received_at" => "received_at",
            "content" => "content",
            "kind" => "kind",
            _ => "created_at",
        };
        let order = if sort_order.to_lowercase() == "asc" { "ASC" } else { "DESC" };
        
        // 総数を取得
        let count_sql = format!("SELECT COUNT(*) FROM events{}", where_clause);
        let mut count_stmt = conn.prepare(&count_sql).ok()?;
        
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let total: usize = count_stmt.query_row(&param_refs[..], |row| row.get(0)).ok()?;
        
        // イベントを取得
        let query_sql = format!(
            "SELECT id, event_id, pubkey, kind, content, created_at, received_at, 
                    kind0_name, is_japanese, embedding, event_type
             FROM events{}
             ORDER BY {} {}
             LIMIT ? OFFSET ?",
            where_clause, sort_column, order
        );
        
        let mut stmt = conn.prepare(&query_sql).ok()?;
        
        // パラメータにLIMITとOFFSETを追加
        let mut all_params = params;
        all_params.push(Box::new(page_size as i64));
        all_params.push(Box::new(offset as i64));
        let param_refs: Vec<&dyn rusqlite::ToSql> = all_params.iter().map(|p| p.as_ref()).collect();
        
        let events = stmt.query_map(&param_refs[..], |row| {
            Ok(VectorizedEvent {
                id: row.get(0)?,
                event_id: row.get(1)?,
                pubkey: row.get(2)?,
                kind: row.get(3)?,
                content: row.get(4)?,
                created_at: row.get(5)?,
                received_at: row.get(6)?,
                kind0_name: row.get(7)?,
                is_japanese: row.get::<_, i32>(8)? != 0,
                has_embedding: row.get::<_, Option<Vec<u8>>>(9)?.is_some(),
                event_type: row.get(10)?,
            })
        }).ok()?
        .collect::<Result<Vec<_>, _>>().ok()?;
        
        let total_pages = (total + page_size - 1) / page_size;
        
        Some(EventsResponse {
            events,
            total,
            page,
            page_size,
            total_pages,
        })
    }).await;
    
    match result {
        Ok(Some(response)) => (StatusCode::OK, Json(response)),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, Json(EventsResponse {
            events: vec![],
            total: 0,
            page: 1,
            page_size,
            total_pages: 0,
        })),
    }
}

