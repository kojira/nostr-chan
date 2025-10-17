use rusqlite::{params, Connection, Result};
use chrono::Utc;

#[allow(dead_code)]
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
#[allow(dead_code)]
pub fn set_system_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    let now = Utc::now().timestamp();
    conn.execute(
        "INSERT OR REPLACE INTO system_settings (key, value, updated_at) VALUES (?, ?, ?)",
        params![key, value, now],
    )?;
    Ok(())
}

// Bot全体一時停止状態の取得
#[allow(dead_code)]
pub fn is_global_pause(conn: &Connection) -> Result<bool> {
    match get_system_setting(conn, "global_pause")? {
        Some(value) => Ok(value == "true"),
        None => Ok(false),
    }
}
