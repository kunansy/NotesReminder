import datetime
from typing import Literal
from uuid import UUID
import re

from reminder.common.schemas import CustomBaseModel


DEMARK_BOLD_PATTERN = re.compile('<span class="?font-weight-bold"?>(.*?)</span>')
DEMARK_ITALIC_PATTERN = re.compile('<span class="?font-italic"?>(.*?)</span>')
DEMARK_CODE_PATTERN = re.compile('<span class="?font-code"?>(.*?)</span>')


def _demark_bold(string: str) -> str:
    return DEMARK_BOLD_PATTERN.sub(r'<bold>\1</bold>', string)


def _demark_italic(string: str) -> str:
    return DEMARK_ITALIC_PATTERN.sub(r'<strong>\1</strong>', string)


def _demark_code(string: str) -> str:
    return DEMARK_CODE_PATTERN.sub(r'<code>\1</code>', string)


def _dereplace_new_lines(string: str) -> str:
    return re.sub(r'<br/?>', '\n', string)


def demark(content: str) -> str:
    return _dereplace_new_lines(
        _demark_code(_demark_italic(_demark_bold(content))))


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

    @property
    def content_html(self) -> str:
        return demark(self.content)

    def _repeated_ago(self) -> int:
        if not self.material_last_repeated_at:
            return 0

        return (datetime.datetime.utcnow() - self.material_last_repeated_at).days

    def repeated_ago(self) -> str:
        if not (repeated_ago := self._repeated_ago()):
            return "-"
        res = ""

        if years := repeated_ago // 365:
            res = f"{years} years, "
        if months := repeated_ago % 12 // 30:
            res = f"{months} months, "
        if days := repeated_ago % 365 % 30:
            res = f"{days} days"

        if res.endswith(' '):
            res = f"{res[:2]}"

        return f"{res} ago"

    def format_note_added_at(self) -> str:
        return self.added_at.strftime("%Y-%m-%d")

    def format(self) -> str:
        return f"«{self.material_title}» – {self.material_authors}\n\n" \
               f"{self.content_html}\n\n" \
               f"Chapter: {self.chapter}\n" \
               f"Page: {self.page} / {self.material_pages}\n" \
               f"Material status: {self.material_current_status}\n" \
               f"Added at: {self.format_note_added_at()}\n" \
               f"Repeats count: {self.material_repeats_count}\n" \
               f"Last repeated: {self.repeated_ago()}\n" \
               f"Total notes count: {self.total_notes_count}"

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
