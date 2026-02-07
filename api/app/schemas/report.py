from enum import StrEnum

from pydantic import BaseModel


class ReportPeriod(StrEnum):
    TODAY = "today"
    WEEK = "week"
    MONTH = "month"


class ReportResponse(BaseModel):
    period: ReportPeriod
    created_count: int
    done_count: int
    archived_count: int
    state_changes_count: int
    comments_count: int
