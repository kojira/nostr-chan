use std::env;
use std::io::{self, Write};

use bot::config;
use bot::database as db;
use bot::embedding;
use bot::conversation;
use bot::gpt;
use nostr_sdk::prelude::*;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: conv_tester <SUBCOMMAND> --db <PATH> [...options]\n  SUBCOMMANDS: init-db | seed | chat | metrics | dump-context");
        std::process::exit(1);
    }

    let sub = args[1].as_str();
    let db_path = parse_flag_value(&args, "--db").unwrap_or_else(|| {
        eprintln!("--db <PATH> is required");
        std::process::exit(1);
    });

    match sub {
        "init-db" => {
            match db::connect_at_path(&db_path) {
                Ok(_) => println!("✅ initialized: {}", db_path),
                Err(e) => {
                    eprintln!("init-db error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "seed" => {
            let bot_pubkey = parse_flag_value(&args, "--bot").unwrap_or_else(|| {
                eprintln!("--bot <hex-pubkey> is required");
                std::process::exit(1);
            });
            let users: usize = parse_flag_value(&args, "--users").and_then(|v| v.parse().ok()).unwrap_or(10);
            let topics = parse_flag_value(&args, "--topics").unwrap_or_else(|| "ラーメン,旅行,暗号資産".to_string());
            let count: usize = parse_flag_value(&args, "--count").and_then(|v| v.parse().ok()).unwrap_or(50);

            if let Err(e) = seed_data(&db_path, &bot_pubkey, users, &topics, count).await {
                eprintln!("seed error: {}", e);
                std::process::exit(1);
            }
        }
        "chat" => {
            let bot_pubkey = parse_flag_value(&args, "--bot").unwrap_or_else(|| {
                eprintln!("--bot <hex-pubkey> is required");
                std::process::exit(1);
            });
            let bot_secret = parse_flag_value(&args, "--bot-secret").unwrap_or_else(|| {
                eprintln!("--bot-secret <nsec|hex> is required");
                std::process::exit(1);
            });
            let user_secret = parse_flag_value(&args, "--user-secret");

            if let Err(e) = chat_repl(&db_path, &bot_pubkey, &bot_secret, user_secret.as_deref()).await {
                eprintln!("chat error: {}", e);
                std::process::exit(1);
            }
        }
        "metrics" => {
            let bot_pubkey = parse_flag_value(&args, "--bot").unwrap_or_else(|| {
                eprintln!("--bot <hex-pubkey> is required");
                std::process::exit(1);
            });
            if let Err(e) = print_metrics(&db_path, &bot_pubkey).await {
                eprintln!("metrics error: {}", e);
                std::process::exit(1);
            }
        }
        "dump-context" => {
            let bot_pubkey = parse_flag_value(&args, "--bot").unwrap_or_else(|| {
                eprintln!("--bot <hex-pubkey> is required");
                std::process::exit(1);
            });
            let user_pubkey = parse_flag_value(&args, "--user").unwrap_or_else(|| {
                eprintln!("--user <hex-pubkey> is required");
                std::process::exit(1);
            });
            let input = parse_flag_value(&args, "--input").unwrap_or_else(|| "こんにちは".to_string());
            if let Err(e) = dump_context(&db_path, &bot_pubkey, &user_pubkey, &input).await {
                eprintln!("dump-context error: {}", e);
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("unknown subcommand: {}", sub);
            std::process::exit(1);
        }
    }
}

fn parse_flag_value(args: &[String], flag: &str) -> Option<String> {
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned()
}

async fn seed_data(db_path: &str, bot_pubkey: &str, users: usize, topics_csv: &str, count: usize) -> Result<(), Box<dyn std::error::Error>> {
    embedding::EmbeddingService::initialize_global()?;
    let conn = db::connect_at_path(db_path)?;

    let topics: Vec<String> = topics_csv.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    if topics.is_empty() { return Err("no topics".into()); }

    let mut user_keys: Vec<Keys> = Vec::new();
    for _ in 0..users { user_keys.push(Keys::generate()); }

    for i in 0..count {
        let user = &user_keys[i % users];
        let topic = &topics[i % topics.len()];
        let content = format!("{}についてどう思いますか？ケース#{}", topic, i + 1);

        let event = EventBuilder::text_note(content.clone()).sign(user).await?;

        let is_japanese = whatlang::detect(&content).map(|d| matches!(d.lang(), whatlang::Lang::Jpn)).unwrap_or(false);
        let language = if is_japanese { Some("ja") } else { None };
        let event_ref_id = db::insert_event(&conn, &event, language)?;

        if let Ok(emb) = embedding::generate_embedding_global(&content) {
            let _ = db::update_event_embedding(&conn, &event.id.to_string(), &emb);
        }

        let event_json = serde_json::to_string(&event)?;
        let thread_root_id = db::extract_thread_root_id(&event_json).ok().flatten();
        let _ = db::insert_conversation_log(&conn, bot_pubkey, event_ref_id, thread_root_id.as_deref(), None, false, false)?;
    }

    println!("✅ seeded: users={}, topics={}, count={}", users, topics.len(), count);
    Ok(())
}

async fn chat_repl(db_path: &str, bot_pubkey: &str, bot_secret: &str, user_secret: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    embedding::EmbeddingService::initialize_global()?;
    let conn = db::connect_at_path(db_path)?;
    
    // configを読み込む
    let file = std::fs::File::open("../config.yml")?;
    let config: config::AppConfig = serde_yaml::from_reader(file)?;

    let bot_keys = Keys::parse(bot_secret)?;
    let user_keys = match user_secret { Some(s) => Keys::parse(s)?, None => Keys::generate() };
    let user_pubkey = user_keys.public_key().to_string();

    println!("BOT: {}\nUSER: {}", bot_pubkey, user_pubkey);
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("> ");
        stdout.flush().ok();
        let mut line = String::new();
        if stdin.read_line(&mut line).is_err() { break; }
        let input = line.trim();
        if input.is_empty() { continue; }
        if input == "/exit" { break; }

        let user_event = EventBuilder::text_note(input.to_string()).sign(&user_keys).await?;
        let is_japanese = whatlang::detect(input).map(|d| matches!(d.lang(), whatlang::Lang::Jpn)).unwrap_or(false);
        let language = if is_japanese { Some("ja") } else { None };
        let event_ref_id = db::insert_event(&conn, &user_event, language)?;
        if let Ok(emb) = embedding::generate_embedding_global(input) { let _ = db::update_event_embedding(&conn, &user_event.id.to_string(), &emb); }
        let event_json = serde_json::to_string(&user_event)?;
        let thread_root_id = db::extract_thread_root_id(&event_json).ok().flatten();
        let _ = db::insert_conversation_log(&conn, bot_pubkey, event_ref_id, thread_root_id.as_deref(), None, false, false)?;

        let context = conversation::prepare_context_for_reply(&conn, bot_pubkey, &user_pubkey, input, 50, &config, None).await?;

        let prompt = "あなたは有益で礼儀正しい日本語のアシスタントです。".to_string();
        let reply = gpt::get_reply_with_context(bot_pubkey, &prompt, input, true, if context.is_empty() { None } else { Some(context) }).await?;
        println!("BOT> {}", reply);

        if reply.is_empty() { continue; }
        let bot_event = EventBuilder::text_note(reply.clone()).sign(&bot_keys).await?;
        let event_ref_id = db::insert_event(&conn, &bot_event, Some("ja"))?;
        if let Ok(emb) = embedding::generate_embedding_global(&reply) { let _ = db::update_event_embedding(&conn, &bot_event.id.to_string(), &emb); }
        let bot_json = serde_json::to_string(&bot_event)?;
        let thread_root_id = db::extract_thread_root_id(&bot_json).ok().flatten();
        let _ = db::insert_conversation_log(&conn, bot_pubkey, event_ref_id, thread_root_id.as_deref(), None, true, false)?;
    }
    Ok(())
}

async fn print_metrics(db_path: &str, bot_pubkey: &str) -> Result<(), Box<dyn std::error::Error>> {
    let conn = db::connect_at_path(db_path)?;
    let mut stmt = conn.prepare(
        "SELECT e.pubkey FROM events e INNER JOIN conversation_logs cl ON e.id = cl.event_ref_id WHERE cl.bot_pubkey = ? ORDER BY e.created_at DESC LIMIT 200"
    )?;
    let mut rows = stmt.query(rusqlite::params![bot_pubkey])?;
    let mut total = 0usize;
    let mut bot_msgs = 0usize;
    let mut users = std::collections::HashSet::new();
    while let Some(row) = rows.next()? {
        total += 1;
        let pubkey: String = row.get(0)?;
        if pubkey == bot_pubkey { bot_msgs += 1; }
        users.insert(pubkey);
    }
    let participants = users.len();
    let bot_ratio = if total > 0 { bot_msgs as f32 / total as f32 } else { 0.0 };
    println!("participants={}, total_msgs={}, bot_ratio={:.2}", participants, total, bot_ratio);
    Ok(())
}

async fn dump_context(db_path: &str, bot_pubkey: &str, user_pubkey: &str, input: &str) -> Result<(), Box<dyn std::error::Error>> {
    let conn = db::connect_at_path(db_path)?;
    
    // configを読み込む
    let file = std::fs::File::open("../config.yml")?;
    let config: config::AppConfig = serde_yaml::from_reader(file)?;
    
    let ctx = conversation::prepare_context_for_reply(&conn, bot_pubkey, user_pubkey, input, 50, &config, None).await?;
    println!("{}", ctx);
    Ok(())
}



