mod types;
mod stats;
mod bots;
mod follower_cache;
mod settings;
mod summaries;

pub use types::{DashboardState, BotInfo};

use axum::{
    routing::{get, post, put, delete},
    Router,
    response::{IntoResponse, Html},
    http::StatusCode,
};
use tower_http::services::ServeDir;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// ダッシュボードサーバーを起動
pub async fn start_dashboard(
    port: u16,
    db_path: String,
    bot_info: Arc<RwLock<BotInfo>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = DashboardState {
        db_path,
        start_time: Arc::new(Instant::now()),
        bot_info,
    };

    // APIルート
    let api_router = Router::new()
        // 統計
        .route("/api/stats", get(stats::stats_handler))
        .route("/api/analytics/daily-replies", get(stats::daily_replies_handler))
        // Bot管理
        .route("/api/bots", get(bots::list_bots_handler))
        .route("/api/bots", post(bots::create_bot_handler))
        .route("/api/bots/generate-key", get(bots::generate_key_handler))
        .route("/api/bots/:pubkey", put(bots::update_bot_handler))
        .route("/api/bots/:pubkey", delete(bots::delete_bot_handler))
        .route("/api/bots/:pubkey/toggle", post(bots::toggle_bot_handler))
        .route("/api/bots/:pubkey/kind0", get(bots::fetch_kind0_handler))
        .route("/api/bots/:pubkey/post", post(bots::post_as_bot_handler))
        .route("/api/bots/:pubkey/replies", get(bots::get_bot_replies_handler))
        .route("/api/bots/:pubkey/summaries", get(summaries::list_summaries_handler))
        .route("/api/summaries/:id", put(summaries::update_summary_handler))
        .route("/api/summaries/:id", delete(summaries::delete_summary_handler))
        // フォロワーキャッシュ
        .route("/api/follower-cache", get(follower_cache::list_follower_cache_handler))
        .route("/api/follower-cache", delete(follower_cache::clear_follower_cache_handler))
        .route("/api/follower-cache/:user_pubkey/:bot_pubkey", put(follower_cache::update_follower_cache_handler))
        .route("/api/follower-cache/:user_pubkey/:bot_pubkey", delete(follower_cache::delete_follower_cache_handler))
        // 設定
        .route("/api/global-pause", get(settings::get_global_pause_handler))
        .route("/api/global-pause", post(settings::set_global_pause_handler))
        .route("/api/settings/follower-cache-ttl", get(settings::get_follower_cache_ttl_handler))
        .route("/api/settings/follower-cache-ttl", post(settings::set_follower_cache_ttl_handler))
        .route("/api/settings/bot-behavior", get(settings::get_bot_behavior_settings_handler))
        .route("/api/settings/bot-behavior", post(settings::set_bot_behavior_settings_handler))
        .route("/api/settings/conversation-limit", get(settings::get_conversation_limit_settings_handler))
        .route("/api/settings/conversation-limit", post(settings::set_conversation_limit_settings_handler))
        .route("/api/settings/rag", get(settings::get_rag_settings_handler))
        .route("/api/settings/rag", post(settings::set_rag_settings_handler))
        .route("/api/settings/gpt", get(settings::get_gpt_settings_handler))
        .route("/api/settings/gpt", post(settings::set_gpt_settings_handler))
        .route("/api/settings/relay", get(settings::get_relay_settings_handler))
        .route("/api/settings/relay", post(settings::set_relay_settings_handler))
        .route("/api/settings/blacklist", get(settings::get_blacklist_settings_handler))
        .route("/api/settings/blacklist", post(settings::set_blacklist_settings_handler))
        .with_state(state);

    // 静的ファイル配信 + APIルート
    // プロジェクトルートからの絶対パス（CARGO_MANIFEST_DIRを使用）
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let dashboard_dir = format!("{}/dashboard", manifest_dir);
    let assets_dir = format!("{}/dashboard/assets", manifest_dir);
    
    println!("📂 Dashboard directory: {}", dashboard_dir);
    println!("📂 Assets directory: {}", assets_dir);
    
    // ディレクトリの存在確認（デバッグ用）
    if std::path::Path::new(&dashboard_dir).exists() {
        println!("✅ Dashboard directory exists");
    } else {
        println!("❌ Dashboard directory NOT found!");
    }
    
    if std::path::Path::new(&assets_dir).exists() {
        println!("✅ Assets directory exists");
    } else {
        println!("❌ Assets directory NOT found!");
    }

    // SPAフォールバックハンドラ
    let dashboard_dir_for_fallback = dashboard_dir.clone();
    
    let app = Router::new()
        .merge(api_router)
        .nest_service("/assets", ServeDir::new(assets_dir))
        .fallback(move || {
            let index_path = format!("{}/index.html", dashboard_dir_for_fallback);
            async move {
                match tokio::fs::read_to_string(&index_path).await {
                    Ok(content) => Html(content).into_response(),
                    Err(_) => (StatusCode::NOT_FOUND, "index.html not found").into_response(),
                }
            }
        });

    let addr = format!("0.0.0.0:{}", port);
    println!("🌐 Dashboard server starting on http://{}", addr);
    println!("   ローカルアクセス: http://127.0.0.1:{}", port);
    println!("   ネットワークアクセス: http://<your-ip>:{}", port);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

