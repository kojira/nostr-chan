use chrono::Utc;
use nostr_sdk::prelude::*;
use rusqlite::{params, Connection, Result};

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

pub fn get_all_persons(conn: &Connection) -> Result<Vec<Person>> {
    let mut stmt = conn.prepare("SELECT * FROM Persons WHERE status=0")?;
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
pub fn get_follower_cache(conn: &Connection, user_pubkey: &str, bot_pubkey: &str, ttl: i64) -> Result<Option<bool>> {
    let now = Utc::now().timestamp();
    let mut stmt = conn.prepare(
        "SELECT is_follower, cached_at FROM follower_cache WHERE user_pubkey = ? AND bot_pubkey = ?"
    )?;
    
    let result = stmt.query_map(params![user_pubkey, bot_pubkey], |row| {
        Ok((row.get::<_, i32>(0)?, row.get::<_, i64>(1)?))
    })?
    .next();
    
    if let Some(Ok((is_follower, cached_at))) = result {
        // Check if cache is still valid
        if now - cached_at < ttl {
            return Ok(Some(is_follower != 0));
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
