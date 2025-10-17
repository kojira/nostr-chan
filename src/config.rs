use serde::{Deserialize, Serialize};
use crate::database as db;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    pub admin_pubkeys: Vec<String>,
    pub root_bot_pubkey: String,
    pub prompt: String,
    pub picture: String,
    pub about: String,
    pub reaction_percent: i64,
    pub reaction_freq: i64,
    pub follower_cache_ttl: i64,
    pub timeline_size: usize,
    pub conversation_limit_count: usize,
    pub conversation_limit_minutes: i64,
    pub rag_similarity_threshold: f32,
    pub blacklist: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GptConfig {
    pub answer_length: i32,
    pub timeout: i32,
    pub search_answer_length: i32,
    pub gemini_search_timeout: i32,
    pub recent_context_count: usize,
    pub summary_threshold: usize,
    pub max_summary_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayConfig {
    pub write: Vec<String>,
    pub read: Vec<String>,
    pub search: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub relay_servers: RelayConfig,
    pub bot: BotConfig,
    pub gpt: GptConfig,
    pub dashboard: DashboardConfig,
}

/// 設定値取得のユーティリティ関数群
impl AppConfig {
    /// i64型の設定値を取得（DB優先、なければconfig値）
    pub fn get_i64_setting(&self, key: &str) -> i64 {
        match db::connect() {
            Ok(conn) => {
                match db::get_system_setting(&conn, key) {
                    Ok(Some(value)) => {
                        value.parse::<i64>().unwrap_or_else(|_| self.get_default_i64(key))
                    }
                    _ => self.get_default_i64(key),
                }
            }
            Err(_) => self.get_default_i64(key),
        }
    }

    /// i32型の設定値を取得（DB優先、なければconfig値）
    pub fn get_i32_setting(&self, key: &str) -> i32 {
        match db::connect() {
            Ok(conn) => {
                match db::get_system_setting(&conn, key) {
                    Ok(Some(value)) => {
                        value.parse::<i32>().unwrap_or_else(|_| self.get_default_i32(key))
                    }
                    _ => self.get_default_i32(key),
                }
            }
            Err(_) => self.get_default_i32(key),
        }
    }

    /// usize型の設定値を取得（DB優先、なければconfig値）
    pub fn get_usize_setting(&self, key: &str) -> usize {
        match db::connect() {
            Ok(conn) => {
                match db::get_system_setting(&conn, key) {
                    Ok(Some(value)) => {
                        value.parse::<usize>().unwrap_or_else(|_| self.get_default_usize(key))
                    }
                    _ => self.get_default_usize(key),
                }
            }
            Err(_) => self.get_default_usize(key),
        }
    }

    /// f32型の設定値を取得（DB優先、なければconfig値）
    pub fn get_f32_setting(&self, key: &str) -> f32 {
        match db::connect() {
            Ok(conn) => {
                match db::get_system_setting(&conn, key) {
                    Ok(Some(value)) => {
                        value.parse::<f32>().unwrap_or_else(|_| self.get_default_f32(key))
                    }
                    _ => self.get_default_f32(key),
                }
            }
            Err(_) => self.get_default_f32(key),
        }
    }

    /// u64型の設定値を取得（DB優先、なければconfig値）
    pub fn get_u64_setting(&self, key: &str) -> u64 {
        match db::connect() {
            Ok(conn) => {
                match db::get_system_setting(&conn, key) {
                    Ok(Some(value)) => {
                        value.parse::<u64>().unwrap_or_else(|_| self.get_default_u64(key))
                    }
                    _ => self.get_default_u64(key),
                }
            }
            Err(_) => self.get_default_u64(key),
        }
    }

    /// デフォルト値を取得（i64）
    fn get_default_i64(&self, key: &str) -> i64 {
        match key {
            "reaction_percent" => self.bot.reaction_percent,
            "reaction_freq" => self.bot.reaction_freq,
            "follower_cache_ttl" => self.bot.follower_cache_ttl,
            "conversation_limit_minutes" => self.bot.conversation_limit_minutes,
            _ => 0,
        }
    }

    /// デフォルト値を取得（i32）
    fn get_default_i32(&self, key: &str) -> i32 {
        match key {
            "gpt_answer_length" => self.gpt.answer_length,
            "gpt_timeout" => self.gpt.timeout,
            "gemini_search_timeout" => self.gpt.gemini_search_timeout,
            _ => 0,
        }
    }

    /// デフォルト値を取得（usize）
    fn get_default_usize(&self, key: &str) -> usize {
        match key {
            "timeline_size" => self.bot.timeline_size,
            "conversation_limit_count" => self.bot.conversation_limit_count,
            "recent_context_count" => self.gpt.recent_context_count,
            "summary_threshold" => self.gpt.summary_threshold,
            "max_summary_tokens" => self.gpt.max_summary_tokens,
            _ => 0,
        }
    }

    /// デフォルト値を取得（f32）
    fn get_default_f32(&self, key: &str) -> f32 {
        match key {
            "rag_similarity_threshold" => self.bot.rag_similarity_threshold,
            _ => 0.0,
        }
    }

    /// デフォルト値を取得（u64）
    fn get_default_u64(&self, key: &str) -> u64 {
        match key {
            "gpt_timeout" => self.gpt.timeout as u64,
            _ => 0,
        }
    }
}
