
import re
from nostr import bech32
from nostr.event import Event, EventKind
from nostr.key import PublicKey
import json


def get_note_id(_id):
  converted_bits = bech32.convertbits(bytes.fromhex(_id), 8, 5)
  return bech32.bech32_encode("note", converted_bits, bech32.Encoding.BECH32)


def get_images_urls(content):
  replaced_content = content
  pattern = re.compile(r'https?://\S+\.(?:jpg|jpeg|png|gif|webp)')
  image_urls = re.findall(pattern, content)
  for image_url in image_urls:
    replaced_content = replaced_content.replace(image_url, '')

  return image_urls, replaced_content


def get_pubkey_hex_from_event(event: Event):
  pubkey_hex = None
  if len(event.tags) and len(event.tags[0]) == 4:
    if event.tags[0][0] == "p" and event.tags[0][3] == "mention":
      pubkey_hex = event.tags[0][1]

  return pubkey_hex


def set_user_meta(privatekey, relay_manager, content):
  event = Event(content=content)
  event.kind = EventKind.SET_METADATA
  privatekey.sign_event(event)
  relay_manager.publish_event(event)


def post_event(privatekey, relay_manager, text, ref_event_id=None, ref_event_public_key=None):
  event = Event(content=text)
  if ref_event_id and ref_event_public_key:
    event.add_event_ref(ref_event_id)
    event.add_pubkey_ref(ref_event_public_key)
  privatekey.sign_event(event)
  relay_manager.publish_event(event)
