use chrono::Utc;
use nostr_sdk::prelude::*;
use rusqlite::{params, Connection, Result};
use serde_json::Value;

pub(crate) fn connect() -> Result<Connection> {
    let conn = Connection::open("../nostrchan.db")?;
    
    // Create follower_cache table if not exists
    conn.execute(
        "CREATE TABLE IF NOT EXISTS follower_cache (
            user_pubkey TEXT NOT NULL,
            bot_pubkey TEXT NOT NULL,
            is_follower INTEGER NOT NULL,
            cached_at INTEGER NOT NULL,
            PRIMARY KEY (user_pubkey, bot_pubkey)
        )",
        [],
    )?;
    
    // Create kind0_cache table if not exists
    conn.execute(
        "CREATE TABLE IF NOT EXISTS kind0_cache (
            pubkey TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            cached_at INTEGER NOT NULL
        )",
        [],
    )?;
    
    // Create timeline table if not exists
    conn.execute(
        "CREATE TABLE IF NOT EXISTS timeline (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            pubkey TEXT NOT NULL,
            name TEXT,
            content TEXT NOT NULL,
            timestamp INTEGER NOT NULL
        )",
        [],
    )?;
    
    // Create index on timestamp for efficient ordering
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_timeline_timestamp ON timeline(timestamp DESC)",
        [],
    )?;
    
    // Create system_settings table for global settings
    conn.execute(
        "CREATE TABLE IF NOT EXISTS system_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )?;
    
    // Create event_queue table for persistent event processing queue
    conn.execute(
        "CREATE TABLE IF NOT EXISTS event_queue (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            event_json TEXT NOT NULL,
            added_at INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending'
        )",
        [],
    )?;
    
    // Create index for efficient queue processing
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_event_queue_status_added 
         ON event_queue(status, added_at)",
        [],
    )?;
    
    // Create events table if not exists
    conn.execute(
        "CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            event_id TEXT UNIQUE NOT NULL,
            event_json TEXT NOT NULL,
            pubkey TEXT NOT NULL,
            kind INTEGER NOT NULL,
            content TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            received_at INTEGER NOT NULL,
            kind0_name TEXT,
            kind0_content TEXT,
            is_japanese INTEGER NOT NULL DEFAULT 0,
            embedding BLOB,
            event_type TEXT
        )",
        [],
    )?;
    
    // Create indexes for events table
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_events_pubkey ON events(pubkey)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_events_kind ON events(kind)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_events_created_at ON events(created_at DESC)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_events_is_japanese ON events(is_japanese)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_events_event_type ON events(event_type)",
        [],
    )?;
    
    // Create conversation_logs table if not exists
    conn.execute(
        "CREATE TABLE IF NOT EXISTS conversation_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            bot_pubkey TEXT NOT NULL,
            event_ref_id INTEGER NOT NULL,
            thread_root_id TEXT,
            mentioned_pubkeys_json TEXT,
            is_bot_message INTEGER NOT NULL DEFAULT 0,
            is_bot_conversation INTEGER NOT NULL DEFAULT 0,
            logged_at INTEGER NOT NULL,
            FOREIGN KEY (event_ref_id) REFERENCES events(id)
        )",
        [],
    )?;
    
    // Create indexes for conversation_logs table
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_conversation_logs_bot_recent ON conversation_logs(bot_pubkey, logged_at DESC)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_conversation_logs_thread ON conversation_logs(thread_root_id)",
        [],
    )?;
    
    // Create conversation_summaries table if not exists
    conn.execute(
        "CREATE TABLE IF NOT EXISTS conversation_summaries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            bot_pubkey TEXT NOT NULL,
            summary TEXT NOT NULL,
            user_input TEXT NOT NULL,
            user_input_embedding BLOB NOT NULL,
            participants_json TEXT,
            from_timestamp INTEGER NOT NULL,
            to_timestamp INTEGER NOT NULL,
            created_at INTEGER NOT NULL
        )",
        [],
    )?;
    
    // Create index for conversation_summaries table
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_summaries_bot ON conversation_summaries(bot_pubkey, created_at DESC)",
        [],
    )?;
    
    Ok(conn)
}

#[derive(Debug, Clone)]
pub struct Person {
    #[allow(dead_code)]
    pub id: i32,
    #[allow(dead_code)]
    pub status: i32,
    pub prompt: String,
    pub pubkey: String,
    pub secretkey: String,
    pub content: String,
    #[allow(dead_code)]
    pub created_at: String,
    #[allow(dead_code)]
    pub updated_at: String,
}

/// Botを追加
pub fn add_person(conn: &Connection, pubkey: &str, secretkey: &str, prompt: &str, content: &str) -> Result<()> {
    let now = Utc::now().timestamp();
    conn.execute(
        "INSERT INTO Persons (status, prompt, pubkey, secretkey, content, created_at) VALUES(0, ?, ?, ?, ?, datetime(?, 'unixepoch'))",
        params![prompt, pubkey, secretkey, content, now],
    )?;
    Ok(())
}

/// Botを更新
pub fn update_person(conn: &Connection, pubkey: &str, secretkey: &str, prompt: &str, content: &str) -> Result<()> {
    conn.execute(
        "UPDATE Persons SET secretkey = ?, prompt = ?, content = ? WHERE pubkey = ?",
        params![secretkey, prompt, content, pubkey],
    )?;
    Ok(())
}

/// Bot削除
pub fn delete_person(conn: &Connection, pubkey: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM Persons WHERE pubkey = ?",
        params![pubkey],
    )?;
    Ok(())
}

/// Botのstatus更新
pub fn update_person_status(conn: &Connection, pubkey: &str, status: i32) -> Result<()> {
    conn.execute(
        "UPDATE Persons SET status = ? WHERE pubkey = ?",
        params![status, pubkey],
    )?;
    Ok(())
}

// システム設定の取得
pub fn get_system_setting(conn: &Connection, key: &str) -> Result<Option<String>> {
    let result = conn.query_row(
        "SELECT value FROM system_settings WHERE key = ?",
        params![key],
        |row| row.get(0),
    );
    
    match result {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

// システム設定の保存
pub fn set_system_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    let now = Utc::now().timestamp();
    conn.execute(
        "INSERT OR REPLACE INTO system_settings (key, value, updated_at) VALUES (?, ?, ?)",
        params![key, value, now],
    )?;
    Ok(())
}

// Bot全体一時停止状態の取得
pub fn is_global_pause(conn: &Connection) -> Result<bool> {
    match get_system_setting(conn, "global_pause")? {
        Some(value) => Ok(value == "true"),
        None => Ok(false),
    }
}

pub fn get_all_persons(conn: &Connection) -> Result<Vec<Person>> {
    let mut stmt = conn.prepare("SELECT * FROM Persons")?;
    let persons = stmt
        .query_map(params![], |row| {
            Ok(Person {
                id: row.get(0)?,
                status: row.get(1)?,
                prompt: row.get(2)?,
                pubkey: row.get(3)?,
                secretkey: row.get(4)?,
                content: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<Person>, _>>()?;

    Ok(persons.clone())
}

pub fn get_person(conn: &Connection, pubkey: &str) -> Result<Person> {
    let mut stmt = conn.prepare("SELECT * FROM Persons WHERE pubkey = ?")?;
    let person = stmt
        .query_map(params![pubkey], |row| {
            Ok(Person {
                id: row.get(0)?,
                status: row.get(1)?,
                prompt: row.get(2)?,
                pubkey: row.get(3)?,
                secretkey: row.get(4)?,
                content: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?
        .next()
        .unwrap()?;

    Ok(person.clone())
}

pub(crate) fn insert_person(
    conn: &Connection,
    keys: &Keys,
    prompt: &str,
    content: &str,
) -> Result<()> {
    let secretkey = keys.secret_key().display_secret().to_string();
    let now = Utc::now();
    let created_at = now.timestamp();
    let mut stmt = conn.prepare(
        "INSERT INTO Persons (status, prompt, pubkey, secretkey, content, created_at) VALUES(0,?,?,?,?,datetime(?, 'unixepoch'))",
    )?;
    stmt.execute(params![
        prompt,
        keys.public_key().to_string(),
        secretkey,
        content,
        created_at
    ])?;

    Ok(())
}

pub(crate) fn update_person_content(conn: &Connection, pubkey: &str, content: &str) -> Result<()> {
    let mut stmt = conn.prepare("UPDATE Persons SET content=? WHERE pubkey=?")?;
    stmt.execute(params![content, pubkey])?;

    Ok(())
}

#[allow(dead_code)]
pub fn get_random_person(conn: &Connection) -> Result<Person> {
    let mut stmt =
        conn.prepare("SELECT * FROM Persons WHERE status=0 ORDER BY RANDOM() LIMIT 1")?;
    let person = stmt
        .query_map(params![], |row| {
            Ok(Person {
                id: row.get(0)?,
                status: row.get(1)?,
                prompt: row.get(2)?,
                pubkey: row.get(3)?,
                secretkey: row.get(4)?,
                content: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?
        .next()
        .unwrap()?;

    Ok(person.clone())
}

// Follower cache functions
// Returns (is_follower, remaining_seconds) if cache is valid, None if expired or not found
pub fn get_follower_cache(conn: &Connection, user_pubkey: &str, bot_pubkey: &str, ttl: i64) -> Result<Option<(bool, i64)>> {
    let now = Utc::now().timestamp();
    let mut stmt = conn.prepare(
        "SELECT is_follower, cached_at FROM follower_cache WHERE user_pubkey = ? AND bot_pubkey = ?"
    )?;
    
    let result = stmt.query_map(params![user_pubkey, bot_pubkey], |row| {
        Ok((row.get::<_, i32>(0)?, row.get::<_, i64>(1)?))
    })?
    .next();
    
    if let Some(Ok((is_follower, cached_at))) = result {
        let age = now - cached_at;
        let remaining = ttl - age;
        
        // Check if cache is still valid
        if age < ttl {
            return Ok(Some((is_follower != 0, remaining)));
        }
    }
    
    Ok(None)
}

pub fn set_follower_cache(conn: &Connection, user_pubkey: &str, bot_pubkey: &str, is_follower: bool) -> Result<()> {
    let now = Utc::now().timestamp();
    conn.execute(
        "INSERT OR REPLACE INTO follower_cache (user_pubkey, bot_pubkey, is_follower, cached_at) VALUES (?, ?, ?, ?)",
        params![user_pubkey, bot_pubkey, if is_follower { 1 } else { 0 }, now],
    )?;
    Ok(())
}

pub fn clear_follower_cache(conn: &Connection) -> Result<usize> {
    let deleted = conn.execute("DELETE FROM follower_cache", [])?;
    Ok(deleted)
}

pub fn delete_user_follower_cache(conn: &Connection, user_pubkey: &str, bot_pubkey: &str) -> Result<usize> {
    let deleted = conn.execute(
        "DELETE FROM follower_cache WHERE user_pubkey = ? AND bot_pubkey = ?",
        params![user_pubkey, bot_pubkey],
    )?;
    Ok(deleted)
}

// Kind 0 cache functions
#[allow(dead_code)]
pub fn get_kind0_cache(conn: &Connection, pubkey: &str, ttl: i64) -> Result<Option<String>> {
    let now = Utc::now().timestamp();
    let mut stmt = conn.prepare(
        "SELECT name, cached_at FROM kind0_cache WHERE pubkey = ?"
    )?;
    
    let result = stmt.query_map(params![pubkey], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?
    .next();
    
    if let Some(Ok((name, cached_at))) = result {
        // Check if cache is still valid
        if now - cached_at < ttl {
            return Ok(Some(name));
        }
    }
    
    Ok(None)
}

#[allow(dead_code)]
pub fn set_kind0_cache(conn: &Connection, pubkey: &str, name: &str) -> Result<()> {
    let now = Utc::now().timestamp();
    conn.execute(
        "INSERT OR REPLACE INTO kind0_cache (pubkey, name, cached_at) VALUES (?, ?, ?)",
        params![pubkey, name, now],
    )?;
    Ok(())
}

// Timeline functions
#[allow(dead_code)]
pub fn add_timeline_post(conn: &Connection, pubkey: &str, name: Option<&str>, content: &str, timestamp: i64) -> Result<()> {
    conn.execute(
        "INSERT INTO timeline (pubkey, name, content, timestamp) VALUES (?, ?, ?, ?)",
        params![pubkey, name, content, timestamp],
    )?;
    Ok(())
}

pub fn get_latest_timeline_posts(conn: &Connection, limit: usize) -> Result<Vec<crate::TimelinePost>> {
    let mut stmt = conn.prepare(
        "SELECT pubkey, name, content, timestamp FROM timeline ORDER BY timestamp DESC LIMIT ?"
    )?;
    
    let posts = stmt.query_map(params![limit], |row| {
        Ok(crate::TimelinePost {
            pubkey: row.get(0)?,
            name: row.get(1)?,
            content: row.get(2)?,
            timestamp: row.get(3)?,
        })
    })?
    .collect::<Result<Vec<_>>>()?;
    
    // Reverse to get chronological order (oldest first)
    Ok(posts.into_iter().rev().collect())
}

#[allow(dead_code)]
pub fn cleanup_old_timeline_posts(conn: &Connection, keep_count: usize) -> Result<usize> {
    // Keep only the latest N posts
    let deleted = conn.execute(
        "DELETE FROM timeline WHERE id NOT IN (
            SELECT id FROM timeline ORDER BY timestamp DESC LIMIT ?
        )",
        params![keep_count],
    )?;
    Ok(deleted)
}

// ========== Events table functions ==========

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct EventRecord {
    pub id: i64,
    pub event_id: String,
    pub event_json: String,
    pub pubkey: String,
    pub kind: i32,
    pub content: String,
    pub created_at: i64,
    pub received_at: i64,
    pub kind0_name: Option<String>,
    pub kind0_content: Option<String>,
    pub is_japanese: bool,
    pub embedding: Option<Vec<u8>>,
    pub event_type: Option<String>,
}

/// イベントをeventsテーブルに保存
#[allow(dead_code)]
pub fn insert_event(
    conn: &Connection,
    event: &Event,
    is_japanese: bool,
    event_type: Option<&str>,
) -> Result<i64> {
    let event_json = serde_json::to_string(event)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    let now = Utc::now().timestamp();
    
    conn.execute(
        "INSERT INTO events (event_id, event_json, pubkey, kind, content, created_at, received_at, is_japanese, event_type)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            event.id.to_string(),
            event_json,
            event.pubkey.to_string(),
            event.kind.as_u16() as i32,
            event.content,
            event.created_at.as_u64() as i64,
            now,
            if is_japanese { 1 } else { 0 },
            event_type,
        ],
    )?;
    
    Ok(conn.last_insert_rowid())
}

/// イベントIDからイベントを取得
#[allow(dead_code)]
pub fn get_event_by_event_id(conn: &Connection, event_id: &str) -> Result<Option<EventRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, event_id, event_json, pubkey, kind, content, created_at, received_at, 
                kind0_name, kind0_content, is_japanese, embedding, event_type 
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
            kind0_name: row.get(8)?,
            kind0_content: row.get(9)?,
            is_japanese: row.get::<_, i32>(10)? != 0,
            embedding: row.get(11)?,
            event_type: row.get(12)?,
        }))
    } else {
        Ok(None)
    }
}

/// イベントのkind0情報を更新
#[allow(dead_code)]
pub fn update_event_kind0(
    conn: &Connection,
    event_id: &str,
    kind0_name: Option<&str>,
    kind0_content: Option<&str>,
) -> Result<()> {
    conn.execute(
        "UPDATE events SET kind0_name = ?, kind0_content = ? WHERE event_id = ?",
        params![kind0_name, kind0_content, event_id],
    )?;
    Ok(())
}

/// イベントのembeddingを更新
pub fn update_event_embedding(conn: &Connection, event_id: &str, embedding: &[f32]) -> Result<()> {
    // f32のスライスをバイト列に変換
    let bytes: Vec<u8> = embedding
        .iter()
        .flat_map(|&f| f.to_le_bytes())
        .collect();
    
    conn.execute(
        "UPDATE events SET embedding = ? WHERE event_id = ?",
        params![bytes, event_id],
    )?;
    Ok(())
}

/// embedding未設定のイベントを取得（バックグラウンド処理用）
pub fn get_events_without_embedding(conn: &Connection, limit: usize) -> Result<Vec<EventRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, event_id, event_json, pubkey, kind, content, created_at, received_at, 
                kind0_name, kind0_content, is_japanese, embedding, event_type 
         FROM events WHERE embedding IS NULL LIMIT ?"
    )?;
    
    let events = stmt.query_map(params![limit], |row| {
        Ok(EventRecord {
            id: row.get(0)?,
            event_id: row.get(1)?,
            event_json: row.get(2)?,
            pubkey: row.get(3)?,
            kind: row.get(4)?,
            content: row.get(5)?,
            created_at: row.get(6)?,
            received_at: row.get(7)?,
            kind0_name: row.get(8)?,
            kind0_content: row.get(9)?,
            is_japanese: row.get::<_, i32>(10)? != 0,
            embedding: row.get(11)?,
            event_type: row.get(12)?,
        })
    })?
    .collect::<Result<Vec<_>>>()?;
    
    Ok(events)
}

// ========== Conversation logs functions ==========

/// 会話ログを記録
#[allow(dead_code)]
pub fn insert_conversation_log(
    conn: &Connection,
    bot_pubkey: &str,
    event_ref_id: i64,
    thread_root_id: Option<&str>,
    mentioned_pubkeys: Option<&[String]>,
    is_bot_message: bool,
    is_bot_conversation: bool,
) -> Result<i64> {
    let mentioned_pubkeys_json = mentioned_pubkeys.map(|pks| {
        serde_json::to_string(pks).unwrap_or_default()
    });
    
    let now = Utc::now().timestamp();
    
    conn.execute(
        "INSERT INTO conversation_logs (bot_pubkey, event_ref_id, thread_root_id, mentioned_pubkeys_json, is_bot_message, is_bot_conversation, logged_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        params![
            bot_pubkey,
            event_ref_id,
            thread_root_id,
            mentioned_pubkeys_json,
            if is_bot_message { 1 } else { 0 },
            if is_bot_conversation { 1 } else { 0 },
            now,
        ],
    )?;
    
    Ok(conn.last_insert_rowid())
}

/// bot別の会話タイムラインを取得
#[allow(dead_code)]
pub fn get_conversation_timeline(
    conn: &Connection,
    bot_pubkey: &str,
    limit: usize,
) -> Result<Vec<EventRecord>> {
    let mut stmt = conn.prepare(
        "SELECT e.id, e.event_id, e.event_json, e.pubkey, e.kind, e.content, e.created_at, e.received_at,
                e.kind0_name, e.kind0_content, e.is_japanese, e.embedding, e.event_type
         FROM events e
         INNER JOIN conversation_logs cl ON e.id = cl.event_ref_id
         WHERE cl.bot_pubkey = ?
         ORDER BY e.created_at DESC
         LIMIT ?"
    )?;
    
    let events = stmt.query_map(params![bot_pubkey, limit], |row| {
        Ok(EventRecord {
            id: row.get(0)?,
            event_id: row.get(1)?,
            event_json: row.get(2)?,
            pubkey: row.get(3)?,
            kind: row.get(4)?,
            content: row.get(5)?,
            created_at: row.get(6)?,
            received_at: row.get(7)?,
            kind0_name: row.get(8)?,
            kind0_content: row.get(9)?,
            is_japanese: row.get::<_, i32>(10)? != 0,
            embedding: row.get(11)?,
            event_type: row.get(12)?,
        })
    })?
    .collect::<Result<Vec<_>>>()?;
    
    // 時系列順（古い順）に反転
    Ok(events.into_iter().rev().collect())
}

/// 特定スレッド内の過去N分間のメッセージ数を取得
#[allow(dead_code)]
pub fn get_thread_message_count(
    conn: &Connection,
    thread_root_id: &str,
    minutes: i64,
) -> Result<usize> {
    let cutoff_time = Utc::now().timestamp() - (minutes * 60);
    
    let mut stmt = conn.prepare(
        "SELECT COUNT(*) FROM conversation_logs 
         WHERE thread_root_id = ? AND logged_at > ?"
    )?;
    
    let count: usize = stmt.query_row(params![thread_root_id, cutoff_time], |row| row.get(0))?;
    Ok(count)
}

/// 特定のbotと相手との過去N分間の会話回数を取得
#[allow(dead_code)]
pub fn get_conversation_count_with_user(
    conn: &Connection,
    bot_pubkey: &str,
    user_pubkey: &str,
    minutes: i64,
) -> Result<usize> {
    let cutoff_time = Utc::now().timestamp() - (minutes * 60);
    
    let mut stmt = conn.prepare(
        "SELECT COUNT(*) FROM conversation_logs cl
         INNER JOIN events e ON cl.event_ref_id = e.id
         WHERE cl.bot_pubkey = ? 
         AND (e.pubkey = ? OR cl.is_bot_message = 1)
         AND cl.logged_at > ?"
    )?;
    
    let count: usize = stmt.query_row(params![bot_pubkey, user_pubkey, cutoff_time], |row| row.get(0))?;
    Ok(count)
}

// ========== Conversation summaries functions ==========

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ConversationSummary {
    pub id: i64,
    pub bot_pubkey: String,
    pub summary: String,
    pub user_input: String,
    pub user_input_embedding: Vec<u8>,
    pub participants_json: Option<String>,
    pub from_timestamp: i64,
    pub to_timestamp: i64,
    pub created_at: i64,
}

/// 要約を保存
#[allow(dead_code)]
pub fn insert_conversation_summary(
    conn: &Connection,
    bot_pubkey: &str,
    summary: &str,
    user_input: &str,
    user_input_embedding: &[f32],
    participants: Option<&[String]>,
    from_timestamp: i64,
    to_timestamp: i64,
) -> Result<i64> {
    // f32のスライスをバイト列に変換
    let embedding_bytes: Vec<u8> = user_input_embedding
        .iter()
        .flat_map(|&f| f.to_le_bytes())
        .collect();
    
    let participants_json = participants.map(|p| {
        serde_json::to_string(p).unwrap_or_default()
    });
    
    let now = Utc::now().timestamp();
    
    conn.execute(
        "INSERT INTO conversation_summaries (bot_pubkey, summary, user_input, user_input_embedding, participants_json, from_timestamp, to_timestamp, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            bot_pubkey,
            summary,
            user_input,
            embedding_bytes,
            participants_json,
            from_timestamp,
            to_timestamp,
            now,
        ],
    )?;
    
    Ok(conn.last_insert_rowid())
}

/// bot別の要約を取得
#[allow(dead_code)]
pub fn get_conversation_summaries(
    conn: &Connection,
    bot_pubkey: &str,
    user_pubkey: &str,
    limit: usize,
) -> Result<Vec<ConversationSummary>> {
    let mut stmt = conn.prepare(
        "SELECT id, bot_pubkey, summary, user_input, user_input_embedding, participants_json, from_timestamp, to_timestamp, created_at
         FROM conversation_summaries
         WHERE bot_pubkey = ?
         AND (participants_json LIKE '%' || ? || '%' OR participants_json IS NULL)
         ORDER BY created_at DESC
         LIMIT ?"
    )?;
    
    let summaries = stmt.query_map(params![bot_pubkey, user_pubkey, limit], |row| {
        Ok(ConversationSummary {
            id: row.get(0)?,
            bot_pubkey: row.get(1)?,
            summary: row.get(2)?,
            user_input: row.get(3)?,
            user_input_embedding: row.get(4)?,
            participants_json: row.get(5)?,
            from_timestamp: row.get(6)?,
            to_timestamp: row.get(7)?,
            created_at: row.get(8)?,
        })
    })?
    .collect::<Result<Vec<_>>>()?;
    
    Ok(summaries)
}

// ========== Helper functions ==========

/// event_jsonからリプライ先event_idを抽出
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

// ========== Migration functions ==========


// ========== Event Queue Management ==========

/// キューにイベントを追加（最大30件まで、古いものを削除）
pub fn enqueue_event(conn: &Connection, event_json: &str) -> Result<i64> {
    let now = Utc::now().timestamp();
    
    // 現在のキューサイズを確認
    let queue_size: i64 = conn.query_row(
        "SELECT COUNT(*) FROM event_queue WHERE status = 'pending'",
        [],
        |row| row.get(0)
    )?;
    
    // 30件を超える場合は古いものを削除
    if queue_size >= 30 {
        let to_delete = queue_size - 29; // 1つ分の余地を作る
        conn.execute(
            "DELETE FROM event_queue 
             WHERE id IN (
                 SELECT id FROM event_queue 
                 WHERE status = 'pending' 
                 ORDER BY added_at ASC 
                 LIMIT ?
             )",
            params![to_delete],
        )?;
    }
    
    // イベントを追加
    conn.execute(
        "INSERT INTO event_queue (event_json, added_at, status) VALUES (?, ?, 'pending')",
        params![event_json, now],
    )?;
    
    Ok(conn.last_insert_rowid())
}

/// キューから次の処理対象イベントを取得（ステータスを'processing'に更新）
pub fn dequeue_event(conn: &Connection) -> Result<Option<(i64, String)>> {
    // トランザクション開始
    let tx = conn.unchecked_transaction()?;
    
    // 最も古いpendingイベントを取得
    let result = tx.query_row(
        "SELECT id, event_json FROM event_queue 
         WHERE status = 'pending' 
         ORDER BY added_at ASC 
         LIMIT 1",
        [],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    );
    
    match result {
        Ok((id, event_json)) => {
            // ステータスをprocessingに更新
            tx.execute(
                "UPDATE event_queue SET status = 'processing' WHERE id = ?",
                params![id],
            )?;
            tx.commit()?;
            Ok(Some((id, event_json)))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            tx.commit()?;
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

/// 処理完了したイベントをキューから削除
pub fn complete_queue_event(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM event_queue WHERE id = ?", params![id])?;
    Ok(())
}

/// 処理失敗したイベントをpendingに戻す
#[allow(dead_code)]
pub fn retry_queue_event(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE event_queue SET status = 'pending' WHERE id = ?",
        params![id],
    )?;
    Ok(())
}

/// キューのサイズを取得
#[allow(dead_code)]
pub fn get_queue_size(conn: &Connection) -> Result<i64> {
    let size: i64 = conn.query_row(
        "SELECT COUNT(*) FROM event_queue WHERE status IN ('pending', 'processing')",
        [],
        |row| row.get(0)
    )?;
    Ok(size)
}

/// 起動時に処理中だったイベントをpendingに戻す
pub fn reset_processing_events(conn: &Connection) -> Result<usize> {
    let updated = conn.execute(
        "UPDATE event_queue SET status = 'pending' WHERE status = 'processing'",
        [],
    )?;
    Ok(updated)
}

// ========== Dashboard Statistics ==========

/// ダッシュボード用の統計データを取得
pub fn get_dashboard_stats(conn: &Connection) -> Result<DashboardStats> {
    let now = Utc::now().timestamp();
    let today_start = now - (now % 86400);
    let week_start = now - (7 * 86400);
    let month_start = now - (30 * 86400);
    
    // 返信統計
    let replies_today: u32 = conn.query_row(
        "SELECT COUNT(*) FROM conversation_logs WHERE timestamp >= ?",
        params![today_start],
        |row| row.get(0)
    ).unwrap_or(0);
    
    let replies_week: u32 = conn.query_row(
        "SELECT COUNT(*) FROM conversation_logs WHERE timestamp >= ?",
        params![week_start],
        |row| row.get(0)
    ).unwrap_or(0);
    
    let replies_month: u32 = conn.query_row(
        "SELECT COUNT(*) FROM conversation_logs WHERE timestamp >= ?",
        params![month_start],
        |row| row.get(0)
    ).unwrap_or(0);
    
    let replies_total: u32 = conn.query_row(
        "SELECT COUNT(*) FROM conversation_logs",
        [],
        |row| row.get(0)
    ).unwrap_or(0);
    
    // 会話統計
    let active_conversations: u32 = conn.query_row(
        "SELECT COUNT(DISTINCT user_pubkey) FROM conversation_logs WHERE timestamp >= ?",
        params![now - 3600], // 過去1時間
        |row| row.get(0)
    ).unwrap_or(0);
    
    let unique_users: u32 = conn.query_row(
        "SELECT COUNT(DISTINCT user_pubkey) FROM conversation_logs",
        [],
        |row| row.get(0)
    ).unwrap_or(0);
    
    // RAG統計
    let vectorized_events: u32 = conn.query_row(
        "SELECT COUNT(*) FROM events WHERE embedding IS NOT NULL AND length(embedding) > 0",
        [],
        |row| row.get(0)
    ).unwrap_or(0);
    
    let total_events: u32 = conn.query_row(
        "SELECT COUNT(*) FROM events",
        [],
        |row| row.get(0)
    ).unwrap_or(0);
    
    let pending_vectorization = total_events.saturating_sub(vectorized_events);
    
    // レート制限されたユーザー数（過去N分間でM回以上会話したユーザー）
    // 簡易実装: 過去3分間で5回以上会話したユーザー数
    let rate_limited_users: u32 = conn.query_row(
        "SELECT COUNT(DISTINCT user_pubkey) FROM (
            SELECT user_pubkey, COUNT(*) as cnt 
            FROM conversation_logs 
            WHERE timestamp >= ? AND is_bot_message = 0
            GROUP BY user_pubkey 
            HAVING cnt >= 5
        )",
        params![now - 180], // 過去3分
        |row| row.get(0)
    ).unwrap_or(0);
    
    // RAG検索統計: conversation_summariesテーブルの件数を使用
    let total_searches: u32 = conn.query_row(
        "SELECT COUNT(*) FROM conversation_summaries",
        [],
        |row| row.get(0)
    ).unwrap_or(0);
    
    // 平均類似度: conversation_summariesが存在しないため、暫定で0.0
    // 実装するには検索履歴テーブルが必要
    let average_similarity: f64 = 0.0;
    
    Ok(DashboardStats {
        replies_today,
        replies_week,
        replies_month,
        replies_total,
        active_conversations,
        unique_users,
        rate_limited_users,
        vectorized_events,
        total_events,
        pending_vectorization,
        total_searches,
        average_similarity,
    })
}

#[derive(Debug, Clone)]
pub struct DashboardStats {
    pub replies_today: u32,
    pub replies_week: u32,
    pub replies_month: u32,
    pub replies_total: u32,
    pub active_conversations: u32,
    pub unique_users: u32,
    pub rate_limited_users: u32,
    pub vectorized_events: u32,
    pub total_events: u32,
    pub pending_vectorization: u32,
    pub total_searches: u32,
    pub average_similarity: f64,
}

