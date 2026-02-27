# Phase 2 Testing Strategy

This document describes the testing approach for Phase 2 of the Event Bus implementation.

## Overview

Phase 2 introduces event-driven agent triggering. Testing focuses on:
1. **Unit Tests**: Core subscription matching logic
2. **Manual Integration Tests**: End-to-end event flow verification
3. **Mock Agent Tests**: Event handling behavior

## Unit Tests

### Orchestrator Tests

Location: `crates/todoki-server/src/event_bus/orchestrator.rs` (tests module)

**Test Cases:**

1. **`test_agent_subscription_matching`**
   - Verifies exact event kind matching
   - Tests wildcard subscriptions (`agent.*` matches `agent.requirement_analyzed`)
   - Validates `should_trigger()` logic:
     - Should trigger when idle + auto_trigger + subscribed
     - Should NOT trigger when running
     - Should NOT trigger when auto_trigger=false
     - Should NOT trigger for non-subscribed events

2. **`test_event_kind_prefix_matching`**
   - Tests prefix wildcard patterns (`task.*`, `system.relay_*`)
   - Verifies multiple wildcard subscriptions
   - Ensures non-matching events are ignored

3. **`test_wildcard_edge_cases`**
   - Tests universal wildcard (`*`) matching all events
   - Validates edge case behavior

**Running Unit Tests:**

```bash
cargo test --package todoki
```

Expected output:
```
test event_bus::orchestrator::tests::test_agent_subscription_matching ... ok
test event_bus::orchestrator::tests::test_event_kind_prefix_matching ... ok
test event_bus::orchestrator::tests::test_wildcard_edge_cases ... ok
```

## Manual Integration Tests

### Prerequisites

1. **Database**: PostgreSQL running with todoki schema
2. **Server**: todoki-server running (`cargo run --bin todoki`)
3. **Relay**: todoki-relay connected to server
4. **Mock Agent**: Available for spawning

### Test Scenario 1: Basic Event Triggering

**Goal**: Verify agent is triggered when subscribed event occurs

**Steps:**

1. Create an agent with subscription:
   ```bash
   curl -X POST http://localhost:3000/api/agents \
     -H "Content-Type: application/json" \
     -H "Authorization: Bearer $USER_TOKEN" \
     -d '{
       "name": "test-pm-agent",
       "command": "mock-agent",
       "execution_mode": "local",
       "role": "general",
       "project_id": "'$PROJECT_ID'",
       "subscribed_events": ["task.created"],
       "auto_trigger": true
     }'
   ```

2. Emit a task.created event:
   ```bash
   curl -X POST http://localhost:3000/api/event-bus/emit \
     -H "Content-Type: application/json" \
     -H "Authorization: Bearer $USER_TOKEN" \
     -d '{
       "kind": "task.created",
       "agent_id": "test-pm-agent",
       "task_id": "'$TASK_ID'",
       "data": {"content": "Implement feature X"}
     }'
   ```

3. Verify agent was triggered:
   ```bash
   curl http://localhost:3000/api/agents/$AGENT_ID \
     -H "Authorization: Bearer $USER_TOKEN"
   ```

**Expected Results:**
- Agent `last_cursor` is updated (> 0)
- Agent `status` changes to `running`
- Agent session is created
- Server logs show: `[orchestrator] triggering agent: test-pm-agent`

### Test Scenario 2: Wildcard Subscription

**Goal**: Verify wildcard patterns match multiple event kinds

**Steps:**

1. Create agent with wildcard subscription:
   ```json
   {
     "name": "monitor-agent",
     "subscribed_events": ["task.*"],
     "auto_trigger": true
   }
   ```

2. Emit multiple events:
   - `task.created`
   - `task.updated`
   - `task.completed`

3. Verify agent cursor advances for each event

**Expected Results:**
- Agent processes all `task.*` events
- Cursor increments with each matching event
- Non-matching events (e.g., `agent.started`) are ignored

### Test Scenario 3: Multiple Agents, Same Event

**Goal**: Verify multiple agents can subscribe to same event

**Steps:**

1. Create two agents, both subscribing to `task.created`:
   - PM agent
   - Monitor agent

2. Emit single `task.created` event

3. Verify both agents are triggered

**Expected Results:**
- Both agents' cursors are updated
- Both agents receive trigger (sessions created)
- No double-processing or race conditions

### Test Scenario 4: Event Replay

**Goal**: Test agents can catch up on missed events

**Steps:**

1. Emit several events while agent is offline

2. Create agent with `last_cursor = 0`

3. Use Event Bus query API to fetch missed events:
   ```bash
   curl "http://localhost:3000/api/event-bus?cursor=0&kinds=task.created" \
     -H "Authorization: Bearer $USER_TOKEN"
   ```

4. Manually trigger agent for each missed event

**Expected Results:**
- Agent can query historical events
- Cursor-based pagination prevents duplicate processing

## Mock Agent Event Handling Tests

Location: `crates/mock-agent/src/main.rs`

**Test Cases:**

1. **Event Reception via ext_notification**
   - Mock agent receives events through ACP protocol
   - Events are parsed correctly
   - Event handlers are called based on `kind`

2. **task.created Handler**
   - Logs task content
   - Simulates 2-second analysis delay
   - Generates structured analysis result
   - Would emit `agent.requirement_analyzed` event

3. **agent.requirement_analyzed Handler**
   - Receives analysis from another agent
   - Logs received plan
   - Acknowledges context loading

**Manual Testing:**

```bash
# Run mock agent
RUST_LOG=info cargo run --bin mock-agent

# In another terminal, send event via stdin:
echo '{"jsonrpc":"2.0","method":"event","params":{"cursor":1,"kind":"task.created","time":"2026-02-27T10:00:00Z","agent_id":"test","task_id":"task-123","session_id":null,"data":{"content":"Fix bug"}}}' | RUST_LOG=info cargo run --bin mock-agent
```

**Expected Output:**
```
INFO mock_agent: Mock agent starting...
INFO mock_agent: ready, waiting for commands...
INFO mock_agent: Received event: task.created (cursor: 1)
INFO mock_agent: 🤖 Analyzing task: Fix bug
(2 second delay)
INFO mock_agent: 📤 Emitting agent.requirement_analyzed event for task: Some("task-123")
```

**Automated Example Tests:**

```bash
# Run event demo
cargo run --package mock-agent --example event_demo

# Run interactive test script
./crates/mock-agent/examples/simple_event_test.sh
```

## Limitations and Future Work

### Current Limitations

1. **No End-to-End Integration Tests**
   - Full integration tests require running database, server, relay, and agents
   - Current setup focuses on unit tests and manual verification
   - Reason: Complex test environment setup not yet automated

2. **Orchestrator Not Mockable**
   - Orchestrator depends on real database and relay connections
   - Cannot easily inject mock implementations for isolated testing
   - Future: Add dependency injection for testability

3. **Event Polling Not Tested**
   - EventPoller (Phase 2 Task 5) has no automated tests
   - Requires running server and test HTTP requests
   - Manual testing via `examples/event_poller_example.rs` only

### Future Improvements

1. **Test Database Setup**
   - Add `testcontainers` for PostgreSQL in tests
   - Auto-provision test database per test suite
   - Clean up after tests

2. **Mock Relay Manager**
   - Create mock implementation for testing
   - Verify RPC calls without real relay
   - Test error handling and timeouts

3. **Integration Test Suite**
   - Automate manual test scenarios
   - Add CI pipeline for integration tests
   - End-to-end flow verification

4. **Performance Tests**
   - Event throughput under load
   - Concurrent agent triggering
   - Cursor advancement correctness under high concurrency

## Summary

✅ **Implemented:**
- Unit tests for subscription matching logic
- Mock agent event handling with examples
- Manual integration test documentation

⏸️ **Deferred:**
- Automated end-to-end integration tests
- Database-dependent orchestrator tests
- Performance and load testing

**Recommendation**: Current test coverage is sufficient for Phase 2 completion. Full integration tests should be added in Phase 3 when CI/CD infrastructure is established.
