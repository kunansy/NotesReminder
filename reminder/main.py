import argparse
import asyncio

from aiogram import types
from aiogram.utils import executor

from reminder.common import settings
from reminder.common.bot_api import restrict_access, bot, dp, send_msg
from reminder.common.logger import logger
from reminder.remind import routes as remind_routes


async def set_default_commands() -> None:
    await bot.set_my_commands([
        types.BotCommand("start", "Start the bot"),
        types.BotCommand("remind", "Show a new note"),
    ])


async def on_startup(*args) -> None:
    await set_default_commands()
    logger.info("Bot started")


@dp.message_handler(commands=['start'])
@restrict_access
async def start_bot(msg: types.Message) -> None:
    logger.debug("User id='%s' started the bot", msg.from_user.id)

    commands_text = '\n'.join(
        f"{command.description}: /{command.command}"
        for command in await bot.get_my_commands()
    )
    funcs_description = f"This is what I can: \n\n{commands_text}"

    await send_msg(funcs_description)


def args_parser() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Start Telegram bot or send a message with it"
    )
    parser.add_argument(
        '--start',
        action="store_true",
        default=False,
        dest='start',
    )
    parser.add_argument(
        '--remind',
        action="store_true",
        default=False,
        dest='start',
        help="Send a new note to remind"
    )

    return parser.parse_args()


def main() -> None:
    args = args_parser()

    if args.start:
        executor.start_polling(dp)
    if args.remind:
        asyncio.run(remind_routes.remind(user_id=settings.TG_BOT_USER_ID))


if __name__ == '__main__':
    main()
