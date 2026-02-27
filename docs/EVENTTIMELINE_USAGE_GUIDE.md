# EventTimeline Component Usage Guide

Complete guide for using the EventTimeline component in the Todoki frontend.

---

## Quick Start

### 1. Basic Usage

```typescript
import { EventTimeline } from '@/components/EventTimeline';

function MyPage() {
  return (
    <EventTimeline
      kinds={['task.*', 'agent.*']}
      showStatus={true}
      maxEvents={50}
    />
  );
}
```

### 2. Task-specific Events

Show events for a specific task:

```typescript
import { EventTimeline } from '@/components/EventTimeline';
import { useParams } from 'react-router-dom';

function TaskDetail() {
  const { id } = useParams();

  return (
    <div>
      <h1>Task Details</h1>

      {/* Task-specific event stream */}
      <EventTimeline
        kinds={['task.*', 'agent.*', 'artifact.*']}
        taskId={id}
        showStatus={false}
        maxEvents={20}
      />
    </div>
  );
}
```

### 3. Agent-specific Events

Monitor a specific agent:

```typescript
function AgentDetail({ agentId }: { agentId: string }) {
  return (
    <EventTimeline
      kinds={['agent.*', 'task.*']}
      agentId={agentId}
      showStatus={true}
      maxEvents={30}
    />
  );
}
```

---

## Component Props

### EventTimelineProps

```typescript
interface EventTimelineProps {
  /** Event kind patterns to subscribe (e.g., ["task.*", "agent.*"]) */
  kinds?: string[];

  /** Starting cursor for historical replay */
  cursor?: number;

  /** Filter by agent ID */
  agentId?: string;

  /** Filter by task ID */
  taskId?: string;

  /** Authentication token (defaults to localStorage.getItem('token')) */
  token?: string;

  /** Show connection status bar (default: true) */
  showStatus?: boolean;

  /** Max events to display (default: 50) */
  maxEvents?: number;
}
```

### Event Kind Patterns

The `kinds` prop accepts wildcard patterns:

| Pattern | Matches | Example Events |
|---------|---------|----------------|
| `*` | All events | Everything |
| `task.*` | All task events | task.created, task.completed, task.failed |
| `agent.*` | All agent events | agent.started, agent.stopped, agent.requirement_analyzed |
| `artifact.*` | All artifact events | artifact.created, artifact.github_pr_opened |
| `permission.*` | All permission events | permission.requested, permission.approved |
| `system.*` | All system events | system.relay_connected, system.relay_disconnected |
| `task.created` | Exact match | Only task.created |

**Multiple patterns:**

```typescript
<EventTimeline kinds={['task.created', 'task.completed', 'agent.*']} />
```

---

## Event Icons and Colors

Events are automatically styled based on their kind:

### Task Events

| Event Kind | Icon | Color |
|------------|------|-------|
| `task.created` | 📝 FileText | Blue |
| `task.completed` | ✅ CheckCircle2 | Green |
| `task.failed` | ❌ XCircle | Red |
| Other task.* | 📋 FileText | Slate |

### Agent Events

| Event Kind | Icon | Color |
|------------|------|-------|
| `agent.started` | ▶️ PlayCircle | Green |
| `agent.stopped` | ⏹️ StopCircle | Yellow |
| `agent.requirement_analyzed` | 🤖 Bot | Purple |
| Other agent.* | 🔧 Bot | Slate |

### Other Events

| Event Kind | Icon | Color |
|------------|------|-------|
| `artifact.*` | 📦 FileText | Indigo |
| `permission.*` | 🔐 Circle | Orange |
| `system.*` | ⚙️ Activity | Slate |

---

## Integration Examples

### Example 1: Full-page Event Monitor

Dedicated page for monitoring all events with filters:

```typescript
// pages/EventsPage.tsx
import { EventTimeline } from '@/components/EventTimeline';

export default function EventsPage() {
  return (
    <div className="container mx-auto mt-12 max-w-5xl">
      <h1>Event Stream</h1>

      <EventTimeline
        kinds={['*']}
        showStatus={true}
        maxEvents={100}
      />
    </div>
  );
}
```

**Route:** `/events`

### Example 2: Embedded in TaskDetail

Show task-related events in task detail page:

```typescript
// pages/TaskDetail.tsx
import { EventTimeline } from '@/components/EventTimeline';
import { useParams } from 'react-router-dom';

export default function TaskDetail() {
  const { id } = useParams();

  return (
    <div>
      {/* Task content, comments, etc. */}

      {/* Real-time event stream for this task */}
      <div className="mt-8">
        <h2>Real-time Events</h2>
        <EventTimeline
          kinds={['task.*', 'agent.*', 'artifact.*']}
          taskId={id}
          showStatus={false}
          maxEvents={20}
        />
      </div>
    </div>
  );
}
```

### Example 3: Agent Activity Monitor

Sidebar showing recent agent activity:

```typescript
// components/AgentSidebar.tsx
import { EventTimeline } from '@/components/EventTimeline';

function AgentSidebar({ agentId }: { agentId: string }) {
  return (
    <aside className="w-80 border-l border-slate-200 p-4">
      <h3 className="text-sm font-medium text-slate-700 mb-4">
        Agent Activity
      </h3>

      <EventTimeline
        kinds={['agent.*']}
        agentId={agentId}
        showStatus={false}
        maxEvents={15}
      />
    </aside>
  );
}
```

### Example 4: Historical Replay Dashboard

Dashboard showing historical event replay:

```typescript
// pages/HistoryDashboard.tsx
import { EventTimeline } from '@/components/EventTimeline';
import { useState } from 'react';

export default function HistoryDashboard() {
  const [cursor, setCursor] = useState(0);

  return (
    <div>
      <h1>Event History</h1>

      <div className="mb-4">
        <label>Starting Cursor:</label>
        <input
          type="number"
          value={cursor}
          onChange={(e) => setCursor(parseInt(e.target.value))}
        />
      </div>

      <EventTimeline
        kinds={['*']}
        cursor={cursor}
        showStatus={true}
        maxEvents={200}
      />
    </div>
  );
}
```

### Example 5: Multi-filter Event Monitor

Allow users to switch between different event filters:

```typescript
// pages/EventMonitor.tsx
import { EventTimeline } from '@/components/EventTimeline';
import { useState } from 'react';

export default function EventMonitor() {
  const [filter, setFilter] = useState<string[]>(['*']);

  const filters = [
    { label: 'All', value: ['*'] },
    { label: 'Tasks', value: ['task.*'] },
    { label: 'Agents', value: ['agent.*'] },
    { label: 'Artifacts', value: ['artifact.*'] },
  ];

  return (
    <div>
      {/* Filter buttons */}
      <div className="flex gap-2 mb-4">
        {filters.map((f) => (
          <button
            key={f.label}
            onClick={() => setFilter(f.value)}
            className={filter === f.value ? 'active' : ''}
          >
            {f.label}
          </button>
        ))}
      </div>

      {/* Event timeline with current filter */}
      <EventTimeline
        kinds={filter}
        showStatus={true}
        maxEvents={50}
      />
    </div>
  );
}
```

---

## Advanced Usage

### Custom Authentication Token

Override the default token from localStorage:

```typescript
import { getCustomToken } from '@/lib/auth';

function SecureEventMonitor() {
  const token = getCustomToken();

  return (
    <EventTimeline
      kinds={['*']}
      token={token}  // Custom token
      showStatus={true}
      maxEvents={50}
    />
  );
}
```

### Catch-up After Disconnect

Resume from last seen cursor:

```typescript
import { EventTimeline } from '@/components/EventTimeline';
import { useState, useEffect } from 'react';

function ResilientEventMonitor() {
  const [lastCursor, setLastCursor] = useState(() => {
    return parseInt(localStorage.getItem('lastEventCursor') || '0');
  });

  return (
    <EventTimeline
      kinds={['*']}
      cursor={lastCursor}  // Resume from last cursor
      showStatus={true}
      maxEvents={50}
    />
  );
}
```

**Note:** The `useEventStream` hook automatically tracks cursors internally.

### Performance Optimization

For high-volume event streams, limit the display:

```typescript
// Show only last 20 events for better performance
<EventTimeline
  kinds={['*']}
  showStatus={true}
  maxEvents={20}  // Lower limit = better performance
/>
```

Memory usage grows linearly with `maxEvents`. Recommended values:
- **Embedded views:** 10-20 events
- **Dedicated pages:** 50-100 events
- **History dashboards:** 100-200 events

---

## Styling and Customization

### Hide Status Bar

For cleaner embedded views:

```typescript
<EventTimeline
  kinds={['task.*']}
  showStatus={false}  // No connection status bar
  maxEvents={15}
/>
```

### Custom Container

Wrap in your own styled container:

```typescript
<div className="border border-slate-300 rounded-lg shadow-sm">
  <div className="p-4 bg-slate-50 border-b border-slate-300">
    <h3 className="text-sm font-medium">Live Events</h3>
  </div>

  <EventTimeline
    kinds={['*']}
    showStatus={false}
    maxEvents={20}
  />
</div>
```

### Responsive Layout

The component is responsive by default. For mobile optimization:

```typescript
<div className="w-full lg:w-3/4">
  <EventTimeline
    kinds={['task.*']}
    showStatus={true}
    maxEvents={30}
  />
</div>
```

---

## Connection States

The EventTimeline displays different states:

### 1. Connecting

Initial state when WebSocket is connecting:
- Shows "Disconnected" status
- Event list is empty or shows skeleton loaders

### 2. Connected

WebSocket connected successfully:
- Shows "Connected" with green dot
- Events stream in real-time

### 3. Replaying

Historical event replay in progress:
- Shows "Replaying history" badge
- Events load rapidly
- "Replay complete: N events" message when done

### 4. Disconnected

Connection lost:
- Shows "Disconnected" with red dot
- "Reconnect" button appears
- Auto-reconnection with exponential backoff

### 5. Error

Server-side error or lag:
- Red error banner with message
- Connection may remain open
- User can manually reconnect

---

## Error Handling

### Authentication Errors

If token is invalid:
- Connection closes immediately
- No error message (security)
- User must refresh token and reload page

### Stream Lag Errors

If client is too slow:
```
Error: Event stream lagged by 50 events, consider reconnecting with cursor
```

**Solution:**
1. Note last cursor received
2. Click "Reconnect" button
3. Historical replay catches up

### Network Errors

Connection drops due to network issues:
- Auto-reconnection every 1s, 2s, 4s, 8s... (up to 30s)
- Up to 10 reconnection attempts
- Manual reconnect resets counter

---

## Demo Page

Visit the demo page to see all usage examples:

**Route:** `/events/demo`

The demo page includes:
1. All Events (no filter)
2. Task Events Only
3. Agent Events Only
4. Historical Replay
5. Minimal UI
6. Integration examples with code snippets

---

## Best Practices

### 1. Choose Appropriate Filters

Don't subscribe to all events unless necessary:

```typescript
// ❌ Too broad for task detail page
<EventTimeline kinds={['*']} taskId={id} />

// ✅ Focused on relevant events
<EventTimeline kinds={['task.*', 'agent.*']} taskId={id} />
```

### 2. Set Reasonable Limits

Don't display too many events:

```typescript
// ❌ May cause performance issues
<EventTimeline kinds={['*']} maxEvents={1000} />

// ✅ Balanced performance and usefulness
<EventTimeline kinds={['*']} maxEvents={50} />
```

### 3. Hide Status in Embedded Views

Keep UI clean when embedding:

```typescript
// ✅ Minimal UI for sidebar/embedded views
<EventTimeline
  kinds={['agent.*']}
  agentId={agentId}
  showStatus={false}
  maxEvents={15}
/>
```

### 4. Use Historical Replay Wisely

Only replay when catching up:

```typescript
// ❌ Always replay from 0 (slow)
<EventTimeline kinds={['*']} cursor={0} />

// ✅ Replay only when needed
const lastCursor = getLastSeenCursor();
<EventTimeline kinds={['*']} cursor={lastCursor} />
```

### 5. Combine with Other UI Elements

EventTimeline works well with tabs, accordions, and collapsible sections:

```typescript
<Tabs>
  <TabsList>
    <TabsTrigger value="comments">Comments</TabsTrigger>
    <TabsTrigger value="events">Events</TabsTrigger>
  </TabsList>

  <TabsContent value="comments">
    {/* Comments section */}
  </TabsContent>

  <TabsContent value="events">
    <EventTimeline kinds={['task.*']} taskId={id} />
  </TabsContent>
</Tabs>
```

---

## Troubleshooting

### Events Not Appearing

**Symptoms:**
- Component shows "Connected" but no events

**Solutions:**
1. Check filter patterns match event kinds
2. Verify server is emitting events (`POST /api/event-bus/emit`)
3. Check browser console for errors
4. Verify `taskId` or `agentId` filters are correct

### Connection Keeps Dropping

**Symptoms:**
- Frequent "Disconnected" status
- Repeated reconnection attempts

**Solutions:**
1. Check server logs for WebSocket errors
2. Verify network stability
3. Increase reconnection timeout (code modification needed)
4. Check if firewall is blocking WebSocket connections

### Events Load Slowly

**Symptoms:**
- Long delay between event emit and display
- "Replaying history" takes too long

**Solutions:**
1. Reduce `maxEvents` limit
2. Use more specific `kinds` filters
3. Check server performance and database load
4. Verify WebSocket is not falling back to polling

### Memory Usage Grows

**Symptoms:**
- Browser tab uses excessive memory
- Page becomes slow over time

**Solutions:**
1. Lower `maxEvents` limit (default 50 → 20)
2. Periodically click "Clear" button
3. Use `showStatus={false}` for embedded views
4. Reload page if memory exceeds 500MB

---

## Related Documentation

- **WebSocket API Reference:** `docs/WEBSOCKET_EVENTS.md`
- **Integration Guide:** `docs/EVENT_TIMELINE_INTEGRATION.md`
- **Testing Guide:** `docs/PHASE3_TESTING.md`
- **Completion Report:** `docs/PHASE3_COMPLETION.md`

---

## Support

For issues or questions:
1. Check browser console for errors
2. Verify server is running (`http://localhost:3000/health`)
3. Check WebSocket connection in Network tab
4. Review server logs for WebSocket errors

---

**Last Updated:** 2026-02-27
