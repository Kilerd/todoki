from datetime import datetime
from enum import StrEnum
from uuid import UUID, uuid4

from sqlalchemy import Column, ForeignKey, String, Text
from sqlalchemy.dialects.postgresql import UUID as PG_UUID
from sqlmodel import Field, Relationship, SQLModel


class TaskStatus(StrEnum):
    BACKLOG = "backlog"
    TODO = "todo"
    IN_PROGRESS = "in-progress"
    IN_REVIEW = "in-review"
    DONE = "done"


TASK_STATUS_ORDER = [
    TaskStatus.BACKLOG,
    TaskStatus.TODO,
    TaskStatus.IN_PROGRESS,
    TaskStatus.IN_REVIEW,
    TaskStatus.DONE,
]


class TaskEventType(StrEnum):
    CREATE = "Create"
    STATUS_CHANGE = "StatusChange"
    UNARCHIVED = "Unarchived"
    ARCHIVED = "Archived"
    CREATE_COMMENT = "CreateComment"


# ============== TaskEvent ==============


class TaskEventBase(SQLModel):
    event_type: TaskEventType
    datetime: datetime
    state: str | None = None


class TaskEvent(TaskEventBase, table=True):
    __tablename__ = "task_events"

    id: UUID = Field(default_factory=uuid4, sa_column=Column(PG_UUID(as_uuid=True), primary_key=True))
    task_id: UUID = Field(
        sa_column=Column(PG_UUID(as_uuid=True), ForeignKey("tasks.id", ondelete="CASCADE"), nullable=False)
    )

    task: "Task" = Relationship(back_populates="events")


class TaskEventResponse(TaskEventBase):
    id: UUID
    task_id: UUID


# ============== TaskComment ==============


class TaskCommentBase(SQLModel):
    content: str


class TaskComment(TaskCommentBase, table=True):
    __tablename__ = "task_comments"

    id: UUID = Field(default_factory=uuid4, sa_column=Column(PG_UUID(as_uuid=True), primary_key=True))
    task_id: UUID = Field(
        sa_column=Column(PG_UUID(as_uuid=True), ForeignKey("tasks.id", ondelete="CASCADE"), nullable=False)
    )
    create_at: datetime

    task: "Task" = Relationship(back_populates="comments")


class TaskCommentResponse(TaskCommentBase):
    id: UUID
    task_id: UUID
    create_at: datetime


class TaskCommentCreate(TaskCommentBase):
    pass


# ============== Task ==============


class TaskBase(SQLModel):
    priority: int = Field(default=0)
    content: str = Field(sa_column=Column(Text, nullable=False))
    group: str = Field(default="default", max_length=255)
    status: TaskStatus = Field(default=TaskStatus.BACKLOG, sa_column=Column(String(50), nullable=False))


class Task(TaskBase, table=True):
    __tablename__ = "tasks"

    id: UUID = Field(default_factory=uuid4, sa_column=Column(PG_UUID(as_uuid=True), primary_key=True))
    create_at: datetime
    archived: bool = Field(default=False)

    events: list[TaskEvent] = Relationship(
        back_populates="task",
        sa_relationship_kwargs={"cascade": "all, delete-orphan", "order_by": "desc(TaskEvent.datetime)"},
    )
    comments: list[TaskComment] = Relationship(
        back_populates="task",
        sa_relationship_kwargs={"cascade": "all, delete-orphan", "order_by": "asc(TaskComment.create_at)"},
    )


class TaskCreate(SQLModel):
    priority: int = Field(default=0)
    content: str
    group: str | None = Field(default="default")
    status: TaskStatus = Field(default=TaskStatus.BACKLOG)


class TaskUpdate(SQLModel):
    priority: int
    content: str
    group: str | None = Field(default="default")


class TaskStatusUpdate(SQLModel):
    status: TaskStatus


class TaskResponse(TaskBase):
    id: UUID
    create_at: datetime
    archived: bool
    events: list[TaskEventResponse] = []
    comments: list[TaskCommentResponse] = []
