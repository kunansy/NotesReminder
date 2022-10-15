from uuid import UUID

from reminder.remind import schemas
from reminder.remind.schemas import LastMaterialRemind


async def _get_notes_count() -> int:
    pass


async def _get_random_note_offset() -> int:
    pass


async def _get_last_material_remind(material_id: UUID) -> LastMaterialRemind:
    pass


async def get_random_note() -> schemas.Note:
    pass


async def insert_notes_history(*,
                               note_id: UUID,
                               user_id: int) -> None:
    pass
