use crate::config::AppConfig;
use crate::database as db;
use crate::embedding;
use crate::gpt;
use chrono::{Local, TimeZone};
use rusqlite::Connection;
use tiktoken_rs::o200k_base;

const SUMMARY_MAX_LENGTH: usize = 1000;

/// トークン数を正確に計算（o200k_base: GPT-4o, GPT-5用）
fn estimate_tokens(text: &str) -> usize {
    let bpe = o200k_base().expect("[Token] tiktoken (o200k_base) 初期化に失敗しました");
    let tokens = bpe.encode_with_special_tokens(text);
    tokens.len()
}

/// トークン制限内に収まる最大のイベント数を探索（新しい方から）
fn find_events_within_token_limit(
    conn: &Connection,
    events: &[db::EventRecord],
    max_tokens: usize,
    prompt_tokens: usize,
) -> Result<usize, Box<dyn std::error::Error>> {
    if events.is_empty() {
        return Ok(0);
    }
    
    // 利用可能なトークン数（プロンプト分を除く）
    let available_tokens = max_tokens.saturating_sub(prompt_tokens);
    
    // 新しい方から順にイベントを追加していき、制限を超えない最大数を探索
    let mut accumulated_tokens = 0;
    let mut count = 0;
    
    for event in events.iter().rev() { // 新しい方から
        // イベントをフォーマット
        let dt = Local.timestamp_opt(event.created_at, 0)
            .single()
            .ok_or("タイムスタンプ変換エラー")?;
        let time_str = dt.format("%m/%d %H:%M").to_string();
        let display_name = event.display_name(conn);
        let line = format!("[{}] {}: {}", time_str, display_name, event.content);
        
        let line_tokens = estimate_tokens(&line);
        
        if accumulated_tokens + line_tokens > available_tokens {
            // これ以上追加できない
            break;
        }
        
        accumulated_tokens += line_tokens;
        count += 1;
    }
    
    println!("[Token Limit] 最大{}件のイベントが{}トークン（制限: {}トークン）", 
             count, accumulated_tokens, available_tokens);
    
    Ok(count)
}

/// イベントリストをタイムライン文字列にフォーマット（下に行くほど新しい）
fn format_timeline_text(conn: &Connection, events: Vec<db::EventRecord>) -> Result<String, Box<dyn std::error::Error>> {
    let mut timeline_lines = Vec::new();
    
    // eventsは古い順に並んでいるが、新しい順（下に行くほど新しい）で表示したいので逆順にする
    let reversed_events: Vec<_> = events.into_iter().rev().collect();
    
    for (i, event) in reversed_events.iter().enumerate() {
        // 日本時間に変換
        let dt = Local.timestamp_opt(event.created_at, 0)
            .single()
            .ok_or("タイムスタンプ変換エラー")?;
        let time_str = dt.format("%m/%d %H:%M").to_string();
        
        // kind0_cacheから名前を取得
        let display_name = event.display_name(conn);
        
        let line = format!("{}. [{}] {}: {}", i + 1, time_str, display_name, event.content);
        timeline_lines.push(line);
    }
    
    Ok(timeline_lines.join("\n"))
}

/// 会話タイムラインを文字列として構築（最大5000文字）
/// user_inputが指定された場合、80%高類似度 + 20%低類似度で多様性を持たせる
pub async fn build_conversation_timeline_with_diversity(
    conn: &Connection,
    bot_pubkey: &str,
    user_input: Option<&str>,
    limit: usize,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut events = db::get_conversation_timeline(conn, bot_pubkey, limit)?;
    
    if events.is_empty() {
        return Ok(String::new());
    }
    
    // 時系列順のみ（embedding無効化のため類似度選別を削除）
    let selected_events = events;
    
    format_timeline_text(conn, selected_events)
}

/// 旧インターフェース（互換性のため）
#[allow(dead_code)]
pub async fn build_conversation_timeline(
    conn: &Connection,
    bot_pubkey: &str,
    limit: usize,
) -> Result<String, Box<dyn std::error::Error>> {
    build_conversation_timeline_with_diversity(conn, bot_pubkey, None, limit).await
}

/// 会話が指定文字数を超える場合に要約を作成
/// 戻り値: Option<(要約テキスト, 要約の終了タイムスタンプ)>
pub async fn summarize_conversation_if_needed(
    conn: &Connection,
    bot_pubkey: &str,
    user_pubkey: &str,
    user_input: &str,
    timeline_text: &str,
    config: &AppConfig,
) -> Result<Option<(String, i64)>, Box<dyn std::error::Error>> {
    let summary_threshold = config.get_usize_setting("summary_threshold");
    
    if timeline_text.len() <= summary_threshold {
        return Ok(None);
    }
    
    println!("[Conversation] タイムラインが{}文字のため要約を作成します（閾値: {}文字）", timeline_text.len(), summary_threshold);
    
    // embedding無効化のため、類似要約検索は行わない
    let similar_summary: Option<db::ConversationSummary> = None;
    
    // Botのパーソナリティを取得
    let bot_person = db::get_person(conn, bot_pubkey)?;
    
    // 要約プロンプト作成（Botのパーソナリティを活かす）
    let summary_prompt = format!(
        "あなたは次の人格です：「{}」\n\nこの人格を保ちつつ、以下の会話履歴を{}文字以内で要約してください。あなたらしい視点で重要なポイントと文脈を保持してください。",
        bot_person.prompt,
        SUMMARY_MAX_LENGTH
    );
    
    // プロンプトのトークン数を推定
    let prompt_tokens = estimate_tokens(&summary_prompt);
    let max_tokens = config.get_usize_setting("max_summary_tokens");
    
    let content_to_summarize = if let Some(prev_summary) = similar_summary {
        // 類似する過去の要約がある場合は、それ以降の会話のみを要約
        println!("[Conversation] 類似する過去の要約を発見: {}", prev_summary.summary);
        
        // 要約の終了時刻以降のイベントを取得（そのユーザーとの会話のみ）
        let events = db::get_conversation_timeline_with_user(conn, bot_pubkey, user_pubkey, 200)?;
        let recent_events: Vec<_> = events
            .into_iter()
            .filter(|e| e.created_at > prev_summary.to_timestamp)
            .collect();
        
        if recent_events.is_empty() {
            // 新しいイベントがない場合は過去の要約をそのまま使用
            return Ok(Some((prev_summary.summary, prev_summary.to_timestamp)));
        }
        
        // トークン制限内に収まる最大のイベント数を探索
        let prev_summary_tokens = estimate_tokens(&prev_summary.summary);
        let available_for_events = max_tokens.saturating_sub(prompt_tokens).saturating_sub(prev_summary_tokens);
        let event_count = find_events_within_token_limit(conn, &recent_events, available_for_events, 0)?;
        
        if event_count == 0 {
            // トークン制限により新規イベントを含められない場合は過去の要約をそのまま使用
            println!("[Conversation] トークン制限により新規イベントを含められません");
            return Ok(Some((prev_summary.summary, prev_summary.to_timestamp)));
        }
        
        // 新しい方からevent_count件を取得
        let events_to_use: Vec<_> = recent_events.iter().rev().take(event_count).rev().cloned().collect();
        
        // 過去の要約 + 新規会話
        let mut lines = vec![format!("【過去の要約】\n{}", prev_summary.summary)];
        for event in &events_to_use {
            let dt = Local.timestamp_opt(event.created_at, 0)
                .single()
                .ok_or("タイムスタンプ変換エラー")?;
            let time_str = dt.format("%m/%d %H:%M").to_string();
            let display_name = if event.pubkey.len() > 8 {
                format!("{}...", &event.pubkey[..8])
            } else {
                event.pubkey.clone()
            };
            lines.push(format!("[{}] {}: {}", time_str, display_name, event.content));
        }
        lines.join("\n")
    } else {
        // 過去の類似要約がない場合は、トークン制限内で全体を要約
        // timeline_textから元のイベントを再取得するのは難しいので、文字列を切り詰める
        let content_tokens = estimate_tokens(timeline_text);
        if content_tokens + prompt_tokens > max_tokens {
            // トークン制限を超える場合は後ろから切り詰める
            let available = max_tokens.saturating_sub(prompt_tokens);
            let ratio = available as f64 / content_tokens as f64;
            let target_len = (timeline_text.len() as f64 * ratio) as usize;
            
            // 文字境界を考慮して切り詰め位置を調整
            let start_pos = timeline_text.len().saturating_sub(target_len);
            let safe_start = timeline_text.char_indices()
                .find(|(idx, _)| *idx >= start_pos)
                .map(|(idx, _)| idx)
                .unwrap_or(timeline_text.len());
            
            let trimmed = &timeline_text[safe_start..];
            println!("[Conversation] トークン制限により{}文字から{}文字に切り詰めました", timeline_text.len(), trimmed.len());
            trimmed.to_string()
        } else {
            timeline_text.to_string()
        }
    };
    
    // GPT APIで要約を生成（カテゴリ: summary）
    let summary = gpt::call_gpt_with_category(&summary_prompt, &content_to_summarize, bot_pubkey, "summary", config).await?;
    
    println!("[Conversation] 要約完了: {} 文字", summary.len());
    
    // 要約をDBに保存（そのユーザーとの会話履歴のみ）
    let events = db::get_conversation_timeline_with_user(conn, bot_pubkey, user_pubkey, 100)?;
    let from_timestamp = events.first().map(|e| e.created_at).unwrap_or(0);
    let to_timestamp = events.last().map(|e| e.created_at).unwrap_or(0);
    
    // 参加者のpubkeyを抽出
    let mut participants: Vec<String> = events
        .iter()
        .map(|e| e.pubkey.clone())
        .collect();
    participants.sort();
    participants.dedup();
    
    // embedding無効化のため、空のベクトルを保存
    let empty_embedding = vec![0.0f32; 384];
    db::insert_conversation_summary(
        conn,
        bot_pubkey,
        &summary,
        user_input,
        &empty_embedding,
        Some(&participants),
        from_timestamp,
        to_timestamp,
    )?;
    
    Ok(Some((summary, to_timestamp)))
}

/// 最も類似する要約を検索
fn search_most_similar_summary(
    conn: &Connection,
    bot_pubkey: &str,
    user_pubkey: &str,
    user_input_embedding: &[f32],
) -> Result<Option<db::ConversationSummary>, Box<dyn std::error::Error>> {
    let summaries = db::get_conversation_summaries(conn, bot_pubkey, user_pubkey, 10)?;
    
    if summaries.is_empty() {
        return Ok(None);
    }
    
    let mut best_summary: Option<db::ConversationSummary> = None;
    let mut best_similarity: f32 = 0.5; // 閾値: 0.5以上の類似度が必要
    
    for summary in summaries {
        // バイト列からf32配列に変換
        let embedding_vec = bytes_to_f32_vec(&summary.user_input_embedding);
        
        // コサイン類似度を計算
        let similarity = embedding::cosine_similarity(user_input_embedding, &embedding_vec)?;
        
        if similarity > best_similarity {
            best_similarity = similarity;
            best_summary = Some(summary);
        }
    }
    
    if let Some(ref summary) = best_summary {
        println!("[Conversation] 最も類似する要約の類似度: {:.3}", best_similarity);
        println!("[Conversation] ユーザー入力: {}", summary.user_input);
    }
    
    Ok(best_summary)
}

/// 返信用のコンテキストを準備
#[allow(dead_code)]
pub async fn prepare_context_for_reply(
    conn: &Connection,
    bot_pubkey: &str,
    user_pubkey: &str,
    user_input: &str,
    limit: usize,
    config: &AppConfig,
    thread_root_id: Option<&str>,
    user_name: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    // 会話タイムラインを構築（80%高類似度 + 20%低類似度）
    let timeline_text = build_conversation_timeline_with_diversity(conn, bot_pubkey, Some(user_input), limit).await?;
    
    if timeline_text.is_empty() {
        return Ok(String::new());
    }
    
    // 設定された文字数を超える場合は要約
    let summary_threshold = config.get_usize_setting("summary_threshold");
    if timeline_text.len() > summary_threshold {
        let recent_count = config.get_usize_setting("recent_context_count");
        
        // スレッド内の全イベントを取得
        let all_events = db::get_conversation_timeline_in_thread(conn, bot_pubkey, user_pubkey, thread_root_id, 200)?;
        
        if all_events.len() > recent_count {
            // recent_count件より多くイベントがある場合のみ要約
            // 古い部分（要約対象）と新しい部分（そのまま保持）に分割
            let cutoff_index = all_events.len().saturating_sub(recent_count);
            let old_events = &all_events[..cutoff_index];
            let recent_events = &all_events[cutoff_index..];
            
            // 古い部分を要約用のテキストに変換
            let old_events_text = format_timeline_text(conn, old_events.to_vec())?;
            
            if !old_events_text.is_empty() {
                // 古い部分を要約
                if let Some((summary, _)) = summarize_conversation_if_needed(conn, bot_pubkey, user_pubkey, user_input, &old_events_text, config).await? {
                    let recent_timeline = format_timeline_text(conn, recent_events.to_vec())?;
                    
                    println!("[Conversation] 要約対象: {}件, 最近のやり取り: {}件", old_events.len(), recent_events.len());
                    
                    let user_label = if let Some(name) = user_name {
                        format!("【{}からあなたへの質問・発言】", name)
                    } else {
                        "【あなたへの質問・発言】".to_string()
                    };
                    
                    return Ok(format!(
                        "【会話の要約】\n{}\n\n【最近のやり取り】\n{}\n\n{}\n{}",
                        summary,
                        recent_timeline,
                        user_label,
                        user_input
                    ));
                }
            }
        }
    }
    
    // 閾値以下の場合はそのまま返す
    let user_label = if let Some(name) = user_name {
        format!("【{}からあなたへの質問・発言】", name)
    } else {
        "【あなたへの質問・発言】".to_string()
    };
    
    Ok(format!("【会話履歴】\n{}\n\n{}\n{}", timeline_text, user_label, user_input))
}

/// バイト列をf32ベクトルに変換
#[allow(dead_code)]
fn bytes_to_f32_vec(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| {
            let arr: [u8; 4] = [chunk[0], chunk[1], chunk[2], chunk[3]];
            f32::from_le_bytes(arr)
        })
        .collect()
}

/// エアリプ用の日本語タイムライン構築（従来のtimeline機能の代替）
#[allow(dead_code)]
pub fn build_japanese_timeline_for_air_reply(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<db::EventRecord>, Box<dyn std::error::Error>> {
    // 日本語のイベントを取得
    let mut stmt = conn.prepare(
        "SELECT id, event_id, event_json, pubkey, kind, content, created_at, received_at,
                language, embedding
         FROM events
         WHERE language = 'ja'
         ORDER BY created_at DESC
         LIMIT ?"
    )?;
    
    let events = stmt.query_map(rusqlite::params![limit], |row| {
        Ok(db::EventRecord {
            id: row.get(0)?,
            event_id: row.get(1)?,
            event_json: row.get(2)?,
            pubkey: row.get(3)?,
            kind: row.get(4)?,
            content: row.get(5)?,
            created_at: row.get(6)?,
            received_at: row.get(7)?,
            language: row.get(8)?,
            embedding: row.get(9)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;
    
    // 時系列順（古い順）に反転
    Ok(events.into_iter().rev().collect())
}

