use axum::{
    extract::{State, Path, Query},
    response::Json,
    http::StatusCode,
};
use super::types::{DashboardState, BotData, BotRequest};
use crate::db;
use serde::{Deserialize, Serialize};

/// Bot一覧取得
pub async fn list_bots_handler(
    State(_state): State<DashboardState>
) -> Result<Json<Vec<BotData>>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let persons = db::get_all_persons(&conn).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let bots: Vec<BotData> = persons.into_iter().map(|p| BotData {
        pubkey: p.pubkey,
        secretkey: p.secretkey,
        prompt: p.prompt,
        content: p.content,
        status: p.status,
        air_reply_single_ratio: Some(p.air_reply_single_ratio),
    }).collect();
    
    Ok(Json(bots))
}

/// Bot作成
pub async fn create_bot_handler(
    State(_state): State<DashboardState>,
    Json(req): Json<BotRequest>,
) -> Result<Json<BotData>, StatusCode> {
    use nostr_sdk::prelude::*;
    
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // secretkeyからpubkeyを取得
    let keys = Keys::parse(&req.secretkey).map_err(|_| StatusCode::BAD_REQUEST)?;
    let pubkey = keys.public_key().to_string();
    
    // DBに追加
    db::add_person(&conn, &pubkey, &req.secretkey, &req.prompt, &req.content)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // 誕生投稿を非同期で送信
    let secretkey = req.secretkey.clone();
    let content = req.content.clone();
    tokio::spawn(async move {
        if let Err(e) = post_birth_announcement(&secretkey, &content).await {
            eprintln!("誕生投稿エラー: {}", e);
        }
    });
    
    Ok(Json(BotData {
        pubkey,
        secretkey: req.secretkey,
        prompt: req.prompt,
        content: req.content,
        status: 0,
        air_reply_single_ratio: req.air_reply_single_ratio,
    }))
}

/// 誕生投稿
async fn post_birth_announcement(secretkey: &str, content_json: &str) -> Result<(), Box<dyn std::error::Error>> {
    use nostr_sdk::prelude::*;
    
    // Botの名前を取得
    let bot_name = if !content_json.is_empty() {
        match serde_json::from_str::<serde_json::Value>(content_json) {
            Ok(json) => {
                json["display_name"].as_str()
                    .or_else(|| json["name"].as_str())
                    .unwrap_or("新しいBot")
                    .to_string()
            }
            Err(_) => "新しいBot".to_string()
        }
    } else {
        "新しいBot".to_string()
    };
    
    let keys = Keys::parse(secretkey)?;
    let client = Client::new(keys);
    
    // config.ymlから設定を読み込む
    let config_path = "../config.yml";
    let file = std::fs::File::open(config_path)?;
    let config: crate::config::AppConfig = serde_yaml::from_reader(file)?;
    
    // リレーに接続
    for relay in &config.relay_servers.write {
        let _ = client.add_relay(relay).await;
    }
    
    client.connect().await;
    
    // 誕生メッセージを投稿（adminコマンドと同じ文面）
    let message = format!("{}です。コンゴトモヨロシク！", bot_name);
    
    let builder = EventBuilder::text_note(message);
    client.send_event_builder(builder).await?;
    
    println!("✨ {}の誕生投稿を送信しました", bot_name);
    
    Ok(())
}

/// Bot更新
pub async fn update_bot_handler(
    State(_state): State<DashboardState>,
    Path(pubkey): Path<String>,
    Json(req): Json<BotRequest>,
) -> Result<Json<BotData>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // 既存のbotを取得
    let persons = db::get_all_persons(&conn).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let existing = persons.iter().find(|p| p.pubkey == pubkey).ok_or(StatusCode::NOT_FOUND)?;
    
    let air_reply_single_ratio = req.air_reply_single_ratio.unwrap_or(30);
    
    // 更新
    db::update_person(&conn, &pubkey, &req.secretkey, &req.prompt, &req.content, air_reply_single_ratio)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(BotData {
        pubkey,
        secretkey: req.secretkey,
        prompt: req.prompt,
        content: req.content,
        status: existing.status,
        air_reply_single_ratio: Some(air_reply_single_ratio),
    }))
}

/// Bot削除
pub async fn delete_bot_handler(
    State(_state): State<DashboardState>,
    Path(pubkey): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    db::delete_person(&conn, &pubkey)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(StatusCode::NO_CONTENT)
}

/// Bot有効/無効切り替え
pub async fn toggle_bot_handler(
    State(_state): State<DashboardState>,
    Path(pubkey): Path<String>,
) -> Result<Json<BotData>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // 既存のbotを取得
    let persons = db::get_all_persons(&conn).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let existing = persons.iter().find(|p| p.pubkey == pubkey).ok_or(StatusCode::NOT_FOUND)?;
    
    // statusを切り替え
    let new_status = if existing.status == 0 { 1 } else { 0 };
    db::update_person_status(&conn, &pubkey, new_status)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(BotData {
        pubkey: existing.pubkey.clone(),
        secretkey: existing.secretkey.clone(),
        prompt: existing.prompt.clone(),
        content: existing.content.clone(),
        status: new_status,
        air_reply_single_ratio: Some(existing.air_reply_single_ratio),
    }))
}

/// ランダムな秘密鍵を生成
pub async fn generate_key_handler(
    State(_state): State<DashboardState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use nostr_sdk::prelude::*;
    
    let keys = Keys::generate();
    let secret_key = keys.secret_key().to_secret_hex();
    
    Ok(Json(serde_json::json!({ 
        "secretkey": secret_key 
    })))
}

/// Kind 0メタデータをリレーから取得
pub async fn fetch_kind0_handler(
    State(_state): State<DashboardState>,
    Path(pubkey): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use nostr_sdk::prelude::*;
    
    let conn = db::connect().map_err(|e| {
        eprintln!("DB接続エラー: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let persons = db::get_all_persons(&conn).map_err(|e| {
        eprintln!("Bot情報取得エラー: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let bot = persons.iter().find(|p| p.pubkey == pubkey)
        .ok_or_else(|| {
            eprintln!("Botが見つかりません: {}", pubkey);
            StatusCode::NOT_FOUND
        })?;
    
    let keys = Keys::parse(&bot.secretkey).map_err(|e| {
        eprintln!("秘密鍵のパースエラー: {}", e);
        StatusCode::BAD_REQUEST
    })?;
    
    let client = Client::new(keys);
    
    let config_path = "../config.yml";
    let file = std::fs::File::open(config_path).map_err(|e| {
        eprintln!("設定ファイルオープンエラー: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let config: crate::config::AppConfig = serde_yaml::from_reader(file).map_err(|e| {
        eprintln!("設定ファイルパースエラー: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    for relay in &config.relay_servers.read {
        if let Err(e) = client.add_relay(relay).await {
            eprintln!("リレー追加エラー ({}): {}", relay, e);
        }
    }
    
    client.connect().await;
    
    let signer = client.signer().await
        .map_err(|e| {
            eprintln!("Signer取得エラー: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    let pubkey = signer.get_public_key().await
        .map_err(|e| {
            eprintln!("公開鍵取得エラー: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    let filter = Filter::new()
        .kind(Kind::Metadata)
        .author(pubkey);
    
    let events = client.fetch_events(filter, std::time::Duration::from_secs(10))
        .await
        .map_err(|e| {
            eprintln!("Kind 0取得エラー: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    let latest_event = events.iter()
        .max_by_key(|e| e.created_at);
    
    if let Some(event) = latest_event {
        Ok(Json(serde_json::json!({ 
            "content": event.content.clone() 
        })))
    } else {
        Ok(Json(serde_json::json!({ 
            "content": "" 
        })))
    }
}

/// Botとして投稿
pub async fn post_as_bot_handler(
    State(_state): State<DashboardState>,
    Path(pubkey): Path<String>,
    Json(req): Json<PostRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use nostr_sdk::prelude::*;
    
    let conn = db::connect().map_err(|e| {
        eprintln!("DB接続エラー: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let persons = db::get_all_persons(&conn).map_err(|e| {
        eprintln!("Bot情報取得エラー: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let bot = persons.iter().find(|p| p.pubkey == pubkey)
        .ok_or_else(|| {
            eprintln!("Botが見つかりません: {}", pubkey);
            StatusCode::NOT_FOUND
        })?;
    
    let keys = Keys::parse(&bot.secretkey).map_err(|e| {
        eprintln!("秘密鍵のパースエラー: {}", e);
        StatusCode::BAD_REQUEST
    })?;
    
    let client = Client::new(keys);
    
    let config_path = "../config.yml";
    let file = std::fs::File::open(config_path).map_err(|e| {
        eprintln!("設定ファイルオープンエラー: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let config: crate::config::AppConfig = serde_yaml::from_reader(file).map_err(|e| {
        eprintln!("設定ファイルパースエラー: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    for relay in &config.relay_servers.write {
        if let Err(e) = client.add_relay(relay).await {
            eprintln!("リレー追加エラー ({}): {}", relay, e);
        }
    }
    
    client.connect().await;
    
    let builder = EventBuilder::text_note(&req.content);
    let event_id = client.send_event_builder(builder)
        .await
        .map_err(|e| {
            eprintln!("投稿送信エラー: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    println!("📝 {}として投稿しました: {}", pubkey, req.content);
    
    Ok(Json(serde_json::json!({ 
        "success": true,
        "event_id": event_id.to_string()
    })))
}

#[derive(Debug, serde::Deserialize)]
pub struct PostRequest {
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct BotReply {
    pub event_id: String,
    pub content: String,
    pub created_at: i64,
    pub reply_to_event_id: Option<String>,
    pub reply_to_user: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReplyQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Bot返信履歴取得
pub async fn get_bot_replies_handler(
    State(_state): State<DashboardState>,
    Path(pubkey): Path<String>,
    Query(query): Query<ReplyQuery>,
) -> Result<Json<Vec<BotReply>>, StatusCode> {
    let conn = db::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);
    
    // eventsテーブルからBotの返信を取得
    // event_jsonからタグ情報を抽出
    let query_str = r#"
        SELECT 
            event_id,
            content,
            created_at,
            event_json
        FROM events
        WHERE pubkey = ?1 AND (event_type = 'bot_reply' OR event_type = 'bot_post')
        ORDER BY created_at DESC
        LIMIT ?2 OFFSET ?3
    "#;
    
    let mut stmt = conn.prepare(query_str)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let replies = stmt.query_map([&pubkey, &limit.to_string(), &offset.to_string()], |row| {
        let event_id: String = row.get(0)?;
        let content: String = row.get(1)?;
        let created_at: i64 = row.get(2)?;
        let event_json: String = row.get(3)?;
        
        // event_jsonからタグ情報を抽出
        let (reply_to_event_id, reply_to_user) = if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&event_json) {
            let mut reply_event_id = None;
            let mut reply_user = None;
            
            if let Some(tags) = json_value["tags"].as_array() {
                for tag in tags {
                    if let Some(tag_array) = tag.as_array() {
                        if tag_array.len() >= 2 {
                            if let Some(tag_type) = tag_array[0].as_str() {
                                if tag_type == "e" {
                                    reply_event_id = tag_array[1].as_str().map(|s| s.to_string());
                                } else if tag_type == "p" {
                                    reply_user = tag_array[1].as_str().map(|s| s.to_string());
                                }
                            }
                        }
                    }
                }
            }
            
            (reply_event_id, reply_user)
        } else {
            (None, None)
        };
        
        Ok(BotReply {
            event_id,
            content,
            created_at,
            reply_to_event_id,
            reply_to_user,
        })
    })
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(replies))
}

