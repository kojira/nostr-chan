use rusqlite::{params, Connection, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// ユーザー属性の構造化データ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAttributes {
    pub nickname: Option<String>,           // 愛称
    pub age: Option<String>,                // 年齢
    pub gender: Option<String>,             // 性別
    pub personality: Option<String>,        // 性格
    pub likes: Vec<String>,                 // 好きなもの
    pub dislikes: Vec<String>,              // 嫌いなもの
    pub family: Option<String>,             // 家族構成
    pub catchphrase: Option<String>,        // 口癖
    pub current_boom: Option<String>,       // マイブーム
    pub occupation: Option<String>,         // 職業・仕事
    pub country: Option<String>,            // 居住国
    pub hobbies: Vec<String>,               // 趣味・特技
    pub values: Option<String>,             // 価値観・信念
    pub recent_events: Option<String>,      // 最近の出来事
    pub conversation_style: Option<String>, // 会話のスタイル
    pub nostr_experience: Option<String>,   // Nostr歴
    pub frequent_topics: Vec<String>,       // よく話す話題
    pub impression: Option<String>,         // 総合的な印象
}

impl UserAttributes {
    /// 空の属性を作成
    pub fn empty() -> Self {
        Self {
            nickname: None,
            age: None,
            gender: None,
            personality: None,
            likes: Vec::new(),
            dislikes: Vec::new(),
            family: None,
            catchphrase: None,
            current_boom: None,
            occupation: None,
            country: None,
            hobbies: Vec::new(),
            values: None,
            recent_events: None,
            conversation_style: None,
            nostr_experience: None,
            frequent_topics: Vec::new(),
            impression: None,
        }
    }

    /// JSONからパース
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// JSONに変換
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// YAMLに変換（GPTプロンプト用）
    pub fn to_yaml_string(&self) -> String {
        serde_yaml::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// ユーザーへの最新の印象を取得（JSON文字列）
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

/// ユーザーへの最新の印象を構造化データとして取得
pub fn get_user_attributes(
    conn: &Connection,
    bot_pubkey: &str,
    user_pubkey: &str,
) -> Result<Option<UserAttributes>> {
    match get_user_impression(conn, bot_pubkey, user_pubkey)? {
        Some(data_str) => {
            // まずJSONとしてパースを試みる
            match UserAttributes::from_json(&data_str) {
                Ok(attrs) => Ok(Some(attrs)),
                Err(_) => {
                    // JSONパースに失敗した場合、古い形式（単純なテキスト）として扱う
                    // impressionフィールドに入れて返す
                    let mut attrs = UserAttributes::empty();
                    attrs.impression = Some(data_str);
                    Ok(Some(attrs))
                }
            }
        }
        None => Ok(None),
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
#[allow(dead_code)]
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

