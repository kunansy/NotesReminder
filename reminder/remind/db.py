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
    count = sa.func.count(1).over(partition_by=models.Repeats.c.material_id)

    stmt = sa.select([models.Repeats.c.repeated_at.label('date'),
                      count.label('count')])\
        .where(models.Repeats.c.material_id == str(material_id)) \
        .order_by(models.Repeats.c.repeated_at.desc())\
        .limit(1)

    async with database.session() as ses:
        last_remind = (await ses.execute(stmt)).mappings().one()

    return LastMaterialRemind(
        reminds_count=last_remind['count'],
        last_reminded_at=last_remind['date']
    )


async def get_random_note() -> schemas.Note:
    pass


async def insert_notes_history(*,
                               note_id: UUID,
                               user_id: int) -> None:
    pass
