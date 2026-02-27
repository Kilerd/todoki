# WebSocket Event Streaming API

Real-time event subscription API for frontend and agents.

## Endpoint

```
GET ws://localhost:3000/ws/event-bus
```

## Authentication

**Preferred**: Authorization header
```
Authorization: Bearer <your-token>
```

**Alternative**: Query parameter (for legacy clients)
```
ws://localhost:3000/ws/event-bus?token=<your-token>
```

**Note**: Query parameter authentication is discouraged. Always prefer using the Authorization header when possible.

## Query Parameters

| Parameter | Type | Description | Example |
|-----------|------|-------------|---------|
| `kinds` | string | Comma-separated event kinds (supports wildcards) | `task.created,agent.*` |
| `cursor` | i64 | Starting cursor for historical replay (optional) | `100` |
| `agent_id` | UUID | Filter events by agent ID (optional) | `a1b2c3d4-...` |
| `task_id` | UUID | Filter events by task ID (optional) | `e5f6g7h8-...` |
| `token` | string | Authentication token (discouraged, use header) | `your-token` |

## Message Format

All messages are JSON with a `type` field indicating the message type.

### Client → Server

Currently, the WebSocket is read-only (server to client only). Future versions may support:
- Subscription filter updates
- Cursor position updates

### Server → Client

#### 1. Subscribed (Acknowledgment)

Sent immediately after connection to confirm subscription parameters.

```json
{
  "type": "subscribed",
  "kinds": ["task.created", "agent.*"],
  "cursor": 100
}
```

#### 2. Event

A new event matching your subscription.

```json
{
  "type": "event",
  "cursor": 101,
  "kind": "task.created",
  "time": "2026-02-27T10:30:00Z",
  "agent_id": "a1b2c3d4-1234-5678-90ab-cdef12345678",
  "session_id": null,
  "task_id": "e5f6g7h8-1234-5678-90ab-cdef12345678",
  "data": {
    "content": "Fix authentication bug",
    "priority": 2,
    "project_id": "proj-123"
  }
}
```

#### 3. Replay Complete

Sent after historical event replay finishes (if `cursor` was provided).

```json
{
  "type": "replay_complete",
  "cursor": 150,
  "count": 50
}
```

After this message, all subsequent events are real-time.

#### 4. Error

Server-side error or warning.

```json
{
  "type": "error",
  "message": "Event stream lagged by 10 events, consider reconnecting with cursor"
}
```

**Common Errors**:
- `Event stream lagged by N events` - Client is too slow, some events may be missed
- `Failed to fetch historical events` - Database error during replay
- Unauthenticated connections are immediately closed

#### 5. Ping/Pong (Heartbeat)

Server sends periodic pings every 30 seconds.

```json
{
  "type": "ping"
}
```

Client should respond with pong (or use WebSocket-level ping/pong frames).

## Usage Examples

### JavaScript (Browser)

```javascript
const ws = new WebSocket('ws://localhost:3000/ws/event-bus?kinds=task.*,agent.*&cursor=0');

// Set authentication header (requires custom WebSocket implementation or proxy)
// Alternatively, use token query parameter (not recommended)

ws.onopen = () => {
  console.log('Connected to event stream');
};

ws.onmessage = (event) => {
  const message = JSON.parse(event.data);

  switch (message.type) {
    case 'subscribed':
      console.log('Subscription confirmed:', message.kinds);
      break;

    case 'event':
      console.log(`Event ${message.cursor}: ${message.kind}`, message.data);
      // Update UI with new event
      updateEventTimeline(message);
      break;

    case 'replay_complete':
      console.log(`Replay done: ${message.count} historical events`);
      break;

    case 'error':
      console.error('Event stream error:', message.message);
      break;

    case 'ping':
      // WebSocket-level ping/pong is automatic
      ws.send(JSON.stringify({ type: 'pong' }));
      break;
  }
};

ws.onerror = (error) => {
  console.error('WebSocket error:', error);
};

ws.onclose = () => {
  console.log('WebSocket closed, reconnecting...');
  // Implement exponential backoff reconnection
  setTimeout(reconnect, 1000);
};

function updateEventTimeline(event) {
  // Add event to UI timeline
  const timeline = document.getElementById('event-timeline');
  const entry = document.createElement('div');
  entry.className = 'event-entry';
  entry.innerHTML = `
    <span class="event-cursor">#${event.cursor}</span>
    <span class="event-kind">${event.kind}</span>
    <span class="event-time">${new Date(event.time).toLocaleTimeString()}</span>
    <pre class="event-data">${JSON.stringify(event.data, null, 2)}</pre>
  `;
  timeline.prepend(entry);
}
```

### Python (Agent SDK)

```python
import websockets
import json
import asyncio

async def subscribe_events():
    uri = "ws://localhost:3000/ws/event-bus?kinds=task.created&cursor=0"
    headers = {"Authorization": "Bearer YOUR_TOKEN"}

    async with websockets.connect(uri, extra_headers=headers) as ws:
        print("Connected to event stream")

        async for message in ws:
            event = json.loads(message)

            if event['type'] == 'subscribed':
                print(f"Subscribed to: {event['kinds']}")

            elif event['type'] == 'event':
                print(f"Event #{event['cursor']}: {event['kind']}")
                await handle_event(event)

            elif event['type'] == 'replay_complete':
                print(f"Replay complete: {event['count']} events")

            elif event['type'] == 'error':
                print(f"Error: {event['message']}")

            elif event['type'] == 'ping':
                # Respond to ping
                await ws.send(json.dumps({'type': 'pong'}))

async def handle_event(event):
    """Process event based on kind"""
    if event['kind'] == 'task.created':
        task_content = event['data']['content']
        print(f"New task: {task_content}")
        # Perform task analysis...

if __name__ == '__main__':
    asyncio.run(subscribe_events())
```

### Rust (Relay Agent)

```rust
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};

async fn subscribe_events() -> Result<(), Box<dyn std::error::Error>> {
    let url = "ws://localhost:3000/ws/event-bus?kinds=task.*&cursor=0";

    let (ws_stream, _) = connect_async(url).await?;
    let (mut write, mut read) = ws_stream.split();

    // Add Authorization header (requires custom request)

    while let Some(msg) = read.next().await {
        let msg = msg?;

        if let Message::Text(text) = msg {
            let event: Value = serde_json::from_str(&text)?;

            match event["type"].as_str() {
                Some("subscribed") => {
                    println!("Subscription confirmed");
                }
                Some("event") => {
                    let kind = event["kind"].as_str().unwrap();
                    let cursor = event["cursor"].as_i64().unwrap();
                    println!("Event #{}: {}", cursor, kind);

                    // Process event
                    handle_event(event).await?;
                }
                Some("replay_complete") => {
                    println!("Historical replay done");
                }
                Some("ping") => {
                    // Respond to ping
                    write.send(Message::Text(
                        json!({"type": "pong"}).to_string()
                    )).await?;
                }
                _ => {}
            }
        }
    }

    Ok(())
}
```

## Event Kind Patterns

### Exact Match
```
?kinds=task.created
```
Only matches `task.created` events.

### Wildcard Match
```
?kinds=task.*
```
Matches all task events: `task.created`, `task.updated`, `task.completed`, etc.

### Multiple Patterns
```
?kinds=task.created,agent.*,system.relay_connected
```
Matches any of:
- `task.created` (exact)
- `agent.*` (all agent events)
- `system.relay_connected` (exact)

### All Events
```
?kinds=*
```
Or omit the `kinds` parameter entirely.

## Historical Replay

Start from a specific cursor to receive historical events before real-time stream:

```
?cursor=100
```

**Flow**:
1. Client connects with `cursor=100`
2. Server sends `subscribed` message
3. Server replays events from cursor 100 (up to 1000 events)
4. Server sends `replay_complete` message
5. Server streams real-time events

**Use Cases**:
- Reconnection after disconnect
- Catching up on missed events
- Initial timeline population

**Limits**:
- Max 1000 events in replay
- If you need more, use HTTP API (`GET /api/event-bus`) for pagination

## Cursor Management

Clients should track the latest `cursor` received to resume after disconnect:

```javascript
let lastCursor = localStorage.getItem('lastEventCursor') || 0;

ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  if (msg.type === 'event') {
    lastCursor = msg.cursor;
    localStorage.setItem('lastEventCursor', lastCursor);
  }
};

// On reconnect
ws = new WebSocket(`ws://localhost:3000/ws/event-bus?cursor=${lastCursor}`);
```

## Error Handling

### Lagged Stream

If the client is too slow, the broadcast channel may lag:

```json
{
  "type": "error",
  "message": "Event stream lagged by 50 events, consider reconnecting with cursor"
}
```

**Resolution**:
1. Note the last `cursor` you received
2. Disconnect
3. Reconnect with `?cursor=<last_cursor>` to catch up

### Authentication Failure

Unauthorized connections are immediately closed with no message.

**Check**:
- Token is valid
- Token is correctly passed (header or query)
- Token hasn't expired

### Connection Drops

WebSocket connections can drop due to network issues. Implement reconnection:

```javascript
let reconnectAttempts = 0;

function connect() {
  const ws = new WebSocket(`ws://...?cursor=${lastCursor}`);

  ws.onclose = () => {
    const delay = Math.min(1000 * Math.pow(2, reconnectAttempts), 30000);
    reconnectAttempts++;
    setTimeout(connect, delay);
  };

  ws.onopen = () => {
    reconnectAttempts = 0;  // Reset on successful connection
  };
}
```

## Performance Considerations

### Filtering

Apply filters to reduce bandwidth:
- Use specific `kinds` instead of wildcard
- Use `agent_id` or `task_id` filters for focused streams

### Batch Processing

If receiving high event volume, batch UI updates:

```javascript
let eventBuffer = [];
let updateScheduled = false;

function onEvent(event) {
  eventBuffer.push(event);

  if (!updateScheduled) {
    updateScheduled = true;
    requestAnimationFrame(() => {
      updateUI(eventBuffer);
      eventBuffer = [];
      updateScheduled = false;
    });
  }
}
```

### Connection Pooling

For multiple subscriptions, reuse a single WebSocket connection and filter client-side.

## Security

### Token Exposure

**Never** include tokens in URLs when possible:
- ❌ `ws://server/ws/event-bus?token=secret123`
- ✅ Use Authorization header (requires custom WebSocket setup)

### Event Permissions

Future versions will enforce:
- Agents can only see events they're authorized for
- User-level permissions for event visibility
- Project-level event isolation

Currently, all authenticated users see all events.

## Monitoring

Server logs WebSocket connections:
```
INFO WebSocket event-bus connection established
  authenticated: true
  kinds: Some(["task.*", "agent.*"])
  cursor: Some(100)

INFO WebSocket connection closed
```

Check server logs for:
- Failed authentication attempts
- Lag warnings
- Connection churn

## Future Enhancements

- **Bidirectional messaging**: Client can update subscription filters
- **Event filtering by project**: `?project_id=...`
- **Event aggregation**: Batch events in time windows
- **Permission-based filtering**: Only see authorized events
- **Compression**: gzip/brotli for high-volume streams
- **Event acknowledgments**: Confirm event processing

## Related Documentation

- [Event Bus Design](./event-bus-design.md)
- [Phase 2 Implementation](./PHASE2_IMPLEMENTATION.md)
- [Quick Start Guide](./PHASE2_QUICKSTART.md)
