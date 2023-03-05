
import yaml
from nostr.key import PrivateKey
from nostr.event import Event
import db
import json
import util

with open("./config.yml", "r") as yml:
  config = yaml.safe_load(yml)


def commandHandler(relay_manager, event: Event):
  def createPerson(relay_manager, prompt, parsed_params):
    privateKey = PrivateKey()
    privateKeyHex = privateKey.hex()
    pubKeyHex = privateKey.public_key.hex()
    content = json.dumps(parsed_params)
    db.addPerson(prompt, pubKeyHex, privateKeyHex, content)
    util.set_user_meta(privateKey, relay_manager, content)
    text = f"はじめまして。\n{parsed_params['display_name']}です。\nコンゴトモヨロシク！"
    util.post_event(privateKey, relay_manager, text, event.id, event.public_key)

  commands = [
      {"command": "!new", "admin": True,
       "handler": lambda relay_manager, prompt, parsed_params:
       createPerson(relay_manager, prompt, parsed_params)},
      # {"command": "!update", "admin": True},
      # {"command": "!silent", "admin": False},
      # {"command": "!hello", "admin": False},
  ]

  mention_pubkey = util.get_pubkey_hex_from_event(event)
  if mention_pubkey is None:
    return False

  persons = db.selectPersons()

  detect_person = False
  for person in persons:
    if person.pubkey == mention_pubkey:
      detect_person = True

  text = event.content.replace("#[0]", "").lstrip()

  for command in commands:
    if text.startswith(command["command"]):
      if command["admin"]:
        if event.public_key in config["admin_pubkeys"]:
          if mention_pubkey == config["root_bot_pubkey"]:
            params = text.split("\n")
            parsed_params = {}
            prompt = ""
            for param in params:
              cols = param.split("=")
              if len(cols) == 2:
                if cols[0] == "prompt":
                  prompt = cols[1]
                else:
                  parsed_params[cols[0]] = cols[1]
            command["handler"](relay_manager, prompt, parsed_params)
            return True
      else:
        if not detect_person:
          return False
        else:
          return False
