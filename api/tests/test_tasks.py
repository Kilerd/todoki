import pytest
from httpx import AsyncClient


@pytest.mark.asyncio
async def test_health_check(client: AsyncClient) -> None:
    response = await client.get("/api")
    assert response.status_code == 200
    assert response.json() == {"status": "ok"}


@pytest.mark.asyncio
async def test_create_todo_task(client: AsyncClient) -> None:
    payload = {
        "priority": 1,
        "content": "Test task",
        "group": "work",
        "task_type": "Todo",
    }
    response = await client.post("/api/tasks", json=payload)
    assert response.status_code == 201
    data = response.json()
    assert data["content"] == "Test task"
    assert data["task_type"] == "Todo"
    assert data["done"] is False
    assert data["archived"] is False


@pytest.mark.asyncio
async def test_create_stateful_task(client: AsyncClient) -> None:
    payload = {
        "priority": 2,
        "content": "Stateful task",
        "task_type": "Stateful",
        "states": ["Draft", "Review", "Published"],
    }
    response = await client.post("/api/tasks", json=payload)
    assert response.status_code == 201
    data = response.json()
    assert data["task_type"] == "Stateful"
    assert data["current_state"] == "Draft"
    assert data["states"] == ["Draft", "Review", "Published"]
    assert data["done"] is False


@pytest.mark.asyncio
async def test_create_stateful_task_requires_states(client: AsyncClient) -> None:
    payload = {
        "priority": 2,
        "content": "Invalid stateful",
        "task_type": "Stateful",
        "states": ["OnlyOne"],
    }
    response = await client.post("/api/tasks", json=payload)
    assert response.status_code == 400


@pytest.mark.asyncio
async def test_todo_status_update(client: AsyncClient) -> None:
    # Create task
    create_resp = await client.post(
        "/api/tasks",
        json={"content": "Todo task", "task_type": "Todo"},
    )
    task_id = create_resp.json()["id"]

    # Mark as Done
    done_resp = await client.post(f"/api/tasks/{task_id}/status", json={"status": "Done"})
    assert done_resp.status_code == 200
    assert done_resp.json()["done"] is True

    # Mark as Open
    open_resp = await client.post(f"/api/tasks/{task_id}/status", json={"status": "Open"})
    assert open_resp.status_code == 200
    assert open_resp.json()["done"] is False


@pytest.mark.asyncio
async def test_stateful_status_update(client: AsyncClient) -> None:
    # Create stateful task
    create_resp = await client.post(
        "/api/tasks",
        json={
            "content": "Workflow task",
            "task_type": "Stateful",
            "states": ["Todo", "InProgress", "Done"],
        },
    )
    task_id = create_resp.json()["id"]
    assert create_resp.json()["current_state"] == "Todo"

    # Move to InProgress
    resp1 = await client.post(f"/api/tasks/{task_id}/status", json={"status": "InProgress"})
    assert resp1.json()["current_state"] == "InProgress"
    assert resp1.json()["done"] is False

    # Move to Done (final state)
    resp2 = await client.post(f"/api/tasks/{task_id}/status", json={"status": "Done"})
    assert resp2.json()["current_state"] == "Done"
    assert resp2.json()["done"] is True


@pytest.mark.asyncio
async def test_archive_unarchive(client: AsyncClient) -> None:
    # Create task
    create_resp = await client.post("/api/tasks", json={"content": "Archive test"})
    task_id = create_resp.json()["id"]

    # Archive
    archive_resp = await client.post(f"/api/tasks/{task_id}/archive")
    assert archive_resp.json()["archived"] is True

    # Unarchive
    unarchive_resp = await client.post(f"/api/tasks/{task_id}/unarchive")
    assert unarchive_resp.json()["archived"] is False


@pytest.mark.asyncio
async def test_add_comment(client: AsyncClient) -> None:
    # Create task
    create_resp = await client.post("/api/tasks", json={"content": "Comment test"})
    task_id = create_resp.json()["id"]

    # Add comment
    comment_resp = await client.post(
        f"/api/tasks/{task_id}/comments",
        json={"content": "This is a comment"},
    )
    assert comment_resp.status_code == 201
    assert comment_resp.json()["content"] == "This is a comment"

    # Verify comment appears in task
    task_resp = await client.get(f"/api/tasks/{task_id}")
    assert len(task_resp.json()["comments"]) == 1


@pytest.mark.asyncio
async def test_delete_task(client: AsyncClient) -> None:
    # Create task
    create_resp = await client.post("/api/tasks", json={"content": "Delete test"})
    task_id = create_resp.json()["id"]

    # Delete
    delete_resp = await client.delete(f"/api/tasks/{task_id}")
    assert delete_resp.status_code == 204

    # Verify gone
    get_resp = await client.get(f"/api/tasks/{task_id}")
    assert get_resp.status_code == 404


@pytest.mark.asyncio
async def test_unauthorized_without_token(client: AsyncClient) -> None:
    # Remove auth header
    client.headers.pop("Authorization", None)
    response = await client.get("/api/tasks")
    assert response.status_code in [401, 422]
