use crate::config;
use crate::database as db;
use crate::database;
use crate::util;
use nostr_sdk::prelude::*;
use serde_json::Value;
use std::future::Future;

// 管理者コマンド定義
pub struct AdminCommand {
    pub name: &'static str,
    pub pattern: &'static str,
    pub description: &'static str,
    pub handler: fn(config::AppConfig, db::Person, Event, Vec<String>) -> std::pin::Pin<Box<dyn Future<Output = Result<()>> + Send>>,
}

pub struct AdminCommandSimple {
    pub name: &'static str,
    pub pattern: &'static str,
    pub description: &'static str,
    pub handler: fn(config::AppConfig, Event, Vec<String>) -> std::pin::Pin<Box<dyn Future<Output = Result<()>> + Send>>,
}

// 管理者コマンドテーブル（person必要）
pub fn get_admin_commands() -> Vec<AdminCommand> {
    vec![
        AdminCommand {
            name: "get_kind0",
            pattern: "get kind 0",
            description: "リレーからkind 0を取得してDBを更新",
            handler: |c, p, e, _| Box::pin(admin_get_kind0(c, p, e)),
        },
        AdminCommand {
            name: "update_kind0",
            pattern: "update kind 0",
            description: "DBのkind 0を更新してブロードキャスト",
            handler: |c, p, e, l| Box::pin(admin_update_kind0(c, p, e, l)),
        },
        AdminCommand {
            name: "broadcast_kind0",
            pattern: "broadcast kind 0",
            description: "DBのkind 0をブロードキャスト",
            handler: |c, p, e, _| Box::pin(admin_broadcast_kind0(c, p, e)),
        },
        AdminCommand {
            name: "clear_follower_cache",
            pattern: "clear_follower_cache",
            description: "全フォロワーキャッシュをクリア",
            handler: |c, p, e, _| Box::pin(admin_clear_follower_cache(c, p, e)),
        },
        AdminCommand {
            name: "clear_follower_cache_space",
            pattern: "clear follower cache",
            description: "全フォロワーキャッシュをクリア",
            handler: |c, p, e, _| Box::pin(admin_clear_follower_cache(c, p, e)),
        },
    ]
}

// 管理者コマンドテーブル（person不要）
pub fn get_admin_commands_simple() -> Vec<AdminCommandSimple> {
    vec![
        AdminCommandSimple {
            name: "new",
            pattern: "new",
            description: "新しいボットを作成",
            handler: |c, e, l| Box::pin(admin_new(c, e, l)),
        },
    ]
}

// ========== 管理者コマンド実装 ==========

async fn admin_new(
    config: config::AppConfig,
    event: Event,
    lines: Vec<String>,
) -> Result<()> {
    let conn = db::connect()?;
    let keys = Keys::generate();
    let prompt = &lines[1];
    let content = &lines[2];
    database::person::insert_person(&conn, &keys, &prompt, &content)?;
    let new_person = db::get_person(&conn, &keys.public_key().to_string()).unwrap();
    util::send_kind0(&new_person.secretkey.to_string(), content).await?;
    let content: Value = serde_json::from_str(content)?;
    let display_name =
        &content["display_name"].to_string()[1..content["display_name"].to_string().len() - 1];
    util::reply_to(
        &config,
        event.clone(),
        new_person,
        &format!("{}です。コンゴトモヨロシク！", display_name),
    )
    .await?;
    Ok(())
}

async fn admin_get_kind0(
    config: config::AppConfig,
    person: db::Person,
    event: Event,
) -> Result<()> {
    println!("get kind 0");
    let _meta_event = util::get_kind0(&person.pubkey, &person.secretkey).await?;
    let conn = db::connect()?;
    database::person::update_person_content(&conn, &person.pubkey, &_meta_event.content.to_string())?;
    util::reply_to(
        &config,
        event.clone(),
        person.clone(),
        &format!("リレーからkind 0を取得してデータベース情報を更新しました"),
    )
    .await?;
    Ok(())
}

async fn admin_update_kind0(
    config: config::AppConfig,
    person: db::Person,
    event: Event,
    lines: Vec<String>,
) -> Result<()> {
    println!("update kind 0");
    let conn = db::connect()?;
    database::person::update_person_content(&conn, &person.pubkey, &lines[1])?;
    util::send_kind0(&person.secretkey.to_string(), &lines[1]).await?;
    util::reply_to(
        &config,
        event.clone(),
        person.clone(),
        &format!("データベースのkind 0を更新してブロードキャストしました"),
    )
    .await?;
    Ok(())
}

async fn admin_broadcast_kind0(
    config: config::AppConfig,
    person: db::Person,
    event: Event,
) -> Result<()> {
    println!("broadcast kind 0");
    util::send_kind0(&person.secretkey.to_string(), &person.content.to_string()).await?;
    util::reply_to(
        &config,
        event.clone(),
        person.clone(),
        &format!("データベースのkind 0の情報をブロードキャストしました"),
    )
    .await?;
    Ok(())
}

async fn admin_clear_follower_cache(
    config: config::AppConfig,
    person: db::Person,
    event: Event,
) -> Result<()> {
    println!("clear follower cache");
    let conn = db::connect()?;
    let deleted_count = db::clear_follower_cache(&conn)?;
    util::reply_to(
        &config,
        event.clone(),
        person.clone(),
        &format!("フォロワーキャッシュをクリアしました（{}件削除）", deleted_count),
    )
    .await?;
    Ok(())
}
