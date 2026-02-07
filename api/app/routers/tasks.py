from uuid import UUID

from fastapi import APIRouter

from app.deps import AuthDep, DbSession
from app.models.task import (
    TaskCommentCreate,
    TaskCommentResponse,
    TaskCreate,
    TaskResponse,
    TaskStatusUpdate,
    TaskUpdate,
)
from app.services import task_service

router = APIRouter(prefix="/tasks", tags=["tasks"])


@router.get("", response_model=list[TaskResponse])
def get_tasks(_: AuthDep, db: DbSession) -> list[TaskResponse]:
    """Get today's tasks (status='todo' and not archived)."""
    tasks = task_service.get_today_tasks(db)
    return [TaskResponse.model_validate(t) for t in tasks]


@router.get("/inbox", response_model=list[TaskResponse])
def get_inbox_tasks(_: AuthDep, db: DbSession) -> list[TaskResponse]:
    """Get inbox tasks (todo, in-progress, in-review and not archived)."""
    tasks = task_service.get_inbox_tasks(db)
    return [TaskResponse.model_validate(t) for t in tasks]


@router.get("/backlog", response_model=list[TaskResponse])
def get_backlog_tasks(_: AuthDep, db: DbSession) -> list[TaskResponse]:
    """Get backlog tasks (status='backlog' and not archived)."""
    tasks = task_service.get_backlog_tasks(db)
    return [TaskResponse.model_validate(t) for t in tasks]


@router.get("/in-progress", response_model=list[TaskResponse])
def get_in_progress_tasks(_: AuthDep, db: DbSession) -> list[TaskResponse]:
    """Get in-progress tasks (status='in-progress' or 'in-review' and not archived)."""
    tasks = task_service.get_in_progress_tasks(db)
    return [TaskResponse.model_validate(t) for t in tasks]


@router.get("/done", response_model=list[TaskResponse])
def get_done_tasks(_: AuthDep, db: DbSession) -> list[TaskResponse]:
    """Get done tasks (status='done' and not archived)."""
    tasks = task_service.get_done_tasks(db)
    return [TaskResponse.model_validate(t) for t in tasks]


@router.get("/done/today", response_model=list[TaskResponse])
def get_today_done_tasks(_: AuthDep, db: DbSession) -> list[TaskResponse]:
    """Get tasks marked done today (not archived)."""
    tasks = task_service.get_today_done_tasks(db)
    return [TaskResponse.model_validate(t) for t in tasks]


@router.post("", response_model=TaskResponse, status_code=201)
def create_task(_: AuthDep, db: DbSession, payload: TaskCreate) -> TaskResponse:
    """Create a new task."""
    task = task_service.create_task(db, payload)
    return TaskResponse.model_validate(task)


@router.get("/{task_id}", response_model=TaskResponse)
def get_task(_: AuthDep, db: DbSession, task_id: UUID) -> TaskResponse:
    """Get task details by ID."""
    task = task_service.get_task_by_id(db, task_id)
    return TaskResponse.model_validate(task)


@router.put("/{task_id}", response_model=TaskResponse)
def update_task(_: AuthDep, db: DbSession, task_id: UUID, payload: TaskUpdate) -> TaskResponse:
    """Update task details."""
    task = task_service.update_task(db, task_id, payload)
    return TaskResponse.model_validate(task)


@router.post("/{task_id}/status", response_model=TaskResponse)
def update_task_status(_: AuthDep, db: DbSession, task_id: UUID, payload: TaskStatusUpdate) -> TaskResponse:
    """Update task status (backlog, todo, in-progress, in-review, done)."""
    task = task_service.update_task_status(db, task_id, payload)
    return TaskResponse.model_validate(task)


@router.post("/{task_id}/archive", response_model=TaskResponse)
def archive_task(_: AuthDep, db: DbSession, task_id: UUID) -> TaskResponse:
    """Archive a task."""
    task = task_service.archive_task(db, task_id)
    return TaskResponse.model_validate(task)


@router.post("/{task_id}/unarchive", response_model=TaskResponse)
def unarchive_task(_: AuthDep, db: DbSession, task_id: UUID) -> TaskResponse:
    """Unarchive a task."""
    task = task_service.unarchive_task(db, task_id)
    return TaskResponse.model_validate(task)


@router.delete("/{task_id}", status_code=204)
def delete_task(_: AuthDep, db: DbSession, task_id: UUID) -> None:
    """Delete a task."""
    task_service.delete_task(db, task_id)


@router.post("/{task_id}/comments", response_model=TaskCommentResponse, status_code=201)
def add_comment(_: AuthDep, db: DbSession, task_id: UUID, payload: TaskCommentCreate) -> TaskCommentResponse:
    """Add a comment to a task."""
    comment = task_service.add_task_comment(db, task_id, payload.content)
    return TaskCommentResponse.model_validate(comment)
