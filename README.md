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
BOT_PUBLICKEY=replace bot public hex key
BOT_SECRETKEY=replace bot secret key (not hex)
PROMPTS=describe the bot personality (commna separated)
ANSER_LENGTH=100
REACTION_FREQ=600

BLACKLIST=blacklist pubkey
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
  - "wss://relay-jp.nostr.wirednet.jp"
  - "wss://relay.damus.io"
  - "wss://relay.nostr.wirednet.jp"
  - "wss://nostr.h3z.jp"
  - "wss://relay.snort.social"
  - "wss://nostr-pub.wellorder.net"
  - "wss://relay.current.fyi"
  - "wss://nos.lol"
```
