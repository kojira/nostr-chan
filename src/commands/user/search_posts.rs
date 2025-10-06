use crate::config;
use crate::db;
use crate::util;
use nostr_sdk::prelude::*;
use chrono::{Duration as ChronoDuration, NaiveDate, NaiveDateTime, Utc, Local, TimeZone};
use std::time::Duration;

// Nostr投稿検索コマンド
pub async fn search_posts(config: config::AppConfig, person: db::Person, event: Event) -> Result<()> {
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
