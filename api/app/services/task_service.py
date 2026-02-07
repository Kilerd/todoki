from datetime import UTC, datetime
from uuid import UUID

from sqlmodel import Session, select

from app.core.exceptions import ResourceNotFoundError
from app.models.task import (
    Task,
    TaskComment,
    TaskCreate,
    TaskEvent,
    TaskEventType,
    TaskStatus,
    TaskStatusUpdate,
    TaskUpdate,
)


def get_today_tasks(db: Session) -> list[Task]:
    """
    Get tasks that are in 'todo' status and not archived.
    """
    stmt = (
        select(Task)
        .where(Task.status == TaskStatus.TODO, Task.archived == False)
        .order_by(Task.priority.desc(), Task.create_at.desc())
    )
    return list(db.exec(stmt).all())


def get_backlog_tasks(db: Session) -> list[Task]:
    """
    Get tasks that are in 'backlog' status and not archived.
    """
    stmt = (
        select(Task)
        .where(Task.status == TaskStatus.BACKLOG, Task.archived == False)
        .order_by(Task.priority.desc(), Task.create_at.desc())
    )
    return list(db.exec(stmt).all())


def get_in_progress_tasks(db: Session) -> list[Task]:
    """
    Get tasks that are in 'in-progress' or 'in-review' status and not archived.
    """
    stmt = (
        select(Task)
        .where(Task.status.in_([TaskStatus.IN_PROGRESS, TaskStatus.IN_REVIEW]), Task.archived == False)
        .order_by(Task.priority.desc(), Task.create_at.desc())
    )
    return list(db.exec(stmt).all())


def get_done_tasks(db: Session) -> list[Task]:
    """
    Get tasks that are in 'done' status and not archived.
    """
    stmt = (
        select(Task)
        .where(Task.status == TaskStatus.DONE, Task.archived == False)
        .order_by(Task.priority.desc(), Task.create_at.desc())
    )
    return list(db.exec(stmt).all())


def get_task_by_id(db: Session, task_id: UUID) -> Task:
    task = db.get(Task, task_id)
    if not task:
        raise ResourceNotFoundError(f"Task {task_id} not found")
    return task


def create_task(db: Session, payload: TaskCreate) -> Task:
    task = Task(
        priority=payload.priority,
        content=payload.content,
        group=payload.group or "default",
        status=payload.status,
        create_at=datetime.now(UTC),
        archived=False,
    )
    db.add(task)
    db.commit()
    db.refresh(task)

    event = TaskEvent(
        task_id=task.id,
        event_type=TaskEventType.CREATE,
        datetime=datetime.now(UTC),
    )
    db.add(event)
    db.commit()
    db.refresh(task)

    return task


def update_task(db: Session, task_id: UUID, payload: TaskUpdate) -> Task:
    task = get_task_by_id(db, task_id)

    task.priority = payload.priority
    task.content = payload.content
    task.group = payload.group or "default"

    db.add(task)
    db.commit()
    db.refresh(task)
    return task


def update_task_status(db: Session, task_id: UUID, payload: TaskStatusUpdate) -> Task:
    task = get_task_by_id(db, task_id)

    new_status = payload.status
    task.status = new_status

    event = TaskEvent(
        task_id=task.id,
        event_type=TaskEventType.STATUS_CHANGE,
        datetime=datetime.now(UTC),
        state=new_status,
    )
    db.add(event)

    db.add(task)
    db.commit()
    db.refresh(task)
    return task


def archive_task(db: Session, task_id: UUID) -> Task:
    task = get_task_by_id(db, task_id)
    task.archived = True

    event = TaskEvent(
        task_id=task.id,
        event_type=TaskEventType.ARCHIVED,
        datetime=datetime.now(UTC),
    )
    db.add(event)
    db.add(task)
    db.commit()
    db.refresh(task)
    return task


def unarchive_task(db: Session, task_id: UUID) -> Task:
    task = get_task_by_id(db, task_id)
    task.archived = False

    event = TaskEvent(
        task_id=task.id,
        event_type=TaskEventType.UNARCHIVED,
        datetime=datetime.now(UTC),
    )
    db.add(event)
    db.add(task)
    db.commit()
    db.refresh(task)
    return task


def delete_task(db: Session, task_id: UUID) -> None:
    task = get_task_by_id(db, task_id)
    db.delete(task)
    db.commit()


def add_task_comment(db: Session, task_id: UUID, content: str) -> TaskComment:
    task = get_task_by_id(db, task_id)

    comment = TaskComment(
        task_id=task.id,
        create_at=datetime.now(UTC),
        content=content,
    )
    db.add(comment)
    db.commit()
    db.refresh(comment)
    return comment
