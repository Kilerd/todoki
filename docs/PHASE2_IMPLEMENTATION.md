# Phase 2 Implementation Plan: Agent Subscription & Triggering

## Overview

This document provides detailed implementation steps for Phase 2 of the Event Bus architecture, focusing on enabling agents to subscribe to events and auto-trigger based on event patterns.

**Duration**: 3-4 days
**Prerequisites**: Phase 1 completed (event storage + basic API)
**Related**: [event-bus-design.md](./event-bus-design.md)

## Goals

- [x] Agents can declare which event kinds they're interested in
- [x] Agents maintain cursor position for incremental event consumption
- [x] Background orchestrator monitors events and triggers subscribed agents
- [x] Relay-managed agents can poll for events
- [x] Mock agent demonstrates event-driven behavior

## Architecture

```
┌──────────────────────────────────────┐
│  Event Bus (from Phase 1)            │
│  - Event Store (PostgreSQL)          │
│  - Publisher (emit events)           │
│  - Subscriber (query events)         │
└────────────┬─────────────────────────┘
             │
             ↓ (new events broadcast)
┌────────────┴─────────────────────────┐
│  Event Orchestrator                  │
│  - Subscribe to new events           │
│  - Match against agent subscriptions │
│  - Trigger agents via Relay RPC      │
│  - Update agent cursors              │
└────────────┬─────────────────────────┘
             │
             ↓ (spawn_session RPC)
┌────────────┴─────────────────────────┐
│  Relay Manager (existing)            │
│  - Route RPC to relay                │
└────────────┬─────────────────────────┘
             │
             ↓ (WebSocket RPC)
┌────────────┴─────────────────────────┐
│  Relay (enhanced)                    │
│  - Spawn agent session               │
│  - NEW: Poll events for agent        │
│  - Forward events to agent stdin     │
└────────────┬─────────────────────────┘
             │
             ↓ (stdin/stdout via ACP)
┌────────────┴─────────────────────────┐
│  Agent (mock-agent enhanced)         │
│  - Read events from stdin            │
│  - Process events                    │
│  - Emit new events via stdout        │
└──────────────────────────────────────┘
```

## Detailed Tasks

### Task 1: Database Schema Migration (30 min)

**Goal**: Add subscription-related fields to `agents` table.

**File**: `migrations/010_agent_subscriptions.sql`

```sql
-- Add agent subscription fields
ALTER TABLE agents
ADD COLUMN subscribed_events TEXT[] DEFAULT '{}',
ADD COLUMN last_cursor BIGINT DEFAULT 0,
ADD COLUMN auto_trigger BOOLEAN DEFAULT false;

-- Index for efficient subscription matching
CREATE INDEX idx_agents_subscribed_events ON agents USING GIN(subscribed_events);

-- Index for finding agents that need cursor updates
CREATE INDEX idx_agents_auto_trigger ON agents(auto_trigger) WHERE auto_trigger = true;

COMMENT ON COLUMN agents.subscribed_events IS 'Event kinds this agent listens to (e.g., ["task.created", "agent.requirement_analyzed"])';
COMMENT ON COLUMN agents.last_cursor IS 'Last event cursor processed by this agent';
COMMENT ON COLUMN agents.auto_trigger IS 'Whether to automatically trigger this agent on matching events';
```

**Testing**:
```bash
# Run migration
sqlx migrate run

# Verify schema
psql $DATABASE_URL -c "\d agents"
```

---

### Task 2: Update Agent Model (30 min)

**Goal**: Add subscription fields to Rust agent model.

**File**: `crates/todoki-server/src/models/agent.rs`

```rust
#[derive(Domain, Debug, Clone, Serialize, Deserialize)]
#[domain(table = "agents")]
pub struct Agent {
    pub id: Uuid,
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub execution_mode: ExecutionMode,
    pub role: AgentRole,
    pub project_id: Uuid,
    pub relay_id: Option<String>,
    pub status: AgentStatus,

    // NEW: Subscription fields
    #[serde(default)]
    pub subscribed_events: Vec<String>,

    #[serde(default)]
    pub last_cursor: i64,

    #[serde(default)]
    pub auto_trigger: bool,

    // ... existing timestamps ...
}

// Add helper methods
impl Agent {
    /// Check if agent subscribes to a given event kind
    pub fn subscribes_to(&self, event_kind: &str) -> bool {
        self.subscribed_events.iter().any(|pattern| {
            // Support prefix matching (e.g., "task.*" matches "task.created")
            event_kind.starts_with(pattern.trim_end_matches('*'))
        })
    }

    /// Check if agent should be triggered
    pub fn should_trigger(&self, event_kind: &str) -> bool {
        self.auto_trigger
            && self.status != AgentStatus::Running
            && self.subscribes_to(event_kind)
    }
}
```

**File**: `crates/todoki-server/src/db/service.rs`

Add database queries:

```rust
impl Database {
    /// Find agents subscribed to a given event kind
    pub async fn list_agents_by_subscription(&self, event_kind: &str) -> Result<Vec<Agent>> {
        let agents = sqlx::query_as!(
            Agent,
            r#"
            SELECT *
            FROM agents
            WHERE $1 = ANY(subscribed_events)
               OR EXISTS (
                   SELECT 1
                   FROM unnest(subscribed_events) AS pattern
                   WHERE $1 LIKE REPLACE(pattern, '*', '%')
               )
            "#,
            event_kind
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(agents)
    }

    /// Update agent's last processed cursor
    pub async fn update_agent_cursor(&self, agent_id: Uuid, cursor: i64) -> Result<()> {
        sqlx::query!(
            "UPDATE agents SET last_cursor = $1 WHERE id = $2",
            cursor,
            agent_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update agent status
    pub async fn update_agent_status(&self, agent_id: Uuid, status: AgentStatus) -> Result<()> {
        sqlx::query!(
            "UPDATE agents SET status = $1 WHERE id = $2",
            status as AgentStatus,
            agent_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
```

**Testing**:
```rust
#[tokio::test]
async fn test_agent_subscription_matching() {
    let agent = Agent {
        subscribed_events: vec!["task.created".into(), "agent.*".into()],
        auto_trigger: true,
        status: AgentStatus::Created,
        ..Default::default()
    };

    assert!(agent.should_trigger("task.created"));
    assert!(agent.should_trigger("agent.requirement_analyzed"));
    assert!(!agent.should_trigger("artifact.created"));
}
```

---

### Task 3: Event Orchestrator Core (2 hours)

**Goal**: Background service that monitors events and triggers agents.

**File**: `crates/todoki-server/src/event_bus/orchestrator.rs`

```rust
use super::{publisher::EventPublisher, types::Event};
use crate::db::Database;
use crate::models::agent::{Agent, AgentStatus};
use crate::relay::manager::RelayManager;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

pub struct EventOrchestrator {
    publisher: Arc<EventPublisher>,
    db: Arc<Database>,
    relay_manager: Arc<RelayManager>,
    active: Arc<RwLock<bool>>,
}

impl EventOrchestrator {
    pub fn new(
        publisher: Arc<EventPublisher>,
        db: Arc<Database>,
        relay_manager: Arc<RelayManager>,
    ) -> Self {
        Self {
            publisher,
            db,
            relay_manager,
            active: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the orchestrator (spawns background task)
    pub async fn start(&self) -> Result<()> {
        if *self.active.read().await {
            warn!("Orchestrator already running");
            return Ok(());
        }

        *self.active.write().await = true;

        let publisher = self.publisher.clone();
        let db = self.db.clone();
        let relay_manager = self.relay_manager.clone();
        let active = self.active.clone();

        tokio::spawn(async move {
            info!("Event orchestrator started");

            let mut event_rx = publisher.subscribe();

            while *active.read().await {
                match event_rx.recv().await {
                    Ok(event) => {
                        if let Err(e) = Self::handle_event(&event, &db, &relay_manager).await {
                            error!("Failed to handle event {}: {}", event.cursor, e);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!("Event orchestrator lagged by {} events", n);
                    }
                    Err(e) => {
                        error!("Event receive error: {}", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                }
            }

            info!("Event orchestrator stopped");
        });

        Ok(())
    }

    /// Stop the orchestrator
    pub async fn stop(&self) {
        *self.active.write().await = false;
    }

    /// Handle a single event (check subscriptions, trigger agents)
    async fn handle_event(
        event: &Event,
        db: &Arc<Database>,
        relay_manager: &Arc<RelayManager>,
    ) -> Result<()> {
        // Find agents subscribed to this event kind
        let agents = db.list_agents_by_subscription(&event.kind).await?;

        if agents.is_empty() {
            return Ok(());
        }

        info!(
            "Event {} (kind: {}) matched {} agent(s)",
            event.cursor,
            event.kind,
            agents.len()
        );

        for agent in agents {
            // Check if agent should be triggered
            if !agent.should_trigger(&event.kind) {
                continue;
            }

            // Update last_cursor before triggering (prevent duplicate triggers)
            if let Err(e) = db.update_agent_cursor(agent.id, event.cursor).await {
                error!("Failed to update cursor for agent {}: {}", agent.name, e);
                continue;
            }

            // Trigger agent
            match Self::trigger_agent(&agent, event, db, relay_manager).await {
                Ok(_) => {
                    info!(
                        "Agent '{}' triggered by event {} (kind: {})",
                        agent.name, event.cursor, event.kind
                    );
                }
                Err(e) => {
                    error!(
                        "Failed to trigger agent '{}' for event {}: {}",
                        agent.name, event.cursor, e
                    );
                }
            }
        }

        Ok(())
    }

    /// Trigger an agent in response to an event
    async fn trigger_agent(
        agent: &Agent,
        event: &Event,
        db: &Arc<Database>,
        relay_manager: &Arc<RelayManager>,
    ) -> Result<()> {
        use crate::models::agent_session::{AgentSession, SessionStatus};
        use todoki_protocol::{RpcRequest, SpawnSessionParams};

        // Check if relay is available
        let relay_id = agent.relay_id.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Agent '{}' has no relay assigned", agent.name)
        })?;

        if !relay_manager.is_relay_connected(relay_id).await {
            return Err(anyhow::anyhow!("Relay '{}' is not connected", relay_id));
        }

        // Create session record
        let session_id = Uuid::new_v4();
        let session = AgentSession {
            id: session_id,
            agent_id: agent.id,
            relay_id: Some(relay_id.clone()),
            status: SessionStatus::Running,
            trigger_event_cursor: Some(event.cursor),
            started_at: Utc::now(),
            finished_at: None,
        };

        db.create_agent_session(&session).await?;

        // Update agent status
        db.update_agent_status(agent.id, AgentStatus::Running).await?;

        // Build spawn request
        let workdir = format!("/tmp/todoki-agent-{}", agent.id);  // TODO: configurable
        let mut env = std::collections::HashMap::new();
        env.insert("TRIGGER_EVENT_KIND".into(), event.kind.clone());
        env.insert("TRIGGER_EVENT_CURSOR".into(), event.cursor.to_string());
        env.insert("TRIGGER_EVENT_DATA".into(), event.data.to_string());

        if let Some(task_id) = event.task_id {
            env.insert("TASK_ID".into(), task_id.to_string());
        }

        let params = SpawnSessionParams {
            agent_id: agent.id.to_string(),
            session_id: session_id.to_string(),
            workdir,
            command: agent.command.clone(),
            args: agent.args.clone(),
            env,
            setup_script: None,
        };

        // Send RPC to relay
        relay_manager
            .send_rpc(relay_id, RpcRequest::SpawnSession(params))
            .await?;

        Ok(())
    }
}
```

**Testing**:
```rust
#[tokio::test]
async fn test_orchestrator_triggers_agent() {
    // Setup: create agent with subscription
    let agent = db.insert_agent(Agent {
        name: "test-agent".into(),
        subscribed_events: vec!["task.created".into()],
        auto_trigger: true,
        ..Default::default()
    }).await.unwrap();

    // Start orchestrator
    let orchestrator = EventOrchestrator::new(publisher.clone(), db.clone(), relay_manager.clone());
    orchestrator.start().await.unwrap();

    // Emit event
    let event = Event {
        kind: "task.created".into(),
        data: json!({"task_id": "test-123"}),
        ..Default::default()
    };
    publisher.emit(event).await.unwrap();

    // Wait for trigger
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify agent was triggered
    let agent = db.get_agent(agent.id).await.unwrap();
    assert_eq!(agent.status, AgentStatus::Running);
}
```

---

### Task 4: Integrate Orchestrator into Server (30 min)

**File**: `crates/todoki-server/src/main.rs`

```rust
// Add orchestrator to server state
#[tokio::main]
async fn main() -> Result<()> {
    // ... existing setup ...

    // Initialize event bus (from Phase 1)
    let event_store = Arc::new(PgEventStore::new(pool.clone()));
    let event_publisher = Arc::new(EventPublisher::new(event_store.clone()));
    let event_subscriber = Arc::new(EventSubscriber::new(event_store.clone()));

    // NEW: Initialize orchestrator
    let orchestrator = Arc::new(EventOrchestrator::new(
        event_publisher.clone(),
        db.clone(),
        relay_manager.clone(),
    ));

    // Start orchestrator
    orchestrator.start().await?;
    info!("Event orchestrator started");

    // ... existing server setup ...

    // Graceful shutdown
    tokio::select! {
        _ = server => {},
        _ = tokio::signal::ctrl_c() => {
            info!("Shutting down...");
            orchestrator.stop().await;
        }
    }

    Ok(())
}
```

---

### Task 5: Relay Event Poller (1.5 hours)

**Goal**: Enable relay-managed agents to poll events from server.

**File**: `crates/todoki-relay/src/event_poller.rs`

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub cursor: i64,
    pub kind: String,
    pub time: String,
    pub agent_id: Uuid,
    pub session_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct EventQueryResponse {
    events: Vec<Event>,
    next_cursor: i64,
}

pub struct EventPoller {
    agent_id: Uuid,
    last_cursor: Arc<RwLock<i64>>,
    server_url: String,
    token: String,
    kinds: Vec<String>,
}

impl EventPoller {
    pub fn new(
        agent_id: Uuid,
        server_url: String,
        token: String,
        kinds: Vec<String>,
    ) -> Self {
        Self {
            agent_id,
            last_cursor: Arc::new(RwLock::new(0)),
            server_url,
            token,
            kinds,
        }
    }

    /// Poll once for new events
    pub async fn poll_once(&self) -> Result<Vec<Event>> {
        let cursor = *self.last_cursor.read().await;

        let url = format!(
            "{}/api/event-bus?cursor={}&kinds={}",
            self.server_url,
            cursor,
            self.kinds.join(",")
        );

        debug!("Polling events from cursor {}", cursor);

        let resp = reqwest::Client::new()
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(
                "Event poll failed: {}",
                resp.status()
            ));
        }

        let data: EventQueryResponse = resp.json().await?;

        if !data.events.is_empty() {
            *self.last_cursor.write().await = data.next_cursor;
            debug!("Polled {} events, next cursor: {}", data.events.len(), data.next_cursor);
        }

        Ok(data.events)
    }

    /// Start continuous polling (spawns background task)
    pub async fn start_polling<F>(&self, handler: F)
    where
        F: Fn(Event) + Send + 'static,
    {
        let poller = self.clone();

        tokio::spawn(async move {
            loop {
                match poller.poll_once().await {
                    Ok(events) => {
                        for event in events {
                            handler(event);
                        }
                    }
                    Err(e) => {
                        error!("Event polling error: {}", e);
                    }
                }

                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });
    }
}

// Make cloneable for spawning
impl Clone for EventPoller {
    fn clone(&self) -> Self {
        Self {
            agent_id: self.agent_id,
            last_cursor: self.last_cursor.clone(),
            server_url: self.server_url.clone(),
            token: self.token.clone(),
            kinds: self.kinds.clone(),
        }
    }
}
```

**File**: `crates/todoki-relay/src/session.rs`

Integrate event poller into session management:

```rust
impl Session {
    // NEW: Start event polling for this session
    pub async fn start_event_polling(&mut self, poller: EventPoller) {
        let stdin_tx = self.stdin_tx.clone();

        poller.start_polling(move |event| {
            // Forward event to agent's stdin as JSON line
            let event_json = serde_json::to_string(&event).unwrap();
            let line = format!("EVENT: {}\n", event_json);

            if let Err(e) = stdin_tx.try_send(line.into_bytes()) {
                tracing::warn!("Failed to forward event to agent: {}", e);
            }
        }).await;
    }
}
```

---

### Task 6: Mock Agent Event Handling (1 hour)

**Goal**: Update mock-agent to demonstrate event-driven behavior.

**File**: `crates/mock-agent/src/main.rs`

```rust
// Add event parsing
#[derive(Debug, Deserialize)]
struct Event {
    cursor: i64,
    kind: String,
    time: String,
    agent_id: String,
    task_id: Option<String>,
    data: serde_json::Value,
}

async fn handle_stdin_line(line: &str) -> Result<()> {
    // Check if line is an event
    if line.starts_with("EVENT: ") {
        let event_json = line.strip_prefix("EVENT: ").unwrap();
        let event: Event = serde_json::from_str(event_json)?;

        info!("Received event: {} (cursor: {})", event.kind, event.cursor);

        // Handle different event kinds
        match event.kind.as_str() {
            "task.created" => handle_task_created(event).await?,
            "agent.requirement_analyzed" => handle_requirement_analyzed(event).await?,
            _ => {
                debug!("Ignoring event kind: {}", event.kind);
            }
        }

        return Ok(());
    }

    // Existing prompt handling
    // ...
}

async fn handle_task_created(event: Event) -> Result<()> {
    let content = event.data.get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("(no content)");

    println!("🤖 Analyzing task: {}", content);

    // Simulate analysis
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Emit analysis result
    let result = json!({
        "plan": format!("Analysis of '{}': Step 1, Step 2, Step 3", content),
        "estimated_effort": "medium",
        "breakdown": [
            {"subtask": "Research", "assignee": "ba-agent"},
            {"subtask": "Implementation", "assignee": "coding-agent"},
        ]
    });

    // Emit event via stdout (ACP format)
    println!(
        "ACP:EMIT_EVENT:{}",
        serde_json::to_string(&json!({
            "kind": "agent.requirement_analyzed",
            "data": result,
            "task_id": event.task_id,
        }))?
    );

    Ok(())
}

async fn handle_requirement_analyzed(event: Event) -> Result<()> {
    let plan = event.data.get("plan")
        .and_then(|v| v.as_str())
        .unwrap_or("(no plan)");

    println!("📋 Received analysis: {}", plan);

    // BA agent would load business context here
    // For now, just acknowledge
    println!("✅ Business context loaded");

    Ok(())
}
```

**Testing**:
```bash
# Manually test mock-agent
echo 'EVENT: {"cursor":1,"kind":"task.created","time":"2026-02-27T10:00:00Z","agent_id":"test","task_id":"abc-123","data":{"content":"Fix bug"}}' | cargo run --bin mock-agent

# Expected output:
# 🤖 Analyzing task: Fix bug
# ACP:EMIT_EVENT:{"kind":"agent.requirement_analyzed","data":{...},"task_id":"abc-123"}
```

---

### Task 7: Integration Testing (1 hour)

**File**: `crates/todoki-server/tests/event_orchestrator_test.rs`

```rust
#[tokio::test]
async fn test_full_event_driven_flow() {
    // Setup test environment
    let (db, publisher, relay_manager) = setup_test_env().await;

    // Create PM agent (listens to task.created)
    let pm_agent = db.insert_agent(Agent {
        name: "pm-agent".into(),
        command: "mock-agent".into(),
        subscribed_events: vec!["task.created".into()],
        auto_trigger: true,
        relay_id: Some("test-relay".into()),
        ..Default::default()
    }).await.unwrap();

    // Create BA agent (listens to agent.requirement_analyzed)
    let ba_agent = db.insert_agent(Agent {
        name: "ba-agent".into(),
        command: "mock-agent".into(),
        subscribed_events: vec!["agent.requirement_analyzed".into()],
        auto_trigger: true,
        relay_id: Some("test-relay".into()),
        ..Default::default()
    }).await.unwrap();

    // Start orchestrator
    let orchestrator = EventOrchestrator::new(
        publisher.clone(),
        db.clone(),
        relay_manager.clone(),
    );
    orchestrator.start().await.unwrap();

    // Create task (emits task.created)
    let task = db.create_task(CreateTaskRequest {
        content: "Implement feature X".into(),
        priority: 1,
        ..Default::default()
    }).await.unwrap();

    publisher.emit(Event {
        kind: "task.created".into(),
        task_id: Some(task.id),
        data: json!({"content": task.content}),
        ..Default::default()
    }).await.unwrap();

    // Wait for PM agent to trigger
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify PM agent started
    let pm_agent = db.get_agent(pm_agent.id).await.unwrap();
    assert_eq!(pm_agent.status, AgentStatus::Running);
    assert!(pm_agent.last_cursor > 0);

    // Simulate PM agent emitting requirement_analyzed
    publisher.emit(Event {
        kind: "agent.requirement_analyzed".into(),
        agent_id: pm_agent.id,
        task_id: Some(task.id),
        data: json!({
            "plan": "Step 1, Step 2, Step 3",
            "estimated_effort": "medium"
        }),
        ..Default::default()
    }).await.unwrap();

    // Wait for BA agent to trigger
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify BA agent started
    let ba_agent = db.get_agent(ba_agent.id).await.unwrap();
    assert_eq!(ba_agent.status, AgentStatus::Running);

    orchestrator.stop().await;
}
```

---

## API Changes

### New Database Queries

```rust
// crates/todoki-server/src/db/service.rs

impl Database {
    pub async fn list_agents_by_subscription(&self, event_kind: &str) -> Result<Vec<Agent>>;
    pub async fn update_agent_cursor(&self, agent_id: Uuid, cursor: i64) -> Result<()>;
    pub async fn update_agent_status(&self, agent_id: Uuid, status: AgentStatus) -> Result<()>;
}
```

### Updated Agent Creation API

**Endpoint**: `POST /api/agents`

**Request Body** (new fields):
```json
{
  "name": "pm-agent",
  "command": "mock-agent",
  "role": "general",
  "subscribed_events": ["task.created"],
  "auto_trigger": true
}
```

**Response** (unchanged):
```json
{
  "id": "uuid",
  "name": "pm-agent",
  "status": "created",
  "subscribed_events": ["task.created"],
  "last_cursor": 0,
  "auto_trigger": true
}
```

---

## Testing Plan

### Unit Tests

1. **Agent subscription matching**
   - Test `Agent::subscribes_to()` with exact match
   - Test wildcard patterns (`"task.*"` matches `"task.created"`)
   - Test negative cases

2. **Database queries**
   - `list_agents_by_subscription()` returns correct agents
   - `update_agent_cursor()` updates cursor value
   - Cursor updates are idempotent

3. **Event poller**
   - Poll returns events after cursor
   - Cursor advances after successful poll
   - Handles empty responses

### Integration Tests

1. **Orchestrator triggers agent**
   - Create agent with subscription
   - Emit matching event
   - Verify agent status changed to Running
   - Verify cursor updated

2. **Agent-to-agent flow**
   - PM agent receives `task.created`
   - PM emits `requirement_analyzed`
   - BA agent receives `requirement_analyzed`
   - Verify both cursors updated

3. **No duplicate triggers**
   - Emit same event twice
   - Verify agent only triggered once (cursor prevents re-trigger)

### Manual Tests

1. **End-to-end flow**
   ```bash
   # Terminal 1: Start server
   cargo run --bin todoki-server

   # Terminal 2: Start relay
   cargo run --bin todoki-relay

   # Terminal 3: Create agent
   curl -X POST http://localhost:3000/api/agents \
     -H "Content-Type: application/json" \
     -d '{"name":"pm-agent","command":"mock-agent","subscribed_events":["task.created"],"auto_trigger":true}'

   # Terminal 4: Create task (triggers agent)
   curl -X POST http://localhost:3000/api/tasks \
     -H "Content-Type: application/json" \
     -d '{"content":"Implement feature X"}'

   # Verify agent started in Terminal 2 (relay logs)
   # Verify agent output in Terminal 1 (server logs)
   ```

---

## Success Criteria

- [ ] Agents can be created with `subscribed_events` and `auto_trigger` fields
- [ ] Database migration adds new columns without breaking existing data
- [ ] Event orchestrator starts on server boot
- [ ] Orchestrator detects matching events and triggers agents
- [ ] Agents receive `TRIGGER_EVENT_*` environment variables
- [ ] Agent cursor updates after each trigger
- [ ] No duplicate triggers (cursor prevents re-processing)
- [ ] Relay event poller works (agents can poll events)
- [ ] Mock agent demonstrates event handling (prints received events)
- [ ] Integration test passes: task.created → PM agent → requirement_analyzed → BA agent

---

## Rollback Plan

If issues arise, rollback steps:

1. **Stop orchestrator**: Set `auto_trigger = false` for all agents
2. **Revert migration**: `sqlx migrate revert`
3. **Deploy previous version**: Restore git commit before Phase 2

---

## Next Steps (Phase 3)

After Phase 2 completes:

1. **Role-specific handlers**: PM/BA/Coding/QA logic
2. **WebSocket event stream**: Real-time frontend updates
3. **Event timeline UI**: Visualize event sequences in task detail page
4. **Advanced patterns**: Event aggregation, correlation, filtering

---

## References

- [Event Bus Design](./event-bus-design.md)
- [Agent Bus Concept](../agent-bus.md)
- [Todoki Protocol](../crates/todoki-protocol/src/lib.rs)
- [Relay Manager](../crates/todoki-server/src/relay/manager.rs)
