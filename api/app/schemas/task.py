from datetime import datetime
from uuid import UUID

from pydantic import BaseModel, Field

from app.models.task import TaskEventType, TaskType


class TaskEventResponse(BaseModel):
    id: UUID
    task_id: UUID
    event_type: TaskEventType
    datetime: datetime
    state: str | None = None

    model_config = {"from_attributes": True}


class TaskCommentResponse(BaseModel):
    id: UUID
    task_id: UUID
    create_at: datetime
    content: str

    model_config = {"from_attributes": True}


class TaskResponse(BaseModel):
    id: UUID
    priority: int
    content: str
    group: str
    task_type: TaskType
    create_at: datetime
    done: bool
    archived: bool
    current_state: str | None = None
    states: list[str] | None = None
    events: list[TaskEventResponse] = []
    comments: list[TaskCommentResponse] = []

    model_config = {"from_attributes": True}


class TaskCreate(BaseModel):
    priority: int = Field(default=0)
    content: str
    group: str | None = Field(default="default")
    task_type: TaskType = Field(default=TaskType.TODO)
    states: list[str] | None = None


class TaskUpdate(BaseModel):
    priority: int
    content: str
    group: str | None = Field(default="default")
    states: list[str] | None = None


class TaskStatusUpdate(BaseModel):
    status: str


class TaskCommentCreate(BaseModel):
    content: str
