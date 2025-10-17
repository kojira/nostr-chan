use axum::{
    extract::{State, Path},
    response::Json,
    http::StatusCode,
};
use super::types::DashboardState;
use crate::{database as db, config};
use serde::Serialize;
use nostr_sdk::{ToBech32, Keys, Client, Filter, Kind, PublicKey};
use std::time::Duration;
use std::fs::File;

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

/// 汎用Kind 0情報取得（eventsテーブルから、なければリレーから取得）
pub async fn get_kind0_handler(
    State(_state): State<DashboardState>,
    Path(pubkey): Path<String>,
) -> (StatusCode, Json<UserKind0>) {
    // hex pubkeyをnpubに変換
    let npub = match nostr_sdk::PublicKey::from_hex(&pubkey) {
        Ok(pk) => pk.to_bech32().unwrap_or_else(|_| pubkey.clone()),
        Err(_) => pubkey.clone(),
    };
    
    // DBからKind 0情報を取得（spawn_blockingで別スレッド実行）
    let pubkey_clone = pubkey.clone();
    let kind0_content: Option<String> = tokio::task::spawn_blocking(move || {
        let conn = db::connect().ok()?;
        let mut stmt = conn.prepare(
            "SELECT content FROM events WHERE pubkey = ? AND kind = 0 ORDER BY created_at DESC LIMIT 1"
        ).ok()?;
        stmt.query_row([pubkey_clone.as_str()], |row| row.get::<_, String>(0)).ok()
    }).await.ok().flatten();
    
    // DBにKind 0情報がある場合
    if let Some(content_str) = kind0_content {
        if let Ok(content) = serde_json::from_str::<serde_json::Value>(&content_str) {
            return (StatusCode::OK, Json(UserKind0 {
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
    
    // DBにない場合はリレーから取得
    println!("[Kind0] DBに情報がないため、リレーから取得: {}", pubkey);
    
    match fetch_kind0_from_relay(&pubkey).await {
        Ok(kind0_info) => {
            // DBに保存（spawn_blockingで別スレッド実行）
            if let Ok(event_json) = &kind0_info.event_json {
                let event_json_clone = event_json.clone();
                tokio::task::spawn_blocking(move || {
                    if let Ok(conn) = db::connect() {
                        if let Ok(event) = serde_json::from_str::<nostr_sdk::Event>(&event_json_clone) {
                            let _ = db::insert_event(&conn, &event, false, Some("kind0"));
                            println!("[Kind0] リレーから取得してDBに保存");
                        }
                    }
                });
            }
        
            return (StatusCode::OK, Json(UserKind0 {
                pubkey: pubkey.clone(),
                npub: npub.clone(),
                name: kind0_info.name,
                display_name: kind0_info.display_name,
                picture: kind0_info.picture,
                about: kind0_info.about,
                nip05: kind0_info.nip05,
            }));
        }
        Err(e) => {
            // リレーからも取得できなかった場合
            println!("[Kind0] リレーからも取得できませんでした: {} ({})", pubkey, e);
        }
    }
    
    // リレーからも取得できなかった場合
    (StatusCode::OK, Json(UserKind0 {
        pubkey,
        npub,
        name: None,
        display_name: None,
        picture: None,
        about: None,
        nip05: None,
    }))
}

/// リレーからKind 0情報を取得
#[derive(Debug)]
struct Kind0Info {
    name: Option<String>,
    display_name: Option<String>,
    picture: Option<String>,
    about: Option<String>,
    nip05: Option<String>,
    event_json: Result<String, String>,
}

async fn fetch_kind0_from_relay(pubkey: &str) -> Result<Kind0Info, Box<dyn std::error::Error>> {
    let file = File::open("../config.yml")?;
    let config: config::AppConfig = serde_yaml::from_reader(file)?;
    
    let public_key = PublicKey::from_hex(pubkey)?;
    let keys = Keys::generate();
    let client = Client::new(keys);
    
    // relay_readから取得
    for relay in config.relay_servers.read.iter() {
        let _ = client.add_relay(relay.clone()).await;
    }
    client.connect().await;
    
    let filter = Filter::new()
        .author(public_key)
        .kind(Kind::Metadata)
        .limit(1);
    
    let events = client.fetch_events(filter, Duration::from_secs(10)).await?;
    let _ = client.shutdown().await;
    
    if let Some(event) = events.first() {
        let event_json = serde_json::to_string(event).map_err(|e| e.to_string());
        
        if let Ok(metadata) = serde_json::from_str::<serde_json::Value>(&event.content) {
            return Ok(Kind0Info {
                name: metadata["name"].as_str().map(|s| s.to_string()),
                display_name: metadata["display_name"].as_str().map(|s| s.to_string()),
                picture: metadata["picture"].as_str().map(|s| s.to_string()),
                about: metadata["about"].as_str().map(|s| s.to_string()),
                nip05: metadata["nip05"].as_str().map(|s| s.to_string()),
                event_json,
            });
        }
    }
    
    Err("Kind 0イベントが見つかりませんでした".into())
}


