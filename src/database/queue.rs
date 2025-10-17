use rusqlite::{params, Connection, Result};
use chrono::Utc;

pub fn enqueue_event(conn: &Connection, event_json: &str) -> Result<i64> {
    let now = Utc::now().timestamp();
    
    // 現在のキューサイズを確認
    let queue_size: i64 = conn.query_row(
        "SELECT COUNT(*) FROM event_queue WHERE status = 'pending'",
        [],
        |row| row.get(0)
    )?;
    
    // 30件を超える場合は古いものを削除
    if queue_size >= 30 {
        let to_delete = queue_size - 29; // 1つ分の余地を作る
        conn.execute(
            "DELETE FROM event_queue 
             WHERE id IN (
                 SELECT id FROM event_queue 
                 WHERE status = 'pending' 
                 ORDER BY added_at ASC 
                 LIMIT ?
             )",
            params![to_delete],
        )?;
    }
    
    // イベントを追加
    conn.execute(
        "INSERT INTO event_queue (event_json, added_at, status) VALUES (?, ?, 'pending')",
        params![event_json, now],
    )?;
    
    Ok(conn.last_insert_rowid())
}

/// キューから次の処理対象イベントを取得（ステータスを'processing'に更新）
pub fn dequeue_event(conn: &Connection) -> Result<Option<(i64, String)>> {
    // トランザクション開始
    let tx = conn.unchecked_transaction()?;
    
    // 最も古いpendingイベントを取得
    let result = tx.query_row(
        "SELECT id, event_json FROM event_queue 
         WHERE status = 'pending' 
         ORDER BY added_at ASC 
         LIMIT 1",
        [],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    );
    
    match result {
        Ok((id, event_json)) => {
            // ステータスをprocessingに更新
            tx.execute(
                "UPDATE event_queue SET status = 'processing' WHERE id = ?",
                params![id],
            )?;
            tx.commit()?;
            Ok(Some((id, event_json)))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            tx.commit()?;
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

/// 処理完了したイベントをキューから削除
pub fn complete_queue_event(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM event_queue WHERE id = ?", params![id])?;
    Ok(())
}

/// 処理失敗したイベントをpendingに戻す
#[allow(dead_code)]
pub fn retry_queue_event(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE event_queue SET status = 'pending' WHERE id = ?",
        params![id],
    )?;
    Ok(())
}

/// キューサイズを取得
#[allow(dead_code)]
pub fn get_queue_size(conn: &Connection) -> Result<i64> {
    let size: i64 = conn.query_row(
        "SELECT COUNT(*) FROM event_queue WHERE status IN ('pending', 'processing')",
        [],
        |row| row.get(0)
    )?;
    Ok(size)
}

/// 処理中のイベントをpendingに戻す（異常終了時のリセット用）
pub fn reset_processing_events(conn: &Connection) -> Result<usize> {
    let updated = conn.execute(
        "UPDATE event_queue SET status = 'pending' WHERE status = 'processing'",
        [],
    )?;
    Ok(updated)
}
