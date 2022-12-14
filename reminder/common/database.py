from contextlib import asynccontextmanager
from typing import AsyncGenerator

import sqlalchemy.sql as sa
from sqlalchemy.ext.asyncio import AsyncSession, create_async_engine

from reminder.common import settings
from reminder.common.logger import logger


class DatabaseException(Exception):
    pass


engine = create_async_engine(
    settings.DB_URI,
    encoding='utf-8',
    connect_args={'timeout': settings.DB_TIMEOUT}
)


@asynccontextmanager
async def session(**kwargs) -> AsyncGenerator[AsyncSession, None]:
    new_session = AsyncSession(**kwargs, bind=engine)
    try:
        yield new_session
        await new_session.commit()
    except Exception as e:
        logger.exception("Error with the session")

        await new_session.rollback()
        raise DatabaseException(e)
    finally:
        await new_session.close()


@asynccontextmanager
async def transaction(**kwargs) -> AsyncGenerator[AsyncSession, None]:
    async with session(**kwargs) as ses:
        async with ses.begin():
            yield ses


async def is_alive() -> bool:
    logger.debug("Checking if the database is alive")

    stmt = sa.text("SELECT 1 + 1 = 2")
    async with session() as ses:
        return await ses.scalar(stmt)
