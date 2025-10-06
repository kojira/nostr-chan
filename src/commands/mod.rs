pub mod user;
pub mod admin;

use crate::config;
use crate::db;
use crate::util;
use nostr_sdk::prelude::*;
use rusqlite::Connection;
use std::future::Future;

// ヘルパー関数: コマンドを非同期実行
fn spawn_command<F>(future: F, error_msg: String)
where
    F: Future<Output = Result<()>> + Send + 'static,
{
    tokio::spawn(async move {
        if let Err(e) = future.await {
            eprintln!("{}: {}", error_msg, e);
        }
    });
}

// コマンドハンドラー（メインエントリーポイント）
pub async fn command_handler(
    config: &config::AppConfig,
    _conn: &Connection,
    persons: &Vec<db::Person>,
    event: &Event,
) -> Result<bool> {
    let admin_pubkeys = &config.bot.admin_pubkeys;
    let persons_ = persons.clone();
    let person_op = util::extract_mention(persons_, &event).unwrap();
    
    if event.content.contains("silent") {
        return Ok(false);
    }

    if person_op.is_none() {
        return Ok(false);
    }
    
    // bot自身の投稿にはコマンド反応しない
    let event_pubkey = event.pubkey.to_string();
    if persons.iter().any(|p| p.pubkey == event_pubkey) {
        return Ok(false);
    }
    
    let person = person_op.unwrap();
    
    // ユーザーコマンドをチェック
    for cmd in user::get_user_commands() {
        for pattern in &cmd.patterns {
            let matched = if cmd.require_start {
                // 文頭チェック（メンション後の文頭も考慮）
                event.content.starts_with(pattern) || 
                event.content.contains(&format!("\n{}", pattern))
            } else {
                event.content.contains(pattern)
            };
            
            if matched {
                spawn_command(
                    (cmd.handler)(config.clone(), person.clone(), event.clone()),
                    format!("{} error", cmd.name)
                );
                return Ok(true);
            }
        }
    }
    
    // 管理者コマンドをチェック
    let is_admin = admin_pubkeys.iter().any(|s| *s == event.pubkey.to_string());
    if !is_admin {
        return Ok(false);
    }
    
    let lines: Vec<String> = event.content.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return Ok(false);
    }
    
    // 管理者コマンド（person不要）
    for cmd in admin::get_admin_commands_simple() {
        if lines[0].contains(cmd.pattern) {
            spawn_command(
                (cmd.handler)(config.clone(), event.clone(), lines.clone()),
                format!("{} error", cmd.name)
            );
            return Ok(true);
        }
    }
    
    // 管理者コマンド（person必要）
    for cmd in admin::get_admin_commands() {
        if lines[0].contains(cmd.pattern) {
            spawn_command(
                (cmd.handler)(config.clone(), person.clone(), event.clone(), lines.clone()),
                format!("{} error", cmd.name)
            );
            return Ok(true);
        }
    }
    
    Ok(false)
}
