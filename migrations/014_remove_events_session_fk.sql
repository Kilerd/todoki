-- Remove session_id foreign key constraint from events table
-- session_id is used for event correlation but doesn't require
-- the session to exist in agent_sessions table

ALTER TABLE events DROP CONSTRAINT IF EXISTS events_session_id_fkey;
