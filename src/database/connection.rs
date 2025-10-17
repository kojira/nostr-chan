use rusqlite::{Connection, Result};

/// デフォルトのデータベースに接続
pub(crate) fn connect() -> Result<Connection> {
    Connection::open("../nostrchan.db")
}

/// 任意パスのSQLiteに接続（テーブル作成は行わない）
#[allow(dead_code)]
pub fn connect_at_path(path: &str) -> Result<Connection> {
    Connection::open(path)
}
