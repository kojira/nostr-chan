use rusqlite::{params, Connection, Result};
use chrono::Utc;

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

pub fn get_all_follower_cache(conn: &Connection) -> Result<Vec<(String, String, bool, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT user_pubkey, bot_pubkey, is_follower, cached_at FROM follower_cache ORDER BY cached_at DESC"
    )?;
    
    let results = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)? == 1,
            row.get::<_, i64>(3)?,
        ))
    })?
    .collect::<Result<Vec<_>>>()?;
    
    Ok(results)
}

pub fn update_follower_cache(conn: &Connection, user_pubkey: &str, bot_pubkey: &str, is_follower: bool) -> Result<()> {
    conn.execute(
        "UPDATE follower_cache SET is_follower = ?, cached_at = ? WHERE user_pubkey = ? AND bot_pubkey = ?",
        params![if is_follower { 1 } else { 0 }, Utc::now().timestamp(), user_pubkey, bot_pubkey],
    )?;
    Ok(())
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
