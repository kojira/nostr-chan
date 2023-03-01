import openai
from dotenv import load_dotenv
import os
import traceback
import random

dotenv_path = os.path.join(os.path.dirname(__file__), '.env')
load_dotenv(dotenv_path)

OPEN_AI_API_KEY = os.environ.get("OPEN_AI_API_KEY")
PROMPTS = os.environ.get("PROMPTS").split(",")
ANSER_LENGTH = os.environ.get("ANSER_LENGTH")

openai.api_key = OPEN_AI_API_KEY


def get_answer(text):
  prompt = random.choice(PROMPTS)
  index = PROMPTS.index(prompt)
  print(prompt)
  prompt += f"次の文章に対して{ANSER_LENGTH}文字程度で返信してください。"
  answer = None
  try:
    response = openai.Completion.create(
        engine="text-davinci-003",
        prompt=f'{prompt}\n"' + text + '"',
        max_tokens=700,
        n=1,
        stop=None,
        temperature=0.8,
        seed=100,
        frequency_penalty=0.1,
    )
    answer = response["choices"][0]["text"].strip()
  except Exception as e:
    trace = traceback.format_exc()
    print(trace)

  return f"({index})" + answer
