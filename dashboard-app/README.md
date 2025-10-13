# Nostr Bot Dashboard

React + Vite + Material-UIで構築されたモダンなダッシュボード

## 📁 プロジェクト構造

```
dashboard-app/
├── src/
│   ├── api/           # API通信ロジック
│   │   └── botApi.js
│   ├── components/    # Reactコンポーネント
│   │   ├── StatsCard.jsx
│   │   ├── BotCard.jsx
│   │   └── BotDialog.jsx
│   ├── hooks/         # カスタムフック
│   │   ├── useBots.js
│   │   └── useStats.js
│   ├── App.jsx        # メインアプリケーション
│   └── main.jsx       # エントリーポイント
├── index.html
├── vite.config.js
└── package.json
```

## 🚀 使い方

### 開発サーバー起動

```bash
cd dashboard-app
pnpm install  # 初回のみ
pnpm dev
```

開発サーバー: http://localhost:3001

### 本番ビルド

```bash
pnpm build
```

ビルド成果物は `../dashboard/` に出力されます。

## 🎨 技術スタック

- **React 19** - UIライブラリ
- **Vite** - 高速ビルドツール
- **Material-UI (MUI) v7** - UIコンポーネントライブラリ
- **Emotion** - CSS-in-JS
- **pnpm** - 高速パッケージマネージャー

## 📦 主要コンポーネント

- `StatsCard` - 統計情報カード
- `BotCard` - Bot情報カード（Material Design）
- `BotDialog` - Bot追加/編集ダイアログ

## 🔄 状態管理

- `useBots` - Bot一覧の取得・管理
- `useStats` - 統計情報の取得・管理

