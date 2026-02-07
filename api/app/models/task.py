from datetime import datetime
from enum import StrEnum
from uuid import uuid4

from sqlalchemy import DateTime, Enum, ForeignKey, String, Text, func
from sqlalchemy.dialects.postgresql import ARRAY, UUID
from sqlalchemy.orm import Mapped, mapped_column, relationship

from app.core.database import Base


class TaskType(StrEnum):
    TODO = "Todo"
    STATEFUL = "Stateful"


class TaskEventType(StrEnum):
    CREATE = "Create"
    DONE = "Done"
    OPEN = "Open"
    UNARCHIVED = "Unarchived"
    ARCHIVED = "Archived"
    UPDATE_STATE = "UpdateState"
    CREATE_COMMENT = "CreateComment"


class Task(Base):
    __tablename__ = "tasks"

    id: Mapped[UUID] = mapped_column(UUID(as_uuid=True), primary_key=True, default=uuid4)
    priority: Mapped[int] = mapped_column(nullable=False)
    content: Mapped[str] = mapped_column(Text, nullable=False)
    group: Mapped[str] = mapped_column(String(255), nullable=False, default="default")
    task_type: Mapped[TaskType] = mapped_column(
        Enum(TaskType, name="task_type_enum", native_enum=False), nullable=False
    )
    create_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), nullable=False, server_default=func.now()
    )
    done: Mapped[bool] = mapped_column(nullable=False, default=False)
    archived: Mapped[bool] = mapped_column(nullable=False, default=False)

    # For stateful tasks
    current_state: Mapped[str | None] = mapped_column(String(255), nullable=True)
    states: Mapped[list[str] | None] = mapped_column(ARRAY(String), nullable=True, default=list)

    events: Mapped[list["TaskEvent"]] = relationship(
        "TaskEvent", back_populates="task", cascade="all, delete-orphan", order_by="desc(TaskEvent.datetime)"
    )
    comments: Mapped[list["TaskComment"]] = relationship(
        "TaskComment", back_populates="task", cascade="all, delete-orphan", order_by="asc(TaskComment.create_at)"
    )


class TaskEvent(Base):
    __tablename__ = "task_events"

    id: Mapped[UUID] = mapped_column(UUID(as_uuid=True), primary_key=True, default=uuid4)
    task_id: Mapped[UUID] = mapped_column(
        UUID(as_uuid=True), ForeignKey("tasks.id", ondelete="CASCADE"), nullable=False
    )
    event_type: Mapped[TaskEventType] = mapped_column(
        Enum(TaskEventType, name="task_event_type_enum", native_enum=False), nullable=False
    )
    datetime: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), nullable=False, server_default=func.now()
    )
    state: Mapped[str | None] = mapped_column(String(255), nullable=True)

    task: Mapped["Task"] = relationship("Task", back_populates="events")


class TaskComment(Base):
    __tablename__ = "task_comments"

    id: Mapped[UUID] = mapped_column(UUID(as_uuid=True), primary_key=True, default=uuid4)
    task_id: Mapped[UUID] = mapped_column(
        UUID(as_uuid=True), ForeignKey("tasks.id", ondelete="CASCADE"), nullable=False
    )
    create_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), nullable=False, server_default=func.now()
    )
    content: Mapped[str] = mapped_column(Text, nullable=False)

    task: Mapped["Task"] = relationship("Task", back_populates="comments")
