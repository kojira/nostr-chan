from sqlalchemy import Column, Integer, Text, DateTime, text
from sqlalchemy.ext.declarative import declarative_base
from datetime import datetime
from sqlalchemy.sql import func
from enum import IntEnum


Base = declarative_base()


class Person(Base):
  class Status(IntEnum):
    ENABLE = 0
    SUSPEND = 1
    DELETED = -1

    def __int__(self):
      return self.value

  __tablename__ = "Persons"
  id = Column(Integer, autoincrement=True, primary_key=True)
  status = Column(Integer)
  prompt = Column(Text)
  pubkey = Column(Text)
  secretkey = Column(Text)
  content = Column(Text)
  created_at = Column(DateTime, default=datetime.utcnow)
  updated_at = Column(
      DateTime,
      server_default=func.now(),
      server_onupdate=text('CURRENT_TIMESTAMP')
  )

  def __init__(self, prompt: str, pubkey: str, secretkey: str, content: str):
    self.status = 0
    self.prompt = prompt
    self.pubkey = pubkey
    self.secretkey = secretkey
    self.content = content


class User(Base):
  class Status(IntEnum):
    ENABLE = 0
    BLOCKED = 403

    def __int__(self):
      return self.value

  __tablename__ = "Users"
  id = Column(Integer, autoincrement=True, primary_key=True)
  status = Column(Integer)
  pubkey = Column(Text)
  name = Column(Text)
  display_name = Column(Text)
  created_at = Column(DateTime, default=datetime.utcnow)
  updated_at = Column(
      DateTime,
      server_default=func.now(),
      server_onupdate=text('CURRENT_TIMESTAMP')
  )

  def __init__(self, pubkey: str, name: str, display_name: str):
    self.pubkey = pubkey
    self.name = name
    self.display_name = display_name
