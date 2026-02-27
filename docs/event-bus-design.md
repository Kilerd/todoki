# Event Bus Architecture Design

## Document Metadata

- **Version**: 1.0
- **Date**: 2026-02-27
- **Status**: Design Phase
- **Related**: [agent-bus.md](../agent-bus.md)

## Executive Summary

This document outlines the architectural transformation needed to implement a complete Event Bus system in todoki, enabling true event-driven agent collaboration as described in `agent-bus.md`. The design enables:

- Agent-to-agent decoupled communication via standardized events
- Incremental event consumption using cursor-based subscriptions
- Event replay and analysis for skill improvement
- Multi-location agent deployment (both relay-managed and standalone)
- Full audit trail of all system activities

## Current State Analysis

### What We Have

```
┌─────────────────────────────────────┐
│  Frontend (React)                    │
│  - Kanban UI                         │
│  - Real-time task updates            │
└──────────────┬──────────────────────┘
               │ REST API + WebSocket
┌──────────────┴──────────────────────┐
│  Todoki Server                       │
│  - Task CRUD                         │
│  - Agent lifecycle management        │
│  - Broadcaster (in-memory only)      │
│  - Permission reviewer               │
└──────────────┬──────────────────────┘
               │ WebSocket
┌──────────────┴──────────────────────┐
│  Relay (Multiple instances)          │
│  - Session manager                   │
│  - ACP protocol handler              │
│  - Buffer to server                  │
└──────────────┬──────────────────────┘
               │ stdio/ACP
┌──────────────┴──────────────────────┐
│  Agent (mock-agent, claude-code)     │
│  - Task execution                    │
│  - Code generation                   │
└─────────────────────────────────────┘
```

**Key Limitations**:

1. **No persistent event log**: `AgentStreamEvent` is broadcast in-memory only
2. **Passive agents**: Agents only respond to task assignments, cannot sense system-wide events
3. **Indirect collaboration**: Agents coordinate through task status changes, not event-driven
4. **No event replay**: Cannot analyze historical event sequences
5. **Limited event semantics**: Only stdout/stderr/system/acp streams, no business-level events

### What We Need

```
┌─────────────────────────────────────┐
│  Frontend (React)                    │
│  - Event timeline visualization      │
└──────────────┬──────────────────────┘
               │
┌──────────────┴──────────────────────┐
│  Todoki Server                       │
│  ┌───────────────────────────────┐  │
│  │  Event Bus Core               │  │
│  │  - Event store (DB/JSONL)     │  │
│  │  - Cursor-based subscription  │  │
│  │  - Event publisher            │  │
│  │  - Event replay engine        │  │
│  └───────────────────────────────┘  │
│  ┌───────────────────────────────┐  │
│  │  Agent Orchestrator           │  │
│  │  - Event routing              │  │
│  │  - Agent subscription mgmt    │  │
│  │  - Auto-trigger logic         │  │
│  └───────────────────────────────┘  │
└──────────────┬──────────────────────┘
               │
     ┌─────────┴─────────┐
     │                   │
┌────┴─────┐      ┌─────┴──────┐
│  Relay   │      │ Standalone │
│  Agent   │      │   Agent    │
│ (Local)  │      │  (Remote)  │
└──────────┘      └────────────┘
     │                   │
     └─────────┬─────────┘
               │
       Event Bus HTTP/WS
```

## Core Design

### 1. Event Data Model

#### Event Structure

```rust
// crates/todoki-server/src/event_bus/types.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Global monotonic sequence number (cursor)
    pub cursor: i64,

    /// Event kind (semantic type)
    pub kind: String,

    /// Timestamp (ISO 8601)
    pub time: DateTime<Utc>,

    /// Agent ID that emitted this event
    pub agent_id: Uuid,

    /// Optional session ID
    pub session_id: Option<Uuid>,

    /// Optional task ID
    pub task_id: Option<Uuid>,

    /// Event-specific data (JSON)
    pub data: serde_json::Value,
}
```

#### Event Storage

**Database Schema**:

```sql
-- migrations/009_create_events_table.sql

CREATE TABLE events (
    cursor BIGSERIAL PRIMARY KEY,
    kind VARCHAR(64) NOT NULL,
    time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    agent_id UUID NOT NULL,
    session_id UUID,
    task_id UUID,
    data JSONB NOT NULL,

    FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE,
    FOREIGN KEY (session_id) REFERENCES agent_sessions(id) ON DELETE SET NULL,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE SET NULL
);

-- Indexes for efficient queries
CREATE INDEX idx_events_time ON events(time);
CREATE INDEX idx_events_kind ON events(kind);
CREATE INDEX idx_events_kind_time ON events(kind, time);
CREATE INDEX idx_events_agent_cursor ON events(agent_id, cursor);
CREATE INDEX idx_events_task ON events(task_id) WHERE task_id IS NOT NULL;
```

**Alternative: JSONL File Store** (for high-throughput scenarios):

```
~/.todoki/event-bus/
├── 2026-02-27.jsonl      # Daily rotation
├── 2026-02-26.jsonl
└── index.db              # SQLite index (cursor, offset, kind)
```

**Recommendation**: Start with PostgreSQL for simplicity, add JSONL export later.

### 2. Event Kinds Taxonomy

```rust
// crates/todoki-server/src/event_bus/kinds.rs

/// Standard event kinds (namespace-prefixed)
pub mod kinds {
    // Task lifecycle
    pub const TASK_CREATED: &str = "task.created";
    pub const TASK_STATUS_CHANGED: &str = "task.status_changed";
    pub const TASK_ASSIGNED: &str = "task.assigned";
    pub const TASK_COMPLETED: &str = "task.completed";
    pub const TASK_FAILED: &str = "task.failed";
    pub const TASK_ARCHIVED: &str = "task.archived";

    // Agent lifecycle
    pub const AGENT_REGISTERED: &str = "agent.registered";
    pub const AGENT_STARTED: &str = "agent.started";
    pub const AGENT_STOPPED: &str = "agent.stopped";
    pub const AGENT_OUTPUT: &str = "agent.output";
    pub const AGENT_ERROR: &str = "agent.error";

    // Agent collaboration (PM → BA → Coding → QA)
    pub const REQUIREMENT_ANALYZED: &str = "agent.requirement_analyzed";
    pub const BUSINESS_CONTEXT_READY: &str = "agent.business_context_ready";
    pub const CODE_REVIEW_REQUESTED: &str = "agent.code_review_requested";
    pub const QA_TEST_PASSED: &str = "agent.qa_test_passed";
    pub const QA_TEST_FAILED: &str = "agent.qa_test_failed";

    // Artifacts
    pub const ARTIFACT_CREATED: &str = "artifact.created";
    pub const GITHUB_PR_OPENED: &str = "artifact.github_pr_opened";
    pub const GITHUB_PR_MERGED: &str = "artifact.github_pr_merged";

    // Permission
    pub const PERMISSION_REQUESTED: &str = "permission.requested";
    pub const PERMISSION_APPROVED: &str = "permission.approved";
    pub const PERMISSION_DENIED: &str = "permission.denied";

    // System
    pub const RELAY_CONNECTED: &str = "system.relay_connected";
    pub const RELAY_DISCONNECTED: &str = "system.relay_disconnected";
}
```

**Event Data Examples**:

```json
{
  "cursor": 12345,
  "kind": "task.created",
  "time": "2026-02-27T10:30:00Z",
  "agent_id": "00000000-0000-0000-0000-000000000000",
  "session_id": null,
  "task_id": "a1b2c3d4-...",
  "data": {
    "content": "Fix authentication bug",
    "priority": 2,
    "project_id": "proj-123"
  }
}
```

```json
{
  "cursor": 12350,
  "kind": "agent.requirement_analyzed",
  "time": "2026-02-27T10:32:15Z",
  "agent_id": "pm-agent-uuid",
  "session_id": "session-xyz",
  "task_id": "a1b2c3d4-...",
  "data": {
    "plan": "1. Review auth flow\n2. Identify bug\n3. Write fix\n4. Add tests",
    "estimated_effort": "medium",
    "breakdown": [
      {"subtask": "Review auth flow", "assignee": "ba-agent"},
      {"subtask": "Implement fix", "assignee": "coding-agent"},
      {"subtask": "Run tests", "assignee": "qa-agent"}
    ]
  }
}
```

### 3. Event Bus Core Modules

#### 3.1 Event Store

```rust
// crates/todoki-server/src/event_bus/store.rs

use super::types::Event;
use anyhow::Result;

#[async_trait::async_trait]
pub trait EventStore: Send + Sync {
    /// Append a new event (returns assigned cursor)
    async fn append(&self, event: Event) -> Result<i64>;

    /// Query events by cursor range
    async fn query(
        &self,
        from_cursor: i64,
        to_cursor: Option<i64>,
        kinds: Option<Vec<String>>,
        agent_id: Option<Uuid>,
        task_id: Option<Uuid>,
        limit: Option<usize>,
    ) -> Result<Vec<Event>>;

    /// Get latest cursor
    async fn latest_cursor(&self) -> Result<i64>;

    /// Delete events older than timestamp (for retention policy)
    async fn prune_before(&self, before: DateTime<Utc>) -> Result<u64>;
}

/// PostgreSQL implementation
pub struct PgEventStore {
    pool: PgPool,
}

impl PgEventStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl EventStore for PgEventStore {
    async fn append(&self, mut event: Event) -> Result<i64> {
        let cursor = sqlx::query_scalar!(
            r#"
            INSERT INTO events (kind, time, agent_id, session_id, task_id, data)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING cursor
            "#,
            event.kind,
            event.time,
            event.agent_id,
            event.session_id,
            event.task_id,
            event.data
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(cursor)
    }

    async fn query(
        &self,
        from_cursor: i64,
        to_cursor: Option<i64>,
        kinds: Option<Vec<String>>,
        agent_id: Option<Uuid>,
        task_id: Option<Uuid>,
        limit: Option<usize>,
    ) -> Result<Vec<Event>> {
        let limit = limit.unwrap_or(1000).min(10000) as i64;

        let events = sqlx::query_as!(
            Event,
            r#"
            SELECT cursor, kind, time, agent_id, session_id, task_id, data
            FROM events
            WHERE cursor >= $1
              AND ($2::BIGINT IS NULL OR cursor <= $2)
              AND ($3::TEXT[] IS NULL OR kind = ANY($3))
              AND ($4::UUID IS NULL OR agent_id = $4)
              AND ($5::UUID IS NULL OR task_id = $5)
            ORDER BY cursor ASC
            LIMIT $6
            "#,
            from_cursor,
            to_cursor,
            kinds.as_deref(),
            agent_id,
            task_id,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }

    async fn latest_cursor(&self) -> Result<i64> {
        let cursor = sqlx::query_scalar!(
            "SELECT COALESCE(MAX(cursor), 0) FROM events"
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(cursor.unwrap_or(0))
    }

    async fn prune_before(&self, before: DateTime<Utc>) -> Result<u64> {
        let result = sqlx::query!(
            "DELETE FROM events WHERE time < $1",
            before
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}
```

#### 3.2 Event Publisher

```rust
// crates/todoki-server/src/event_bus/publisher.rs

use super::store::EventStore;
use super::types::Event;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct EventPublisher {
    store: Arc<dyn EventStore>,
    broadcaster: broadcast::Sender<Event>,
}

impl EventPublisher {
    pub fn new(store: Arc<dyn EventStore>) -> Self {
        let (broadcaster, _) = broadcast::channel(1024);
        Self { store, broadcaster }
    }

    /// Emit a new event (persists to store + broadcasts to subscribers)
    pub async fn emit(&self, mut event: Event) -> Result<i64> {
        // Assign cursor by storing
        let cursor = self.store.append(event.clone()).await?;
        event.cursor = cursor;

        // Broadcast to in-memory subscribers (best-effort)
        let _ = self.broadcaster.send(event);

        Ok(cursor)
    }

    /// Subscribe to real-time events (returns receiver)
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.broadcaster.subscribe()
    }
}
```

#### 3.3 Event Subscriber

```rust
// crates/todoki-server/src/event_bus/subscriber.rs

use super::store::EventStore;
use super::types::Event;
use anyhow::Result;
use std::sync::Arc;

pub struct EventSubscriber {
    store: Arc<dyn EventStore>,
}

impl EventSubscriber {
    pub fn new(store: Arc<dyn EventStore>) -> Self {
        Self { store }
    }

    /// Poll events since last cursor (for HTTP polling)
    pub async fn poll(
        &self,
        from_cursor: i64,
        kinds: Option<Vec<String>>,
        agent_id: Option<Uuid>,
        task_id: Option<Uuid>,
        limit: Option<usize>,
    ) -> Result<Vec<Event>> {
        self.store.query(from_cursor, None, kinds, agent_id, task_id, limit).await
    }

    /// Get latest cursor (for initialization)
    pub async fn latest_cursor(&self) -> Result<i64> {
        self.store.latest_cursor().await
    }

    /// Replay events between two cursors (for analysis)
    pub async fn replay(
        &self,
        from_cursor: i64,
        to_cursor: i64,
        kinds: Option<Vec<String>>,
    ) -> Result<Vec<Event>> {
        self.store.query(from_cursor, Some(to_cursor), kinds, None, None, None).await
    }
}
```

### 4. API Endpoints

#### 4.1 Event Bus HTTP API

```rust
// crates/todoki-server/src/api/event_bus.rs

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct EventQueryParams {
    /// Starting cursor (exclusive)
    pub cursor: i64,

    /// Event kinds to filter (comma-separated)
    pub kinds: Option<String>,

    /// Filter by agent ID
    pub agent_id: Option<Uuid>,

    /// Filter by task ID
    pub task_id: Option<Uuid>,

    /// Max events to return (default: 100, max: 1000)
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct EventQueryResponse {
    pub events: Vec<Event>,
    pub next_cursor: i64,
}

/// GET /api/event-bus
/// Query events with cursor-based pagination
pub async fn query_events(
    State(subscriber): State<Arc<EventSubscriber>>,
    Query(params): Query<EventQueryParams>,
) -> Result<Json<EventQueryResponse>, StatusCode> {
    let kinds = params.kinds.map(|s| s.split(',').map(String::from).collect());

    let events = subscriber
        .poll(params.cursor, kinds, params.agent_id, params.task_id, params.limit)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let next_cursor = events.last().map(|e| e.cursor).unwrap_or(params.cursor);

    Ok(Json(EventQueryResponse { events, next_cursor }))
}

/// GET /api/event-bus/latest
/// Get latest cursor (for initialization)
pub async fn get_latest_cursor(
    State(subscriber): State<Arc<EventSubscriber>>,
) -> Result<Json<i64>, StatusCode> {
    let cursor = subscriber
        .latest_cursor()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(cursor))
}

#[derive(Debug, Deserialize)]
pub struct ReplayParams {
    pub from_cursor: i64,
    pub to_cursor: i64,
    pub kinds: Option<String>,
}

/// POST /api/event-bus/replay
/// Replay historical events (for analysis)
pub async fn replay_events(
    State(subscriber): State<Arc<EventSubscriber>>,
    Query(params): Query<ReplayParams>,
) -> Result<Json<Vec<Event>>, StatusCode> {
    let kinds = params.kinds.map(|s| s.split(',').map(String::from).collect());

    let events = subscriber
        .replay(params.from_cursor, params.to_cursor, kinds)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(events))
}

#[derive(Debug, Deserialize)]
pub struct EmitEventRequest {
    pub kind: String,
    pub data: serde_json::Value,
    pub task_id: Option<Uuid>,
}

/// POST /api/event-bus/emit
/// Emit a new event (for standalone agents)
pub async fn emit_event(
    State(publisher): State<Arc<EventPublisher>>,
    auth: AuthenticatedAgent,  // Extract agent ID from JWT
    Json(req): Json<EmitEventRequest>,
) -> Result<Json<i64>, StatusCode> {
    let event = Event {
        cursor: 0,  // Will be assigned
        kind: req.kind,
        time: Utc::now(),
        agent_id: auth.agent_id,
        session_id: None,
        task_id: req.task_id,
        data: req.data,
    };

    let cursor = publisher
        .emit(event)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(cursor))
}
```

#### 4.2 WebSocket Subscription

```rust
// crates/todoki-server/src/api/event_bus_ws.rs

use axum::{
    extract::{ws::WebSocket, Query, State, WebSocketUpgrade},
    response::Response,
};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct WsSubscribeParams {
    /// Event kinds to subscribe (comma-separated)
    pub kinds: Option<String>,

    /// Starting cursor (if resuming)
    pub cursor: Option<i64>,
}

/// GET /ws/event-bus
/// Subscribe to real-time events via WebSocket
pub async fn event_bus_websocket(
    ws: WebSocketUpgrade,
    State(publisher): State<Arc<EventPublisher>>,
    State(subscriber): State<Arc<EventSubscriber>>,
    Query(params): Query<WsSubscribeParams>,
    auth: AuthenticatedAgent,
) -> Response {
    ws.on_upgrade(|socket| handle_event_bus_socket(socket, publisher, subscriber, params, auth))
}

async fn handle_event_bus_socket(
    socket: WebSocket,
    publisher: Arc<EventPublisher>,
    subscriber: Arc<EventSubscriber>,
    params: WsSubscribeParams,
    auth: AuthenticatedAgent,
) {
    let (mut tx, mut rx) = socket.split();

    let kinds_filter: Option<Vec<String>> = params.kinds.map(|s| s.split(',').map(String::from).collect());

    // Step 1: Send historical events if cursor provided
    if let Some(cursor) = params.cursor {
        match subscriber.poll(cursor, kinds_filter.clone(), None, None, Some(100)).await {
            Ok(events) => {
                for event in events {
                    if should_send_event(&event, &kinds_filter, auth.agent_id) {
                        let _ = tx.send(axum::extract::ws::Message::Text(
                            serde_json::to_string(&event).unwrap()
                        )).await;
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to fetch historical events: {}", e);
            }
        }
    }

    // Step 2: Subscribe to real-time events
    let mut event_rx = publisher.subscribe();

    loop {
        tokio::select! {
            // Receive events from broadcast
            event = event_rx.recv() => {
                match event {
                    Ok(event) => {
                        if should_send_event(&event, &kinds_filter, auth.agent_id) {
                            let msg = axum::extract::ws::Message::Text(
                                serde_json::to_string(&event).unwrap()
                            );
                            if tx.send(msg).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Event stream lagged by {} events", n);
                        // TODO: Send catch-up events
                    }
                    Err(_) => break,
                }
            }

            // Handle client messages (ping/pong, subscription updates)
            msg = rx.next() => {
                match msg {
                    Some(Ok(axum::extract::ws::Message::Close(_))) => break,
                    Some(Ok(axum::extract::ws::Message::Ping(data))) => {
                        let _ = tx.send(axum::extract::ws::Message::Pong(data)).await;
                    }
                    Some(Err(e)) => {
                        tracing::error!("WebSocket error: {}", e);
                        break;
                    }
                    None => break,
                    _ => {}
                }
            }
        }
    }
}

fn should_send_event(event: &Event, kinds_filter: &Option<Vec<String>>, _agent_id: Uuid) -> bool {
    // Filter by event kind
    if let Some(ref kinds) = kinds_filter {
        if !kinds.iter().any(|k| event.kind.starts_with(k)) {
            return false;
        }
    }

    // TODO: Add permission checks (agents can only see events they're authorized for)

    true
}
```

### 5. Agent Subscription & Triggering

#### 5.1 Agent Subscription Model

```rust
// Update: crates/todoki-server/src/models/agent.rs

#[derive(Domain)]
#[domain(table = "agents")]
pub struct Agent {
    // ... existing fields ...

    /// Event kinds this agent subscribes to (triggers on match)
    #[serde(default)]
    pub subscribed_events: Vec<String>,

    /// Last processed cursor (for incremental consumption)
    #[serde(default)]
    pub last_cursor: i64,

    /// Auto-start on matching event?
    #[serde(default)]
    pub auto_trigger: bool,
}
```

**Database Migration**:

```sql
-- migrations/010_agent_subscriptions.sql

ALTER TABLE agents
ADD COLUMN subscribed_events TEXT[] DEFAULT '{}',
ADD COLUMN last_cursor BIGINT DEFAULT 0,
ADD COLUMN auto_trigger BOOLEAN DEFAULT false;

CREATE INDEX idx_agents_subscribed_events ON agents USING GIN(subscribed_events);
```

#### 5.2 Event Orchestrator

```rust
// crates/todoki-server/src/event_bus/orchestrator.rs

use super::{publisher::EventPublisher, subscriber::EventSubscriber, types::Event};
use crate::models::agent::Agent;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct EventOrchestrator {
    publisher: Arc<EventPublisher>,
    subscriber: Arc<EventSubscriber>,
    db: Arc<Database>,
    active: Arc<RwLock<bool>>,
}

impl EventOrchestrator {
    pub fn new(
        publisher: Arc<EventPublisher>,
        subscriber: Arc<EventSubscriber>,
        db: Arc<Database>,
    ) -> Self {
        Self {
            publisher,
            subscriber,
            db,
            active: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the orchestrator (background task)
    pub async fn start(&self) -> Result<()> {
        *self.active.write().await = true;

        let publisher = self.publisher.clone();
        let subscriber = self.subscriber.clone();
        let db = self.db.clone();
        let active = self.active.clone();

        tokio::spawn(async move {
            let mut event_rx = publisher.subscribe();

            while *active.read().await {
                match event_rx.recv().await {
                    Ok(event) => {
                        if let Err(e) = Self::handle_event(&event, &db).await {
                            tracing::error!("Failed to handle event {}: {}", event.cursor, e);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Event receive error: {}", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the orchestrator
    pub async fn stop(&self) {
        *self.active.write().await = false;
    }

    /// Handle a single event (check subscriptions, trigger agents)
    async fn handle_event(event: &Event, db: &Arc<Database>) -> Result<()> {
        // Find agents subscribed to this event kind
        let agents = db.list_agents_by_subscription(&event.kind).await?;

        for agent in agents {
            if !agent.auto_trigger {
                continue;
            }

            // Skip if agent already running
            if agent.status == AgentStatus::Running {
                continue;
            }

            // Update last_cursor
            db.update_agent_cursor(agent.id, event.cursor).await?;

            // Trigger agent (spawn session)
            match Self::trigger_agent(&agent, event, db).await {
                Ok(_) => {
                    tracing::info!(
                        "Agent {} triggered by event {} (kind: {})",
                        agent.name, event.cursor, event.kind
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to trigger agent {} for event {}: {}",
                        agent.name, event.cursor, e
                    );
                }
            }
        }

        Ok(())
    }

    /// Trigger an agent in response to an event
    async fn trigger_agent(agent: &Agent, event: &Event, db: &Arc<Database>) -> Result<()> {
        // TODO: Call relay to spawn session
        // This will be integrated with existing relay RPC mechanism

        // Create session record
        let session = AgentSession {
            id: Uuid::new_v4(),
            agent_id: agent.id,
            relay_id: agent.relay_id.clone(),
            status: SessionStatus::Running,
            trigger_event_cursor: Some(event.cursor),
            // ... other fields
        };

        db.create_agent_session(&session).await?;

        // Send RPC to relay (existing mechanism)
        // relay_manager.spawn_session(...).await?;

        Ok(())
    }
}
```

### 6. Integration with Existing Code

#### 6.1 Emit Events from Existing Flows

**Task creation** (`crates/todoki-server/src/api/tasks.rs`):

```rust
pub async fn create_task(
    State(db): State<Arc<Database>>,
    State(publisher): State<Arc<EventPublisher>>,  // NEW
    Json(req): Json<CreateTaskRequest>,
) -> Result<Json<Task>, StatusCode> {
    let task = db.create_task(&req).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Emit event
    publisher.emit(Event {
        cursor: 0,
        kind: kinds::TASK_CREATED.to_string(),
        time: Utc::now(),
        agent_id: Uuid::nil(),  // System event
        session_id: None,
        task_id: Some(task.id),
        data: serde_json::json!({
            "content": task.content,
            "priority": task.priority,
            "project_id": task.project_id,
        }),
    }).await.ok();

    Ok(Json(task))
}
```

**Agent output** (`crates/todoki-server/src/relay/broadcaster.rs`):

```rust
pub async fn broadcast_output(
    &self,
    agent_id: Uuid,
    session_id: Uuid,
    stream: &str,
    message: &str,
    publisher: &Arc<EventPublisher>,  // NEW
) -> Result<()> {
    // Existing broadcast logic
    // ...

    // Also emit to event bus
    publisher.emit(Event {
        cursor: 0,
        kind: kinds::AGENT_OUTPUT.to_string(),
        time: Utc::now(),
        agent_id,
        session_id: Some(session_id),
        task_id: None,  // TODO: lookup from session
        data: serde_json::json!({
            "stream": stream,
            "message": message,
        }),
    }).await.ok();

    Ok(())
}
```

#### 6.2 Agent Polling via Relay

For relay-managed agents, add event polling capability:

```rust
// crates/todoki-relay/src/event_poller.rs

pub struct EventPoller {
    agent_id: Uuid,
    last_cursor: Arc<RwLock<i64>>,
    server_url: String,
    token: String,
}

impl EventPoller {
    pub async fn poll_once(&self) -> Result<Vec<Event>> {
        let cursor = *self.last_cursor.read().await;

        let url = format!(
            "{}/api/event-bus?cursor={}&agent_id={}",
            self.server_url, cursor, self.agent_id
        );

        let resp: EventQueryResponse = reqwest::Client::new()
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?
            .json()
            .await?;

        if !resp.events.is_empty() {
            *self.last_cursor.write().await = resp.next_cursor;
        }

        Ok(resp.events)
    }
}
```

### 7. Standalone Agent Support

#### 7.1 Self-Registration API

```rust
// crates/todoki-server/src/api/agents.rs

#[derive(Debug, Deserialize)]
pub struct RegisterAgentRequest {
    pub name: String,
    pub role: AgentRole,
    pub subscribed_events: Vec<String>,
    pub auto_trigger: bool,
}

#[derive(Debug, Serialize)]
pub struct RegisterAgentResponse {
    pub agent_id: Uuid,
    pub token: String,  // JWT for authentication
}

/// POST /api/agents/register
/// Allow standalone agents to self-register
pub async fn register_agent(
    State(db): State<Arc<Database>>,
    State(jwt_secret): State<String>,
    Json(req): Json<RegisterAgentRequest>,
) -> Result<Json<RegisterAgentResponse>, StatusCode> {
    let agent = Agent {
        id: Uuid::new_v4(),
        name: req.name,
        command: String::new(),  // Standalone (no command)
        args: vec![],
        execution_mode: ExecutionMode::Remote,
        role: req.role,
        project_id: Uuid::nil(),  // TODO: Associate with project
        relay_id: None,
        status: AgentStatus::Created,
        subscribed_events: req.subscribed_events,
        last_cursor: 0,
        auto_trigger: req.auto_trigger,
    };

    db.insert_agent(&agent).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Generate JWT token
    let token = generate_agent_token(agent.id, &jwt_secret)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(RegisterAgentResponse {
        agent_id: agent.id,
        token,
    }))
}
```

#### 7.2 Standalone Agent SDK (Example in Python)

```python
# agents/sdk/python/todoki_agent/__init__.py

import requests
from typing import List, Dict, Any, Optional
import time

class TodokiAgent:
    def __init__(self, server_url: str, name: str, role: str, subscribed_events: List[str]):
        self.server_url = server_url.rstrip('/')
        self.name = name
        self.role = role
        self.subscribed_events = subscribed_events
        self.agent_id = None
        self.token = None
        self.last_cursor = 0

    def register(self) -> None:
        """Register agent with server"""
        resp = requests.post(
            f"{self.server_url}/api/agents/register",
            json={
                "name": self.name,
                "role": self.role,
                "subscribed_events": self.subscribed_events,
                "auto_trigger": False,  # Manual triggering
            }
        )
        resp.raise_for_status()
        data = resp.json()
        self.agent_id = data["agent_id"]
        self.token = data["token"]
        print(f"Registered as agent {self.agent_id}")

    def poll_events(self) -> List[Dict[str, Any]]:
        """Poll for new events"""
        resp = requests.get(
            f"{self.server_url}/api/event-bus",
            params={
                "cursor": self.last_cursor,
                "kinds": ",".join(self.subscribed_events),
                "limit": 100,
            },
            headers={"Authorization": f"Bearer {self.token}"}
        )
        resp.raise_for_status()
        data = resp.json()
        events = data["events"]
        if events:
            self.last_cursor = data["next_cursor"]
        return events

    def emit_event(self, kind: str, data: Dict[str, Any], task_id: Optional[str] = None) -> int:
        """Emit a new event"""
        resp = requests.post(
            f"{self.server_url}/api/event-bus/emit",
            json={
                "kind": kind,
                "data": data,
                "task_id": task_id,
            },
            headers={"Authorization": f"Bearer {self.token}"}
        )
        resp.raise_for_status()
        cursor = resp.json()
        return cursor

    def run(self, handler):
        """Main event loop"""
        print(f"Agent {self.name} listening for events: {self.subscribed_events}")
        while True:
            try:
                events = self.poll_events()
                for event in events:
                    handler(self, event)
            except KeyboardInterrupt:
                print("Shutting down...")
                break
            except Exception as e:
                print(f"Error: {e}")
            time.sleep(1)  # Poll every 1 second

# Example usage
if __name__ == "__main__":
    def handle_event(agent: TodokiAgent, event: Dict[str, Any]):
        print(f"Received event: {event['kind']} (cursor: {event['cursor']})")

        if event["kind"] == "task.created":
            # Analyze requirement
            task_id = event["task_id"]
            content = event["data"]["content"]

            # Do some work (call LLM, etc.)
            plan = f"Analysis of '{content}': ..."

            # Emit result
            agent.emit_event(
                kind="agent.requirement_analyzed",
                data={"plan": plan, "estimated_effort": "medium"},
                task_id=task_id
            )

    agent = TodokiAgent(
        server_url="http://localhost:3000",
        name="pm-agent",
        role="general",
        subscribed_events=["task.created"]
    )
    agent.register()
    agent.run(handle_event)
```

## Implementation Phases

### Phase 1: Event Storage & Basic API (2-3 days)

**Goals**:
- Persistent event storage in PostgreSQL
- HTTP API for querying events (cursor-based)
- Event emission from existing workflows

**Deliverables**:
- Database migration (events table)
- `event_bus` module (store, publisher, subscriber)
- API endpoints (`GET /api/event-bus`, `POST /api/event-bus/emit`)
- Integration: emit events on task creation, status change, agent start/stop

**Files to create/modify**:
- `migrations/009_create_events_table.sql`
- `crates/todoki-server/src/event_bus/` (new module)
- `crates/todoki-server/src/api/event_bus.rs`
- `crates/todoki-server/src/api/tasks.rs` (add event emission)
- `crates/todoki-server/src/relay/broadcaster.rs` (add event emission)

**Testing**:
- Create task → verify `task.created` event in DB
- Query events by cursor
- Emit custom event via API

### Phase 2: Agent Subscription & Triggering (3-4 days)

**Goals**:
- Agents can subscribe to event kinds
- Auto-trigger agents on matching events
- Relay-managed agents poll events

**Deliverables**:
- Agent model update (subscribed_events, last_cursor, auto_trigger)
- Event orchestrator (background task)
- Relay event poller integration

**Files to create/modify**:
- `migrations/010_agent_subscriptions.sql`
- `crates/todoki-server/src/models/agent.rs`
- `crates/todoki-server/src/event_bus/orchestrator.rs`
- `crates/todoki-relay/src/event_poller.rs`
- `crates/mock-agent/src/main.rs` (add event handling)

**Testing**:
- Create agent with subscribed_events=["task.created"]
- Create task → agent auto-triggers
- Verify agent cursor updates

### Phase 3: Agent-to-Agent Workflows (4-5 days)

**Goals**:
- PM → BA → Coding → QA event-driven pipeline
- Frontend event timeline visualization

**Deliverables**:
- Agent role-specific event handlers
- WebSocket subscription (`ws://server/ws/event-bus`)
- Frontend task detail page with event timeline

**Files to create/modify**:
- `crates/todoki-server/src/api/event_bus_ws.rs`
- `crates/todoki-server/src/agent_roles/` (role-specific handlers)
- `web/src/components/EventTimeline.tsx`
- `web/src/pages/TaskDetail.tsx` (add timeline)

**Testing**:
- Create task with PM agent
- PM emits `requirement_analyzed`
- BA agent triggers, emits `business_context_ready`
- Coding agent triggers, emits `artifact.created`
- QA agent triggers, emits `qa_test_passed`
- Task status → Done

### Phase 4: Standalone Agent Support (2-3 days)

**Goals**:
- Self-registration API
- Python SDK for standalone agents
- Documentation & examples

**Deliverables**:
- `/api/agents/register` endpoint
- JWT authentication for agents
- Python SDK (`todoki-agent` package)
- Example standalone agents (PM, BA, QA)

**Files to create/modify**:
- `crates/todoki-server/src/api/agents.rs` (add registration)
- `agents/sdk/python/todoki_agent/` (new SDK)
- `agents/examples/pm_agent.py`
- `docs/standalone-agents.md`

**Testing**:
- Run standalone PM agent in Docker
- Agent registers, subscribes to events
- Agent processes task.created → emits requirement_analyzed
- Verify event flow

### Phase 5: Event Replay & Analytics (Optional, 2-3 days)

**Goals**:
- Checkpoint-based event replay
- Aggregate event statistics
- Skill extraction from event patterns

**Deliverables**:
- `/api/event-bus/replay` endpoint
- Event analysis UI (admin page)
- Skill recommendation based on event patterns

**Files to create/modify**:
- `crates/todoki-server/src/event_bus/analytics.rs`
- `web/src/pages/EventAnalytics.tsx`

## Comparison: Before vs After

| Aspect | Current (v0.1) | After Event Bus (v0.2) |
|--------|----------------|------------------------|
| **Event Storage** | In-memory broadcast only | Persistent DB + JSONL export |
| **Event Semantics** | stdout/stderr/system/acp | Rich business events (task.*, agent.*, artifact.*) |
| **Agent Awareness** | Passive (task-assigned only) | Active (subscribe to system events) |
| **Agent Collaboration** | Indirect (via task status) | Direct (event-driven pipelines) |
| **Event Replay** | Not supported | Full replay with cursor ranges |
| **Agent Deployment** | Relay-managed only | Relay-managed + Standalone (HTTP/WS) |
| **Debugging** | View agent logs | Replay event sequences |
| **Extensibility** | Add code for new agents | Agents self-register via API |

## Security Considerations

1. **Event Visibility**: Not all agents should see all events
   - Implement permission checks in WebSocket/HTTP handlers
   - Filter events by project/team membership

2. **Event Tampering**: Prevent malicious event emission
   - JWT authentication for all API calls
   - Validate event schemas before storing

3. **Rate Limiting**: Prevent event spam
   - Rate-limit `/api/event-bus/emit` per agent
   - Max events per minute/hour

4. **Data Retention**: Don't store events forever
   - Implement retention policy (e.g., 90 days)
   - Archive old events to S3/object storage

## Open Questions

1. **Event Schema Versioning**: How to handle event data structure changes over time?
   - **Proposal**: Add `schema_version` field, support multiple versions in readers

2. **Event Ordering Guarantees**: What if events arrive out of order?
   - **Current**: Cursor is globally monotonic (DB sequence)
   - **Future**: Add vector clocks for distributed scenarios

3. **Large Event Data**: What if event payload is huge (e.g., full file diff)?
   - **Proposal**: Store large payloads in object storage, event contains URL

4. **Cross-Project Events**: Can agents see events from other projects?
   - **Proposal**: Add `project_id` to events, filter by project membership

## References

- Original design: [agent-bus.md](../agent-bus.md)
- Existing relay protocol: [todoki-protocol](../crates/todoki-protocol/)
- Agent model: [models/agent.rs](../crates/todoki-server/src/models/agent.rs)

## Changelog

- **2026-02-27**: Initial design document (v1.0)
