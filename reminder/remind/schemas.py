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

    total_notes_count: int

    material_title: str
    material_authors: str
    material_pages: int
    material_tags: str
    material_current_status: Literal['queue', 'reading', 'completed']
    material_repeats_count: int = 0
    material_last_repeated_at: datetime.datetime | None = None

    def format(self) -> str:
        pass

    def __repr__(self) -> str:
        indent = '\t'
        fields = '\n'.join(
            f"{indent}{name}={value!r}"
            for name, value in self.dict().items()
        )
        return f"{self.__class__.__name__}(\n{fields}\n)"


class LastMaterialRemind(CustomBaseModel):
    reminds_count: int
    last_reminded_at: datetime.datetime
