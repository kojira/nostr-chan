use crate::{config, db, gpt, util, conversation, dashboard};
use nostr_sdk::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;

/// イベント処理のメイン関数
pub async fn process_event(
    config: config::AppConfig,
    bot_info: Arc<RwLock<dashboard::BotInfo>>,
    event: Event,
) -> Result<(), Box<dyn std::error::Error>> {
    use whatlang::{detect, Lang};
    use chrono::TimeZone;
    
    // DB接続
    let conn = db::connect()?;
    
    // 言語判定
    let japanese = if let Some(lang) = detect(&event.content) {
        matches!(lang.lang(), Lang::Jpn)
    } else {
        false
    };
    
    // イベントをeventsテーブルに保存
    if event.kind == Kind::TextNote {
        let event_type = if japanese { Some("air_reply") } else { None };
        match db::insert_event(&conn, &event, japanese, event_type) {
            Ok(event_ref_id) => {
                // kind 0情報を非同期で取得して更新
                let conn_clone = db::connect()?;
                let event_id = event.id.to_string();
                let pubkey = event.pubkey.to_string();
                tokio::spawn(async move {
                    if let Ok(name) = util::get_user_name(&pubkey).await {
                        let _ = db::update_event_kind0_name(&conn_clone, &event_id, Some(&name));
                    }
                });
                let _ = event_ref_id;
            }
            Err(e) => {
                if !e.to_string().contains("UNIQUE constraint failed") {
                    eprintln!("[Worker] Failed to save event: {}", e);
                }
            }
        }
    }
    
    // 処理判定
    if event.content.is_empty() {
        return Ok(());
    }
    
    // personsを取得
    let persons = db::get_all_persons(&conn)?;
    
    // 有効なBotのみをフィルタ（status == 0）
    let active_persons: Vec<db::Person> = persons.iter()
        .filter(|p| p.status == 0)
        .cloned()
        .collect();
    
    if active_persons.is_empty() {
        return Ok(());
    }
    
    // ブラックリストチェック
    if is_blacklisted(&conn, &event.pubkey.to_string())? {
        println!("[Worker] ブラックリストのユーザーからの投稿をスキップ: {}", event.pubkey);
        return Ok(());
    }
    
    // メンション判定（有効なBotのみ）
    let person_op = util::extract_mention(active_persons.clone(), &event)?;
    let has_mention = person_op.is_some();
    
    // エアリプは日本語のみ
    if !has_mention && !japanese {
        return Ok(());
    }
    
    // 確率判定（有効なBotのみ）
    let (mut should_post, _) = util::judge_post(&config, active_persons.clone(), &event)?;
    
    // Personを決定
    let person = if let Some(p) = person_op {
        p
    } else {
        // ランダム選択も有効なBotから
        if active_persons.is_empty() {
            return Ok(());
        }
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        active_persons.choose(&mut rng).unwrap().clone()
    };
    
    // Botのステータスチェック（無効化されていたらスキップ）
    if person.status != 0 {
        println!("🚫 Bot無効化中のため、返信をスキップ: {} ({})", person.pubkey, event.id);
        return Ok(());
    }
    
    // メンションの場合は必ず返信
    if has_mention {
        should_post = true;
    }
    
    if !should_post {
        return Ok(());
    }
    
    // グローバル一時停止チェック（返信処理の直前）
    if db::is_global_pause(&conn)? {
        println!("⏸️ グローバル一時停止中のため、返信をスキップ: {}", event.id);
        return Ok(());
    }
    
    // フォロワーチェック
    let is_follower = util::is_follower(&event.pubkey.to_string(), &person.secretkey).await?;
    if !is_follower {
        println!("[Worker] Not a follower, skipping");
        return Ok(());
    }
    
    // 会話回数制限チェック（メンション時のみ）
    if has_mention {
        let conversation_count = db::get_conversation_count_with_user(
            &conn,
            &person.pubkey,
            &event.pubkey.to_string(),
            config.bot.conversation_limit_minutes,
        )?;
        
        if conversation_count >= config.bot.conversation_limit_count {
            println!(
                "[Worker] 会話回数制限: {}分間で{}回 (制限: {}回)",
                config.bot.conversation_limit_minutes,
                conversation_count,
                config.bot.conversation_limit_count
            );
            return Ok(());
        }
    }
    
    // 会話ログに記録（メンション時のみ）
    let conversation_log_id = if has_mention {
        let event_record = db::get_event_by_event_id(&conn, &event.id.to_string())?;
        let event_ref_id = if let Some(record) = event_record {
            record.id
        } else {
            let event_type = Some("mention");
            db::insert_event(&conn, &event, japanese, event_type)?
        };
        
        let event_json = serde_json::to_string(&event)?;
        let mentioned_pubkeys = db::extract_mentioned_pubkeys(&event_json).ok();
        let thread_root_id = db::extract_thread_root_id(&event_json).ok().flatten();
        
        let all_bot_pubkeys: Vec<String> = active_persons.iter().map(|p| p.pubkey.clone()).collect();
        let is_bot_conversation = if let Some(ref pks) = mentioned_pubkeys {
            db::detect_bot_conversation(pks, &all_bot_pubkeys)
        } else {
            false
        };
        
        match db::insert_conversation_log(
            &conn,
            &person.pubkey,
            event_ref_id,
            thread_root_id.as_deref(),
            mentioned_pubkeys.as_ref().map(|v| v.as_slice()),
            false,
            is_bot_conversation,
        ) {
            Ok(log_id) => Some(log_id),
            Err(e) => {
                eprintln!("[Worker] 会話ログ記録エラー: {}", e);
                None
            }
        }
    } else {
        None
    };
    
    let has_conversation_log = conversation_log_id.is_some();
    
    // プロンプト準備
    let mut prompt = person.prompt.clone();
    let user_pubkey = event.pubkey.to_string();
    
    if has_mention {
        if let Ok(user_name) = util::get_user_name(&user_pubkey).await {
            if !user_name.ends_with("...") {
                prompt = format!("{}。話しかけてきた相手の名前は「{}」です。", prompt, user_name);
            }
        }
    }
    
    // 会話コンテキストを準備
    let context = if has_conversation_log {
        match conversation::prepare_context_for_reply(
            &conn,
            &person.pubkey,
            &user_pubkey,
            &event.content,
            50,
        ).await {
            Ok(ctx) => {
                if ctx.is_empty() {
                    None
                } else {
                    Some(ctx)
                }
            },
            Err(e) => {
                eprintln!("[Worker] コンテキスト準備エラー: {}", e);
                None
            }
        }
    } else {
        // エアリプモード: 単一投稿 vs タイムライン全体の判定
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let random_value: i32 = rng.gen_range(0..100);
        let use_single_post = random_value < person.air_reply_single_ratio;
        
        match conversation::build_japanese_timeline_for_air_reply(&conn, config.bot.timeline_size) {
            Ok(events) => {
                if events.is_empty() {
                    None
                } else if use_single_post {
                    // 単一投稿モード: タイムラインから1つだけランダムに選択
                    use rand::seq::SliceRandom;
                    if let Some(selected_event) = events.choose(&mut rng) {
                        let dt = chrono::Local.timestamp_opt(selected_event.created_at, 0).single().unwrap();
                        let time_str = dt.format("%m/%d %H:%M").to_string();
                        let display_name = selected_event.kind0_name.clone().unwrap_or_else(|| {
                            if selected_event.pubkey.len() > 8 {
                                format!("{}...", &selected_event.pubkey[..8])
                            } else {
                                selected_event.pubkey.clone()
                            }
                        });
                        println!("[Worker] エアリプモード: 単一投稿 ({}%)", person.air_reply_single_ratio);
                        Some(format!("【投稿】[{}] {}: {}", time_str, display_name, selected_event.content))
                    } else {
                        None
                    }
                } else {
                    // タイムライン全体モード: 複数投稿を表示
                    let timeline_lines: Vec<String> = events.iter()
                        .enumerate()
                        .map(|(i, ev)| {
                            let dt = chrono::Local.timestamp_opt(ev.created_at, 0).single().unwrap();
                            let time_str = dt.format("%m/%d %H:%M").to_string();
                            let display_name = ev.kind0_name.clone().unwrap_or_else(|| {
                                if ev.pubkey.len() > 8 {
                                    format!("{}...", &ev.pubkey[..8])
                                } else {
                                    ev.pubkey.clone()
                                }
                            });
                            format!("{}. [{}] {}: {}", i + 1, time_str, display_name, ev.content)
                        })
                        .collect();
                    println!("[Worker] エアリプモード: タイムライン全体 ({}%)", 100 - person.air_reply_single_ratio);
                    Some(format!("【タイムライン】\n{}", timeline_lines.join("\n")))
                }
            }
            Err(e) => {
                eprintln!("[Worker] タイムライン取得エラー: {}", e);
                None
            }
        }
    };
    
    // GPT応答生成
    let reply = gpt::get_reply_with_context(&prompt, &event.content, has_mention, context).await?;
    
    if reply.is_empty() {
        return Ok(());
    }
    
    println!("[Worker] Replying: {}", reply);
    
    // 返信送信
    let sent_event = if has_mention {
        match util::reply_to(&config, event.clone(), person.clone(), &reply).await {
            Ok(evt) => {
                // ダッシュボードに最終返信時刻を更新
                {
                    let mut info = bot_info.write().await;
                    info.last_reply_timestamp = Utc::now().timestamp();
                }
                Some(evt)
            }
            Err(e) => {
                eprintln!("[Worker] Failed to reply: {}", e);
                None
            }
        }
    } else if event.kind == Kind::TextNote {
        let send_ok = util::send_to(&config, event.clone(), person.clone(), &reply).await.is_ok();
        if send_ok {
            if let Ok(bot_keys) = Keys::parse(&person.secretkey) {
                let event_builder = EventBuilder::text_note(&reply);
                event_builder.sign(&bot_keys).await.ok()
            } else {
                None
            }
        } else {
            eprintln!("[Worker] Failed to send");
            None
        }
    } else {
        None
    };
    
    // bot自身の発言を記録
    if let Some(bot_event) = sent_event {
        let event_type = if has_conversation_log { Some("bot_reply") } else { Some("bot_post") };
        match db::insert_event(&conn, &bot_event, true, event_type) {
            Ok(event_ref_id) => {
                if has_conversation_log {
                    let event_json = serde_json::to_string(&bot_event).unwrap_or_default();
                    let thread_root_id = db::extract_thread_root_id(&event_json).ok().flatten();
                    
                    if let Err(e) = db::insert_conversation_log(
                        &conn,
                        &person.pubkey,
                        event_ref_id,
                        thread_root_id.as_deref(),
                        None,
                        true,
                        false,
                    ) {
                        eprintln!("[Worker] bot発言の会話ログ記録エラー: {}", e);
                    }
                }
            }
            Err(e) => {
                if !e.to_string().contains("UNIQUE constraint failed") {
                    eprintln!("[Worker] bot発言の保存エラー: {}", e);
                }
            }
        }
        
        // タイムラインに追加
        if let Err(e) = db::add_timeline_post(
            &conn,
            &person.pubkey,
            Some("Bot"),
            &reply,
            Utc::now().timestamp()
        ) {
            eprintln!("[Worker] Failed to save bot timeline post: {}", e);
        }
        
        let _ = db::cleanup_old_timeline_posts(&conn, config.bot.timeline_size);
    }
    
    Ok(())
}

/// ブラックリストチェック
fn is_blacklisted(conn: &rusqlite::Connection, pubkey: &str) -> Result<bool, Box<dyn std::error::Error>> {
    if let Some(blacklist_str) = db::get_system_setting(conn, "blacklist")? {
        let blacklist: Vec<&str> = blacklist_str.split(',').filter(|s| !s.is_empty()).collect();
        Ok(blacklist.contains(&pubkey))
    } else {
        Ok(false)
    }
}

