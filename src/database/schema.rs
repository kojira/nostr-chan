use rusqlite::{Connection, Result};

/// データベースの初期化（テーブル作成とマイグレーション）
/// 起動時に1回だけ呼ぶこと
pub fn initialize_db(conn: &Connection) -> Result<()> {
    // テーブル作成
    create_tables(conn)?;
    
    // トークンカテゴリの初期化
    super::migration::initialize_token_categories(conn)?;
    
    // マイグレーション実行
    super::migration::migrate_token_usage_table(conn)?;
    super::migration::migrate_add_token_text_columns(conn)?;
    super::migration::migrate_add_air_reply_single_ratio(conn)?;
    super::migration::migrate_remove_kind0_content(conn)?;
    super::migration::migrate_normalize_events_table(conn)?; // events正規化
    
    Ok(())
}

/// 全テーブルを作成
fn create_tables(conn: &Connection) -> Result<()> {
    // follower_cache table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS follower_cache (
            user_pubkey TEXT NOT NULL,
            bot_pubkey TEXT NOT NULL,
            is_follower INTEGER NOT NULL,
            cached_at INTEGER NOT NULL,
            PRIMARY KEY (user_pubkey, bot_pubkey)
        )",
        [],
    )?;
    
    // kind0_cache table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS kind0_cache (
            pubkey TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            cached_at INTEGER NOT NULL
        )",
        [],
    )?;
    
    // timeline table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS timeline (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            pubkey TEXT NOT NULL,
            name TEXT,
            content TEXT NOT NULL,
            timestamp INTEGER NOT NULL
        )",
        [],
    )?;
    
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_timeline_timestamp ON timeline(timestamp DESC)",
        [],
    )?;
    
    // system_settings table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS system_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )?;
    
    // event_queue table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS event_queue (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            event_json TEXT NOT NULL,
            added_at INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending'
        )",
        [],
    )?;
    
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_event_queue_status_added 
         ON event_queue(status, added_at)",
        [],
    )?;
    
    // events table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            event_id TEXT UNIQUE NOT NULL,
            event_json TEXT NOT NULL,
            pubkey TEXT NOT NULL,
            kind INTEGER NOT NULL,
            content TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            received_at INTEGER NOT NULL,
            kind0_name TEXT,
            language TEXT,
            embedding BLOB,
            event_type TEXT
        )",
        [],
    )?;
    
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_events_pubkey ON events(pubkey)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_events_kind ON events(kind)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_events_created_at ON events(created_at DESC)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_events_language ON events(language)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_events_event_type ON events(event_type)",
        [],
    )?;
    
    // conversation_logs table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS conversation_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            bot_pubkey TEXT NOT NULL,
            event_ref_id INTEGER NOT NULL,
            thread_root_id TEXT,
            mentioned_pubkeys_json TEXT,
            is_bot_message INTEGER NOT NULL DEFAULT 0,
            is_bot_conversation INTEGER NOT NULL DEFAULT 0,
            logged_at INTEGER NOT NULL,
            FOREIGN KEY (event_ref_id) REFERENCES events(id)
        )",
        [],
    )?;
    
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_conversation_logs_bot_recent ON conversation_logs(bot_pubkey, logged_at DESC)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_conversation_logs_thread ON conversation_logs(thread_root_id)",
        [],
    )?;
    
    // conversation_summaries table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS conversation_summaries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            bot_pubkey TEXT NOT NULL,
            summary TEXT NOT NULL,
            user_input TEXT NOT NULL,
            user_input_embedding BLOB NOT NULL,
            participants_json TEXT,
            from_timestamp INTEGER NOT NULL,
            to_timestamp INTEGER NOT NULL,
            created_at INTEGER NOT NULL
        )",
        [],
    )?;
    
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_summaries_bot ON conversation_summaries(bot_pubkey, created_at DESC)",
        [],
    )?;
    
    // token_categories table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS token_categories (
            id INTEGER PRIMARY KEY,
            name TEXT UNIQUE NOT NULL,
            display_name TEXT NOT NULL
        )",
        [],
    )?;
    
    // token_usage table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS token_usage (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            bot_pubkey TEXT NOT NULL,
            category_id INTEGER NOT NULL,
            prompt_tokens INTEGER NOT NULL,
            completion_tokens INTEGER NOT NULL,
            total_tokens INTEGER NOT NULL,
            prompt_text TEXT NOT NULL,
            completion_text TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            FOREIGN KEY (category_id) REFERENCES token_categories(id)
        )",
        [],
    )?;
    
    Ok(())
}

/// 任意パスのSQLiteに接続（テーブル作成は行わない）
#[allow(dead_code)]
pub fn connect_at_path(path: &str) -> Result<Connection> {
    Connection::open(path)
}
