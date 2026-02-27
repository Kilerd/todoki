-- Create events table for Event Bus
-- Phase 1: Event Bus Core

CREATE TABLE IF NOT EXISTS events (
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
CREATE INDEX idx_events_session ON events(session_id) WHERE session_id IS NOT NULL;

-- Comments for documentation
COMMENT ON TABLE events IS 'Event Bus storage. All system events are persisted here for agent collaboration, replay, and audit.';
COMMENT ON COLUMN events.cursor IS 'Global monotonic sequence number for event ordering and incremental consumption';
COMMENT ON COLUMN events.kind IS 'Event kind (namespace.action format, e.g., "task.created", "agent.started")';
COMMENT ON COLUMN events.time IS 'Event timestamp (UTC)';
COMMENT ON COLUMN events.agent_id IS 'Agent that emitted this event (use 00000000-0000-0000-0000-000000000000 for system events)';
COMMENT ON COLUMN events.data IS 'Event-specific data (JSON)';
