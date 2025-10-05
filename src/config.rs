use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    pub admin_pubkeys: Vec<String>,
    pub root_bot_pubkey: String,
    pub prompt: String,
    pub picture: String,
    pub about: String,
    pub reaction_percent: i64,
    pub reaction_freq: i64,
    pub blacklist: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GptConfig {
    pub answer_length: i32,
    pub timeout: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayConfig {
    pub write: Vec<String>,
    pub read: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub relay_servers: RelayConfig,
    pub bot: BotConfig,
    pub gpt: GptConfig,
}
