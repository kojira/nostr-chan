import openai
from dotenv import load_dotenv
import os
import traceback
import yaml

dotenv_path = os.path.join(os.path.dirname(__file__), '.env')
load_dotenv(dotenv_path)

openai.api_key = os.environ.get("OPEN_AI_API_KEY")

with open("./config.yml", "r") as yml:
  config = yaml.safe_load(yml)

answer_length = int(config["answer_length"])


def get_answer(prompt, text):
  print(prompt)
  prompt = f"これはあなたの人格です。'{prompt}'\nこの人格を演じて次の文章に対して{answer_length}文字程度で返信してください。"
  answer = None
  try:
    response = openai.ChatCompletion.create(
        model="gpt-3.5-turbo",
        messages=[
            {"role": "system", "content": f"{prompt}"},
            {"role": "user", "content": f"{text}"},
        ],
        presence_penalty=-0.5,
        frequency_penalty=-0,
        top_p=0.9,
        timeout=30
    )
    answer = response["choices"][0]["message"]["content"]

  except Exception as e:
    trace = traceback.format_exc()
    print(trace)

  return answer
