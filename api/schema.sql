-- Todoki Database Schema
-- Run this script to create tables in a new database

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE tasks (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    priority      INTEGER NOT NULL DEFAULT 0,
    content       TEXT NOT NULL,
    "group"       TEXT NOT NULL DEFAULT 'default',
    task_type     TEXT NOT NULL DEFAULT 'Todo',
    create_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    done          BOOLEAN NOT NULL DEFAULT FALSE,
    archived      BOOLEAN NOT NULL DEFAULT FALSE,
    current_state TEXT,
    states        TEXT[] DEFAULT ARRAY[]::TEXT[]
);

CREATE TABLE task_events (
    id         UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    task_id    UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    datetime   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    state      TEXT,
    from_state TEXT
);

CREATE TABLE task_comments (
    id        UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    task_id   UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    create_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    content   TEXT NOT NULL
);

CREATE INDEX idx_task_events_task_id ON task_events(task_id);
CREATE INDEX idx_task_events_datetime ON task_events(datetime DESC);
CREATE INDEX idx_task_comments_task_id ON task_comments(task_id);
CREATE INDEX idx_tasks_done_archived ON tasks(done, archived);
