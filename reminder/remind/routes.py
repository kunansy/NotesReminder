from aiogram import types

from reminder.common.bot_api import restrict_access, dp, send_msg
from reminder.common.logger import logger
from reminder.remind import db


async def remind(*,
                 user_id: int) -> None:
    note = await db.get_remind_note()
    logger.info("[%s] remind note %s", user_id, note.note_id)

    await send_msg(note.format(), chat_id=user_id)
    await db.insert_notes_history(note_id=note.note_id, user_id=user_id)


@dp.message_handler(commands=['remind'])
@restrict_access
async def remind_note(msg: types.Message) -> None:
    user_id = msg.from_user.id
    logger.debug("User id='%s' reminds a note", user_id)

    await remind(user_id=user_id)
