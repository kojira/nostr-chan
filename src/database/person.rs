use rusqlite::{params, Connection, Result};
use chrono::Utc;
use nostr_sdk::prelude::Keys;

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
    pub air_reply_single_ratio: i32,
}

/// Botを追加
pub fn add_person(conn: &Connection, pubkey: &str, secretkey: &str, prompt: &str, content: &str, air_reply_single_ratio: Option<i32>) -> Result<()> {
    let now = Utc::now().timestamp();
    let ratio = air_reply_single_ratio.unwrap_or(30); // デフォルト30
    conn.execute(
        "INSERT INTO Persons (status, prompt, pubkey, secretkey, content, air_reply_single_ratio, created_at) VALUES(0, ?, ?, ?, ?, ?, datetime(?, 'unixepoch'))",
        params![prompt, pubkey, secretkey, content, ratio, now],
    )?;
    Ok(())
}

/// Botを更新
pub fn update_person(conn: &Connection, pubkey: &str, secretkey: &str, prompt: &str, content: &str, air_reply_single_ratio: i32) -> Result<()> {
    conn.execute(
        "UPDATE Persons SET secretkey = ?, prompt = ?, content = ?, air_reply_single_ratio = ? WHERE pubkey = ?",
        params![secretkey, prompt, content, air_reply_single_ratio, pubkey],
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

// ========== Analytics ==========

/// Bot毎の日別投稿数を取得（過去N日分）
pub fn get_bot_daily_reply_counts(conn: &Connection, days: i64) -> Result<Vec<(String, String, i64)>> {
    let cutoff_time = Utc::now().timestamp() - (days * 24 * 60 * 60);
    
    let mut stmt = conn.prepare(
        "SELECT 
            cl.bot_pubkey,
            date(cl.logged_at, 'unixepoch') as date,
            COUNT(*) as count
         FROM conversation_logs cl
         WHERE cl.is_bot_message = 1
         AND cl.logged_at >= ?
         GROUP BY cl.bot_pubkey, date
         ORDER BY date ASC"
    )?;
    
    let results = stmt.query_map(params![cutoff_time], |row| {
        Ok((
            row.get::<_, String>(0)?, // bot_pubkey
            row.get::<_, String>(1)?, // date (YYYY-MM-DD)
            row.get::<_, i64>(2)?,    // count
        ))
    })?
    .collect::<Result<Vec<_>>>()?;
    
    Ok(results)
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
                air_reply_single_ratio: row.get(8).unwrap_or(30),
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
                air_reply_single_ratio: row.get(8).unwrap_or(30),
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
                air_reply_single_ratio: row.get(8).unwrap_or(30),
            })
        })?
        .next()
        .unwrap()?;

    Ok(person.clone())
}
