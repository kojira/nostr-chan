import socket
import time
from sqlalchemy.orm import sessionmaker
from sqlalchemy import update, desc
from sqlalchemy.dialects.mysql import insert

from sqlalchemy import create_engine

from models import (
    Base,
    Person,
    User,
)

from contextlib import contextmanager

import os
from easydict import EasyDict as edict

url = 'sqlite:///nostrchan.db'
engine = create_engine(url, echo=False)
Base.metadata.create_all(bind=engine)
Session = sessionmaker(
    bind=engine, autoflush=False, autocommit=False, expire_on_commit=False
)


@contextmanager
def session_scope():
  session = Session()
  try:
    yield session
    session.commit()
  except:
    import sys
    import traceback
    from datetime import datetime

    exc_type, exc_value, exc_traceback = sys.exc_info()
    err_text = datetime.now().strftime("%Y-%m-%d %H:%M:%S") + repr(
        traceback.format_exception(exc_type, exc_value, exc_traceback)
    )
    sys.stderr.write(err_text)
    print(err_text)
    session.rollback()
    raise
  finally:
    session.close()


def addPerson(prompt, pubkey, secretkey, content):
  with session_scope() as session:
    query = session.query(Person).filter(Person.pubkey == pubkey)
    persons = query.one_or_none()
    if not persons:
      person = Person(prompt, pubkey, secretkey, content)
      session.add(person)
      session.commit()


def selectPersons():
  with session_scope() as session:
    query = session.query(Person).filter(Person.status == Person.Status.ENABLE)
    persons = query.all()
    return persons


def updatePersonStatus(pubkey, status: Person.Status):
  with session_scope() as session:
    stmt = update(Person).where(Person.ipubkeyd == pubkey).values(status=status)
    session.execute(stmt)
    session.commit()


def suspendPerson(pubkey):
  updatePersonStatus(pubkey, Person.Status.SUSPEND)


def resumePerson(pubkey):
  updatePersonStatus(pubkey, Person.Status.ENABLE)
