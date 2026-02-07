-- Todoki initial schema
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE tasks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    priority INTEGER NOT NULL DEFAULT 0,
    content TEXT NOT NULL,
    "group" VARCHAR(255) NOT NULL DEFAULT 'default',
    status VARCHAR(50) NOT NULL DEFAULT 'backlog',
    create_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    archived BOOLEAN NOT NULL DEFAULT false
);

CREATE TABLE task_events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    task_id UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    event_type VARCHAR(50) NOT NULL,
    datetime TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    state VARCHAR(50),
    from_state VARCHAR(50)
);

CREATE TABLE task_comments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    task_id UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    create_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for common queries
CREATE INDEX idx_tasks_status_archived ON tasks(status, archived);
CREATE INDEX idx_task_events_task_id ON task_events(task_id);
CREATE INDEX idx_task_events_datetime ON task_events(datetime DESC);
CREATE INDEX idx_task_comments_task_id ON task_comments(task_id);
