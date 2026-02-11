-- Add project_id to agents table
ALTER TABLE agents ADD COLUMN project_id UUID REFERENCES projects(id) ON DELETE RESTRICT;

-- Migrate existing agents to default Inbox project
UPDATE agents SET project_id = (SELECT id FROM projects WHERE name = 'Inbox') WHERE project_id IS NULL;

-- Make project_id NOT NULL
ALTER TABLE agents ALTER COLUMN project_id SET NOT NULL;

-- Add index for performance
CREATE INDEX idx_agents_project_id ON agents(project_id);
