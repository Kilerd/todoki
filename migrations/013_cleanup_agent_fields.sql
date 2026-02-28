-- Cleanup: Remove fields that don't need to be persisted
-- These fields are runtime information managed in memory

-- Drop old relay_id columns (runtime info, managed by RelayManager)
ALTER TABLE agents DROP COLUMN IF EXISTS relay_id;
ALTER TABLE agent_sessions DROP COLUMN IF EXISTS relay_id;

-- Drop subscription fields (runtime info, provided on WebSocket connect)
ALTER TABLE agents DROP COLUMN IF EXISTS subscribed_events;
ALTER TABLE agents DROP COLUMN IF EXISTS last_cursor;
ALTER TABLE agents DROP COLUMN IF EXISTS auto_trigger;
