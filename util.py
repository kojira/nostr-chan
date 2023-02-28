
import re
from nostr import bech32


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
