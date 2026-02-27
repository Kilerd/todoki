# Phase 2 Quick Start Guide

Quick guide to running and testing the event-driven agent system.

## Prerequisites

- PostgreSQL running
- Rust toolchain installed
- Environment variables set:
  ```bash
  export DATABASE_URL="postgres://user:pass@localhost/todoki"
  export USER_TOKEN="your-auth-token"
  ```

## 1. Run the Server

```bash
# From project root
cargo run --bin todoki
```

Server starts on `http://localhost:3000`

The EventOrchestrator automatically starts in the background.

## 2. Create a Test Project

```bash
PROJECT_ID=$(curl -X POST http://localhost:3000/api/projects \
  -H "Authorization: Bearer $USER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"test-project"}' | jq -r '.id')

echo "Created project: $PROJECT_ID"
```

## 3. Create an Event-Driven Agent

```bash
AGENT_ID=$(curl -X POST http://localhost:3000/api/agents \
  -H "Authorization: Bearer $USER_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"name\": \"pm-agent\",
    \"command\": \"mock-agent\",
    \"execution_mode\": \"local\",
    \"role\": \"general\",
    \"project_id\": \"$PROJECT_ID\",
    \"subscribed_events\": [\"task.created\"],
    \"auto_trigger\": true
  }" | jq -r '.id')

echo "Created agent: $AGENT_ID"
```

## 4. Emit an Event

```bash
curl -X POST http://localhost:3000/api/event-bus/emit \
  -H "Authorization: Bearer $USER_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"kind\": \"task.created\",
    \"agent_id\": \"pm-agent\",
    \"task_id\": \"$(uuidgen)\",
    \"data\": {\"content\": \"Implement feature X\"}
  }"
```

## 5. Verify Agent Was Triggered

```bash
# Check agent status
curl http://localhost:3000/api/agents/$AGENT_ID \
  -H "Authorization: Bearer $USER_TOKEN" | jq '{
    name: .name,
    status: .status,
    last_cursor: .last_cursor,
    subscribed_events: .subscribed_events
  }'
```

Expected output:
```json
{
  "name": "pm-agent",
  "status": "running",
  "last_cursor": 1,
  "subscribed_events": ["task.created"]
}
```

## 6. Query Events

```bash
# Get all events
curl "http://localhost:3000/api/event-bus?cursor=0" \
  -H "Authorization: Bearer $USER_TOKEN" | jq '.'

# Filter by event kind
curl "http://localhost:3000/api/event-bus?cursor=0&kinds=task.created" \
  -H "Authorization: Bearer $USER_TOKEN" | jq '.'
```

## 7. Test Event Poller (Relay-side)

```bash
# Set environment
export AGENT_ID="test-poller"
export SERVER_URL="http://localhost:3000"
export USER_TOKEN="your-token"

# Run poller example
cargo run --package todoki-relay --example event_poller_example
```

## 8. Test Mock Agent Event Handling

```bash
# Run event demo
cargo run --package mock-agent --example event_demo

# Interactive test
./crates/mock-agent/examples/simple_event_test.sh
```

Then paste this JSON into stdin:
```json
{"jsonrpc":"2.0","method":"event","params":{"cursor":1,"kind":"task.created","time":"2026-02-27T10:00:00Z","agent_id":"test","task_id":"task-123","session_id":null,"data":{"content":"Fix bug"}}}
```

Expected output:
```
INFO mock_agent: Received event: task.created (cursor: 1)
INFO mock_agent: 🤖 Analyzing task: Fix bug
INFO mock_agent: 📤 Emitting agent.requirement_analyzed event for task: Some("task-123")
```

## 9. Test Wildcard Subscriptions

Create agent with wildcard:
```bash
curl -X POST http://localhost:3000/api/agents \
  -H "Authorization: Bearer $USER_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"name\": \"monitor-agent\",
    \"command\": \"mock-agent\",
    \"execution_mode\": \"local\",
    \"role\": \"general\",
    \"project_id\": \"$PROJECT_ID\",
    \"subscribed_events\": [\"task.*\"],
    \"auto_trigger\": true
  }"
```

Emit different task events:
```bash
# task.created
curl -X POST http://localhost:3000/api/event-bus/emit \
  -H "Authorization: Bearer $USER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"kind":"task.created","data":{"content":"Test"}}'

# task.updated
curl -X POST http://localhost:3000/api/event-bus/emit \
  -H "Authorization: Bearer $USER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"kind":"task.updated","data":{"status":"in_progress"}}'

# task.completed
curl -X POST http://localhost:3000/api/event-bus/emit \
  -H "Authorization: Bearer $USER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"kind":"task.completed","data":{"result":"success"}}'
```

Monitor agent should be triggered for all three events.

## 10. Check Server Logs

Watch for orchestrator activity:
```bash
tail -f server.log | grep orchestrator
```

Expected log lines:
```
[orchestrator] polling for new events from cursor: 0
[orchestrator] found 1 new events
[orchestrator] triggering agent: pm-agent (event: task.created)
[orchestrator] agent triggered successfully: pm-agent
```

## 11. Run Unit Tests

```bash
# All tests
cargo test --package todoki

# Orchestrator tests only
cargo test --package todoki event_bus::orchestrator::tests
```

Expected:
```
test event_bus::orchestrator::tests::test_agent_subscription_matching ... ok
test event_bus::orchestrator::tests::test_event_kind_prefix_matching ... ok
test event_bus::orchestrator::tests::test_wildcard_edge_cases ... ok
```

## Troubleshooting

### Agent not triggered?

Check:
1. Agent has `auto_trigger = true`
2. Agent status is not `running` (already active)
3. Event kind matches agent's `subscribed_events`
4. Orchestrator is running (check server logs)

### Event not appearing?

Check:
1. Event was successfully emitted (check API response)
2. Use Event Bus query API to verify event exists:
   ```bash
   curl "http://localhost:3000/api/event-bus?cursor=0" -H "Authorization: Bearer $USER_TOKEN"
   ```

### Relay not connected?

Check:
1. Relay is running: `cargo run --bin todoki-relay`
2. Relay WebSocket connected to server
3. Agent has valid `relay_id`

## Next Steps

- Read full documentation: `docs/PHASE2_COMPLETION.md`
- Review testing strategy: `docs/PHASE2_TESTING.md`
- Explore mock agent: `crates/mock-agent/EVENT_HANDLING.md`
- Design Phase 3 features

## Common Event Kinds

Predefined in `crates/todoki-server/src/event_bus/kinds.rs`:

**Task Events:**
- `task.created`
- `task.updated`
- `task.completed`
- `task.assigned`

**Agent Events:**
- `agent.started`
- `agent.stopped`
- `agent.requirement_analyzed`
- `agent.business_context_ready`
- `agent.code_review_requested`

**System Events:**
- `system.relay_connected`
- `system.relay_disconnected`

**Use wildcards:**
- `task.*` - All task events
- `agent.*` - All agent events
- `*` - All events

## Example Workflow

Complete multi-agent workflow:

```bash
# 1. Create PM agent (analyzes tasks)
PM_ID=$(curl -X POST http://localhost:3000/api/agents \
  -H "Authorization: Bearer $USER_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"name\": \"pm-agent\",
    \"command\": \"mock-agent\",
    \"subscribed_events\": [\"task.created\"],
    \"auto_trigger\": true,
    \"project_id\": \"$PROJECT_ID\"
  }" | jq -r '.id')

# 2. Create BA agent (receives analysis)
BA_ID=$(curl -X POST http://localhost:3000/api/agents \
  -H "Authorization: Bearer $USER_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"name\": \"ba-agent\",
    \"command\": \"mock-agent\",
    \"subscribed_events\": [\"agent.requirement_analyzed\"],
    \"auto_trigger\": true,
    \"project_id\": \"$PROJECT_ID\"
  }" | jq -r '.id')

# 3. Create task (triggers PM agent)
TASK_ID=$(uuidgen)
curl -X POST http://localhost:3000/api/event-bus/emit \
  -H "Authorization: Bearer $USER_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"kind\": \"task.created\",
    \"task_id\": \"$TASK_ID\",
    \"data\": {\"content\": \"Implement user auth\"}
  }"

# Wait 3 seconds for PM to process

# 4. PM emits analysis (triggers BA agent)
curl -X POST http://localhost:3000/api/event-bus/emit \
  -H "Authorization: Bearer $USER_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"kind\": \"agent.requirement_analyzed\",
    \"agent_id\": \"$PM_ID\",
    \"task_id\": \"$TASK_ID\",
    \"data\": {
      \"plan\": \"1. Design, 2. Implement, 3. Test\",
      \"estimated_effort\": \"high\"
    }
  }"

# 5. Check both agents were triggered
curl http://localhost:3000/api/agents/$PM_ID -H "Authorization: Bearer $USER_TOKEN" | jq '.last_cursor'
curl http://localhost:3000/api/agents/$BA_ID -H "Authorization: Bearer $USER_TOKEN" | jq '.last_cursor'
```

Both cursors should be > 0.
