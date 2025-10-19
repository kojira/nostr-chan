use rusqlite::{params, Connection, Result};
use chrono::Utc;

pub fn get_dashboard_stats(conn: &Connection) -> Result<DashboardStats> {
    let now = Utc::now().timestamp();
    let today_start = now - (now % 86400);
    let week_start = now - (7 * 86400);
    let month_start = now - (30 * 86400);
    
    // 返信統計（is_bot_message = 1 のもののみカウント）
    let replies_today: u32 = conn.query_row(
        "SELECT COUNT(*) FROM conversation_logs WHERE is_bot_message = 1 AND logged_at >= ?",
        params![today_start],
        |row| row.get(0)
    ).unwrap_or(0);
    
    let replies_week: u32 = conn.query_row(
        "SELECT COUNT(*) FROM conversation_logs WHERE is_bot_message = 1 AND logged_at >= ?",
        params![week_start],
        |row| row.get(0)
    ).unwrap_or(0);
    
    let replies_month: u32 = conn.query_row(
        "SELECT COUNT(*) FROM conversation_logs WHERE is_bot_message = 1 AND logged_at >= ?",
        params![month_start],
        |row| row.get(0)
    ).unwrap_or(0);
    
    let replies_total: u32 = conn.query_row(
        "SELECT COUNT(*) FROM conversation_logs WHERE is_bot_message = 1",
        [],
        |row| row.get(0)
    ).unwrap_or(0);
    
    // 会話統計
    let unique_users: u32 = conn.query_row(
        "SELECT COUNT(DISTINCT user_pubkey) FROM conversation_logs",
        [],
        |row| row.get(0)
    ).unwrap_or(0);
    
    // RAG統計（embedding無効化のため常に0）
    let vectorized_events: u32 = 0;
    
    let total_events: u32 = conn.query_row(
        "SELECT COUNT(*) FROM events",
        [],
        |row| row.get(0)
    ).unwrap_or(0);
    
    let pending_vectorization = total_events.saturating_sub(vectorized_events);
    
    // レート制限されたユーザー数（過去N分間でM回以上会話したユーザー）
    // 簡易実装: 過去3分間で5回以上会話したユーザー数
    let rate_limited_users: u32 = conn.query_row(
        "SELECT COUNT(DISTINCT user_pubkey) FROM (
            SELECT user_pubkey, COUNT(*) as cnt 
            FROM conversation_logs 
            WHERE timestamp >= ? AND is_bot_message = 0
            GROUP BY user_pubkey 
            HAVING cnt >= 5
        )",
        params![now - 180], // 過去3分
        |row| row.get(0)
    ).unwrap_or(0);
    
    // RAG検索統計: conversation_summariesテーブルの件数を使用
    let total_searches: u32 = conn.query_row(
        "SELECT COUNT(*) FROM conversation_summaries",
        [],
        |row| row.get(0)
    ).unwrap_or(0);
    
    // 平均類似度: conversation_summariesが存在しないため、暫定で0.0
    // 実装するには検索履歴テーブルが必要
    let average_similarity: f64 = 0.0;
    
    Ok(DashboardStats {
        replies_today,
        replies_week,
        replies_month,
        replies_total,
        unique_users,
        rate_limited_users,
        vectorized_events,
        total_events,
        pending_vectorization,
        total_searches,
        average_similarity,
    })
}

#[derive(Debug, Clone)]
pub struct DashboardStats {
    pub replies_today: u32,
    pub replies_week: u32,
    pub replies_month: u32,
    pub replies_total: u32,
    pub unique_users: u32,
    pub rate_limited_users: u32,
    pub vectorized_events: u32,
    pub total_events: u32,
    pub pending_vectorization: u32,
    pub total_searches: u32,
    pub average_similarity: f64,
}
