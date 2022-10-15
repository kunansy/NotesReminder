import random
from uuid import UUID

import sqlalchemy.sql as sa

from reminder.common import database
from reminder.models import models
from reminder.remind import schemas
from reminder.remind.schemas import LastMaterialRemind


async def _get_notes_count() -> int:
    stmt = sa.select(sa.func.count(1))\
        .select_from(models.Notes)\
        .where(models.Notes.c.is_deleted == False)

    async with database.session() as ses:
        return await ses.scalar(stmt)


def _get_random_note_offset(notes_count: int) -> int:
    return random.randint(0, notes_count - 1)


async def _get_last_material_remind(material_id: UUID) -> LastMaterialRemind:
    pass


async def get_random_note() -> schemas.Note:
    pass


async def insert_notes_history(*,
                               note_id: UUID,
                               user_id: int) -> None:
    pass
