import json
import time
from datetime import datetime, timedelta

from nostr.filter import Filter, Filters
from nostr.event import Event, EventKind
from nostr.relay_manager import RelayManager
from nostr.key import PrivateKey
import yaml
import util
import gpt
from dotenv import load_dotenv
import os
import random

from langdetect import detect

dotenv_path = os.path.join(os.path.dirname(__file__), '.env')
load_dotenv(dotenv_path)

BOT_PUBLICKEY = os.environ.get("BOT_PUBLICKEY")
BOT_SECRETKEY = os.environ.get("BOT_SECRETKEY")
REACTION_FREQ = int(os.environ.get("REACTION_FREQ"))
BLACKLIST = os.environ.get("BLACKLIST").split(",")

with open("./config.yml", "r") as yml:
  config = yaml.safe_load(yml)

subscription_id = "nostr-chan"

today = datetime.now()
start_timestamp = today.timestamp()

follower_list = []


def connect_relay():
  today = datetime.now()
  before_min = today - timedelta(minutes=20)
  since = before_min.timestamp()

  filters = Filters(
      [
          Filter(kinds=[EventKind.TEXT_NOTE], since=since),
      ]
  )
  relay_manager = RelayManager()
  add_all_relay(relay_manager, config["relay_servers"])

  relay_manager.add_subscription_on_all_relays(subscription_id, filters)
  time.sleep(1.25)

  return relay_manager


def add_all_relay(relay_manager, relay_servers):
  for relay_server in relay_servers:
    relay_manager.add_relay(relay_server)


def close_relay(relay_manager):
  relay_manager.close_all_relay_connections()


def reconnect_all_relay(relay_manager):
  print("reconnect_all_relay start")
  close_relay(relay_manager)
  time.sleep(2)
  relay_manager = connect_relay()
  time.sleep(2)
  print("reconnect_all_relay done")
  return relay_manager


def is_follower(bot_pubkey, pubkey):
  if pubkey in follower_list:
    return True

  filters = Filters([
      Filter(authors=[pubkey], kinds=[EventKind.CONTACTS], limit=1),
  ])
  subscription_id = "nostr-chan-get-kind3"

  relay_manager = RelayManager()
  for relay_server in config["relay_servers"]:
    relay_manager.add_relay(relay_server)

  relay_manager.add_subscription_on_all_relays(subscription_id, filters)
  time.sleep(1.25)

  event_list = []
  give_up_count = 0
  while True:
    while relay_manager.message_pool.has_events():
      event_msg = relay_manager.message_pool.get_event()
      event_list.append(event_msg.event)
    if len(event_list) > 0 or give_up_count > 10:
      break
    time.sleep(1)
    give_up_count += 1

  relay_manager.close_all_relay_connections()

  follower = False
  if len(event_list) > 0:
    kind3_list = sorted(event_list, key=lambda x: x.created_at, reverse=True)
    for tag in kind3_list[0].tags:
      if len(tag) >= 2 and tag[0] == 'p' and tag[1] == bot_pubkey:
        follower = True
        follower_list.append(pubkey)
        break
  return follower


relay_manager = connect_relay()

no_event_count = 0

privatekey = PrivateKey.from_nsec(BOT_SECRETKEY)

posted_timestamp = start_timestamp

while True:
  events = 0
  while relay_manager.message_pool.has_events():
    events += 1
    event_msg = relay_manager.message_pool.get_event()

    tag_json = json.dumps(event_msg.event.tags)
    event_datetime = datetime.fromtimestamp(event_msg.event.created_at)
    texts = [
        datetime.fromtimestamp(event_msg.event.created_at).strftime(
            "%Y/%m/%d %H:%M:%S"
        ),
        util.get_note_id(event_msg.event.id),
        event_msg.event.public_key,
        str(event_msg.event.kind),
        event_msg.event.content,
        event_msg.event.signature,
    ]
    if event_msg.event.created_at > posted_timestamp:
      print("\n".join(texts))
      print(event_msg.event.tags)
      if event_msg.event.public_key not in BLACKLIST:
        if 10 <= len(event_msg.event.content) <= 140:
          try:
            lang = detect(event_msg.event.content)
          except:
            lang = ""

          if lang == "ja":
            print("lang:", lang)
            post = False
            if event_msg.event.created_at > (posted_timestamp + REACTION_FREQ):
              post = True
            elif random.uniform(0, 100) <= 5:
              post = True
            print("post:", post)
            if post:
              result = is_follower(BOT_PUBLICKEY, event_msg.event.public_key)
              print("is_follower:", result)
              if result:
                text = event_msg.event.content
                answer = gpt.get_answer(text)
                print("answer:", answer)
                event = Event(content=answer)
                event.add_event_ref(event_msg.event.id)
                event.add_pubkey_ref(event_msg.event.public_key)
                privatekey.sign_event(event)
                relay_manager.publish_event(event)
                now = datetime.now()
                posted_timestamp = now.timestamp()

  if events == 0:
    no_event_count += 1
    if no_event_count % 100 == 0:
      print("no events", no_event_count)
  else:
    no_event_count = 0

  if no_event_count >= 300:
    relay_manager = reconnect_all_relay(relay_manager)
    no_event_count = 0
    now = datetime.now()
    posted_timestamp = now.timestamp()
  time.sleep(1)
