use rusqlite::{params, Connection, Result};
use chrono::Utc;

/// ユーザーへの最新の印象を取得
pub fn get_user_impression(
    conn: &Connection,
    bot_pubkey: &str,
    user_pubkey: &str,
) -> Result<Option<String>> {
    let result = conn.query_row(
        "SELECT impression FROM user_impressions 
         WHERE bot_pubkey = ? AND user_pubkey = ?
         ORDER BY created_at DESC
         LIMIT 1",
        params![bot_pubkey, user_pubkey],
        |row| row.get(0),
    );
    
    match result {
        Ok(impression) => Ok(Some(impression)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

/// ユーザーへの印象を保存（履歴として追加）
pub fn save_user_impression(
    conn: &Connection,
    bot_pubkey: &str,
    user_pubkey: &str,
    impression: &str,
) -> Result<()> {
    let now = Utc::now().timestamp();
    
    conn.execute(
        "INSERT INTO user_impressions (bot_pubkey, user_pubkey, impression, created_at)
         VALUES (?, ?, ?, ?)",
        params![bot_pubkey, user_pubkey, impression, now],
    )?;
    
    Ok(())
}

/// ユーザーへの印象の変遷履歴を取得
pub fn get_user_impression_history(
    conn: &Connection,
    bot_pubkey: &str,
    user_pubkey: &str,
    limit: usize,
) -> Result<Vec<UserImpressionRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, bot_pubkey, user_pubkey, impression, created_at
         FROM user_impressions
         WHERE bot_pubkey = ? AND user_pubkey = ?
         ORDER BY created_at DESC
         LIMIT ?",
    )?;
    
    let records = stmt.query_map(params![bot_pubkey, user_pubkey, limit], |row| {
        Ok(UserImpressionRecord {
            id: row.get(0)?,
            bot_pubkey: row.get(1)?,
            user_pubkey: row.get(2)?,
            impression: row.get(3)?,
            created_at: row.get(4)?,
        })
    })?;
    
    let mut result = Vec::new();
    for record in records {
        result.push(record?);
    }
    
    Ok(result)
}

/// Bot別のユーザー印象一覧を取得（最新の印象のみ、ダッシュボード用）
pub fn get_all_user_impressions(
    conn: &Connection,
    bot_pubkey: &str,
    limit: usize,
    offset: usize,
) -> Result<Vec<UserImpressionRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, bot_pubkey, user_pubkey, impression, created_at
         FROM user_impressions
         WHERE bot_pubkey = ? AND id IN (
             SELECT MAX(id) FROM user_impressions
             WHERE bot_pubkey = ?
             GROUP BY user_pubkey
         )
         ORDER BY created_at DESC
         LIMIT ? OFFSET ?",
    )?;
    
    let records = stmt.query_map(params![bot_pubkey, bot_pubkey, limit, offset], |row| {
        Ok(UserImpressionRecord {
            id: row.get(0)?,
            bot_pubkey: row.get(1)?,
            user_pubkey: row.get(2)?,
            impression: row.get(3)?,
            created_at: row.get(4)?,
        })
    })?;
    
    let mut result = Vec::new();
    for record in records {
        result.push(record?);
    }
    
    Ok(result)
}

/// ユーザー印象を持つユーザー数を取得
pub fn count_users_with_impressions(conn: &Connection, bot_pubkey: &str) -> Result<usize> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(DISTINCT user_pubkey) FROM user_impressions WHERE bot_pubkey = ?",
        params![bot_pubkey],
        |row| row.get(0),
    )?;
    Ok(count as usize)
}

/// 特定ユーザーの印象履歴件数を取得
pub fn count_user_impression_history(
    conn: &Connection,
    bot_pubkey: &str,
    user_pubkey: &str,
) -> Result<usize> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM user_impressions WHERE bot_pubkey = ? AND user_pubkey = ?",
        params![bot_pubkey, user_pubkey],
        |row| row.get(0),
    )?;
    Ok(count as usize)
}

/// ユーザー印象レコード
#[derive(Debug, Clone)]
pub struct UserImpressionRecord {
    pub id: i64,
    pub bot_pubkey: String,
    pub user_pubkey: String,
    pub impression: String,
    pub created_at: i64,
}

