use crate::config;
use crate::database as db;
use crate::util;
use nostr_sdk::prelude::*;

// ヘルプコマンド
pub async fn show_help(config: config::AppConfig, person: db::Person, event: Event) -> Result<()> {
    let admin_pubkeys = &config.bot.admin_pubkeys;
    let is_admin = admin_pubkeys.iter().any(|s| *s == event.pubkey.to_string());
    
    // コマンド引数を抽出（help の後に特定コマンド名があるか）
    let content = event.content.clone();
    let help_arg = if content.contains("help ") {
        content.split("help ").nth(1).map(|s| s.trim())
    } else if content.contains("ヘルプ ") {
        content.split("ヘルプ ").nth(1).map(|s| s.trim())
    } else {
        None
    };
    
    // 特定コマンドの詳細ヘルプを表示
    if let Some(cmd_name) = help_arg {
        if !cmd_name.is_empty() {
            // ユーザーコマンドから検索
            for cmd in super::get_user_commands() {
                if cmd.name == cmd_name || cmd.patterns.iter().any(|p| *p == cmd_name) {
                    let mut reply = format!("【{}】\n\n", cmd.patterns.join(" / "));
                    if let Some(detailed) = cmd.detailed_help {
                        reply.push_str(detailed);
                    } else {
                        reply.push_str(cmd.description);
                    }
                    util::reply_to(&config, event, person, &reply).await?;
                    return Ok(());
                }
            }
            
            // 管理者コマンドから検索（管理者のみ）
            if is_admin {
                for cmd in super::super::admin::get_admin_commands() {
                    if cmd.name == cmd_name || cmd.pattern == cmd_name {
                        let reply = format!("【{}】\n\n{}", cmd.pattern, cmd.description);
                        util::reply_to(&config, event, person, &reply).await?;
                        return Ok(());
                    }
                }
                for cmd in super::super::admin::get_admin_commands_simple() {
                    if cmd.name == cmd_name || cmd.pattern == cmd_name {
                        let reply = format!("【{}】\n\n{}", cmd.pattern, cmd.description);
                        util::reply_to(&config, event, person, &reply).await?;
                        return Ok(());
                    }
                }
            }
            
            // コマンドが見つからない場合
            util::reply_to(&config, event, person, &format!("コマンド「{}」が見つかりません。\n「help」で全コマンド一覧を表示します。", cmd_name)).await?;
            return Ok(());
        }
    }
    
    // 全コマンド一覧を表示
    let mut reply = "【利用可能なコマンド】\n\n".to_string();
    reply.push_str("詳細は「help コマンド名」で確認できます。\n\n");
    
    // ユーザーコマンド
    reply.push_str("■ ユーザーコマンド\n");
    for cmd in super::get_user_commands() {
        reply.push_str(&format!("・{}\n  {}\n", cmd.patterns.join(" / "), cmd.description));
    }
    
    // 管理者コマンド（管理者のみ）
    if is_admin {
        reply.push_str("\n■ 管理者コマンド\n");
        for cmd in super::super::admin::get_admin_commands() {
            reply.push_str(&format!("・{}\n  {}\n", cmd.pattern, cmd.description));
        }
        for cmd in super::super::admin::get_admin_commands_simple() {
            reply.push_str(&format!("・{}\n  {}\n", cmd.pattern, cmd.description));
        }
    }
    
    util::reply_to(&config, event, person, &reply).await?;
    Ok(())
}
