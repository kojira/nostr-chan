use crate::{config, database as db};

/// システム設定をconfig.ymlの値で初期化（DBに値がない場合のみ）
pub fn initialize_system_settings(conn: &rusqlite::Connection, config: &config::AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    // フォロワーキャッシュTTL
    if db::get_system_setting(conn, "follower_cache_ttl")?.is_none() {
        db::set_system_setting(conn, "follower_cache_ttl", &config.bot.follower_cache_ttl.to_string())?;
        println!("⚙️ フォロワーキャッシュTTL: {}秒", config.bot.follower_cache_ttl);
    }
    
    // Bot設定
    if db::get_system_setting(conn, "reaction_percent")?.is_none() {
        db::set_system_setting(conn, "reaction_percent", &config.bot.reaction_percent.to_string())?;
        println!("⚙️ リアクション確率: {}%", config.bot.reaction_percent);
    }
    if db::get_system_setting(conn, "reaction_freq")?.is_none() {
        db::set_system_setting(conn, "reaction_freq", &config.bot.reaction_freq.to_string())?;
        println!("⚙️ リアクション頻度: {}秒", config.bot.reaction_freq);
    }
    if db::get_system_setting(conn, "timeline_size")?.is_none() {
        db::set_system_setting(conn, "timeline_size", &config.bot.timeline_size.to_string())?;
        println!("⚙️ タイムラインサイズ: {}", config.bot.timeline_size);
    }
    
    // 会話制限設定
    if db::get_system_setting(conn, "conversation_limit_count")?.is_none() {
        db::set_system_setting(conn, "conversation_limit_count", &config.bot.conversation_limit_count.to_string())?;
        println!("⚙️ 会話制限回数: {}回", config.bot.conversation_limit_count);
    }
    if db::get_system_setting(conn, "conversation_limit_minutes")?.is_none() {
        db::set_system_setting(conn, "conversation_limit_minutes", &config.bot.conversation_limit_minutes.to_string())?;
        println!("⚙️ 会話制限時間: {}分", config.bot.conversation_limit_minutes);
    }
    
    // GPT設定
    if db::get_system_setting(conn, "gpt_answer_length")?.is_none() {
        db::set_system_setting(conn, "gpt_answer_length", &config.gpt.answer_length.to_string())?;
        println!("⚙️ GPT回答長: {}文字", config.gpt.answer_length);
    }
    if db::get_system_setting(conn, "gpt_timeout")?.is_none() {
        db::set_system_setting(conn, "gpt_timeout", &config.gpt.timeout.to_string())?;
        println!("⚙️ GPTタイムアウト: {}秒", config.gpt.timeout);
    }
    if db::get_system_setting(conn, "gemini_search_timeout")?.is_none() {
        db::set_system_setting(conn, "gemini_search_timeout", &config.gpt.gemini_search_timeout.to_string())?;
        println!("⚙️ Gemini Searchタイムアウト: {}秒", config.gpt.gemini_search_timeout);
    }
    if db::get_system_setting(conn, "recent_context_count")?.is_none() {
        db::set_system_setting(conn, "recent_context_count", &config.gpt.recent_context_count.to_string())?;
        println!("⚙️ 最近のやり取り件数: {}件", config.gpt.recent_context_count);
    }
    if db::get_system_setting(conn, "summary_threshold")?.is_none() {
        db::set_system_setting(conn, "summary_threshold", &config.gpt.summary_threshold.to_string())?;
        println!("⚙️ 要約開始閾値: {}文字", config.gpt.summary_threshold);
    }
    if db::get_system_setting(conn, "max_summary_tokens")?.is_none() {
        db::set_system_setting(conn, "max_summary_tokens", &config.gpt.max_summary_tokens.to_string())?;
        println!("⚙️ 要約最大トークン数: {}トークン", config.gpt.max_summary_tokens);
    }
    
    // リレー設定
    if db::get_system_setting(conn, "relay_write")?.is_none() {
        let write_relays = config.relay_servers.write.join(",");
        db::set_system_setting(conn, "relay_write", &write_relays)?;
        println!("⚙️ 書き込みリレー: {}", write_relays);
    }
    if db::get_system_setting(conn, "relay_read")?.is_none() {
        let read_relays = config.relay_servers.read.join(",");
        db::set_system_setting(conn, "relay_read", &read_relays)?;
        println!("⚙️ 読み込みリレー: {}", read_relays);
    }
    if db::get_system_setting(conn, "relay_search")?.is_none() {
        let search_relays = config.relay_servers.search.join(",");
        db::set_system_setting(conn, "relay_search", &search_relays)?;
        println!("⚙️ 検索リレー: {}", search_relays);
    }
    
    // ブラックリスト
    if db::get_system_setting(conn, "blacklist")?.is_none() {
        let blacklist = config.bot.blacklist.join(",");
        db::set_system_setting(conn, "blacklist", &blacklist)?;
        println!("⚙️ ブラックリスト: {}件", config.bot.blacklist.len());
    }
    
    Ok(())
}
