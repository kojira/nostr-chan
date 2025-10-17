use rusqlite::{Connection, Result, params};
use crate::database::token_usage::TokenCategory;

/// Personsテーブルにair_reply_single_ratioカラムを追加するマイグレーション
pub(crate) fn migrate_add_air_reply_single_ratio(conn: &Connection) -> Result<()> {
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
pub(crate) fn migrate_remove_kind0_content(conn: &Connection) -> Result<()> {
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
pub(crate) fn initialize_token_categories(conn: &Connection) -> Result<()> {
    for category in TokenCategory::all() {
        conn.execute(
            "INSERT OR IGNORE INTO token_categories (id, name, display_name) VALUES (?, ?, ?)",
            params![category as i32, category.name(), category.display_name()],
        )?;
    }
    println!("⚙️ トークンカテゴリ: {}種類を初期化", TokenCategory::all().len());
    Ok(())
}

/// 既存のtoken_usageテーブルを正規化したスキーマへマイグレーション
pub(crate) fn migrate_token_usage_table(conn: &Connection) -> Result<()> {
    // カラムの存在確認
    let has_category_column: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('token_usage') WHERE name = 'category'")?
        .query_row([], |row| row.get(0))
        .map(|count: i32| count > 0)?;
    
    let has_category_id_column: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('token_usage') WHERE name = 'category_id'")?
        .query_row([], |row| row.get(0))
        .map(|count: i32| count > 0)?;
    
    let has_bot_pubkey_column: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('token_usage') WHERE name = 'bot_pubkey'")?
        .query_row([], |row| row.get(0))
        .map(|count: i32| count > 0)?;
    
    // 既に新しいスキーマになっている場合はインデックスのみ作成して終了
    if !has_category_column && has_category_id_column && has_bot_pubkey_column {
        // インデックスを作成
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_token_usage_bot ON token_usage(bot_pubkey, created_at DESC)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_token_usage_category ON token_usage(category_id, created_at DESC)",
            [],
        )?;
        return Ok(());
    }
    
    // category_idはあるがbot_pubkeyが無い場合（部分的なマイグレーション済み）
    if has_category_id_column && !has_bot_pubkey_column {
        println!("🔄 マイグレーション: token_usageテーブルにbot_pubkeyカラムを追加");
        
        // 外部キー制約を一時的に無効化
        conn.execute("PRAGMA foreign_keys = OFF", [])?;
        
        // 既存データを一時テーブルに退避
        conn.execute(
            "CREATE TEMPORARY TABLE token_usage_backup AS SELECT * FROM token_usage",
            [],
        )?;
        
        // 古いテーブルを削除
        conn.execute("DROP TABLE token_usage", [])?;
        
        // 新しいスキーマでテーブルを作成
        conn.execute(
            "CREATE TABLE token_usage (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                bot_pubkey TEXT NOT NULL,
                category_id INTEGER NOT NULL,
                prompt_tokens INTEGER NOT NULL,
                completion_tokens INTEGER NOT NULL,
                total_tokens INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (category_id) REFERENCES token_categories(id)
            )",
            [],
        )?;
        
        // Personsテーブルから最初のbotを取得（デフォルト値として使用）
        let default_bot_pubkey: String = conn
            .query_row("SELECT pubkey FROM Persons LIMIT 1", [], |row| row.get(0))
            .unwrap_or_else(|_| "unknown".to_string());
        
        // データを新しいテーブルに挿入（bot_pubkeyを追加）
        conn.execute(
            &format!("INSERT INTO token_usage (id, bot_pubkey, category_id, prompt_tokens, completion_tokens, total_tokens, created_at)
                      SELECT id, '{}', category_id, prompt_tokens, completion_tokens, total_tokens, created_at
                      FROM token_usage_backup", default_bot_pubkey),
            [],
        )?;
        
        // 一時テーブルを削除
        conn.execute("DROP TABLE token_usage_backup", [])?;
        
        // 外部キー制約を再度有効化
        conn.execute("PRAGMA foreign_keys = ON", [])?;
        
        // インデックスを作成
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_token_usage_bot ON token_usage(bot_pubkey, created_at DESC)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_token_usage_category ON token_usage(category_id, created_at DESC)",
            [],
        )?;
        
        println!("✅ マイグレーション完了: bot_pubkeyカラムを追加");
        return Ok(());
    }
    
    // 古いスキーマ（categoryカラムあり）からのマイグレーション
    if has_category_column {
        println!("🔄 マイグレーション: token_usageテーブルを正規化");
        
        // 外部キー制約を一時的に無効化
        conn.execute("PRAGMA foreign_keys = OFF", [])?;
        
        // 既存データを一時テーブルに退避
        conn.execute(
            "CREATE TEMPORARY TABLE token_usage_backup AS SELECT * FROM token_usage",
            [],
        )?;
        
        // 古いテーブルを削除
        conn.execute("DROP TABLE token_usage", [])?;
        
        // 新しいスキーマでテーブルを作成
        conn.execute(
            "CREATE TABLE token_usage (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                bot_pubkey TEXT NOT NULL,
                category_id INTEGER NOT NULL,
                prompt_tokens INTEGER NOT NULL,
                completion_tokens INTEGER NOT NULL,
                total_tokens INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (category_id) REFERENCES token_categories(id)
            )",
            [],
        )?;
        
        // データを変換して新しいテーブルに挿入
        // カテゴリ文字列をIDに変換
        let mut stmt = conn.prepare(
            "SELECT id, category, prompt_tokens, completion_tokens, total_tokens, created_at 
             FROM token_usage_backup"
        )?;
        
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,      // id
                row.get::<_, String>(1)?,   // category
                row.get::<_, i64>(2)?,      // prompt_tokens
                row.get::<_, i64>(3)?,      // completion_tokens
                row.get::<_, i64>(4)?,      // total_tokens
                row.get::<_, i64>(5)?,      // created_at
            ))
        })?;
        
        // Personsテーブルから最初のbotを取得（デフォルト値として使用）
        let default_bot_pubkey: String = conn
            .query_row("SELECT pubkey FROM Persons LIMIT 1", [], |row| row.get(0))
            .unwrap_or_else(|_| "unknown".to_string());
        
        for row in rows {
            let (id, category_str, prompt_tokens, completion_tokens, total_tokens, created_at) = row?;
            
            // カテゴリ文字列をIDに変換
            let category_id = match category_str.as_str() {
                "reply" => TokenCategory::Reply as i32,
                "air_reply" => TokenCategory::AirReply as i32,
                "summary" => TokenCategory::Summary as i32,
                "search_initial_reply" => TokenCategory::SearchInitialReply as i32,
                "search_keyword_extraction" => TokenCategory::SearchKeywordExtraction as i32,
                "search_final_reply" => TokenCategory::SearchFinalReply as i32,
                _ => TokenCategory::Reply as i32, // デフォルト
            };
            
            conn.execute(
                "INSERT INTO token_usage (id, bot_pubkey, category_id, prompt_tokens, completion_tokens, total_tokens, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                params![id, &default_bot_pubkey, category_id, prompt_tokens, completion_tokens, total_tokens, created_at],
            )?;
        }
        
        // 一時テーブルを削除
        conn.execute("DROP TABLE token_usage_backup", [])?;
        
        // 外部キー制約を再度有効化
        conn.execute("PRAGMA foreign_keys = ON", [])?;
        
        // インデックスを作成
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_token_usage_bot ON token_usage(bot_pubkey, created_at DESC)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_token_usage_category ON token_usage(category_id, created_at DESC)",
            [],
        )?;
        
        println!("✅ マイグレーション完了: token_usageテーブルを正規化");
    }
    
    Ok(())
}

/// token_usageテーブルにprompt_textとcompletion_textカラムを追加するマイグレーション
pub(crate) fn migrate_add_token_text_columns(conn: &Connection) -> Result<()> {
    // カラムの存在確認
    let has_prompt_text: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('token_usage') WHERE name = 'prompt_text'")?
        .query_row([], |row| row.get(0))
        .map(|count: i32| count > 0)?;
    
    let has_completion_text: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('token_usage') WHERE name = 'completion_text'")?
        .query_row([], |row| row.get(0))
        .map(|count: i32| count > 0)?;
    
    // 既に両方のカラムがある場合は何もしない
    if has_prompt_text && has_completion_text {
        return Ok(());
    }
    
    println!("🔄 マイグレーション: token_usageテーブルにテキストカラムを追加");
    
    // 外部キー制約を一時的に無効化
    conn.execute("PRAGMA foreign_keys = OFF", [])?;
    
    // 既存データを一時テーブルに退避
    conn.execute(
        "CREATE TEMPORARY TABLE token_usage_backup AS SELECT * FROM token_usage",
        [],
    )?;
    
    // 古いテーブルを削除
    conn.execute("DROP TABLE token_usage", [])?;
    
    // 新しいスキーマでテーブルを作成
    conn.execute(
        "CREATE TABLE token_usage (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            bot_pubkey TEXT NOT NULL,
            category_id INTEGER NOT NULL,
            prompt_tokens INTEGER NOT NULL,
            completion_tokens INTEGER NOT NULL,
            total_tokens INTEGER NOT NULL,
            prompt_text TEXT NOT NULL DEFAULT '',
            completion_text TEXT NOT NULL DEFAULT '',
            created_at INTEGER NOT NULL,
            FOREIGN KEY (category_id) REFERENCES token_categories(id)
        )",
        [],
    )?;
    
    // データを新しいテーブルに挿入（テキストカラムは空文字列）
    conn.execute(
        "INSERT INTO token_usage (id, bot_pubkey, category_id, prompt_tokens, completion_tokens, total_tokens, prompt_text, completion_text, created_at)
         SELECT id, bot_pubkey, category_id, prompt_tokens, completion_tokens, total_tokens, '', '', created_at
         FROM token_usage_backup",
        [],
    )?;
    
    // 一時テーブルを削除
    conn.execute("DROP TABLE token_usage_backup", [])?;
    
    // 外部キー制約を再度有効化
    conn.execute("PRAGMA foreign_keys = ON", [])?;
    
    // インデックスを作成
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_token_usage_bot ON token_usage(bot_pubkey, created_at DESC)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_token_usage_category ON token_usage(category_id, created_at DESC)",
        [],
    )?;
    
    println!("✅ マイグレーション完了: テキストカラムを追加");
    
    Ok(())
}

/// eventsテーブルを正規化：is_japanese→language、event_type削除、kind0_name削除
pub(crate) fn migrate_normalize_events_table(conn: &Connection) -> Result<()> {
    // カラムの存在確認
    let has_is_japanese: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('events') WHERE name = 'is_japanese'")?
        .query_row([], |row| row.get(0))
        .map(|count: i32| count > 0)?;
    
    let has_language: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('events') WHERE name = 'language'")?
        .query_row([], |row| row.get(0))
        .map(|count: i32| count > 0)?;
    
    let has_event_type: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('events') WHERE name = 'event_type'")?
        .query_row([], |row| row.get(0))
        .map(|count: i32| count > 0)?;
    
    let has_kind0_name: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('events') WHERE name = 'kind0_name'")?
        .query_row([], |row| row.get(0))
        .map(|count: i32| count > 0)?;
    
    // 既に正規化済み（language あり、event_type/kind0_name なし）の場合は何もしない
    if has_language && !has_event_type && !has_kind0_name && !has_is_japanese {
        return Ok(());
    }
    
    // マイグレーションが必要
    if !has_is_japanese && !has_event_type && !has_kind0_name {
        return Ok(()); // 新規環境
    }
    
    println!("🔄 マイグレーション: eventsテーブルを正規化");
    println!("   - is_japanese → language");
    println!("   - event_type → 削除（不要）");
    println!("   - kind0_name → 削除（kind0_cacheをJOINで参照）");
    
    // 外部キー制約を一時的に無効化
    conn.execute("PRAGMA foreign_keys = OFF", [])?;
    conn.execute("BEGIN TRANSACTION", [])?;
    
    let migration_result = (|| {
        // 元のデータ件数を記録
        let original_count: i64 = conn.query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))?;
        println!("   📊 元のデータ件数: {}", original_count);
        
        // 前回の失敗で残っているかもしれないevents_newテーブルを削除
        conn.execute("DROP TABLE IF EXISTS events_new", [])?;
        
        // 新しいテーブルを作成（正規化後）
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
                language TEXT,
                embedding BLOB
            )",
            [],
        )?;
        
        // データをコピー
        if has_is_japanese {
            // is_japanese → language変換
            conn.execute(
                "INSERT INTO events_new 
                 SELECT id, event_id, event_json, pubkey, kind, content, created_at, received_at, 
                        CASE WHEN is_japanese = 1 THEN 'ja' ELSE NULL END as language,
                        embedding
                 FROM events",
                [],
            )?;
        } else if has_language {
            // 既にlanguageがある場合
            conn.execute(
                "INSERT INTO events_new 
                 SELECT id, event_id, event_json, pubkey, kind, content, created_at, received_at, 
                        language, embedding
                 FROM events",
                [],
            )?;
        } else {
            // languageもis_japaneseもない場合
            conn.execute(
                "INSERT INTO events_new 
                 SELECT id, event_id, event_json, pubkey, kind, content, created_at, received_at, 
                        NULL as language, embedding
                 FROM events",
                [],
            )?;
        }
        
        // データ件数を検証
        let new_count: i64 = conn.query_row("SELECT COUNT(*) FROM events_new", [], |row| row.get(0))?;
        println!("   📊 コピー後のデータ件数: {}", new_count);
        
        if original_count != new_count {
            return Err(rusqlite::Error::QueryReturnedNoRows); // データ損失を検知したらエラー
        }
        
        // 古いテーブルを削除
        conn.execute("DROP TABLE events", [])?;
        
        // 新しいテーブルをリネーム
        conn.execute("ALTER TABLE events_new RENAME TO events", [])?;
        
        // インデックスを再作成
        conn.execute("CREATE INDEX IF NOT EXISTS idx_events_pubkey ON events(pubkey)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_events_kind ON events(kind)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_events_created_at ON events(created_at DESC)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_events_language ON events(language)", [])?;
        
        Ok(())
    })();
    
    match migration_result {
        Ok(_) => {
            conn.execute("COMMIT", [])?;
            println!("✅ マイグレーション完了: eventsテーブルを正規化");
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK", []);
            println!("❌ マイグレーション失敗: {:?}", e);
            println!("🔄 ロールバックしました");
            return Err(e);
        }
    }
    
    // 外部キー制約を再度有効化
    conn.execute("PRAGMA foreign_keys = ON", [])?;
    
    Ok(())
}

