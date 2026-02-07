from datetime import UTC, datetime
from uuid import UUID

from sqlalchemy import select, text
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy.orm import selectinload

from app.core.exceptions import InvalidStatesError, ResourceNotFoundError
from app.models.task import Task, TaskComment, TaskEvent, TaskEventType, TaskType
from app.schemas.task import TaskCreate, TaskStatusUpdate, TaskUpdate


async def get_today_tasks(db: AsyncSession) -> list[Task]:
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

    result = await db.execute(query)
    task_ids = [row[0] for row in result.fetchall()]

    if not task_ids:
        return []

    stmt = (
        select(Task)
        .options(selectinload(Task.events), selectinload(Task.comments))
        .where(Task.id.in_(task_ids))
        .order_by(Task.priority.desc(), Task.create_at.desc())
    )
    result = await db.execute(stmt)
    return list(result.scalars().all())


async def get_task_by_id(db: AsyncSession, task_id: UUID) -> Task:
    stmt = (
        select(Task).options(selectinload(Task.events), selectinload(Task.comments)).where(Task.id == task_id)
    )
    result = await db.execute(stmt)
    task = result.scalar_one_or_none()
    if not task:
        raise ResourceNotFoundError(f"Task {task_id} not found")
    return task


async def create_task(db: AsyncSession, payload: TaskCreate) -> Task:
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
    await db.flush()

    event = TaskEvent(
        task_id=task.id,
        event_type=TaskEventType.CREATE,
        datetime=datetime.now(UTC),
    )
    db.add(event)
    await db.flush()

    return task


async def update_task(db: AsyncSession, task_id: UUID, payload: TaskUpdate) -> Task:
    task = await get_task_by_id(db, task_id)

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

    await db.flush()
    return task


async def update_task_status(db: AsyncSession, task_id: UUID, payload: TaskStatusUpdate) -> Task:
    task = await get_task_by_id(db, task_id)

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

    await db.flush()
    return task


async def archive_task(db: AsyncSession, task_id: UUID) -> Task:
    task = await get_task_by_id(db, task_id)
    task.archived = True

    event = TaskEvent(
        task_id=task.id,
        event_type=TaskEventType.ARCHIVED,
        datetime=datetime.now(UTC),
    )
    db.add(event)
    await db.flush()
    return task


async def unarchive_task(db: AsyncSession, task_id: UUID) -> Task:
    task = await get_task_by_id(db, task_id)
    task.archived = False

    event = TaskEvent(
        task_id=task.id,
        event_type=TaskEventType.UNARCHIVED,
        datetime=datetime.now(UTC),
    )
    db.add(event)
    await db.flush()
    return task


async def delete_task(db: AsyncSession, task_id: UUID) -> None:
    task = await get_task_by_id(db, task_id)
    await db.delete(task)
    await db.flush()


async def add_task_comment(db: AsyncSession, task_id: UUID, content: str) -> TaskComment:
    task = await get_task_by_id(db, task_id)

    comment = TaskComment(
        task_id=task.id,
        create_at=datetime.now(UTC),
        content=content,
    )
    db.add(comment)
    await db.flush()
    return comment
