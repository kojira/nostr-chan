use chrono::NaiveDateTime;
use mysql::{params, prelude::*, Opts, OptsBuilder};
use nostr_sdk::Kind;
use r2d2_mysql::{mysql::Error as MysqlError, MySqlConnectionManager};
use std::env;
use std::primitive::str;

const DATABASE_USER: &str = "MYSQL_USER";
const DATABASE_PASS: &str = "MYSQL_PASSWORD";
const DATABASE_NAME: &str = "MYSQL_DATABASE";

const DATABASE_POOL_SIZE: u32 = 4;

fn env_var(name: &str, def_var: Option<String>) -> String {
    let env_var = env::var(name);
    return match def_var {
        Some(v) => env_var.unwrap_or(v),
        _ => env_var.expect(format!("{} must be set", name).as_str()),
    };
}

pub(crate) fn connect() -> Result<r2d2::Pool<MySqlConnectionManager>, MysqlError> {
    let db_user = env_var(DATABASE_USER, None);
    let db_pass = env_var(DATABASE_PASS, None);
    let db_name = env_var(DATABASE_NAME, None);
    let db_url = format!(
        "mysql://{user}:{pass}@{host}:{port}/{name}",
        user = db_user,
        pass = db_pass,
        // host = "127.0.0.1",
        host = "db",
        port = "3306",
        name = db_name
    );
    println!("db connect");
    let opts = Opts::from_url(&db_url).unwrap();
    let builder = OptsBuilder::from_opts(opts);
    let manager = MySqlConnectionManager::new(builder);
    println!("db connect2");

    let pool = r2d2::Pool::builder()
        .max_size(DATABASE_POOL_SIZE)
        .build(manager)
        .unwrap();
    println!("db connect3");
    Ok(pool)
}

pub fn to_unix_timestamp(datetime_str: &str) -> Option<i64> {
    // 日時文字列をDateTimeオブジェクトに変換する
    let datetime = NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S")
        .map_err(|e| e.to_string())
        .ok()?;

    // Unixタイムスタンプを取得する
    Some(datetime.timestamp() as i64)
}

pub struct _Event {
    pub pubkey: String,
    pub content: String,
}

pub fn select_events(
    pool: &r2d2::Pool<MySqlConnectionManager>,
    kind: Kind,
    _from: &str,
    _to: &str,
) -> Vec<_Event> {
    let pool = pool.clone();
    let mut conn = pool.get().unwrap();

    let mut events_result: Vec<_Event> = vec![];

    let mut rows = conn
        .exec_iter(
            "SELECT * FROM events WHERE event_created_at >= :from AND event_created_at < :to AND tags not like '%mostr%' and kind=:kind ORDER BY event_created_at ASC",
            params! {
                "kind" => kind.as_u32(),
                "from" => format!("{}",_from),
                "to" => format!("{}",_to),
            }
        )
        .unwrap();

    while let Some(row) = rows.next() {
        let row = row.unwrap();
        let pubkey: String = row.get("pubkey").unwrap();

        let content: String = row.get("content").unwrap();
        let event = _Event {
            pubkey: pubkey,
            content: content,
        };
        events_result.push(event);
    }

    events_result
}
