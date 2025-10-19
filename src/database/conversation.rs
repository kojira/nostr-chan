use rusqlite::{params, Connection, Result};
use chrono::Utc;
use super::EventRecord;

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
        "SELECT e.id, e.event_id, e.event_json, e.pubkey, e.kind, e.content, e.created_at, e.received_at, e.language
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
            language: row.get(8)?,
        })
    })?
    .collect::<Result<Vec<_>>>()?;
    
    // 時系列順（古い順）に反転
    Ok(events.into_iter().rev().collect())
}

/// 特定ユーザーとの会話履歴を取得
pub fn get_conversation_timeline_with_user(
    conn: &Connection,
    bot_pubkey: &str,
    user_pubkey: &str,
    limit: usize,
) -> Result<Vec<EventRecord>> {
    let mut stmt = conn.prepare(
        "SELECT e.id, e.event_id, e.event_json, e.pubkey, e.kind, e.content, e.created_at, e.received_at,
                e.language
         FROM events e
         INNER JOIN conversation_logs cl ON e.id = cl.event_ref_id
         WHERE cl.bot_pubkey = ?
           AND e.pubkey IN (?, ?)
         ORDER BY e.created_at DESC
         LIMIT ?"
    )?;
    
    let events = stmt.query_map(params![bot_pubkey, user_pubkey, bot_pubkey, limit], |row| {
        Ok(EventRecord {
            id: row.get(0)?,
            event_id: row.get(1)?,
            event_json: row.get(2)?,
            pubkey: row.get(3)?,
            kind: row.get(4)?,
            content: row.get(5)?,
            created_at: row.get(6)?,
            received_at: row.get(7)?,
            language: row.get(8)?,
        })
    })?
    .collect::<Result<Vec<_>>>()?;
    
    // 時系列順（古い順）に反転
    Ok(events.into_iter().rev().collect())
}

/// 特定スレッド内の会話履歴を取得（ユーザーとBotの会話のみ）
pub fn get_conversation_timeline_in_thread(
    conn: &Connection,
    bot_pubkey: &str,
    user_pubkey: &str,
    thread_root_id: Option<&str>,
    limit: usize,
) -> Result<Vec<EventRecord>> {
    let mut stmt = if thread_root_id.is_some() {
        // スレッドが指定されている場合
        conn.prepare(
            "SELECT e.id, e.event_id, e.event_json, e.pubkey, e.kind, e.content, e.created_at, e.received_at,
                    e.language
             FROM events e
             INNER JOIN conversation_logs cl ON e.id = cl.event_ref_id
             WHERE cl.bot_pubkey = ?
               AND cl.thread_root_id = ?
               AND e.pubkey IN (?, ?)
             ORDER BY e.created_at DESC
             LIMIT ?"
        )?
    } else {
        // スレッドが指定されていない場合（thread_root_idがNULLのもの）
        conn.prepare(
            "SELECT e.id, e.event_id, e.event_json, e.pubkey, e.kind, e.content, e.created_at, e.received_at,
                    e.language
             FROM events e
             INNER JOIN conversation_logs cl ON e.id = cl.event_ref_id
             WHERE cl.bot_pubkey = ?
               AND cl.thread_root_id IS NULL
               AND e.pubkey IN (?, ?)
             ORDER BY e.created_at DESC
             LIMIT ?"
        )?
    };
    
    let events = if let Some(thread_id) = thread_root_id {
        stmt.query_map(params![bot_pubkey, thread_id, user_pubkey, bot_pubkey, limit], |row| {
            Ok(EventRecord {
                id: row.get(0)?,
                event_id: row.get(1)?,
                event_json: row.get(2)?,
                pubkey: row.get(3)?,
                kind: row.get(4)?,
                content: row.get(5)?,
                created_at: row.get(6)?,
                received_at: row.get(7)?,
                language: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?
    } else {
        stmt.query_map(params![bot_pubkey, user_pubkey, bot_pubkey, limit], |row| {
            Ok(EventRecord {
                id: row.get(0)?,
                event_id: row.get(1)?,
                event_json: row.get(2)?,
                pubkey: row.get(3)?,
                kind: row.get(4)?,
                content: row.get(5)?,
                created_at: row.get(6)?,
                received_at: row.get(7)?,
                language: row.get(8)?,
            })
        })?
        .collect::<Result<_>>()?
    };
    
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
    participants: Option<&[String]>,
    from_timestamp: i64,
    to_timestamp: i64,
) -> Result<i64> {
    let participants_json = participants.map(|p| {
        serde_json::to_string(p).unwrap_or_default()
    });
    
    let now = Utc::now().timestamp();
    
    conn.execute(
        "INSERT INTO conversation_summaries (bot_pubkey, summary, user_input, participants_json, from_timestamp, to_timestamp, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        params![
            bot_pubkey,
            summary,
            user_input,
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
        "SELECT id, bot_pubkey, summary, user_input, participants_json, from_timestamp, to_timestamp, created_at
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
            participants_json: row.get(4)?,
            from_timestamp: row.get(5)?,
            to_timestamp: row.get(6)?,
            created_at: row.get(7)?,
        })
    })?
    .collect::<Result<Vec<_>>>()?;
    
    Ok(summaries)
}
