import os
from pathlib import Path

from environs import Env


env = Env()
env.read_env()

DSN_TEMPLATE = "postgresql+asyncpg://{username}:{password}" \
               "@{host}:{port}/{db_name}"

API_VERSION = '0.1.0'
if (version_file := Path('VERSION')).exists():
    API_VERSION = version_file.read_text().strip()

with env.prefixed('TG_BOT_'):
    TG_BOT_TOKEN = env('TOKEN')
    TG_BOT_USER_ID = env.int('USER_ID')

with env.prefixed("DB_"):
    DB_HOST = env("HOST")
    DB_PORT = env.int("PORT")
    DB_NAME = env("NAME")
    DB_USERNAME = env("USERNAME")
    DB_PASSWORD = env("PASSWORD")

    DB_TIMEOUT = env.int('TIMEOUT', 5)

DB_URI = DSN_TEMPLATE.format(
    username=DB_USERNAME,
    password=DB_PASSWORD,
    host=DB_HOST,
    port=DB_PORT,
    db_name=DB_NAME
)

with env.prefixed("LOGGER_"):
    LOGGER_NAME = env("NAME", "NotesReminder")
    LOGGER_LEVEL = env.log_level("LEVEL", 'debug')

os.environ.clear()
