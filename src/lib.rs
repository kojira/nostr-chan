pub mod config;
pub mod database;
pub use database as db;  // 外部クレートからdb::でアクセス可能に
pub mod gpt;
pub mod commands;
pub mod util;
pub mod conversation;
pub mod dashboard;
pub mod init;
pub mod event_processor;

// main.rs 内の公開構造体
#[derive(Clone, Debug)]
pub struct TimelinePost {
    pub pubkey: String,
    pub name: Option<String>,
    pub content: String,
    pub timestamp: i64,
}


