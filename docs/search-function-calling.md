# 検索機能のFunction Calling実装計画

## 概要
自然言語での検索質問に対して、LLMがFunction Callingで検索を実行し、結果を整形して返答する機能を実装する。

## 要件

### ユーザー体験
```
ユーザー: 「検索 去年の8月くらいのxxx関連の話題なんだっけ？」
    ↓
Bot: 「2024年8月にxxxについて〇〇さんが投稿していました：
      『投稿内容...』
      詳細: nostr:note1abc...」
```

### 動作フロー
1. ユーザーが「検索」で始まる文章を入力
2. LLMがFunction Callingで検索パラメータを決定
   - キーワード抽出
   - 期間解析（相対日付→絶対日付）
   - 著者指定（あれば）
3. 検索実行（二段階）
   - Phase 1: ローカルDB（events）検索
   - Phase 2: リレー検索（見つからない場合のみ）
4. LLMが結果を自然な文章で整形
   - 投稿内容の要約
   - `nostr:note1...` リンク付与

## 実装計画

### 1. 調査・準備
- [x] `openai-api-rs` 7.0.0のFunction Calling対応確認
- [ ] Function Calling実装例の調査
- [ ] 既存の`search_posts`コマンドとの統合方法検討

### 2. 検索関数の実装
```rust
// Function定義
search_events {
  keywords: Vec<String>,        // 検索キーワード
  start_date: Option<String>,   // "2024-08-01"
  end_date: Option<String>,     // "2024-08-31"
  author: Option<String>,       // npub or hex
  limit: usize,                 // デフォルト10
  source: "local" | "relay"     // 検索ソース
}
```

#### 実装ステップ
1. ローカルDB検索関数の実装
   - `search_events_local(keywords, start_date, end_date, author, limit)`
   - 既存の`search_posts`ロジックを活用
2. リレー検索関数の実装
   - `search_events_relay(keywords, start_date, end_date, author, limit)`
   - タイムアウト設定（10秒程度）
3. 統合検索関数
   - ローカル優先、見つからなければリレー

### 3. GPT統合

#### システムプロンプトの拡張
```
現在日時: 2025-10-19 (日) 15:30 JST

利用可能な関数:
- search_events: Nostrの投稿を検索します
  - keywords: 検索キーワード（配列）
  - start_date: 開始日（YYYY-MM-DD形式、省略可）
  - end_date: 終了日（YYYY-MM-DD形式、省略可）
  - author: 投稿者（npub/hex、省略可）
  - limit: 取得件数（デフォルト10）
```

#### Function Calling実装
1. `gpt.rs`に新しい関数追加
   - `call_gpt_with_tools()`: Tools APIを使用
2. Function Call受信・実行
   - JSON解析
   - 検索実行
   - 結果をGPTに返す
3. 最終応答生成

### 4. トリガー条件
- 文章が「検索」で始まる場合のみFunction Callingモードを使用
- それ以外は通常の会話モード
- コスト削減とパフォーマンス向上

### 5. エラーハンドリング
- リレー接続失敗 → エラー内容を投稿
- タイムアウト → 「リレーからの取得に時間がかかっています」
- 検索結果0件 → 「該当する投稿が見つかりませんでした」
- Function Call解析エラー → エラー内容を投稿

## 技術的考慮点

### 1. openai-api-rs対応状況
- バージョン: 7.0.0
- Function Calling（Tools API）対応: 要確認
- 代替案: `async-openai`クレートへの移行

### 2. 日付解析
- 現在日時をシステムプロンプトに明示
- 相対日付の例:
  - 「去年の8月」→ 2024-08-01 ~ 2024-08-31
  - 「先月」→ 2025-09-01 ~ 2025-09-30
  - 「3日前」→ 2025-10-16

### 3. パフォーマンス
- ローカルDB検索: 高速（<100ms）
- リレー検索: 遅い可能性（1-10秒）
- タイムアウト設定が重要

### 4. コスト
- Function Calling使用時のトークン消費増加
- 「検索」トリガーで限定的に使用

## 既存機能との関係

### search_postsコマンド
- 現在: 明示的な検索コマンド
- 今後: Function Callingと共存
  - 明示的検索: `search_posts`（高速、確実）
  - 自然言語検索: Function Calling（柔軟、会話的）

## 実装優先順位

### Phase 1: 基本実装
1. openai-api-rsのFunction Calling対応確認
2. ローカルDB検索のみ実装
3. 簡単な日付解析（絶対日付のみ）

### Phase 2: 拡張
1. リレー検索追加
2. 相対日付解析の改善
3. エラーハンドリングの強化

### Phase 3: 最適化
1. キャッシング
2. パフォーマンス改善
3. コスト最適化

## 次のステップ
1. openai-api-rsのFunction Calling実装例を調査
2. 小さなプロトタイプで動作確認
3. 段階的に機能拡張

