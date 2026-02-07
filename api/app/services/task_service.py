from datetime import UTC, datetime
from uuid import UUID

from sqlmodel import Session, select, text

from app.core.exceptions import InvalidStatesError, ResourceNotFoundError
from app.models.task import (
    Task,
    TaskComment,
    TaskCreate,
    TaskEvent,
    TaskEventType,
    TaskStatusUpdate,
    TaskType,
    TaskUpdate,
)


def get_today_tasks(db: Session) -> list[Task]:
    """
    Get tasks that are either:
    - Not done and not archived
    - Or have activity today (based on latest event datetime)
    """
    query = text("""
        WITH task_latest_active_datetime AS (
            SELECT DISTINCT ON (task_id) id, task_id, datetime
            FROM task_events
            ORDER BY task_id, datetime DESC
        )
        SELECT tasks.id
        FROM tasks
        LEFT JOIN task_latest_active_datetime ON tasks.id = task_latest_active_datetime.task_id
        WHERE (done = false AND archived = false)
           OR (task_latest_active_datetime.datetime AT TIME ZONE 'Asia/Hong_Kong')::date =
              (NOW() AT TIME ZONE 'Asia/Hong_Kong')::date
    """)

    result = db.exec(query)
    task_ids = [row[0] for row in result.fetchall()]

    if not task_ids:
        return []

    stmt = select(Task).where(Task.id.in_(task_ids)).order_by(Task.priority.desc(), Task.create_at.desc())
    return list(db.exec(stmt).all())


def get_task_by_id(db: Session, task_id: UUID) -> Task:
    task = db.get(Task, task_id)
    if not task:
        raise ResourceNotFoundError(f"Task {task_id} not found")
    return task


def create_task(db: Session, payload: TaskCreate) -> Task:
    current_state = None
    if payload.task_type == TaskType.STATEFUL:
        states = payload.states or []
        if len(states) < 2:
            raise InvalidStatesError()
        current_state = states[0]

    task = Task(
        priority=payload.priority,
        content=payload.content,
        group=payload.group or "default",
        task_type=payload.task_type,
        create_at=datetime.now(UTC),
        done=False,
        archived=False,
        current_state=current_state,
        states=payload.states,
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

    current_state: str | None = None
    if payload.states:
        if task.current_state and task.current_state in payload.states:
            current_state = task.current_state
        else:
            current_state = payload.states[0]
        task_type = TaskType.STATEFUL
    else:
        task_type = TaskType.TODO

    task.priority = payload.priority
    task.content = payload.content
    task.group = payload.group or "default"
    task.states = payload.states
    task.current_state = current_state
    task.task_type = task_type

    db.add(task)
    db.commit()
    db.refresh(task)
    return task


def update_task_status(db: Session, task_id: UUID, payload: TaskStatusUpdate) -> Task:
    task = get_task_by_id(db, task_id)

    if task.task_type == TaskType.TODO:
        is_done = payload.status == "Done"
        task.done = is_done

        event = TaskEvent(
            task_id=task.id,
            event_type=TaskEventType.DONE if is_done else TaskEventType.OPEN,
            datetime=datetime.now(UTC),
        )
        db.add(event)

    elif task.task_type == TaskType.STATEFUL:
        next_state = payload.status
        states = task.states or []

        try:
            pos = states.index(next_state)
        except ValueError:
            raise InvalidStatesError(f"Invalid state: {next_state}")

        is_done = pos == len(states) - 1
        task.current_state = next_state
        task.done = is_done

        event = TaskEvent(
            task_id=task.id,
            event_type=TaskEventType.UPDATE_STATE,
            datetime=datetime.now(UTC),
            state=next_state,
        )
        db.add(event)

        if is_done:
            done_event = TaskEvent(
                task_id=task.id,
                event_type=TaskEventType.DONE,
                datetime=datetime.now(UTC),
            )
            db.add(done_event)

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
