-- Add agent subscription fields for event-driven triggering
-- Phase 2: Agent Subscription & Triggering

-- Add subscription-related columns
ALTER TABLE agents
ADD COLUMN IF NOT EXISTS subscribed_events TEXT[] DEFAULT '{}',
ADD COLUMN IF NOT EXISTS last_cursor BIGINT DEFAULT 0,
ADD COLUMN IF NOT EXISTS auto_trigger BOOLEAN DEFAULT false;

-- Index for efficient subscription matching
-- GIN index supports array containment queries (@>)
CREATE INDEX IF NOT EXISTS idx_agents_subscribed_events
ON agents USING GIN(subscribed_events);

-- Index for finding agents that need to be triggered
CREATE INDEX IF NOT EXISTS idx_agents_auto_trigger
ON agents(auto_trigger)
WHERE auto_trigger = true;

-- Index for cursor-based queries
CREATE INDEX IF NOT EXISTS idx_agents_last_cursor
ON agents(last_cursor);

-- Add comments for documentation
COMMENT ON COLUMN agents.subscribed_events IS
'Event kinds this agent listens to. Supports wildcards (e.g., ["task.created", "agent.*"]). When a matching event is published, the agent may be auto-triggered.';

COMMENT ON COLUMN agents.last_cursor IS
'Last event cursor processed by this agent. Used for incremental event consumption to prevent duplicate triggers.';

COMMENT ON COLUMN agents.auto_trigger IS
'Whether to automatically trigger (start) this agent when a subscribed event is published. If false, agent must be started manually.';
