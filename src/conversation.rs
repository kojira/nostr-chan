use crate::db;
use crate::embedding;
use crate::gpt;
use chrono::{Local, TimeZone};
use rusqlite::Connection;

const MAX_TIMELINE_LENGTH: usize = 5000;
const SUMMARY_MAX_LENGTH: usize = 1000;

/// 会話タイムラインを文字列として構築（最大5000文字）
pub fn build_conversation_timeline(
    conn: &Connection,
    bot_pubkey: &str,
    limit: usize,
) -> Result<String, Box<dyn std::error::Error>> {
    let events = db::get_conversation_timeline(conn, bot_pubkey, limit)?;
    
    if events.is_empty() {
        return Ok(String::new());
    }
    
    let mut timeline_lines = Vec::new();
    
    for (i, event) in events.iter().enumerate() {
        // 日本時間に変換
        let dt = Local.timestamp_opt(event.created_at, 0)
            .single()
            .ok_or("タイムスタンプ変換エラー")?;
        let time_str = dt.format("%m/%d %H:%M").to_string();
        
        // 名前を取得（なければpubkeyの先頭8文字）
        let display_name = event.kind0_name.clone().unwrap_or_else(|| {
            if event.pubkey.len() > 8 {
                format!("{}...", &event.pubkey[..8])
            } else {
                event.pubkey.clone()
            }
        });
        
        let line = format!("{}. [{}] {}: {}", i + 1, time_str, display_name, event.content);
        timeline_lines.push(line);
    }
    
    let timeline_text = timeline_lines.join("\n");
    
    Ok(timeline_text)
}

/// 会話が5000文字を超える場合に要約を作成
pub async fn summarize_conversation_if_needed(
    conn: &Connection,
    bot_pubkey: &str,
    user_input: &str,
    timeline_text: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    if timeline_text.len() <= MAX_TIMELINE_LENGTH {
        return Ok(None);
    }
    
    println!("[Conversation] タイムラインが{}文字のため要約を作成します", timeline_text.len());
    
    // ユーザー入力をベクトル化
    let user_input_embedding = embedding::generate_embedding_global(user_input)?;
    
    // 過去の類似要約を検索
    let similar_summary = search_most_similar_summary(conn, bot_pubkey, &user_input_embedding)?;
    
    // 要約プロンプト作成
    let summary_prompt = format!(
        "以下の会話履歴を{}文字以内で要約してください。重要なポイントと文脈を保持してください。",
        SUMMARY_MAX_LENGTH
    );
    
    let content_to_summarize = if let Some(prev_summary) = similar_summary {
        // 類似する過去の要約がある場合は、それ以降の会話のみを要約
        println!("[Conversation] 類似する過去の要約を発見: {}", prev_summary.summary);
        
        // 要約の終了時刻以降のイベントを取得
        let events = db::get_conversation_timeline(conn, bot_pubkey, 100)?;
        let recent_events: Vec<_> = events
            .iter()
            .filter(|e| e.created_at > prev_summary.to_timestamp)
            .collect();
        
        if recent_events.is_empty() {
            // 新しいイベントがない場合は過去の要約をそのまま使用
            return Ok(Some(prev_summary.summary));
        }
        
        // 過去の要約 + 新規会話
        let mut lines = vec![format!("【過去の要約】\n{}", prev_summary.summary)];
        for event in recent_events {
            let dt = Local.timestamp_opt(event.created_at, 0)
                .single()
                .ok_or("タイムスタンプ変換エラー")?;
            let time_str = dt.format("%m/%d %H:%M").to_string();
            let display_name = event.kind0_name.clone().unwrap_or_else(|| {
                if event.pubkey.len() > 8 {
                    format!("{}...", &event.pubkey[..8])
                } else {
                    event.pubkey.clone()
                }
            });
            lines.push(format!("[{}] {}: {}", time_str, display_name, event.content));
        }
        lines.join("\n")
    } else {
        // 過去の類似要約がない場合は全体を要約
        timeline_text.to_string()
    };
    
    // GPT APIで要約を生成
    let summary = gpt::call_gpt(&summary_prompt, &content_to_summarize).await?;
    
    println!("[Conversation] 要約完了: {} 文字", summary.len());
    
    // 要約をDBに保存
    let events = db::get_conversation_timeline(conn, bot_pubkey, 100)?;
    let from_timestamp = events.first().map(|e| e.created_at).unwrap_or(0);
    let to_timestamp = events.last().map(|e| e.created_at).unwrap_or(0);
    
    // 参加者のpubkeyを抽出
    let mut participants: Vec<String> = events
        .iter()
        .map(|e| e.pubkey.clone())
        .collect();
    participants.sort();
    participants.dedup();
    
    db::insert_conversation_summary(
        conn,
        bot_pubkey,
        &summary,
        user_input,
        &user_input_embedding,
        Some(&participants),
        from_timestamp,
        to_timestamp,
    )?;
    
    Ok(Some(summary))
}

/// 最も類似する要約を検索
fn search_most_similar_summary(
    conn: &Connection,
    bot_pubkey: &str,
    user_input_embedding: &[f32],
) -> Result<Option<db::ConversationSummary>, Box<dyn std::error::Error>> {
    let summaries = db::get_conversation_summaries(conn, bot_pubkey, 10)?;
    
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
pub async fn prepare_context_for_reply(
    conn: &Connection,
    bot_pubkey: &str,
    user_input: &str,
    limit: usize,
) -> Result<String, Box<dyn std::error::Error>> {
    // 会話タイムラインを構築
    let timeline_text = build_conversation_timeline(conn, bot_pubkey, limit)?;
    
    if timeline_text.is_empty() {
        return Ok(String::new());
    }
    
    // 5000文字を超える場合は要約
    if timeline_text.len() > MAX_TIMELINE_LENGTH {
        if let Some(summary) = summarize_conversation_if_needed(conn, bot_pubkey, user_input, &timeline_text).await? {
            // 要約がある場合は要約を返す
            return Ok(format!("【会話の要約】\n{}\n\n【現在の発言】\n{}", summary, user_input));
        }
    }
    
    // 5000文字以下の場合はそのまま返す
    Ok(format!("【会話履歴】\n{}\n\n【現在の発言】\n{}", timeline_text, user_input))
}

/// バイト列をf32ベクトルに変換
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
pub fn build_japanese_timeline_for_air_reply(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<db::EventRecord>, Box<dyn std::error::Error>> {
    // 日本語のイベントを取得
    let mut stmt = conn.prepare(
        "SELECT id, event_id, event_json, pubkey, kind, content, created_at, received_at,
                kind0_name, kind0_content, is_japanese, embedding, event_type
         FROM events
         WHERE is_japanese = 1 AND event_type = 'air_reply'
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
            kind0_name: row.get(8)?,
            kind0_content: row.get(9)?,
            is_japanese: row.get::<_, i32>(10)? != 0,
            embedding: row.get(11)?,
            event_type: row.get(12)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;
    
    // 時系列順（古い順）に反転
    Ok(events.into_iter().rev().collect())
}

