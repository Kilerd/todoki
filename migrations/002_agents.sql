-- Agent support for remote execution

-- Agents table
CREATE TABLE agents (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    workdir TEXT NOT NULL,
    command TEXT NOT NULL,
    args TEXT NOT NULL DEFAULT '[]',
    execution_mode VARCHAR(50) NOT NULL DEFAULT 'local',
    relay_id VARCHAR(255),
    status VARCHAR(50) NOT NULL DEFAULT 'created',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Agent sessions table
CREATE TABLE agent_sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    relay_id VARCHAR(255),
    status VARCHAR(50) NOT NULL DEFAULT 'running',
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at TIMESTAMPTZ
);

-- Agent events (output stream)
CREATE TABLE agent_events (
    id BIGSERIAL PRIMARY KEY,
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    session_id UUID NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
    seq BIGINT NOT NULL,
    ts TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    stream VARCHAR(50) NOT NULL DEFAULT 'stdout',
    message TEXT NOT NULL
);

-- Indexes
CREATE INDEX idx_agents_status ON agents(status);
CREATE INDEX idx_agent_sessions_agent_id ON agent_sessions(agent_id);
CREATE INDEX idx_agent_sessions_relay_id ON agent_sessions(relay_id);
CREATE INDEX idx_agent_events_agent_seq ON agent_events(agent_id, seq);
CREATE INDEX idx_agent_events_session_id ON agent_events(session_id);
