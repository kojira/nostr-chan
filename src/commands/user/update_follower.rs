use crate::config;
use crate::db;
use crate::util;
use nostr_sdk::prelude::*;

// フォロワーキャッシュ更新コマンド
pub async fn update_my_follower_cache(config: config::AppConfig, person: db::Person, event: Event) -> Result<()> {
    let user_pubkey = event.pubkey.to_string();
    let conn = db::connect()?;
    
    // キャッシュを削除
    let deleted = db::delete_user_follower_cache(&conn, &user_pubkey, &person.pubkey)?;
    
    // 新しくフォロワー状態を取得（キャッシュに保存される）
    let is_follower = util::is_follower(&user_pubkey, &person.secretkey).await?;
    
    let reply = format!(
        "フォロワーキャッシュを更新しました。\n削除: {}件\n現在のステータス: {}",
        deleted,
        if is_follower { "フォロワー" } else { "非フォロワー" }
    );
    
    util::reply_to(&config, event, person, &reply).await?;
    Ok(())
}
