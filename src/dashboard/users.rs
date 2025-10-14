use axum::{
    extract::{State, Path},
    response::Json,
    http::StatusCode,
};
use super::types::DashboardState;
use crate::db;
use serde::Serialize;
use nostr_sdk::ToBech32;

#[derive(Debug, Serialize)]
pub struct UserKind0 {
    pub pubkey: String,
    pub npub: String,
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub picture: Option<String>,
    pub about: Option<String>,
    pub nip05: Option<String>,
}

/// 汎用Kind 0情報取得（eventsテーブルから）
pub async fn get_kind0_handler(
    State(_state): State<DashboardState>,
    Path(pubkey): Path<String>,
) -> Result<Json<UserKind0>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // eventsテーブルからkind0情報を取得
    let mut stmt = conn.prepare(
        "SELECT kind0_content FROM events WHERE pubkey = ? AND kind0_content IS NOT NULL LIMIT 1"
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let kind0_content: Option<String> = stmt.query_row([&pubkey], |row| row.get(0)).ok();
    
    // hex pubkeyをnpubに変換
    let npub = match nostr_sdk::PublicKey::from_hex(&pubkey) {
        Ok(pk) => pk.to_bech32().unwrap_or_else(|_| pubkey.clone()),
        Err(_) => pubkey.clone(),
    };
    
    if let Some(content_str) = kind0_content {
        // JSONパース
        if let Ok(content) = serde_json::from_str::<serde_json::Value>(&content_str) {
            return Ok(Json(UserKind0 {
                pubkey: pubkey.clone(),
                npub: npub.clone(),
                name: content["name"].as_str().map(|s| s.to_string()),
                display_name: content["display_name"].as_str().map(|s| s.to_string()),
                picture: content["picture"].as_str().map(|s| s.to_string()),
                about: content["about"].as_str().map(|s| s.to_string()),
                nip05: content["nip05"].as_str().map(|s| s.to_string()),
            }));
        }
    }
    
    // kind0情報が見つからない場合
    Ok(Json(UserKind0 {
        pubkey,
        npub,
        name: None,
        display_name: None,
        picture: None,
        about: None,
        nip05: None,
    }))
}

