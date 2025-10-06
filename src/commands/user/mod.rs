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
"投稿を検索します。期間・投稿者指定も可能です。

【使い方】
検索 キーワード [期間] [@投稿者]

【期間指定】
・指定なし: 全期間から検索
・7d: 過去7日間
・30d: 過去30日間
・1h: 過去1時間
・24h: 過去24時間
・2024-10-01: 指定日以降
・2024-10-01 14:30: 指定日時以降（Tなしでも可）
・2024-10-01~2024-10-31: 期間範囲
・2024-10-01 14:30~2024-10-31 18:00: 日時範囲
・2024-10-01~: 指定日以降
・~2024-10-31: 指定日以前

【投稿者指定】
・@npub1...: npub形式
・@hex...: hex形式（64文字）

※ ~ の代わりに 〜 も使用可能
※ 期間と投稿者は順不同

【例】
検索 Nostr
検索 Nostr 7d
検索 Nostr 2024-10-01
検索 Nostr 2024-10-01 14:30
検索 Nostr 2024-10-01~2024-10-31
検索 Nostr @npub1...
検索 Nostr 7d @npub1...
検索 Nostr 2024-10-01 14:30〜2024-10-31 18:00 @npub1..."
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
    use std::collections::HashMap;
    use serde_json::Value;
    
    println!("zap_ranking");
    let pubkey = &event.pubkey.to_string();
    let text = &format!("「現在から過去1年分のzapを集計します。しばらくお待ち下さい。」をあなたらしく言い換えてください。元の文章に含まれる内容が欠落しないようにしてください。「」内に入る文字だけを返信してください。カギカッコは不要です。");
    let reply = gpt::get_reply(&person.prompt, text, true, None).await.unwrap();
    let root_event: Event;
    if reply.len() > 0 {
        root_event = util::reply_to(&config, event.clone(), person.clone(), &reply).await?;
    } else {
        return Ok(());
    }
    
    let receive_zap_events = util::get_zap_received(pubkey).await?;
    let mut all_zap: u64 = 0;
    let mut zap_by_pubkey: HashMap<String, (u64, u64)> = HashMap::new(); // pubkeyごとの集計を保持するHashMap

    for zap_event in &receive_zap_events {
        let mut bolt11_string = "".to_string();
        let mut pubkey_str = "".to_string();
        for tag in zap_event.tags.iter() {
            if let Some(standardized) = tag.as_standardized() {
                match standardized {
                    TagStandard::Bolt11(bolt11) => {
                        bolt11_string = bolt11.to_string();
                    }
                    TagStandard::Description(description) => {
                        if let Ok(content_json) = serde_json::from_str::<Value>(&description) {
                            if let Some(pk) = content_json.get("pubkey").and_then(|k| k.as_str()) {
                                // descriptionの中のhex pubkeyをnpub形式に変換
                                if let Ok(key) = PublicKey::parse(pk) {
                                    pubkey_str = key.to_bech32().unwrap();
                                }
                            }
                        }
                    }
                    _ => {} // 他のタグは無視
                }
            }
        }
        let bolt11 = util::decode_bolt11_invoice(&bolt11_string);
        if let Ok(bolt11) = bolt11 {
            if let Some(raw_amount) = bolt11.amount_milli_satoshis() {
                all_zap += raw_amount;
                // pubkeyに基づいてraw_amountを集計
                let entry = zap_by_pubkey.entry(pubkey_str.to_string()).or_insert((0, 0));
                entry.0 += raw_amount; // zap合計を更新
                entry.1 += 1; // 回数
            }
        }
    }
    
    println!("Total raw amount: {:?}", all_zap);
    
    // HashMapからVecへ変換
    let mut zap_vec: Vec<(String, (u64, u64))> = zap_by_pubkey.into_iter().collect();
    // zap合計で降順ソート
    zap_vec.sort_by(|a, b| b.1.0.cmp(&a.1.0));
    
    let sender_ranking = zap_vec
        .iter()
        .take(10)
        .enumerate()
        .map(|(index, (pubkey, (zap, count)))| {
            format!("{}: {} Zap: {}, Count: {}", index + 1, pubkey, util::format_with_commas(zap / 1000), count)
        })
        .collect::<Vec<_>>()
        .join("\n");
    
    println!("Top 10 pubkeys by zap:");
    let root_event2 = root_event.clone();

    util::reply_to_by_event_id_pubkey(
        &config,
        root_event2.id,
        event.pubkey,
        person.clone(),
        &format!(
            "受取りzap総額:{} satoshi\n投げてくれた人Top10:\n{}",
            util::format_with_commas(all_zap / 1000),
            sender_ranking
        ),
    )
    .await?;
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
    use chrono::{Duration as ChronoDuration, NaiveDate, NaiveDateTime, Utc};
    
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
        util::reply_to(&config, event, person, "検索キーワードを指定してください。\n例: 検索 Nostr\n例: 検索 Nostr 7d\n例: 検索 Nostr 2024-10-01~2024-10-31\n例: 検索 Nostr @npub1...\n例: 検索 Nostr 2024-10-01 14:30~2024-10-31 18:00 @npub1...").await?;
        return Ok(());
    }
    
    // キーワードと日時オプション、pubkeyオプションを分離
    let parts: Vec<&str> = args.split_whitespace().collect();
    let keyword = parts[0];
    
    // pubkey指定を探す (@npub1... または @hex形式)
    let mut author_pubkey: Option<String> = None;
    let mut time_parts: Vec<&str> = Vec::new();
    
    for part in parts.iter().skip(1) {
        // pubkey判定: @で始まる、またはnpub1/nostr:で始まる
        let is_pubkey = part.starts_with('@') || part.starts_with("npub1") || part.starts_with("nostr:");
        
        if is_pubkey {
            // @, nostr: プレフィックスを除去
            let mut pubkey_str = part.trim_start_matches('@');
            pubkey_str = pubkey_str.trim_start_matches("nostr:");
            
            if pubkey_str.starts_with("npub1") {
                // npub形式をhexに変換
                if let Ok(pk) = PublicKey::from_bech32(pubkey_str) {
                    author_pubkey = Some(pk.to_hex());
                } else {
                    util::reply_to(&config, event, person, "npub形式が不正です。\n例: npub1..., @npub1..., nostr:npub1...").await?;
                    return Ok(());
                }
            } else if pubkey_str.len() == 64 && part.starts_with('@') {
                // hex形式（@付きの場合のみ）
                author_pubkey = Some(pubkey_str.to_string());
            } else {
                util::reply_to(&config, event, person, "pubkey形式が不正です。\n例: npub1..., @npub1..., nostr:npub1..., @hex").await?;
                return Ok(());
            }
        } else {
            // 時刻関連の要素を収集
            time_parts.push(*part);
        }
    }
    
    // 時刻要素を結合（スペース区切りの日時範囲に対応）
    println!("DEBUG: time_parts = {:?}", time_parts);
    let time_option = if !time_parts.is_empty() {
        Some(time_parts.join(" "))
    } else {
        None
    };
    
    println!("Keyword: '{}', Time option: {:?}, Author: {:?}", keyword, time_option, author_pubkey);
    
    // 日本時間のNaiveDateTimeをUTCタイムスタンプに変換
    let jst_to_utc_timestamp = |dt: NaiveDateTime| -> Timestamp {
        let utc_timestamp = dt.and_utc().timestamp() - 9 * 3600;
        Timestamp::from(utc_timestamp as u64)
    };
    
    // 日時パース用のヘルパー関数（日本時間として扱う）
    let parse_datetime = |s: &str| -> Option<Timestamp> {
        // 日時形式 (2024-10-01T14:30 または 2024-10-01 14:30)
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M") {
            return Some(jst_to_utc_timestamp(dt));
        }
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M") {
            return Some(jst_to_utc_timestamp(dt));
        }
        // 日付形式 (2024-10-01)
        if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
            let datetime = date.and_hms_opt(0, 0, 0).unwrap();
            return Some(jst_to_utc_timestamp(datetime));
        }
        None
    };
    
    // 日時オプションをパース
    let (since_timestamp, until_timestamp) = if let Some(opt) = time_option.as_ref() {
        // 全角チルダを半角に変換
        let opt = opt.replace('〜', "~");
        
        if opt.contains('~') {
            // 範囲指定 (例: 2024-10-01~2024-10-31, 2024-10-01T14:30~2024-10-31T18:00)
            let parts: Vec<&str> = opt.split('~').collect();
            let since = if parts[0].is_empty() {
                None
            } else if let Some(ts) = parse_datetime(parts[0]) {
                Some(ts)
            } else {
                util::reply_to(&config, event, person, "開始日時の形式が不正です。\n例: 2024-10-01 または 2024-10-01T14:30").await?;
                return Ok(());
            };
            
            let until = if parts.len() > 1 && !parts[1].is_empty() {
                // 日時形式の場合はそのまま、日付形式の場合は23:59:59を追加
                if let Ok(dt) = NaiveDateTime::parse_from_str(parts[1], "%Y-%m-%dT%H:%M") {
                    Some(jst_to_utc_timestamp(dt))
                } else if let Ok(dt) = NaiveDateTime::parse_from_str(parts[1], "%Y-%m-%d %H:%M") {
                    Some(jst_to_utc_timestamp(dt))
                } else if let Ok(date) = NaiveDate::parse_from_str(parts[1], "%Y-%m-%d") {
                    // 終了日は23:59:59まで含める
                    let datetime = date.and_hms_opt(23, 59, 59).unwrap();
                    Some(jst_to_utc_timestamp(datetime))
                } else {
                    util::reply_to(&config, event, person, "終了日時の形式が不正です。\n例: 2024-10-31, 2024-10-31 18:00, 2024-10-31T18:00").await?;
                    return Ok(());
                }
            } else {
                None
            };
            
            (since, until)
        } else if opt.ends_with('d') {
            // 日数指定 (例: 7d, 30d)
            if let Ok(days) = opt.trim_end_matches('d').parse::<i64>() {
                let since = Utc::now() - ChronoDuration::days(days);
                (Some(Timestamp::from(since.timestamp() as u64)), None)
            } else {
                return {
                    util::reply_to(&config, event, person, "日数指定の形式が不正です。\n例: 7d (過去7日間)").await?;
                    Ok(())
                };
            }
        } else if opt.ends_with('h') {
            // 時間指定 (例: 1h, 24h)
            if let Ok(hours) = opt.trim_end_matches('h').parse::<i64>() {
                let since = Utc::now() - ChronoDuration::hours(hours);
                (Some(Timestamp::from(since.timestamp() as u64)), None)
            } else {
                return {
                    util::reply_to(&config, event, person, "時間指定の形式が不正です。\n例: 1h (過去1時間)").await?;
                    Ok(())
                };
            }
        } else if let Some(ts) = parse_datetime(&opt) {
            // 日付/日時指定 (例: 2024-10-01 または 2024-10-01T14:30) - 指定日時以降
            (Some(ts), None)
        } else {
            util::reply_to(&config, event, person, "日時指定の形式が不正です。\n例: 7d, 1h, 2024-10-01, 2024-10-01T14:30, 2024-10-01~2024-10-31").await?;
            return Ok(());
        }
    } else {
        (None, None)
    };
    
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
    
    if let Some(author) = author_pubkey {
        if let Ok(pk) = PublicKey::from_hex(&author) {
            filter = filter.author(pk);
            println!("Searching author: {}", author);
        }
    }
    
    if let Some(since) = since_timestamp {
        filter = filter.since(since);
        println!("Searching since: {}", since);
    }
    
    if let Some(until) = until_timestamp {
        filter = filter.until(until);
        println!("Searching until: {}", until);
    }
    
    println!("Filter: {:?}", filter);
    println!("Fetching events...");
    
    let events = client.fetch_events(filter, Duration::from_secs(10)).await?;
    println!("Found {} events", events.len());
    
    client.shutdown().await;
    
    // 検索コマンドを実行した投稿自体を除外 + 日時範囲でフィルタリング
    let filtered_events: Vec<_> = events.into_iter()
        .filter(|e| {
            // 検索コマンド自体を除外
            if e.id == event.id {
                return false;
            }
            
            // 日時範囲チェック
            let timestamp = e.created_at.as_u64() as i64;
            
            if let Some(since) = since_timestamp {
                if timestamp < since.as_u64() as i64 {
                    println!("Filtered out (before since): event timestamp={}, since={}", timestamp, since.as_u64());
                    return false;
                }
            }
            
            if let Some(until) = until_timestamp {
                if timestamp > until.as_u64() as i64 {
                    println!("Filtered out (after until): event timestamp={}, until={}", timestamp, until.as_u64());
                    return false;
                }
            }
            
            true
        })
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
