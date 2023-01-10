import random
from functools import wraps
from typing import Callable
from uuid import UUID

import sqlalchemy.sql as sa
from sqlalchemy.engine import RowMapping

from reminder.common import database, settings
from reminder.common.logger import logger
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


async def _get_last_material_remind(material_id: UUID) -> LastMaterialRemind | None:
    count = sa.func.count(1).over(partition_by=models.Repeats.c.material_id)

    stmt = sa.select([models.Repeats.c.repeated_at.label('date'),
                      count.label('count')])\
        .where(models.Repeats.c.material_id == str(material_id)) \
        .order_by(models.Repeats.c.repeated_at.desc())\
        .limit(1)

    async with database.session() as ses:
        if last_remind := (await ses.execute(stmt)).mappings().one_or_none():
            return LastMaterialRemind(
                reminds_count=last_remind['count'],
                last_reminded_at=last_remind['date']
            )

    return None


def _get_unique_random_note(f: Callable) -> Callable:

    @wraps(f)
    async def wrapped(notes_count: int, remind_statistics: dict[str, int], *args, **kwargs):

        note, counter = await f(notes_count, *args, **kwargs), 0
        while remind_statistics.get(str(note.note_id), 0) >= min(remind_statistics.values()):
            if counter >= settings.NOTES_ITER_LIMIT:
                break
            counter += 1
            logger.debug("Note '%s' even repeated", note.note_id)
            logger.debug("Search for unique note, iter=%s", counter)

            note = await f(notes_count, *args, **kwargs)

        logger.debug("Note found for %s iters", counter)

        return note

    return wrapped


@_get_unique_random_note
async def _get_random_note(notes_count: int) -> RowMapping:
    offset = _get_random_note_offset(notes_count)

    stmt = sa.select([models.Notes,
                      models.Materials.c.title.label('material_title'),
                      models.Materials.c.authors.label('material_authors'),
                      models.Materials.c.pages.label('material_pages'),
                      models.Materials.c.tags.label('material_tags'),
                      sa.text("CASE WHEN statuses IS NULL THEN 'queue'"
                              "WHEN statuses.completed_at IS NULL THEN 'reading'"
                              "ELSE 'completed' END AS material_current_status"),
                      ]) \
        .join(models.Materials,
              models.Materials.c.material_id == models.Notes.c.material_id) \
        .join(models.Statuses,
              models.Statuses.c.material_id == models.Notes.c.material_id) \
        .where(models.Notes.c.is_deleted == False) \
        .limit(1).offset(offset)

    async with database.session() as ses:
        return (await ses.execute(stmt)).mappings().one()


async def get_remind_note() -> schemas.Note:
    notes_count = await _get_notes_count()
    remind_statistics = await get_remind_statistics()
    # we should generate offset of this func because of closure
    note = await _get_random_note(notes_count, remind_statistics)

    last_repeat_dict = {}
    if last_repeat := await _get_last_material_remind(material_id=note['material_id']):
        last_repeat_dict = {
            'material_repeats_count': last_repeat.reminds_count,
            'material_last_repeated_at': last_repeat.last_reminded_at
        }

    return schemas.Note(
        **note,
        **last_repeat_dict,
        total_notes_count=notes_count,
    )


async def get_remind_statistics() -> dict[str, int]:
    stmt = sa.select([models.Notes.c.note_id,
                      sa.func.count(1).over(partition_by=models.NoteRepeatsHistory.c.note_id)])\
        .join(models.NoteRepeatsHistory,
              models.NoteRepeatsHistory.c.note_id == models.Notes.c.note_id,
              isouter=True)\
        .where(models.Notes.c.is_deleted == False)

    async with database.session() as ses:
        return {
            note_id: count
            for (note_id, count) in (await ses.execute(stmt)).all()
        }


def _get_remind_note_id(remind_stats: dict[str, int]) -> str:
    if not remind_stats:
        raise ValueError("Remind status could not be empty")

    min_repeats_count = min(remind_stats.values())
    sample = [
        note_id
        for note_id, repeats_count in remind_stats.items()
        if repeats_count == min_repeats_count
    ]
    return random.choice(sample)


async def insert_notes_history(*,
                               note_id: UUID,
                               user_id: int) -> None:
    logger.debug("Inserting repeat for note_id=%s", note_id)

    values = {
        "note_id": str(note_id),
        "user_id": user_id,
    }
    stmt = models.NoteRepeatsHistory\
        .insert().values(values)\
        .returning(models.NoteRepeatsHistory.c.repeat_id)

    async with database.session() as ses:
        repeat_id = await ses.scalar(stmt)

    logger.debug("Repeat_id=%s for note_id=%s inserted", repeat_id, note_id)
