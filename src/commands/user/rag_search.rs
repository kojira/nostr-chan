use crate::config::AppConfig;
use crate::database as db;
use crate::embedding;
use crate::util;
use chrono::{Local, TimeZone};
use nostr_sdk::prelude::*;
use rusqlite::Connection;

pub async fn rag_search(config: AppConfig, person: db::Person, event: Event) -> Result<()> {
    let content = event.content.clone();
    
    // コマンドパターンを削除してクエリを抽出
    let query = content
        .trim_start_matches("rag")
        .trim_start_matches("意味検索")
        .trim();
    
    if query.is_empty() {
        util::reply_to(
            &config,
            event,
            person,
            "検索クエリを指定してください。\n\n【使い方】\nrag [検索したい内容]\n意味検索 [検索したい内容]\n\n【例】\nrag Nostrの使い方\n意味検索 botの作り方",
        )
        .await?;
        return Ok(());
    }
    
    println!("[RAG Search] クエリ: {}", query);
    
    // クエリをベクトル化（Resultを即座に分岐）
    let query_embedding;
    {
        match embedding::generate_embedding_global(query) {
            Ok(emb) => {
                query_embedding = emb;
            }
            Err(e) => {
                let _ = e; // エラーを即座に破棄
                eprintln!("[RAG Search] ベクトル化エラー");
                util::reply_to(
                    &config,
                    event,
                    person,
                    "検索に失敗しました。Embeddingサービスが利用できません。",
                )
                .await?;
                return Ok(());
            }
        }
    }
    
    // データベースから類似イベントを検索
    let conn = db::connect()?;
    let threshold = config.get_f32_setting("rag_similarity_threshold");
    let similar_events;
    {
        match search_similar_events(&conn, &query_embedding, 5, threshold) {
            Ok(events) => {
                similar_events = events;
            }
            Err(e) => {
                let _ = e; // エラーを即座に破棄
                eprintln!("[RAG Search] 検索エラー");
                util::reply_to(
                    &config,
                    event,
                    person,
                    "検索に失敗しました。データベースエラーが発生しました。",
                )
                .await?;
                return Ok(());
            }
        }
    }
    
    if similar_events.is_empty() {
        util::reply_to(
            &config,
            event,
            person,
            "検索結果が見つかりませんでした。\n（まだベクトル化されたイベントがない可能性があります）",
        )
        .await?;
        return Ok(());
    }
    
    // 検索結果をフォーマット
    let mut result_lines = vec![format!("「{}」の検索結果:", query)];
    result_lines.push(String::new());
    
    for (event_record, similarity) in similar_events.iter() {
        // 日時をフォーマット（日本時間）
        let dt = Local
            .timestamp_opt(event_record.created_at, 0)
            .single()
            .ok_or("タイムスタンプ変換エラー")?;
        let time_str = dt.format("%m/%d %H:%M").to_string();
        
        // event_idをbech32形式（note1...）に変換
        let note_id = match EventId::from_hex(&event_record.event_id) {
            Ok(eid) => eid.to_bech32().unwrap(),
            Err(_) => event_record.event_id.clone(),
        };
        
        result_lines.push(format!("[{}] 類似度:{:.3} nostr:{}", time_str, similarity, note_id));
    }
    
    let response = result_lines.join("\n");
    
    util::reply_to(&config, event, person, &response).await?;
    
    Ok(())
}

/// ベクトル類似度で検索
fn search_similar_events(
    conn: &Connection,
    query_embedding: &[f32],
    limit: usize,
    threshold: f32,
) -> std::result::Result<Vec<(db::EventRecord, f32)>, String> {
    // embedding付きの日本語イベントを全取得
    let mut stmt = conn.prepare(
        "SELECT id, event_id, event_json, pubkey, kind, content, created_at, received_at,
                language, embedding
         FROM events
         WHERE language = 'ja' AND embedding IS NOT NULL"
    ).map_err(|e| format!("SQL prepare error: {}", e))?;
    
    let events = stmt
        .query_map(rusqlite::params![], |row| {
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
        })
        .map_err(|e| format!("SQL query error: {}", e))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| format!("SQL collect error: {}", e))?;
    
    println!("[RAG Search] 検索対象: {}件のイベント", events.len());
    
    // 各イベントとの類似度を計算
    let mut scored_events: Vec<(db::EventRecord, f32)> = Vec::new();
    
    for event_record in events {
        if let Some(embedding_bytes) = &event_record.embedding {
            // バイト列からf32配列に変換
            let embedding = bytes_to_f32_vec(embedding_bytes);
            
            // ゼロベクトル（空コンテンツ）はスキップ
            if embedding.iter().all(|&x| x == 0.0) {
                continue;
            }
            
            // コサイン類似度を計算
            if let Ok(similarity) = embedding::cosine_similarity(query_embedding, &embedding) {
                // 閾値以上のみを追加
                if similarity >= threshold {
                    scored_events.push((event_record, similarity));
                }
            }
        }
    }
    
    // 類似度でソート（降順）
    scored_events.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    
    // 上位N件を返す
    scored_events.truncate(limit);
    
    Ok(scored_events)
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

