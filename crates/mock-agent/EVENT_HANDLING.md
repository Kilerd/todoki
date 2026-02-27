# Mock Agent Event Handling

This document explains how the mock-agent handles events in the Todoki system.

## Overview

Mock-agent now supports event-driven task processing through the ACP (Agent Client Protocol) `ext_notification` method. Events are sent by the relay (which receives them from the server's Event Orchestrator) and processed asynchronously.

## Event Flow

```
Server EventBus
    ↓
EventOrchestrator (monitors and triggers)
    ↓
Relay (forwards to subscribed agents)
    ↓
Agent ext_notification handler
    ↓
Event processing logic
```

## Supported Events

### 1. `task.created`

Triggered when a new task is created.

**Event Structure:**
```json
{
  "cursor": 1,
  "kind": "task.created",
  "time": "2026-02-27T10:00:00Z",
  "agent_id": "planner-agent",
  "task_id": "task-123",
  "session_id": null,
  "data": {
    "content": "Implement user authentication",
    "priority": "high"
  }
}
```

**Handler Behavior:**
- Logs task content
- Simulates 2-second analysis
- Generates analysis plan with breakdown
- Logs result (in real system, would emit `agent.requirement_analyzed` event)

### 2. `agent.requirement_analyzed`

Triggered when requirements have been analyzed.

**Event Structure:**
```json
{
  "cursor": 2,
  "kind": "agent.requirement_analyzed",
  "time": "2026-02-27T10:00:05Z",
  "agent_id": "planner-agent",
  "task_id": "task-123",
  "session_id": "session-456",
  "data": {
    "plan": "1. Design auth flow, 2. Implement JWT, 3. Add middleware",
    "estimated_effort": "high",
    "breakdown": [
      {"subtask": "Architecture design", "assignee": "architect-agent"},
      {"subtask": "JWT implementation", "assignee": "coding-agent"}
    ]
  }
}
```

**Handler Behavior:**
- Logs received plan
- Acknowledges business context loading
- Ready for next step

## Testing

### 1. Run the event demo

Shows example event structures and event flow:

```bash
cargo run --package mock-agent --example event_demo
```

### 2. Interactive testing

Start the agent and manually send events:

```bash
./crates/mock-agent/examples/simple_event_test.sh
```

Then paste this JSON-RPC event into stdin:

```json
{"jsonrpc":"2.0","method":"event","params":{"cursor":1,"kind":"task.created","time":"2026-02-27T10:00:00Z","agent_id":"test-agent","task_id":"task-123","session_id":null,"data":{"content":"Fix authentication bug"}}}
```

Expected output (via stderr):
```
INFO mock_agent: Received event: task.created (cursor: 1)
INFO mock_agent: 🤖 Analyzing task: Fix authentication bug
INFO mock_agent: 📤 Emitting agent.requirement_analyzed event for task: Some("task-123")
```

### 3. Automated bash script testing

Full automated test with multiple events (requires `jq`):

```bash
./crates/mock-agent/examples/test_event_handling.sh
```

## Implementation Details

### Event Structure

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
struct Event {
    cursor: i64,
    kind: String,
    time: String,
    agent_id: String,
    session_id: Option<String>,
    task_id: Option<String>,
    data: serde_json::Value,
}
```

### Event Reception

Events are received via ACP's `ext_notification` method:

```rust
async fn ext_notification(
    &self,
    args: ExtNotification,
) -> agent_client_protocol::Result<()> {
    if args.method.as_ref() == "event" {
        let event: Event = serde_json::from_str(args.params.get())?;
        // Handle event based on kind
        match event.kind.as_str() {
            "task.created" => Self::handle_task_created(event).await?,
            "agent.requirement_analyzed" => Self::handle_requirement_analyzed(event).await?,
            _ => debug!("Ignoring event kind: {}", event.kind),
        }
    }
    Ok(())
}
```

### Event Handlers

Each event kind has a dedicated handler method:

- `handle_task_created()` - Process new task events
- `handle_requirement_analyzed()` - Process analyzed requirements

Handlers are async and can perform I/O operations, emit new events, or trigger other agents.

## Integration with Todoki System

In the full Todoki system:

1. **Server**: Stores events in Event Bus (Postgres table)
2. **EventOrchestrator**: Monitors new events and triggers subscribed agents
3. **Relay**: Forwards events to agents via WebSocket + ACP
4. **Agent**: Receives events through `ext_notification`, processes them, and may emit new events

The mock-agent demonstrates this pattern for testing and development purposes.

## Adding New Event Handlers

To add support for a new event kind:

1. Add a new match arm in `ext_notification`:
   ```rust
   "new.event.kind" => Self::handle_new_event(event).await?,
   ```

2. Implement the handler method:
   ```rust
   async fn handle_new_event(event: Event) -> Result<(), Box<dyn std::error::Error>> {
       // Your processing logic
       Ok(())
   }
   ```

3. Add tests and examples in `examples/` directory

## Logging

The agent uses `tracing` for structured logging. Set `RUST_LOG` environment variable to control log level:

```bash
RUST_LOG=debug cargo run --bin mock-agent
RUST_LOG=info cargo run --bin mock-agent
```

All logs go to stderr to avoid interfering with stdin/stdout ACP communication.
