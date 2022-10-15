import random
from uuid import UUID

import sqlalchemy.sql as sa
from sqlalchemy.engine import RowMapping

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


async def _get_random_note(offset: int) -> RowMapping:
    stmt = sa.select([models.Notes,
                      models.Materials.c.title.label('material_title'),
                      models.Materials.c.authors.label('material_authors'),
                      sa.text("CASE WHEN statuses IS NULL THEN 'queue'"
                              "WHEN statuses.completed_at IS NULL THEN 'reading'"
                              "ELSE 'reading' END AS material_current_status"),
                      ]) \
        .join(models.Materials,
              models.Materials.c.material_id == models.Notes.c.material_id) \
        .join(models.Statuses,
              models.Statuses.c.material_id == models.Notes.c.material_id) \
        .where(models.Notes.c.is_deleted == False) \
        .limit(1).offset(offset)

    async with database.session() as ses:
        return (await ses.execute(stmt)).mappings().one()


async def get_random_note() -> schemas.Note:
    notes_count = await _get_notes_count()
    offset = _get_random_note_offset(notes_count)

    note = await _get_random_note(offset)
    last_repeat = await _get_last_material_remind(material_id=note['material_id'])

    return schemas.Note(
        **note,
        notes_count=notes_count,
        material_repeats_count=last_repeat.reminds_count,
        material_last_repeated_at=last_repeat.last_reminded_at
    )


async def insert_notes_history(*,
                               note_id: UUID,
                               user_id: int) -> None:
    pass
