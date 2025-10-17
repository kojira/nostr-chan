// データベースモジュール
// db.rsをリファクタリングして機能別に分割

// 外部公開用の再エクスポートで未使用warningを抑制
#![allow(unused_imports)]

pub mod connection;
pub mod schema;
pub mod migration;
pub mod person;
pub mod settings;
pub mod cache;
pub mod events;
pub mod conversation;
pub mod queue;
pub mod token_usage;
pub mod stats;
pub mod impression;
pub mod mental_state;

// 接続関数を再エクスポート
pub(crate) use connection::connect;
pub use connection::connect_at_path;

// スキーマ初期化を再エクスポート
pub use schema::initialize_db;

// Person関連を再エクスポート
pub use person::{
    Person, add_person, update_person, delete_person, update_person_status,
    get_bot_daily_reply_counts, get_all_persons, get_person, get_random_person
};

// クレート内部用（再エクスポート不可）
// - person::insert_person
// - person::update_person_content

// システム設定を再エクスポート
pub use settings::{get_system_setting, set_system_setting, is_global_pause};

// キャッシュ関連を再エクスポート
pub use cache::{
    get_follower_cache, set_follower_cache, clear_follower_cache,
    delete_user_follower_cache, get_all_follower_cache, update_follower_cache,
    get_kind0_cache, set_kind0_cache,
    add_timeline_post, get_latest_timeline_posts, cleanup_old_timeline_posts
};

// イベント関連を再エクスポート
pub use events::{
    EventRecord, insert_event, get_event_by_event_id,
    update_event_embedding, get_events_without_embedding,
    extract_reply_to_event_id, extract_mentioned_pubkeys, extract_thread_root_id,
    detect_bot_conversation
};

// 会話ログ・要約を再エクスポート
pub use conversation::{
    insert_conversation_log, get_conversation_timeline,
    get_conversation_timeline_with_user, get_conversation_timeline_in_thread,
    get_thread_message_count, get_conversation_count_with_user,
    ConversationSummary, insert_conversation_summary, get_conversation_summaries
};

// キュー関連を再エクスポート
pub use queue::{
    enqueue_event, dequeue_event, complete_queue_event, retry_queue_event,
    get_queue_size, reset_processing_events
};

// トークン使用量を再エクスポート
pub use token_usage::{
    TokenCategory, record_token_usage, TokenUsageStats, get_token_usage_stats,
    DailyTokenUsage, get_daily_token_usage, get_daily_token_usage_with_range,
    get_daily_token_usage_by_bot, get_daily_token_usage_by_bot_with_range
};

// 統計を再エクスポート
pub use stats::{get_dashboard_stats, DashboardStats};

// ユーザー印象を再エクスポート
pub use impression::{
    get_user_impression, save_user_impression, get_user_impression_history,
    get_all_user_impressions, count_users_with_impressions, count_user_impression_history,
    UserImpressionRecord
};

// mental_stateモジュールから再エクスポート
pub use mental_state::{
    get_bot_mental_state, save_bot_mental_state, get_bot_mental_state_history,
    count_bot_mental_state_history, MentalDiary, BotMentalStateRecord
};
