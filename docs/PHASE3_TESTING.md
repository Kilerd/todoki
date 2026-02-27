# Phase 3 Testing Strategy

This document describes the testing approach for Phase 3 of the Event Bus implementation (WebSocket API and Frontend EventTimeline).

## Overview

Phase 3 introduces real-time event streaming via WebSocket and frontend visualization. Testing focuses on:
1. **WebSocket Integration Tests**: End-to-end WebSocket connection and event delivery
2. **Frontend Hook Tests**: React hook functionality and state management
3. **Manual UI Tests**: Visual verification of EventTimeline component

## WebSocket Integration Tests

### Location

`crates/todoki-server/tests/websocket_integration.rs`

### Test Cases

#### 1. `test_websocket_basic_connection`

Verifies basic WebSocket connection and subscription.

**Steps:**
- Connect to `/ws/event-bus` with authentication
- Receive subscription confirmation message

**Expected:**
- Connection established successfully
- `subscribed` message received with `kinds` array
- Connection remains open

#### 2. `test_websocket_authentication_failure`

Tests authentication rejection for invalid tokens.

**Steps:**
- Connect with invalid token
- Observe connection behavior

**Expected:**
- Connection rejected or immediately closed
- No messages received after authentication failure

#### 3. `test_websocket_realtime_event_delivery`

Verifies real-time event streaming.

**Steps:**
- Connect with `kinds=task.*` filter
- Emit `task.created` event via HTTP API
- Receive event via WebSocket

**Expected:**
- Event received within 5 seconds
- Event contains correct `type`, `kind`, `agent_id`, `data`, `cursor`
- Event matches emitted data

#### 4. `test_websocket_kind_filtering`

Tests event kind pattern filtering.

**Steps:**
- Connect with `kinds=agent.*` filter
- Emit `task.created` event (should be filtered out)
- Emit `agent.started` event (should be received)

**Expected:**
- Only `agent.started` event is received
- `task.created` event is not delivered to client

#### 5. `test_websocket_historical_replay`

Tests historical event replay from cursor.

**Steps:**
- Emit 5 events
- Connect with `cursor=0` to replay all events
- Receive historical events and `replay_complete` message

**Expected:**
- All historical events are replayed
- `replay_complete` message indicates end of replay
- Real-time events follow after replay

#### 6. `test_websocket_multiple_clients`

Tests broadcast to multiple concurrent clients.

**Steps:**
- Connect two WebSocket clients with same filter
- Emit single event
- Verify both clients receive the event

**Expected:**
- Both clients receive identical event
- Same cursor value for both
- No message duplication

#### 7. `test_websocket_heartbeat`

Tests server ping/pong heartbeat mechanism.

**Steps:**
- Connect and wait for up to 35 seconds
- Receive `ping` message from server
- Send `pong` response

**Expected:**
- Server sends `ping` every 30 seconds
- Client can respond with `pong`
- Connection stays alive

#### 8. `test_websocket_reconnection_scenario`

Tests cursor-based reconnection and catch-up.

**Steps:**
- Connect and receive event at cursor N
- Disconnect
- Emit 3 more events while disconnected
- Reconnect with `cursor=N`
- Receive missed events via replay

**Expected:**
- All events emitted during disconnect are replayed
- Client catches up to current state
- No events are lost

### Running WebSocket Tests

```bash
# Set environment variable
export USER_TOKEN=your-test-token

# Run all WebSocket tests (requires running server)
cargo test --test websocket_integration -- --ignored --test-threads=1

# Run specific test
cargo test --test websocket_integration test_websocket_basic_connection -- --ignored

# With output
cargo test --test websocket_integration -- --ignored --nocapture
```

**Prerequisites:**
- Server running: `cargo run --bin todoki`
- PostgreSQL database initialized
- `USER_TOKEN` environment variable set

**Note:** Tests are marked with `#[ignore]` because they require a running server. Run manually or in CI with proper setup.

## Frontend Hook Tests

### Location

`web/src/hooks/useEventStream.test.ts`

### Test Cases

#### 1. `should initialize with empty events and disconnected state`

Verifies initial hook state.

**Expected:**
- `events`: empty array
- `isConnected`: false
- `isReplaying`: false
- `error`: null

#### 2. `should connect to WebSocket on mount`

Tests automatic connection on mount.

**Expected:**
- WebSocket connection is initiated
- `isConnected` becomes true after connection opens

#### 3. `should build correct WebSocket URL with parameters`

Verifies URL construction with filters.

**Parameters:**
- `kinds`: `['task.*', 'agent.*']`
- `cursor`: 100
- `agentId`: 'agent-123'
- `taskId`: 'task-456'
- `token`: 'test-token'

**Expected:**
- URL contains all query parameters
- Parameters are properly encoded

#### 4. `should handle incoming events`

Tests event message handling.

**Steps:**
- Connect
- Simulate incoming event message
- Verify event is added to `events` array

**Expected:**
- Event is parsed correctly
- Event appears in `events` state
- Event structure matches `Event` interface

#### 5. `should set isReplaying when cursor is provided`

Tests replay state management.

**Expected:**
- `isReplaying` is true when `cursor` option is provided
- `isReplaying` becomes false after `replay_complete` message

#### 6. `should handle server errors`

Tests error message handling.

**Steps:**
- Receive error message from server
- Verify `error` state is updated

**Expected:**
- `error` contains server error message
- UI can display error to user

#### 7. `should reconnect with exponential backoff`

Tests auto-reconnection logic.

**Steps:**
- Simulate disconnect
- Verify reconnection attempts with increasing delays

**Expected:**
- First reconnect after 1 second
- Second reconnect after 2 seconds
- Third reconnect after 4 seconds
- Maximum delay capped at 30 seconds

#### 8. `should stop reconnecting after max attempts`

Tests reconnection limit.

**Expected:**
- Stops after `maxReconnectAttempts` (default: 10)
- No infinite reconnection loop

#### 9. `should manually reconnect and reset attempt counter`

Tests `reconnect()` function.

**Expected:**
- Manual reconnect resets attempt counter
- Allows retrying after max attempts reached

#### 10. `should clear events`

Tests `clearEvents()` function.

**Expected:**
- `events` array is emptied
- Connection remains open

#### 11. `should disconnect on unmount`

Tests cleanup on component unmount.

**Expected:**
- WebSocket is closed
- Reconnection timers are cleared
- No memory leaks

### Running Frontend Tests

```bash
cd web

# Run all tests
npm test

# Run with coverage
npm run test:coverage

# Run in watch mode
npm run test:watch
```

**Note:** These tests use mocked WebSocket. For true E2E testing, use Playwright or Cypress.

## Manual UI Tests

### Test Environment

1. Start server:
   ```bash
   cargo run --bin todoki
   ```

2. Start frontend:
   ```bash
   cd web
   npm run dev
   ```

3. Open browser: `http://localhost:5201`

### Test Scenarios

#### Scenario 1: Events Page Basic Functionality

**URL:** `/events`

**Steps:**
1. Navigate to Events page
2. Verify connection status shows "Connected" (green dot)
3. Verify events counter starts at 0

**Expected:**
- Page loads without errors
- WebSocket connects automatically
- UI shows connection status

#### Scenario 2: Real-time Event Display

**Steps:**
1. Open Events page
2. In another terminal, emit event:
   ```bash
   curl -X POST http://localhost:3000/api/event-bus/emit \
     -H "Content-Type: application/json" \
     -H "Authorization: Bearer $USER_TOKEN" \
     -d '{
       "kind": "task.created",
       "agent_id": "test-agent",
       "data": {"content": "Manual test task"}
     }'
   ```
3. Observe Events page

**Expected:**
- New event appears immediately (< 1 second)
- Event shows correct icon (📝 for task.created)
- Event displays cursor, time, agent_id, task_id
- Data can be expanded with "View data" toggle

#### Scenario 3: Event Filtering

**Steps:**
1. Open Events page
2. Click "Task Events" quick filter
3. Emit task event (should appear)
4. Emit agent event (should NOT appear)
5. Click "All Events" filter
6. Emit agent event (should appear now)

**Expected:**
- Filters apply immediately
- Only matching events are displayed
- Filter state persists during session

#### Scenario 4: Historical Replay

**Steps:**
1. Emit 10 events
2. Open Events page with cursor parameter:
   - Manually add `?cursor=0` to URL, or
   - Use cursor input field in UI
3. Observe event list

**Expected:**
- "Replaying history" badge appears
- Historical events load (up to 1000)
- "Replay complete: N events" status shown
- Real-time events continue after replay

#### Scenario 5: Connection Recovery

**Steps:**
1. Open Events page (connected)
2. Stop server: `Ctrl+C` in server terminal
3. Observe UI shows "Disconnected" (red dot)
4. Restart server
5. Wait for auto-reconnect (up to 30 seconds)

**Expected:**
- Connection status updates to "Disconnected"
- Reconnect attempts shown in browser console
- Automatic reconnection when server is back
- No manual refresh needed

#### Scenario 6: Manual Reconnect

**Steps:**
1. Open Events page
2. Stop server
3. Click "Reconnect" button
4. Observe connection attempts

**Expected:**
- Manual reconnect button appears when disconnected
- Clicking button triggers immediate reconnect attempt
- Button is disabled during reconnect

#### Scenario 7: Event Data Expansion

**Steps:**
1. Open Events page
2. Wait for events with complex data
3. Click "View data" on an event

**Expected:**
- JSON data expands in formatted view
- Data is syntax-highlighted
- Can toggle collapse/expand

#### Scenario 8: Multiple Browser Tabs

**Steps:**
1. Open Events page in two browser tabs
2. Emit event
3. Observe both tabs

**Expected:**
- Both tabs receive same events
- Both tabs show same cursor values
- No interference between tabs

#### Scenario 9: Task-specific Events

**Steps:**
1. Create a task
2. Open task detail page: `/inbox/{task_id}`
3. Execute task on relay (if EventTimeline is integrated)
4. Observe task-specific events

**Expected (future integration):**
- Task detail page shows EventTimeline component
- Only events related to this task are displayed
- Events update in real-time

## Performance Testing

### Load Test: Event Throughput

**Goal:** Verify system handles high event volume

**Setup:**
```bash
# Start server
cargo run --bin todoki --release

# Run load test
for i in {1..1000}; do
  curl -X POST http://localhost:3000/api/event-bus/emit \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $USER_TOKEN" \
    -d "{\"kind\":\"test.event\",\"agent_id\":\"load-test\",\"data\":{\"index\":$i}}"
done
```

**Observe:**
- Open Events page during load test
- Monitor browser memory usage
- Check event delivery latency
- Verify no events are lost

**Expected:**
- Events delivered within 1-2 seconds
- Browser memory stays below 200MB
- No WebSocket disconnections
- All 1000 events accounted for

### Memory Test: Long-running Connection

**Goal:** Verify no memory leaks over time

**Steps:**
1. Open Events page
2. Let connection run for 1 hour
3. Emit events periodically (1 per minute)
4. Monitor browser memory

**Expected:**
- Memory stays stable (< 100MB growth per hour)
- No performance degradation
- Connection remains stable

## Browser Compatibility

### Tested Browsers

- ✅ Chrome 120+ (Primary)
- ✅ Firefox 120+
- ✅ Safari 17+
- ✅ Edge 120+

### Known Issues

- Safari may show warnings for WebSocket protocol
- Firefox may require secure WebSocket (wss://) in production

## Accessibility Testing

### Keyboard Navigation

- ✅ All interactive elements focusable
- ✅ Tab order is logical
- ✅ No keyboard traps

### Screen Reader

- ✅ Connection status announced
- ✅ Event items have proper labels
- ✅ Error messages are announced

## Future Testing Enhancements

### Automated E2E Tests

Use Playwright for full E2E testing:

```typescript
// e2e/events.spec.ts
import { test, expect } from '@playwright/test';

test('real-time event display', async ({ page }) => {
  await page.goto('/events');

  // Verify connected
  await expect(page.getByText('Connected')).toBeVisible();

  // Emit event via API
  // ... (trigger backend)

  // Verify event appears
  await expect(page.getByText('task.created')).toBeVisible({ timeout: 5000 });
});
```

### Performance Monitoring

Integrate performance metrics:

- WebSocket message rate (events/second)
- Event processing latency (emit to display)
- Component render time
- Memory usage tracking

### Visual Regression Testing

Use Playwright snapshots for UI consistency:

```typescript
test('events page layout', async ({ page }) => {
  await page.goto('/events');
  await expect(page).toHaveScreenshot('events-page.png');
});
```

## Summary

✅ **Implemented:**
- WebSocket integration tests (8 test cases)
- Frontend hook unit tests (11 test cases)
- Comprehensive manual test scenarios
- Performance testing guidelines

⏸️ **Future Work:**
- Automated E2E tests with Playwright
- CI/CD pipeline integration
- Visual regression testing
- Performance benchmarking automation

**Current Status:** Phase 3 testing framework is complete. All core functionality can be verified through combination of automated tests and manual scenarios.
