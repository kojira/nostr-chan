use crate::config;
use crate::db;
use nostr_sdk::prelude::*;
use std::future::Future;

mod fortune;
mod zap_ranking;
mod update_follower;
mod search_posts;
mod help;
mod search_web;
mod rag_search;

// ユーザーコマンド定義
pub struct UserCommand {
    pub name: &'static str,
    pub patterns: Vec<&'static str>,
    pub description: &'static str,
    pub detailed_help: Option<&'static str>,  // 詳細ヘルプ
    pub require_start: bool,  // コマンドが文頭にあることを要求
    pub handler: fn(config::AppConfig, db::Person, Event) -> std::pin::Pin<Box<dyn Future<Output = Result<()>> + Send>>,
}

// ユーザーコマンドテーブル
pub fn get_user_commands() -> Vec<UserCommand> {
    vec![
        UserCommand {
            name: "fortune",
            patterns: vec!["占って"],
            description: "今日の運勢を占います",
            detailed_help: Some("占い師があなたの今日の運勢を占います。\n運の良さ、ラッキーアイテム、ラッキーカラーなどを教えてくれます。"),
            require_start: false,
            handler: |c, p, e| Box::pin(fortune::fortune(c, p, e)),
        },
        UserCommand {
            name: "zap_ranking",
            patterns: vec!["zap ranking"],
            description: "過去1年分のzapランキングを表示します",
            detailed_help: Some("過去1年間に受け取ったzapの合計金額ランキングを表示します。"),
            require_start: false,
            handler: |c, p, e| Box::pin(zap_ranking::zap_ranking(c, p, e)),
        },
        UserCommand {
            name: "update_follower",
            patterns: vec!["update follower", "フォロワー更新"],
            description: "自分のフォロワーキャッシュを更新します",
            detailed_help: Some("あなたのフォロワー状態のキャッシュを強制的に更新します。\nフォローしたばかりなのに反応がない場合などに使用してください。"),
            require_start: false,
            handler: |c, p, e| Box::pin(update_follower::update_my_follower_cache(c, p, e)),
        },
        UserCommand {
            name: "search",
            patterns: vec!["search", "検索"],
            description: "投稿を検索します",
            detailed_help: Some(
"投稿を検索します。期間・投稿者指定も可能です。

【使い方】
検索 キーワード [期間] [@投稿者]

【期間指定】
・指定なし: 全期間から検索
・7d: 過去7日間
・30d: 過去30日間
・1h: 過去1時間
・24h: 過去24時間
・2024-10-01: 指定日以降
・2024-10-01 14:30: 指定日時以降（Tなしでも可）
・2024-10-01~2024-10-31: 期間範囲
・2024-10-01 14:30~2024-10-31 18:00: 日時範囲
・2024-10-01~: 指定日以降
・~2024-10-31: 指定日以前

【投稿者指定】
・@npub1...: npub形式
・@hex...: hex形式（64文字）

※ ~ の代わりに 〜 も使用可能
※ 期間と投稿者は順不同

【例】
検索 Nostr
検索 Nostr 7d
検索 Nostr 2024-10-01
検索 Nostr 2024-10-01 14:30
検索 Nostr 2024-10-01~2024-10-31
検索 Nostr @npub1...
検索 Nostr 7d @npub1...
検索 Nostr 2024-10-01 14:30〜2024-10-31 18:00 @npub1..."
            ),
            require_start: true,  // 文頭必須
            handler: |c, p, e| Box::pin(search_posts::search_posts(c, p, e)),
        },
        UserCommand {
            name: "search_web",
            patterns: vec!["調べて"],
            description: "Web検索を行います（Gemini CLI使用）",
            detailed_help: Some("Gemini CLIを使ってWeb検索を行い、結果を要約して返答します。\n\n【使い方】\n調べて [検索したい内容]\n\n【例】\n調べて Rustの最新バージョン\n調べて 今日の天気"),
            require_start: false,
            handler: |c, p, e| Box::pin(search_web::search_web(c, p, e)),
        },
        UserCommand {
            name: "rag",
            patterns: vec!["rag", "意味検索"],
            description: "AIによる意味検索（ベクトル類似度）",
            detailed_help: Some(
"AIによる意味検索を行います。キーワード一致ではなく、意味的に似ている投稿を探します。

【使い方】
rag [検索したい内容]
意味検索 [検索したい内容]

【特徴】
・ベクトル類似度による意味的な検索
・キーワードが完全一致しなくても類似する内容を発見
・日本語投稿のみが対象
・上位5件を類似度スコア付きで表示

【例】
rag Nostrの使い方
意味検索 プログラミングの学び方
rag AIとの付き合い方"
            ),
            require_start: true,  // 文頭必須
            handler: |c, p, e| Box::pin(rag_search::rag_search(c, p, e)),
        },
        UserCommand {
            name: "help",
            patterns: vec!["help", "ヘルプ"],
            description: "利用可能なコマンド一覧を表示します",
            detailed_help: Some("利用可能なコマンドの一覧を表示します。\n\n【使い方】\nhelp: 全コマンド一覧\nhelp コマンド名: 特定コマンドの詳細ヘルプ\n\n【例】\nhelp\nhelp search\nhelp rag"),
            require_start: false,
            handler: |c, p, e| Box::pin(help::show_help(c, p, e)),
        },
    ]
}

// ========== ユーザーコマンド実装 ==========
// 全てのコマンドは個別ファイルに分離されています