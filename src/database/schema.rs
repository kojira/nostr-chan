use rusqlite::{Connection, Result};
use crate::database::token_usage::TokenCategory;

/// データベースの初期化（テーブル作成とマイグレーション）
/// 起動時に1回だけ呼ぶこと
pub fn initialize_db(conn: &Connection) -> Result<()> {
    
    // Create follower_cache table if not exists
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
    
    // Create kind0_cache table if not exists
    conn.execute(
        "CREATE TABLE IF NOT EXISTS kind0_cache (
            pubkey TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            cached_at INTEGER NOT NULL
        )",
        [],
    )?;
    
    // Create timeline table if not exists
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
    
    // Create index on timestamp for efficient ordering
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_timeline_timestamp ON timeline(timestamp DESC)",
        [],
    )?;
    
    // Create system_settings table for global settings
    conn.execute(
        "CREATE TABLE IF NOT EXISTS system_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )?;
    
    // Create event_queue table for persistent event processing queue
    conn.execute(
        "CREATE TABLE IF NOT EXISTS event_queue (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            event_json TEXT NOT NULL,
            added_at INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending'
        )",
        [],
    )?;
    
    // Create index for efficient queue processing
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_event_queue_status_added 
         ON event_queue(status, added_at)",
        [],
    )?;
    
    // Create events table if not exists
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
            kind0_content TEXT,
            is_japanese INTEGER NOT NULL DEFAULT 0,
            embedding BLOB,
            event_type TEXT
        )",
        [],
    )?;
    
    // Create indexes for events table
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
        "CREATE INDEX IF NOT EXISTS idx_events_is_japanese ON events(is_japanese)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_events_event_type ON events(event_type)",
        [],
    )?;
    
    // Create conversation_logs table if not exists
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
    
    // Create indexes for conversation_logs table
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_conversation_logs_bot_recent ON conversation_logs(bot_pubkey, logged_at DESC)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_conversation_logs_thread ON conversation_logs(thread_root_id)",
        [],
    )?;
    
    // Create conversation_summaries table if not exists
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
    
    // Create index for conversation_summaries table
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_summaries_bot ON conversation_summaries(bot_pubkey, created_at DESC)",
        [],
    )?;
    
    // Create token_categories table for normalized category management
    conn.execute(
        "CREATE TABLE IF NOT EXISTS token_categories (
            id INTEGER PRIMARY KEY,
            name TEXT UNIQUE NOT NULL,
            display_name TEXT NOT NULL
        )",
        [],
    )?;
    
    // Create token_usage table for tracking token consumption (マイグレーション前の初期作成のみ)
    // Note: 既存テーブルがある場合はマイグレーションで処理される
    conn.execute(
        "CREATE TABLE IF NOT EXISTS token_usage (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            bot_pubkey TEXT NOT NULL,
            category_id INTEGER NOT NULL,
            prompt_tokens INTEGER NOT NULL,
            completion_tokens INTEGER NOT NULL,
            total_tokens INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            FOREIGN KEY (category_id) REFERENCES token_categories(id),
            FOREIGN KEY (bot_pubkey) REFERENCES Persons(pubkey)
        )",
        [],
    )?;
    
    // インデックスはマイグレーション後に作成される
    
    // トークンカテゴリの初期化
    initialize_token_categories(&conn)?;
    
    // マイグレーション: Personsテーブルにair_reply_single_ratioカラムを追加
    migrate_add_air_reply_single_ratio(&conn)?;
    
    // マイグレーション: eventsテーブルからkind0_contentカラムを削除
    migrate_remove_kind0_content(&conn)?;
    
    Ok(())
}

/// 任意パスのSQLiteに接続（テーブル作成は行わない）
#[allow(dead_code)]
pub fn connect_at_path(path: &str) -> Result<Connection> {
    Connection::open(path)
}
fn migrate_add_air_reply_single_ratio(conn: &Connection) -> Result<()> {
    // カラムが存在するかチェック
    let column_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('Persons') WHERE name='air_reply_single_ratio'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0) > 0;
    
    if !column_exists {
        println!("🔄 マイグレーション: Personsテーブルにair_reply_single_ratioカラムを追加");
        conn.execute(
            "ALTER TABLE Persons ADD COLUMN air_reply_single_ratio INTEGER NOT NULL DEFAULT 30",
            [],
        )?;
        println!("✅ マイグレーション完了: air_reply_single_ratio (デフォルト: 30%)");
    }
    
    Ok(())
}

/// eventsテーブルからkind0_contentカラムを削除するマイグレーション
fn migrate_remove_kind0_content(conn: &Connection) -> Result<()> {
    // カラムが存在するかチェック
    let column_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('events') WHERE name='kind0_content'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0) > 0;
    
    if !column_exists {
        return Ok(());
    }
    
    println!("🔄 マイグレーション: eventsテーブルからkind0_contentカラムを削除");
    
    // SQLiteではALTER TABLE DROP COLUMNが使えないので、テーブルを再作成する
    // 外部キー制約があるため、慎重に処理する
    
    // 1. 外部キー制約を一時的に無効化
    conn.execute("PRAGMA foreign_keys = OFF", [])?;
    
    // 2. トランザクション開始
    conn.execute("BEGIN TRANSACTION", [])?;
    
    // トランザクション内の処理（エラー時はロールバック）
    let migration_result = (|| {
        // 3. 前回の失敗で残っているかもしれないevents_newテーブルを削除
        conn.execute("DROP TABLE IF EXISTS events_new", [])?;
        
        // 4. 新しいテーブルを作成（kind0_contentなし）
        conn.execute(
            "CREATE TABLE events_new (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_id TEXT UNIQUE NOT NULL,
                event_json TEXT NOT NULL,
                pubkey TEXT NOT NULL,
                kind INTEGER NOT NULL,
                content TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                received_at INTEGER NOT NULL,
                kind0_name TEXT,
                is_japanese INTEGER NOT NULL DEFAULT 0,
                embedding BLOB,
                event_type TEXT
            )",
            [],
        )?;
        
        // 5. データをコピー（kind0_content以外）
        conn.execute(
            "INSERT INTO events_new 
             SELECT id, event_id, event_json, pubkey, kind, content, created_at, received_at, 
                    kind0_name, is_japanese, embedding, event_type
             FROM events",
            [],
        )?;
        
        // 6. 古いテーブルを削除
        conn.execute("DROP TABLE events", [])?;
        
        // 7. 新しいテーブルをリネーム
        conn.execute("ALTER TABLE events_new RENAME TO events", [])?;
        
        // 8. インデックスを再作成
        conn.execute("CREATE INDEX IF NOT EXISTS idx_events_pubkey ON events(pubkey)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_events_kind ON events(kind)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_events_created_at ON events(created_at DESC)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_events_is_japanese ON events(is_japanese)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_events_event_type ON events(event_type)", [])?;
        
        Ok(())
    })();
    
    // 9. トランザクションの結果に応じてコミットまたはロールバック
    match migration_result {
        Ok(_) => {
            conn.execute("COMMIT", [])?;
            println!("✅ マイグレーション完了: kind0_contentカラムを削除（データは保持）");
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK", []);
            println!("❌ マイグレーション失敗: {:?}", e);
            println!("🔄 ロールバックしました（データは元の状態に戻りました）");
            return Err(e);
        }
    }
    
    // 10. 外部キー制約を再度有効化
    conn.execute("PRAGMA foreign_keys = ON", [])?;
    
    Ok(())
}

/// トークンカテゴリをDBに初期化
fn initialize_token_categories(conn: &Connection) -> Result<()> {
    use rusqlite::params;
    
    for category in TokenCategory::all() {
        conn.execute(
            "INSERT OR IGNORE INTO token_categories (id, name, display_name) VALUES (?, ?, ?)",
            params![category as i32, category.name(), category.display_name()],
        )?;
    }
    println!("⚙️ トークンカテゴリ: {}種類を初期化", TokenCategory::all().len());
    Ok(())
}
