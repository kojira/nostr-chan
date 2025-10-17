use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use chrono::Utc;

/// Bot心境の構造化データ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MentalDiary {
    pub mood: String,                          // 現在の気分
    pub favorite_people: Vec<String>,          // 好きな人・気になる人
    pub disliked_people: Vec<String>,          // 苦手な人
    pub trusted_people: Vec<String>,           // 信頼できる人
    pub current_interests: Vec<String>,        // 今興味のあること
    pub want_to_learn: Vec<String>,            // 学びたいこと
    pub bored_with: Vec<String>,               // 飽きたこと
    pub short_term_goals: String,              // 短期目標
    pub long_term_goals: String,               // 長期目標
    pub concerns: String,                      // 悩み・課題
    pub recent_happy_events: String,           // 嬉しかったこと
    pub recent_sad_events: String,             // 悲しかったこと
    pub recent_surprises: String,              // 驚いたこと
    pub self_changes: String,                  // 自分の変化
    pub personality_state: String,             // 人格の状態
}

impl MentalDiary {
    /// 空の心境を作成
    pub fn empty() -> Self {
        Self {
            mood: String::new(),
            favorite_people: Vec::new(),
            disliked_people: Vec::new(),
            trusted_people: Vec::new(),
            current_interests: Vec::new(),
            want_to_learn: Vec::new(),
            bored_with: Vec::new(),
            short_term_goals: String::new(),
            long_term_goals: String::new(),
            concerns: String::new(),
            recent_happy_events: String::new(),
            recent_sad_events: String::new(),
            recent_surprises: String::new(),
            self_changes: String::new(),
            personality_state: String::new(),
        }
    }

    /// YAML形式の文字列に変換（プロンプト用）
    pub fn to_yaml_string(&self) -> String {
        let mut lines = Vec::new();
        
        if !self.mood.is_empty() {
            lines.push(format!("mood: {}", self.mood));
        }
        
        if !self.favorite_people.is_empty() {
            lines.push("favorite_people:".to_string());
            for person in &self.favorite_people {
                lines.push(format!("  - {}", person));
            }
        }
        
        if !self.disliked_people.is_empty() {
            lines.push("disliked_people:".to_string());
            for person in &self.disliked_people {
                lines.push(format!("  - {}", person));
            }
        }
        
        if !self.trusted_people.is_empty() {
            lines.push("trusted_people:".to_string());
            for person in &self.trusted_people {
                lines.push(format!("  - {}", person));
            }
        }
        
        if !self.current_interests.is_empty() {
            lines.push("current_interests:".to_string());
            for interest in &self.current_interests {
                lines.push(format!("  - {}", interest));
            }
        }
        
        if !self.want_to_learn.is_empty() {
            lines.push("want_to_learn:".to_string());
            for item in &self.want_to_learn {
                lines.push(format!("  - {}", item));
            }
        }
        
        if !self.bored_with.is_empty() {
            lines.push("bored_with:".to_string());
            for item in &self.bored_with {
                lines.push(format!("  - {}", item));
            }
        }
        
        if !self.short_term_goals.is_empty() {
            lines.push(format!("short_term_goals: {}", self.short_term_goals));
        }
        
        if !self.long_term_goals.is_empty() {
            lines.push(format!("long_term_goals: {}", self.long_term_goals));
        }
        
        if !self.concerns.is_empty() {
            lines.push(format!("concerns: {}", self.concerns));
        }
        
        if !self.recent_happy_events.is_empty() {
            lines.push(format!("recent_happy_events: {}", self.recent_happy_events));
        }
        
        if !self.recent_sad_events.is_empty() {
            lines.push(format!("recent_sad_events: {}", self.recent_sad_events));
        }
        
        if !self.recent_surprises.is_empty() {
            lines.push(format!("recent_surprises: {}", self.recent_surprises));
        }
        
        if !self.self_changes.is_empty() {
            lines.push(format!("self_changes: {}", self.self_changes));
        }
        
        if !self.personality_state.is_empty() {
            lines.push(format!("personality_state: {}", self.personality_state));
        }
        
        lines.join("\n")
    }
}

/// Bot心境レコード
#[derive(Debug, Clone)]
pub struct BotMentalStateRecord {
    pub id: i64,
    pub bot_pubkey: String,
    pub mental_state_json: String,
    pub created_at: i64,
}

impl BotMentalStateRecord {
    /// JSON文字列をMentalDiaryにパース
    pub fn parse_mental_diary(&self) -> Result<MentalDiary, serde_json::Error> {
        serde_json::from_str(&self.mental_state_json)
    }
}

/// Botの最新の心境を取得
pub fn get_bot_mental_state(
    conn: &Connection,
    bot_pubkey: &str,
) -> Result<Option<MentalDiary>> {
    let mut stmt = conn.prepare(
        "SELECT id, bot_pubkey, mental_state_json, created_at 
         FROM bot_mental_state 
         WHERE bot_pubkey = ? 
         ORDER BY created_at DESC 
         LIMIT 1"
    )?;
    
    let mut rows = stmt.query(params![bot_pubkey])?;
    
    if let Some(row) = rows.next()? {
        let record = BotMentalStateRecord {
            id: row.get(0)?,
            bot_pubkey: row.get(1)?,
            mental_state_json: row.get(2)?,
            created_at: row.get(3)?,
        };
        
        match record.parse_mental_diary() {
            Ok(diary) => Ok(Some(diary)),
            Err(e) => {
                eprintln!("[MentalState] JSONパースエラー: {}", e);
                Ok(None)
            }
        }
    } else {
        Ok(None)
    }
}

/// Botの心境を保存（履歴として追加）
pub fn save_bot_mental_state(
    conn: &Connection,
    bot_pubkey: &str,
    mental_diary: &MentalDiary,
) -> Result<()> {
    let mental_state_json = serde_json::to_string(mental_diary)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    
    let now = Utc::now().timestamp();
    
    conn.execute(
        "INSERT INTO bot_mental_state (bot_pubkey, mental_state_json, created_at) 
         VALUES (?, ?, ?)",
        params![bot_pubkey, mental_state_json, now],
    )?;
    
    Ok(())
}

/// Botの心境履歴を取得
pub fn get_bot_mental_state_history(
    conn: &Connection,
    bot_pubkey: &str,
    limit: usize,
    offset: usize,
) -> Result<Vec<BotMentalStateRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, bot_pubkey, mental_state_json, created_at 
         FROM bot_mental_state 
         WHERE bot_pubkey = ? 
         ORDER BY created_at DESC 
         LIMIT ? OFFSET ?"
    )?;
    
    let rows = stmt.query_map(params![bot_pubkey, limit, offset], |row| {
        Ok(BotMentalStateRecord {
            id: row.get(0)?,
            bot_pubkey: row.get(1)?,
            mental_state_json: row.get(2)?,
            created_at: row.get(3)?,
        })
    })?;
    
    let mut records = Vec::new();
    for record in rows {
        records.push(record?);
    }
    
    Ok(records)
}

/// Bot心境の履歴件数を取得
pub fn count_bot_mental_state_history(
    conn: &Connection,
    bot_pubkey: &str,
) -> Result<usize> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM bot_mental_state WHERE bot_pubkey = ?",
        params![bot_pubkey],
        |row| row.get(0),
    )?;
    
    Ok(count as usize)
}

