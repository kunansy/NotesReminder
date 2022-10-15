import asyncio
from functools import wraps
from typing import Callable

from aiogram import Bot, Dispatcher, types
from aiogram.utils import exceptions

from reminder.common import settings
from reminder.common.logger import logger


bot = Bot(settings.TG_BOT_TOKEN)
dp = Dispatcher(bot)


class TelegramException(Exception):
    pass


async def _send_msg(*,
                    msg: str,
                    chat_id: int | None = None) -> None:
    chat_id = chat_id or types.User.get_current().id

    try:
        await bot.send_message(
            chat_id, msg,
            disable_web_page_preview=True,
        )
    except exceptions.RetryAfter as e:
        logger.error("[%s]: Flood limit is exceeded. Sleep %ss.",
                     chat_id, e.timeout)
        await asyncio.sleep(e.timeout)
        return await send_msg(msg, chat_id=chat_id)
    except exceptions.TelegramAPIError as e:
        logger.error("[%s]: Error '%s'", chat_id, repr(e))
        raise TelegramException(e) from e
    else:
        logger.debug("[%s]: success", chat_id)


async def send_msg(msg: str,
                   chat_id: int | None = None) -> None:
    try:
        await _send_msg(msg=msg, chat_id=chat_id)
    except exceptions.BotBlocked:
        logger.error("[%s]: Bot blocked", chat_id)
    except exceptions.ChatNotFound:
        logger.error("[%s]: Chat not found", chat_id)
    except exceptions.UserDeactivated:
        logger.error("[%s]: user is deactivated", chat_id)
    except exceptions.TelegramAPIError:
        logger.exception("[%s]: failed", chat_id)


def restrict_access(func: Callable) -> Callable:
    @wraps(func)
    async def wrapped(msg: types.Message, *args, **kwargs):
        if (user_id := msg.from_user.id) not in settings.TG_BOT_USER_IDS:
            logger.warning("Access for user id='%s' declined", user_id)
            raise TelegramException(f"Access for user {user_id=} declined")

        return await func(msg, *args, **kwargs)

    return wrapped
