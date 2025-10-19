use rusqlite::{params, Connection, Result};
use serde_json::Value;
use nostr_sdk::prelude::Event;
use chrono::Utc;

#[derive(Debug, Clone)]
pub struct EventRecord {
    pub id: i64,
    #[allow(dead_code)]
    pub event_id: String,
    #[allow(dead_code)]
    pub event_json: String,
    pub pubkey: String,
    #[allow(dead_code)]
    pub kind: i32,
    pub content: String,
    pub created_at: i64,
    #[allow(dead_code)]
    pub received_at: i64,
    #[allow(dead_code)]
    pub language: Option<String>,
}

impl EventRecord {
    /// 表示名を取得（kind0_cacheから取得、なければpubkey短縮）
    pub fn display_name(&self, conn: &Connection) -> String {
        // utilのget_kind0_metadataを使用（eventsテーブル優先、なければリレーから取得）
        if let Some(content) = crate::util::get_kind0_metadata(conn, &self.pubkey) {
            // JSONから名前を抽出
            if let Ok(metadata) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(name) = metadata.get("display_name")
                    .or_else(|| metadata.get("name"))
                    .and_then(|n| n.as_str())
                {
                    return name.to_string();
                }
            }
        }
        
        // 取得できない場合はpubkey短縮表示
        if self.pubkey.len() > 8 {
            format!("{}...", &self.pubkey[..8])
        } else {
            self.pubkey.clone()
        }
    }
}

/// イベントをeventsテーブルに保存
#[allow(dead_code)]
pub fn insert_event(
    conn: &Connection,
    event: &Event,
    language: Option<&str>,
) -> Result<i64> {
    // kind 0の場合はupsert処理を使用
    if event.kind.as_u16() == 0 {
        return upsert_kind0_event(conn, event);
    }
    
    let event_json = serde_json::to_string(event)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    let now = Utc::now().timestamp();
    
    conn.execute(
        "INSERT INTO events (event_id, event_json, pubkey, kind, content, created_at, received_at, language)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            event.id.to_string(),
            event_json,
            event.pubkey.to_string(),
            event.kind.as_u16() as i32,
            event.content,
            event.created_at.as_u64() as i64,
            now,
            language,
        ],
    )?;
    
    Ok(conn.last_insert_rowid())
}

/// kind 0イベントをupsert（同じpubkeyの古いkind 0は削除して最新のみ保持）
fn upsert_kind0_event(conn: &Connection, event: &Event) -> Result<i64> {
    let event_json = serde_json::to_string(event)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    let now = Utc::now().timestamp();
    let pubkey = event.pubkey.to_string();
    let event_created_at = event.created_at.as_u64() as i64;
    
    // 同じpubkeyの既存kind 0を確認
    let existing_created_at: Option<i64> = conn.query_row(
        "SELECT created_at FROM events WHERE pubkey = ? AND kind = 0 ORDER BY created_at DESC LIMIT 1",
        params![pubkey],
        |row| row.get(0)
    ).ok();
    
    // 既存のものより古い場合は保存しない
    if let Some(existing_ts) = existing_created_at {
        if event_created_at <= existing_ts {
            eprintln!("[Kind0] Ignoring older kind 0 event (existing: {}, new: {})", existing_ts, event_created_at);
            return Ok(0); // 保存しない
        }
    }
    
    // 同じpubkeyの古いkind 0を削除
    conn.execute(
        "DELETE FROM events WHERE pubkey = ? AND kind = 0",
        params![pubkey],
    )?;
    
    // 新しいkind 0を挿入
    conn.execute(
        "INSERT INTO events (event_id, event_json, pubkey, kind, content, created_at, received_at, language)
         VALUES (?, ?, ?, ?, ?, ?, ?, NULL)",
        params![
            event.id.to_string(),
            event_json,
            pubkey,
            0,
            event.content,
            event_created_at,
            now,
        ],
    )?;
    
    Ok(conn.last_insert_rowid())
}

/// イベントIDからイベントを取得
#[allow(dead_code)]
pub fn get_event_by_event_id(conn: &Connection, event_id: &str) -> Result<Option<EventRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, event_id, event_json, pubkey, kind, content, created_at, received_at, language
         FROM events WHERE event_id = ?"
    )?;
    
    let mut rows = stmt.query(params![event_id])?;
    
    if let Some(row) = rows.next()? {
        Ok(Some(EventRecord {
            id: row.get(0)?,
            event_id: row.get(1)?,
            event_json: row.get(2)?,
            pubkey: row.get(3)?,
            kind: row.get(4)?,
            content: row.get(5)?,
            created_at: row.get(6)?,
            received_at: row.get(7)?,
            language: row.get(8)?,
        }))
    } else {
        Ok(None)
    }
}

#[allow(dead_code)]
pub fn extract_reply_to_event_id(event_json: &str) -> Result<Option<String>> {
    let event: Value = serde_json::from_str(event_json)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    
    if let Some(tags) = event["tags"].as_array() {
        for tag in tags {
            if let Some(tag_array) = tag.as_array() {
                if tag_array.len() >= 2 {
                    if let Some(tag_name) = tag_array[0].as_str() {
                        if tag_name == "e" {
                            if let Some(event_id) = tag_array[1].as_str() {
                                return Ok(Some(event_id.to_string()));
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(None)
}

/// event_jsonからメンションされた全pubkeyを抽出
#[allow(dead_code)]
pub fn extract_mentioned_pubkeys(event_json: &str) -> Result<Vec<String>> {
    let event: Value = serde_json::from_str(event_json)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    
    let mut pubkeys = Vec::new();
    
    if let Some(tags) = event["tags"].as_array() {
        for tag in tags {
            if let Some(tag_array) = tag.as_array() {
                if tag_array.len() >= 2 {
                    if let Some(tag_name) = tag_array[0].as_str() {
                        if tag_name == "p" {
                            if let Some(pubkey) = tag_array[1].as_str() {
                                pubkeys.push(pubkey.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(pubkeys)
}

/// event_jsonからスレッドroot_idを抽出
#[allow(dead_code)]
pub fn extract_thread_root_id(event_json: &str) -> Result<Option<String>> {
    let event: Value = serde_json::from_str(event_json)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    
    if let Some(tags) = event["tags"].as_array() {
        for tag in tags {
            if let Some(tag_array) = tag.as_array() {
                if tag_array.len() >= 4 {
                    if let Some(tag_name) = tag_array[0].as_str() {
                        if tag_name == "e" {
                            if let Some(marker) = tag_array[3].as_str() {
                                if marker == "root" {
                                    if let Some(event_id) = tag_array[1].as_str() {
                                        return Ok(Some(event_id.to_string()));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(None)
}

/// bot同士の会話を検出
#[allow(dead_code)]
pub fn detect_bot_conversation(mentioned_pubkeys: &[String], all_bot_pubkeys: &[String]) -> bool {
    let bot_mention_count = mentioned_pubkeys
        .iter()
        .filter(|pk| all_bot_pubkeys.contains(pk))
        .count();
    
    bot_mention_count > 0
}
