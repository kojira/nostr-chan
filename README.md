# Nostr chan

Nostr-chan is a bot that lives on Nostr.

## startup

```sh
git clone https://github.com/kojira/nostr-chan.git
cd nostr-chan
cp .env.example .env
```

Change the contents of `.env` according to the environment.
```
OPEN_AI_API_KEY=replace your open ai key
BOT_SECRETKEY=replace bot secret hex key
```

Run with this command

```sh
docker compose up -d
```

## relay setting

Please rewrite as appropriate

`common/config.yml`

Please rewrite as appropriate

```yml
relay_servers:
  write:
    - "wss://nostr-relay.nokotaro.com"
    - "wss://relay-jp.nostr.wirednet.jp"
    - "wss://relay.nostr.wirednet.jp"
    - "wss://relay.damus.io"
    - "wss://yabu.me"
    - "wss://nos.lol"
    - "wss://r.kojira.io"
    - "wss://relay.nostr.band"
  read:
    - "wss://yabu.me"
    - "wss://r.kojira.io"

bot:
  admin_pubkeys:
    - admin hex pubkey
  root_bot_pubkey: root bot hex pubkey
  prompt: あなたはNostrというプロトコルを利用して作られた場所で、先鋭化した人物が集うインターネット上のコミュニティのアイドルです。17歳の元気な女の子で名前はNostrちゃんです。あたまに⚡が刺さっています。
  picture: https://nostr.kojira.net/nostrchan.webp
  about: kojira(npub1k0jrarx8um0lyw3nmysn50539ky4k8p7gfgzgrsvn8d7lccx3d0s38dczd) が管理するNostrアイドルだよ！フォロワーにだけたまにお返事するよ！
  reaction_percent: 5
  reaction_freq: 600
  blacklist:
    - blacklist hex pubkey

gpt:
  answer_length: 100
  timeout: 30
```

# commands

|command|type|description|example|
|---|---|---|---|
|占って|user|占いを行う||
|zap ranking|user|zapの受信ランキングを集計する||
|new|admin|新たにbotをspawnさせる。２行目がprompt、３行目がkind 0のcontentに使うjson文字列|あなたはNostr集落で流行った「かす」という言葉の言い換えの「ﾌﾞﾙﾙｯﾁﾓ」という単語のミームが擬人化された存在です。以下は発言のサンプルです。\n〜中略〜ﾌﾞﾙﾙﾁﾓは大好物のおやつをもってくるﾁﾓ🍭\nﾌﾞﾙﾙﾁﾓの笑顔は世界を救うﾁﾓ！🌈<br>{"name":"brrcm","display_name":"ﾌﾞﾙﾙｯﾁﾓ","about":"ﾌﾞﾙﾙｯﾁﾓ!!\nbot管理人:kojira(@kojira)","lud16":"totalnest47@walletofsatoshi.com","nip05":"brrcm@kojira.io","nip05valid":true,"picture":"https://kojira.io/brrcm.webp"}|
|get kind 0|admin|メンションしたbotのkind 0をリレーから取得してDBに保存する||
|update kind 0|admin|メンションしたbotのkind 0を２行目のjson文字列を使って更新する||
|broadcast kind 0|admin|メンションしたbotのkind 0をDBから読み込んでブロードキャストする||
