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
    pub detailed_help: Option<&'static str>,  // 詳細ヘルプ
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
            detailed_help: Some("占い師があなたの今日の運勢を占います。\n運の良さ、ラッキーアイテム、ラッキーカラーなどを教えてくれます。"),
            require_start: false,
            handler: |c, p, e| Box::pin(fortune(c, p, e)),
        },
        UserCommand {
            name: "zap_ranking",
            patterns: vec!["zap ranking"],
            description: "過去1年分のzapランキングを表示します",
            detailed_help: Some("過去1年間に受け取ったzapの合計金額ランキングを表示します。\n（現在準備中）"),
            require_start: false,
            handler: |c, p, e| Box::pin(zap_ranking(c, p, e)),
        },
        UserCommand {
            name: "update_follower",
            patterns: vec!["update follower", "フォロワー更新"],
            description: "自分のフォロワーキャッシュを更新します",
            detailed_help: Some("あなたのフォロワー状態のキャッシュを強制的に更新します。\nフォローしたばかりなのに反応がない場合などに使用してください。"),
            require_start: false,
            handler: |c, p, e| Box::pin(update_my_follower_cache(c, p, e)),
        },
        UserCommand {
            name: "search",
            patterns: vec!["search", "検索"],
            description: "投稿を検索します",
            detailed_help: Some(
"投稿を検索します。期間指定も可能です。

【使い方】
検索 キーワード [期間]

【期間指定】
・指定なし: 全期間から検索
・7d: 過去7日間
・30d: 過去30日間
・1h: 過去1時間
・24h: 過去24時間
・2024-10-01: 指定日以降

【例】
検索 Nostr
検索 Nostr 7d
検索 Nostr 2024-10-01"
            ),
            require_start: true,  // 文頭必須
            handler: |c, p, e| Box::pin(search_posts(c, p, e)),
        },
        UserCommand {
            name: "help",
            patterns: vec!["help", "ヘルプ"],
            description: "利用可能なコマンド一覧を表示します",
            detailed_help: Some("利用可能なコマンドの一覧を表示します。\n\n【使い方】\nhelp: 全コマンド一覧\nhelp コマンド名: 特定コマンドの詳細ヘルプ\n\n【例】\nhelp\nhelp search"),
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
    use chrono::{Duration as ChronoDuration, NaiveDate, Utc};
    
    // コマンドからキーワードと日時オプションを抽出
    let content = event.content.clone();
    let args = if content.contains("search ") {
        content.split("search ").nth(1).unwrap_or("").trim()
    } else if content.contains("検索 ") {
        content.split("検索 ").nth(1).unwrap_or("").trim()
    } else {
        ""
    };
    
    println!("=== Search Command ===");
    println!("Original content: {}", content);
    println!("Extracted args: '{}'", args);
    
    if args.is_empty() {
        util::reply_to(&config, event, person, "検索キーワードを指定してください。\n例: 検索 Nostr\n例: 検索 Nostr 7d (過去7日間)\n例: 検索 Nostr 2024-10-01 (指定日以降)").await?;
        return Ok(());
    }
    
    // キーワードと日時オプションを分離
    let parts: Vec<&str> = args.split_whitespace().collect();
    let keyword = parts[0];
    let time_option = parts.get(1).copied();
    
    println!("Keyword: '{}', Time option: {:?}", keyword, time_option);
    
    // 日時オプションをパース
    let since_timestamp = if let Some(opt) = time_option {
        if opt.ends_with('d') {
            // 日数指定 (例: 7d, 30d)
            if let Ok(days) = opt.trim_end_matches('d').parse::<i64>() {
                let since = Utc::now() - ChronoDuration::days(days);
                Some(Timestamp::from(since.timestamp() as u64))
            } else {
                None
            }
        } else if opt.ends_with('h') {
            // 時間指定 (例: 1h, 24h)
            if let Ok(hours) = opt.trim_end_matches('h').parse::<i64>() {
                let since = Utc::now() - ChronoDuration::hours(hours);
                Some(Timestamp::from(since.timestamp() as u64))
            } else {
                None
            }
        } else if let Ok(date) = NaiveDate::parse_from_str(opt, "%Y-%m-%d") {
            // 日付指定 (例: 2024-10-01)
            let datetime = date.and_hms_opt(0, 0, 0).unwrap();
            let timestamp = datetime.and_utc().timestamp();
            Some(Timestamp::from(timestamp as u64))
        } else {
            None
        }
    } else {
        None
    };
    
    if time_option.is_some() && since_timestamp.is_none() {
        util::reply_to(&config, event, person, "日時指定の形式が不正です。\n例: 7d (過去7日間)\n例: 1h (過去1時間)\n例: 2024-10-01 (指定日以降)").await?;
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
    let mut filter = Filter::new()
        .kind(Kind::TextNote)
        .search(keyword)
        .limit(10);
    
    if let Some(since) = since_timestamp {
        filter = filter.since(since);
        println!("Searching since: {}", since);
    }
    
    println!("Filter: {:?}", filter);
    println!("Fetching events...");
    
    let events = client.fetch_events(filter, Duration::from_secs(10)).await?;
    println!("Found {} events", events.len());
    
    client.shutdown().await;
    
    // 検索コマンドを実行した投稿自体を除外
    let filtered_events: Vec<_> = events.into_iter()
        .filter(|e| e.id != event.id)
        .collect();
    
    if filtered_events.is_empty() {
        println!("No results found for keyword: {}", keyword);
        util::reply_to(&config, event, person, &format!("「{}」の検索結果が見つかりませんでした。", keyword)).await?;
        return Ok(());
    }
    
    // 最新5件に絞る
    let mut sorted_events = filtered_events;
    sorted_events.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let top_events: Vec<_> = sorted_events.into_iter().take(5).collect();
    
    // 結果を整形
    let time_range_text = if let Some(opt) = time_option {
        format!("（{}）", opt)
    } else {
        "（全期間）".to_string()
    };
    let mut reply = format!("【検索結果: {}】{} 最新{}件\n\n", keyword, time_range_text, top_events.len());
    for search_event in &top_events {
        // 日時をフォーマット（日本時間）
        use chrono::{Local, TimeZone};
        let dt = Local.timestamp_opt(search_event.created_at.as_u64() as i64, 0).unwrap();
        let time_str = dt.format("%m/%d %H:%M").to_string();
        
        // note1形式に変換（イベントID）
        let note = search_event.id.to_bech32().unwrap();
        
        println!("Result: [{}] nostr:{}", time_str, note);
        reply.push_str(&format!("[{}] nostr:{}\n", time_str, note));
    }
    
    println!("======================");
    util::reply_to(&config, event, person, &reply).await?;
    Ok(())
}

async fn show_help(config: config::AppConfig, person: db::Person, event: Event) -> Result<()> {
    let admin_pubkeys = &config.bot.admin_pubkeys;
    let is_admin = admin_pubkeys.iter().any(|s| *s == event.pubkey.to_string());
    
    // コマンド引数を抽出（help の後に特定コマンド名があるか）
    let content = event.content.clone();
    let help_arg = if content.contains("help ") {
        content.split("help ").nth(1).map(|s| s.trim())
    } else if content.contains("ヘルプ ") {
        content.split("ヘルプ ").nth(1).map(|s| s.trim())
    } else {
        None
    };
    
    // 特定コマンドの詳細ヘルプを表示
    if let Some(cmd_name) = help_arg {
        if !cmd_name.is_empty() {
            // ユーザーコマンドから検索
            for cmd in get_user_commands() {
                if cmd.name == cmd_name || cmd.patterns.iter().any(|p| *p == cmd_name) {
                    let mut reply = format!("【{}】\n\n", cmd.patterns.join(" / "));
                    if let Some(detailed) = cmd.detailed_help {
                        reply.push_str(detailed);
                    } else {
                        reply.push_str(cmd.description);
                    }
                    util::reply_to(&config, event, person, &reply).await?;
                    return Ok(());
                }
            }
            
            // 管理者コマンドから検索（管理者のみ）
            if is_admin {
                for cmd in super::admin::get_admin_commands() {
                    if cmd.name == cmd_name || cmd.pattern == cmd_name {
                        let reply = format!("【{}】\n\n{}", cmd.pattern, cmd.description);
                        util::reply_to(&config, event, person, &reply).await?;
                        return Ok(());
                    }
                }
                for cmd in super::admin::get_admin_commands_simple() {
                    if cmd.name == cmd_name || cmd.pattern == cmd_name {
                        let reply = format!("【{}】\n\n{}", cmd.pattern, cmd.description);
                        util::reply_to(&config, event, person, &reply).await?;
                        return Ok(());
                    }
                }
            }
            
            // コマンドが見つからない場合
            util::reply_to(&config, event, person, &format!("コマンド「{}」が見つかりません。\n「help」で全コマンド一覧を表示します。", cmd_name)).await?;
            return Ok(());
        }
    }
    
    // 全コマンド一覧を表示
    let mut reply = "【利用可能なコマンド】\n\n".to_string();
    reply.push_str("詳細は「help コマンド名」で確認できます。\n\n");
    
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
