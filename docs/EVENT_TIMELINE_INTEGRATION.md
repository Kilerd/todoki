# EventTimeline Component Integration Guide

The EventTimeline component provides real-time event streaming visualization for the Todoki frontend.

## Components

### 1. `useEventStream` Hook

Located at: `web/src/hooks/useEventStream.ts`

A React hook that manages WebSocket connection to the event bus.

**Features:**
- WebSocket connection with auto-reconnection (exponential backoff)
- Historical event replay from cursor
- Event filtering by kinds, agent ID, task ID
- Connection state management
- Error handling

**Usage:**

```typescript
import { useEventStream } from '@/hooks/useEventStream';

const { events, isConnected, isReplaying, error, reconnect, clearEvents } = useEventStream({
  kinds: ['task.*', 'agent.*'],
  cursor: 100,
  agentId: 'some-agent-id',
  taskId: 'some-task-id',
  token: localStorage.getItem('token') || '',
  autoReconnect: true,
  maxReconnectAttempts: 10,
});
```

### 2. `EventTimeline` Component

Located at: `web/src/components/EventTimeline.tsx`

A visual timeline component that displays events in real-time.

**Features:**
- Event list with icons based on event kind
- Color-coded event types
- Expandable event data details
- Connection status indicator
- Replay progress indicator
- Manual reconnect and clear actions

**Props:**

```typescript
interface EventTimelineProps {
  kinds?: string[];           // Event kind patterns (e.g., ["task.*", "agent.*"])
  cursor?: number;            // Starting cursor for historical replay
  agentId?: string;           // Filter by agent ID
  taskId?: string;            // Filter by task ID
  token?: string;             // Authentication token
  showStatus?: boolean;       // Show connection status bar (default: true)
  maxEvents?: number;         // Max events to display (default: 50)
  autoScroll?: boolean;       // Auto-scroll to latest event
}
```

**Usage:**

```typescript
import { EventTimeline } from '@/components/EventTimeline';

function MyComponent() {
  return (
    <EventTimeline
      kinds={['task.*']}
      taskId={taskId}
      showStatus={true}
      maxEvents={50}
    />
  );
}
```

### 3. `EventsPage` - Standalone Events Page

Located at: `web/src/pages/EventsPage.tsx`

A dedicated page for viewing the event stream with advanced filtering controls.

**Features:**
- Quick filter patterns (All, Task, Agent, Artifact, Permission, System)
- Custom kind filter with wildcard support
- Historical replay with cursor input
- Filter state management

**Route:** `/events`

## Integration Examples

### Example 1: Add EventTimeline to TaskDetail Page

Show task-specific events in the TaskDetail page:

```typescript
// web/src/pages/TaskDetail.tsx

import { EventTimeline } from '@/components/EventTimeline';

export default function TaskDetail() {
  const { id } = useParams();
  // ... existing code

  return (
    <div className="container mx-auto mt-12 max-w-3xl pb-12">
      {/* ... existing content ... */}

      {/* Task-specific Events */}
      <div className="mt-8">
        <h2 className="text-xs font-medium text-slate-400 uppercase tracking-wider mb-4">
          Real-time Events
        </h2>
        <EventTimeline
          kinds={['task.*', 'agent.*']}
          taskId={id}
          showStatus={false}
          maxEvents={20}
        />
      </div>
    </div>
  );
}
```

### Example 2: Agent-specific Events

Show events for a specific agent:

```typescript
import { EventTimeline } from '@/components/EventTimeline';

function AgentDetail({ agentId }: { agentId: string }) {
  return (
    <div>
      <h2>Agent Events</h2>
      <EventTimeline
        kinds={['agent.*']}
        agentId={agentId}
        showStatus={true}
        maxEvents={30}
      />
    </div>
  );
}
```

### Example 3: Subscribe to Specific Event Kinds

```typescript
import { EventTimeline } from '@/components/EventTimeline';

function TaskCreationMonitor() {
  return (
    <EventTimeline
      kinds={['task.created', 'task.completed']}
      showStatus={true}
      maxEvents={100}
    />
  );
}
```

## Event Icons and Colors

The component automatically assigns icons and colors based on event kind:

| Event Kind Pattern | Icon | Color |
|-------------------|------|-------|
| `task.created` | FileText | Blue |
| `task.completed` | CheckCircle2 | Green |
| `task.failed` | XCircle | Red |
| `agent.started` | PlayCircle | Green |
| `agent.stopped` | StopCircle | Yellow |
| `agent.requirement_analyzed` | Bot | Purple |
| `artifact.*` | FileText | Indigo |
| `permission.*` | Circle | Orange |
| `system.*` | Activity | Slate |

## Styling

The component uses Tailwind CSS and shadcn/ui components for styling:

- Uses `Card`, `Badge`, `Button`, `Skeleton` from shadcn/ui
- Color scheme matches the rest of the Todoki UI
- Hover effects and transitions for better UX
- Responsive design

## WebSocket Connection Management

The hook manages WebSocket connections intelligently:

1. **Auto-connect on mount**: Connects when component mounts
2. **Auto-reconnect**: Exponential backoff (1s, 2s, 4s, ..., max 30s)
3. **Clean disconnect**: Closes connection when component unmounts
4. **Manual reconnect**: `reconnect()` function resets attempt counter
5. **Connection state**: `isConnected` boolean tracks WebSocket state

## Performance Considerations

- **Max events limit**: Prevents memory growth by keeping only last N events
- **Event batching**: React state updates are batched by default
- **Efficient re-renders**: Uses `useMemo` for computed values
- **Minimal DOM updates**: Only new events trigger re-renders

## Error Handling

The component handles various error scenarios:

- **Connection failures**: Shows "Disconnected" status with reconnect button
- **Authentication errors**: Connection immediately closes (no message)
- **Stream lag**: Server sends error message, displayed in red banner
- **Parse errors**: Logged to console, doesn't break UI

## Testing

### Manual Testing

1. Start the server: `cargo run --bin todoki-server`
2. Start the frontend: `cd web && npm run dev`
3. Navigate to `/events`
4. Verify:
   - WebSocket connects
   - Events appear in real-time
   - Filters work correctly
   - Reconnection works after server restart
   - Historical replay works with cursor parameter

### Integration Testing

```typescript
import { render, screen, waitFor } from '@testing-library/react';
import { EventTimeline } from '@/components/EventTimeline';

test('displays events in real-time', async () => {
  render(<EventTimeline kinds={['task.*']} />);

  await waitFor(() => {
    expect(screen.getByText(/Connected/i)).toBeInTheDocument();
  });

  // Trigger events from backend
  // ...

  await waitFor(() => {
    expect(screen.getByText(/task.created/i)).toBeInTheDocument();
  });
});
```

## Future Enhancements

Potential improvements for the EventTimeline component:

1. **Virtual scrolling**: Handle thousands of events efficiently
2. **Event search**: Filter events by content/data
3. **Event grouping**: Group related events (e.g., by task or session)
4. **Time range filter**: Show events from specific time period
5. **Event export**: Download events as JSON/CSV
6. **Event notifications**: Desktop notifications for important events
7. **Custom event renderers**: Allow custom rendering for specific event kinds
8. **Live update indicator**: Show "New events" badge when auto-scroll is off
9. **Event detail modal**: Full-screen view for complex event data
10. **WebSocket metrics**: Show latency, message rate, reconnection stats

## Troubleshooting

### Events not appearing

1. Check WebSocket connection status (should be "Connected")
2. Verify authentication token is valid
3. Check browser console for errors
4. Verify server is running and WebSocket endpoint is accessible
5. Check event kind filters match emitted events

### Connection keeps dropping

1. Check network stability
2. Verify server is not restarting frequently
3. Check server logs for WebSocket errors
4. Increase `maxReconnectAttempts` if needed

### Historical replay not working

1. Verify `cursor` parameter is set correctly
2. Check that events exist at that cursor position
3. Server only replays up to 1000 events (use HTTP API for more)
4. Verify authentication token has permission to read historical events

## Related Documentation

- [WebSocket Events API](./WEBSOCKET_EVENTS.md)
- [Event Bus Design](./event-bus-design.md)
- [Phase 2 Implementation](./PHASE2_IMPLEMENTATION.md)
