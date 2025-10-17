use rusqlite::{params, Connection, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum TokenCategory {
    Reply = 1,
    AirReply = 2,
    Summary = 3,
    SearchInitialReply = 4,
    SearchKeywordExtraction = 5,
    SearchFinalReply = 6,
}

impl TokenCategory {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "reply" => Some(Self::Reply),
            "air_reply" => Some(Self::AirReply),
            "summary" => Some(Self::Summary),
            "search_initial_reply" => Some(Self::SearchInitialReply),
            "search_keyword_extraction" => Some(Self::SearchKeywordExtraction),
            "search_final_reply" => Some(Self::SearchFinalReply),
            _ => None,
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            Self::Reply => "reply",
            Self::AirReply => "air_reply",
            Self::Summary => "summary",
            Self::SearchInitialReply => "search_initial_reply",
            Self::SearchKeywordExtraction => "search_keyword_extraction",
            Self::SearchFinalReply => "search_final_reply",
        }
    }
    
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Reply => "メンション返信",
            Self::AirReply => "エアリプ",
            Self::Summary => "会話要約",
            Self::SearchInitialReply => "検索一次回答",
            Self::SearchKeywordExtraction => "キーワード抽出",
            Self::SearchFinalReply => "検索最終回答",
        }
    }
    
    /// 全カテゴリを取得（マイグレーションなどで使用）
    #[allow(dead_code)]
    pub fn all() -> Vec<Self> {
        vec![
            Self::Reply,
            Self::AirReply,
            Self::Summary,
            Self::SearchInitialReply,
            Self::SearchKeywordExtraction,
            Self::SearchFinalReply,
        ]
    }
}

/// トークン使用量を記録
pub fn record_token_usage(
    conn: &Connection,
    bot_pubkey: &str,
    category: &str,
    prompt_tokens: usize,
    completion_tokens: usize,
    prompt_text: &str,
    completion_text: &str,
) -> Result<()> {
    let category_enum = TokenCategory::from_str(category)
        .ok_or_else(|| rusqlite::Error::InvalidParameterName(format!("Unknown category: {}", category)))?;
    let now = chrono::Utc::now().timestamp();
    let total_tokens = prompt_tokens + completion_tokens;
    
    conn.execute(
        "INSERT INTO token_usage (bot_pubkey, category_id, prompt_tokens, completion_tokens, total_tokens, prompt_text, completion_text, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        params![bot_pubkey, category_enum as i32, prompt_tokens as i64, completion_tokens as i64, total_tokens as i64, prompt_text, completion_text, now],
    )?;
    
    println!("[Token] Bot: {}, カテゴリ: {} ({}), プロンプト: {}トークン, 完了: {}トークン, 合計: {}トークン", 
             &bot_pubkey[..8], category_enum.display_name(), category_enum.name(), prompt_tokens, completion_tokens, total_tokens);
    
    Ok(())
}

/// トークン使用量の統計を取得（期間指定）
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TokenUsageStats {
    pub category: String,
    pub total_prompt_tokens: i64,
    pub total_completion_tokens: i64,
    pub total_tokens: i64,
    pub count: i64,
}

#[allow(dead_code)]
pub fn get_token_usage_stats(
    conn: &Connection,
    from_timestamp: i64,
    to_timestamp: i64,
) -> Result<Vec<TokenUsageStats>> {
    let mut stmt = conn.prepare(
        "SELECT 
            tc.name,
            SUM(tu.prompt_tokens) as total_prompt,
            SUM(tu.completion_tokens) as total_completion,
            SUM(tu.total_tokens) as total,
            COUNT(*) as count
         FROM token_usage tu
         JOIN token_categories tc ON tu.category_id = tc.id
         WHERE tu.created_at >= ? AND tu.created_at <= ?
         GROUP BY tc.name
         ORDER BY total DESC"
    )?;
    
    let stats = stmt.query_map(params![from_timestamp, to_timestamp], |row| {
        Ok(TokenUsageStats {
            category: row.get(0)?,
            total_prompt_tokens: row.get(1)?,
            total_completion_tokens: row.get(2)?,
            total_tokens: row.get(3)?,
            count: row.get(4)?,
        })
    })?
    .collect::<Result<Vec<_>>>()?;
    
    Ok(stats)
}

/// 時系列でのトークン使用量を取得（日別集計）
#[derive(Debug, Clone, serde::Serialize)]
pub struct DailyTokenUsage {
    pub date: String, // YYYY-MM-DD
    pub category: String,
    pub total_tokens: i64,
    pub count: i64,
}

pub fn get_daily_token_usage(
    conn: &Connection,
    days: i64,
) -> Result<Vec<DailyTokenUsage>> {
    let now = chrono::Utc::now().timestamp();
    let from_timestamp = now - (days * 86400);
    
    let mut stmt = conn.prepare(
        "SELECT 
            date(tu.created_at, 'unixepoch') as day,
            tc.name,
            SUM(tu.total_tokens) as total,
            COUNT(*) as count
         FROM token_usage tu
         JOIN token_categories tc ON tu.category_id = tc.id
         WHERE tu.created_at >= ?
         GROUP BY day, tc.name
         ORDER BY day DESC, tc.name"
    )?;
    
    let usage = stmt.query_map(params![from_timestamp], |row| {
        Ok(DailyTokenUsage {
            date: row.get(0)?,
            category: row.get(1)?,
            total_tokens: row.get(2)?,
            count: row.get(3)?,
        })
    })?
    .collect::<Result<Vec<_>>>()?;
    
    Ok(usage)
}

pub fn get_daily_token_usage_with_range(
    conn: &Connection,
    from_date: &str, // YYYY-MM-DD
    to_date: &str,   // YYYY-MM-DD
) -> Result<Vec<DailyTokenUsage>> {
    let mut stmt = conn.prepare(
        "SELECT 
            date(tu.created_at, 'unixepoch') as day,
            tc.name,
            SUM(tu.total_tokens) as total,
            COUNT(*) as count
         FROM token_usage tu
         JOIN token_categories tc ON tu.category_id = tc.id
         WHERE date(tu.created_at, 'unixepoch') >= ? 
           AND date(tu.created_at, 'unixepoch') <= ?
         GROUP BY day, tc.name
         ORDER BY day DESC, tc.name"
    )?;
    
    let usage = stmt.query_map(params![from_date, to_date], |row| {
        Ok(DailyTokenUsage {
            date: row.get(0)?,
            category: row.get(1)?,
            total_tokens: row.get(2)?,
            count: row.get(3)?,
        })
    })?
    .collect::<Result<Vec<_>>>()?;
    
    Ok(usage)
}

/// Bot別の日別トークン使用量を取得
#[allow(dead_code)]
pub fn get_daily_token_usage_by_bot(
    conn: &Connection,
    bot_pubkey: &str,
    days: i64,
) -> Result<Vec<DailyTokenUsage>> {
    let now = chrono::Utc::now().timestamp();
    let from_timestamp = now - (days * 86400);
    
    let mut stmt = conn.prepare(
        "SELECT 
            date(tu.created_at, 'unixepoch') as day,
            tc.name,
            SUM(tu.total_tokens) as total,
            COUNT(*) as count
         FROM token_usage tu
         JOIN token_categories tc ON tu.category_id = tc.id
         WHERE tu.bot_pubkey = ?
           AND tu.created_at >= ?
         GROUP BY day, tc.name
         ORDER BY day DESC, tc.name"
    )?;
    
    let usage = stmt.query_map(params![bot_pubkey, from_timestamp], |row| {
        Ok(DailyTokenUsage {
            date: row.get(0)?,
            category: row.get(1)?,
            total_tokens: row.get(2)?,
            count: row.get(3)?,
        })
    })?
    .collect::<Result<Vec<_>>>()?;
    
    Ok(usage)
}

/// Bot別の日付範囲指定でトークン使用量を取得
#[allow(dead_code)]
pub fn get_daily_token_usage_by_bot_with_range(
    conn: &Connection,
    bot_pubkey: &str,
    from_date: &str,
    to_date: &str,
) -> Result<Vec<DailyTokenUsage>> {
    let mut stmt = conn.prepare(
        "SELECT 
            date(tu.created_at, 'unixepoch') as day,
            tc.name,
            SUM(tu.total_tokens) as total,
            COUNT(*) as count
         FROM token_usage tu
         JOIN token_categories tc ON tu.category_id = tc.id
         WHERE tu.bot_pubkey = ?
           AND date(tu.created_at, 'unixepoch') >= ? 
           AND date(tu.created_at, 'unixepoch') <= ?
         GROUP BY day, tc.name
         ORDER BY day DESC, tc.name"
    )?;
    
    let usage = stmt.query_map(params![bot_pubkey, from_date, to_date], |row| {
        Ok(DailyTokenUsage {
            date: row.get(0)?,
            category: row.get(1)?,
            total_tokens: row.get(2)?,
            count: row.get(3)?,
        })
    })?
    .collect::<Result<Vec<_>>>()?;
    
    Ok(usage)
}
