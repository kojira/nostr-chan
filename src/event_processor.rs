use crate::{config, database as db, gpt, util, conversation, dashboard};
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
        let language = if japanese { Some("ja") } else { None };
        match db::insert_event(&conn, &event, language) {
            Ok(event_ref_id) => {
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
    
    println!("[EventProcessor] Total bots: {}, Active bots: {}", persons.len(), active_persons.len());
    
    if active_persons.is_empty() {
        println!("[EventProcessor] No active bots, skipping event");
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
        let limit_minutes = config.get_i64_setting("conversation_limit_minutes");
        let limit_count = config.get_usize_setting("conversation_limit_count");
        
        let conversation_count = db::get_conversation_count_with_user(
            &conn,
            &person.pubkey,
            &event.pubkey.to_string(),
            limit_minutes,
        )?;
        
        if conversation_count >= limit_count {
            println!(
                "[Worker] 会話回数制限: {}分間で{}回 (制限: {}回)",
                limit_minutes,
                conversation_count,
                limit_count
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
            let language = if japanese { Some("ja") } else { None };
            db::insert_event(&conn, &event, language)?
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
    let prompt = person.prompt.clone();
    let user_pubkey = event.pubkey.to_string();
    
    // ユーザー名を取得（メンション時のみ）
    let user_name = if has_mention {
        util::get_user_name(&user_pubkey).await.ok()
            .filter(|name| !name.ends_with("..."))
    } else {
        None
    };
    
    // 会話コンテキストを準備
    let context = if has_mention {
        // メンション時：スレッドの会話履歴を取得
        let event_json = serde_json::to_string(&event)?;
        let thread_root_id = db::extract_thread_root_id(&event_json).ok().flatten();
        
        match conversation::prepare_context_for_reply(
            &conn,
            &person.pubkey,
            &user_pubkey,
            &event.content,
            50,
            &config,
            thread_root_id.as_deref(),
            user_name.as_deref(),
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
        
        let timeline_size = config.get_usize_setting("timeline_size");
        
        match conversation::build_japanese_timeline_for_air_reply(&conn, timeline_size) {
            Ok(events) => {
                if events.is_empty() {
                    None
                } else if use_single_post {
                    // 単一投稿モード: タイムラインから1つだけランダムに選択
                    use rand::seq::SliceRandom;
                    if let Some(selected_event) = events.choose(&mut rng) {
                        let dt = chrono::Local.timestamp_opt(selected_event.created_at, 0).single().unwrap();
                        let time_str = dt.format("%m/%d %H:%M").to_string();
                        let display_name = selected_event.display_name(&conn);
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
                            let display_name = ev.display_name(&conn);
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
    
    // GPT応答生成（メンションの場合は印象＋心境付き、エアリプの場合は心境のみ）
    let reply = if has_mention {
        match gpt::get_reply_with_mental_diary(&person.pubkey, &event.pubkey.to_string(), &prompt, &event.content, context, user_name.as_deref()).await {
            Ok(response) => response.reply,
            Err(e) => {
                eprintln!("[GPT Error] {}", e);
                return Ok(());
            }
        }
    } else {
        // エアリプ時も心境を参照・更新
        gpt::get_air_reply_with_mental_diary(&person.pubkey, &prompt, &event.content, has_mention, context).await?
    };
    
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
        if has_conversation_log {
            if let Err(e) = util::log_event_to_conversation(&bot_event, &person.pubkey, true) {
                eprintln!("[Worker] bot発言の会話ログ記録エラー: {}", e);
            }
        } else {
            // bot_postとして保存（会話ログには記録しない）
            if let Err(e) = db::insert_event(&conn, &bot_event, None) {
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
        
        let timeline_size = config.get_usize_setting("timeline_size");
        let _ = db::cleanup_old_timeline_posts(&conn, timeline_size);
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

