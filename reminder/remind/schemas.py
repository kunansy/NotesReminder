import datetime
from typing import Literal
from uuid import UUID

from reminder.common.schemas import CustomBaseModel


class Note(CustomBaseModel):
    note_id: UUID
    material_id: UUID
    content: str
    added_at: datetime.datetime
    chapter: int
    page: int

    material_title: str
    material_authors: str
    material_current_status: Literal['queue', 'reading', 'completed']
    material_repeats_count: int
    material_last_repeated_at: datetime.datetime


class LastMaterialRemind(CustomBaseModel):
    reminds_count: int
    last_reminded_at: datetime.datetime
