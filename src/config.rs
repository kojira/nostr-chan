use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BotConfig {
    pub admin_pubkeys: Vec<String>,
    pub root_bot_pubkey: String,
    pub prompt: String,
    pub picture: String,
    pub about: String,
    pub reaction_freq: i32,
    pub blacklist: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GptConfig {
    pub answer_length: i32,
    pub timeout: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub relay_servers: Vec<String>,
    pub bot: BotConfig,
    pub gpt: GptConfig,
}
