use crate::config;
use crate::db;
use crate::gpt;
use crate::util;
use nostr_sdk::prelude::*;
use std::future::Future;
use std::time::Duration;

// ユーザーコマンド定義
pub struct UserCommand {
    pub name: &'static str,
    pub patterns: Vec<&'static str>,
    pub description: &'static str,
    pub require_start: bool,  // コマンドが文頭にあることを要求
    pub handler: fn(config::AppConfig, db::Person, Event) -> std::pin::Pin<Box<dyn Future<Output = Result<()>> + Send>>,
}

// ユーザーコマンドテーブル
pub fn get_user_commands() -> Vec<UserCommand> {
    vec![
        UserCommand {
            name: "fortune",
            patterns: vec!["占って"],
            description: "今日の運勢を占います",
            require_start: false,
            handler: |c, p, e| Box::pin(fortune(c, p, e)),
        },
        UserCommand {
            name: "zap_ranking",
            patterns: vec!["zap ranking"],
            description: "過去1年分のzapランキングを表示します",
            require_start: false,
            handler: |c, p, e| Box::pin(zap_ranking(c, p, e)),
        },
        UserCommand {
            name: "update_follower",
            patterns: vec!["update follower", "フォロワー更新"],
            description: "自分のフォロワーキャッシュを更新します",
            require_start: false,
            handler: |c, p, e| Box::pin(update_my_follower_cache(c, p, e)),
        },
        UserCommand {
            name: "search",
            patterns: vec!["search", "検索"],
            description: "投稿を検索します（例: search キーワード）",
            require_start: true,  // 文頭必須
            handler: |c, p, e| Box::pin(search_posts(c, p, e)),
        },
        UserCommand {
            name: "help",
            patterns: vec!["help", "ヘルプ"],
            description: "利用可能なコマンド一覧を表示します",
            require_start: false,
            handler: |c, p, e| Box::pin(show_help(c, p, e)),
        },
    ]
}

// ========== ユーザーコマンド実装 ==========

async fn fortune(config: config::AppConfig, person: db::Person, event: Event) -> Result<()> {
    let prompt = "あなたは占い師です。今日の運勢を占ってください。";
    let user_text = "今日の運勢を教えて";
    let reply = gpt::get_reply(prompt, user_text, true, None).await?;
    util::reply_to(&config, event, person, &reply).await?;
    Ok(())
}

async fn zap_ranking(config: config::AppConfig, person: db::Person, event: Event) -> Result<()> {
    // 簡易実装: 実際のzap集計はutil::get_zap_receivedを使用
    let reply = "Zapランキング機能は現在準備中です。".to_string();
    util::reply_to(&config, event, person, &reply).await?;
    Ok(())
}

async fn update_my_follower_cache(config: config::AppConfig, person: db::Person, event: Event) -> Result<()> {
    let user_pubkey = event.pubkey.to_string();
    let conn = db::connect()?;
    
    // キャッシュを削除
    let deleted = db::delete_user_follower_cache(&conn, &user_pubkey, &person.pubkey)?;
    
    // 新しくフォロワー状態を取得（キャッシュに保存される）
    let is_follower = util::is_follower(&user_pubkey, &person.secretkey).await?;
    
    let reply = format!(
        "フォロワーキャッシュを更新しました。\n削除: {}件\n現在のステータス: {}",
        deleted,
        if is_follower { "フォロワー" } else { "非フォロワー" }
    );
    
    util::reply_to(&config, event, person, &reply).await?;
    Ok(())
}

async fn search_posts(config: config::AppConfig, person: db::Person, event: Event) -> Result<()> {
    // コマンドからキーワードを抽出
    let content = event.content.clone();
    let keyword = if content.contains("search ") {
        content.split("search ").nth(1).unwrap_or("").trim()
    } else if content.contains("検索 ") {
        content.split("検索 ").nth(1).unwrap_or("").trim()
    } else {
        ""
    };
    
    println!("=== Search Command ===");
    println!("Original content: {}", content);
    println!("Extracted keyword: '{}'", keyword);
    
    if keyword.is_empty() {
        util::reply_to(&config, event, person, "検索キーワードを指定してください。\n例: search Nostr").await?;
        return Ok(());
    }
    
    // 検索リレーに接続
    let keys = Keys::generate();
    let client = Client::new(keys);
    
    println!("Connecting to search relays:");
    for relay in config.relay_servers.search.iter() {
        println!("  - {}", relay);
        client.add_relay(relay.clone()).await?;
    }
    client.connect().await;
    
    // 検索クエリ（最新10件）
    let filter = Filter::new()
        .kind(Kind::TextNote)
        .search(keyword)
        .limit(10);
    
    println!("Filter: {:?}", filter);
    println!("Fetching events...");
    
    let events = client.fetch_events(filter, Duration::from_secs(10)).await?;
    println!("Found {} events", events.len());
    
    client.shutdown().await;
    
    if events.is_empty() {
        println!("No results found for keyword: {}", keyword);
        util::reply_to(&config, event, person, &format!("「{}」の検索結果が見つかりませんでした。", keyword)).await?;
        return Ok(());
    }
    
    // 最新5件に絞る
    let mut sorted_events: Vec<_> = events.into_iter().collect();
    sorted_events.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let top_events: Vec<_> = sorted_events.into_iter().take(5).collect();
    
    // 結果を整形
    let mut reply = format!("【検索結果: {}】（最新{}件）\n\n", keyword, top_events.len());
    for search_event in &top_events {
        // 日時をフォーマット（日本時間）
        use chrono::{Local, TimeZone};
        let dt = Local.timestamp_opt(search_event.created_at.as_u64() as i64, 0).unwrap();
        let time_str = dt.format("%m/%d %H:%M").to_string();
        
        // npub形式に変換
        let npub = search_event.pubkey.to_bech32().unwrap();
        
        // UTF-8文字境界を考慮した切り出し
        let content = if search_event.content.chars().count() > 50 {
            let truncated: String = search_event.content.chars().take(50).collect();
            format!("{}...", truncated)
        } else {
            search_event.content.clone()
        };
        
        println!("Result: [{}] nostr:{} - {}", time_str, npub, content);
        reply.push_str(&format!("[{}] nostr:{}\n{}\n\n", time_str, npub, content));
    }
    
    println!("======================");
    util::reply_to(&config, event, person, &reply).await?;
    Ok(())
}

async fn show_help(config: config::AppConfig, person: db::Person, event: Event) -> Result<()> {
    let admin_pubkeys = &config.bot.admin_pubkeys;
    let is_admin = admin_pubkeys.iter().any(|s| *s == event.pubkey.to_string());
    
    let mut reply = "【利用可能なコマンド】\n\n".to_string();
    
    // ユーザーコマンド
    reply.push_str("■ ユーザーコマンド\n");
    for cmd in get_user_commands() {
        reply.push_str(&format!("・{}\n  {}\n", cmd.patterns.join(" / "), cmd.description));
    }
    
    // 管理者コマンド（管理者のみ）
    if is_admin {
        reply.push_str("\n■ 管理者コマンド\n");
        for cmd in super::admin::get_admin_commands() {
            reply.push_str(&format!("・{}\n  {}\n", cmd.pattern, cmd.description));
        }
        for cmd in super::admin::get_admin_commands_simple() {
            reply.push_str(&format!("・{}\n  {}\n", cmd.pattern, cmd.description));
        }
    }
    
    util::reply_to(&config, event, person, &reply).await?;
    Ok(())
}
