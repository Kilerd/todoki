-- Create projects table
CREATE TABLE projects (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL UNIQUE,
    description TEXT,
    color VARCHAR(7) NOT NULL DEFAULT '#6B7280',
    archived BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create default project for migration
INSERT INTO projects (name, description, color) VALUES ('Inbox', 'Default inbox project', '#3B82F6');

-- Add project_id to tasks (nullable initially for migration)
ALTER TABLE tasks ADD COLUMN project_id UUID REFERENCES projects(id) ON DELETE RESTRICT;

-- Migrate existing groups to projects (skip 'default' as it maps to 'Inbox')
INSERT INTO projects (name)
SELECT DISTINCT "group" FROM tasks
WHERE "group" IS NOT NULL AND "group" != '' AND "group" != 'default'
ON CONFLICT (name) DO NOTHING;

-- Update tasks with project_id based on group name
UPDATE tasks t SET project_id = p.id FROM projects p WHERE t."group" = p.name;

-- Set default project (Inbox) for tasks with 'default' group or NULL project_id
UPDATE tasks SET project_id = (SELECT id FROM projects WHERE name = 'Inbox')
WHERE project_id IS NULL;

-- Now make project_id NOT NULL
ALTER TABLE tasks ALTER COLUMN project_id SET NOT NULL;

-- Drop group column
ALTER TABLE tasks DROP COLUMN "group";

-- Add indexes for performance
CREATE INDEX idx_tasks_project_id ON tasks(project_id);
CREATE INDEX idx_projects_name ON projects(name);
CREATE INDEX idx_projects_archived ON projects(archived);
