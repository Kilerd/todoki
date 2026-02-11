-- Task can be linked to an Agent for automated execution
ALTER TABLE tasks ADD COLUMN agent_id UUID REFERENCES agents(id) ON DELETE SET NULL;
CREATE INDEX idx_tasks_agent_id ON tasks(agent_id);

-- Project templates for different roles
ALTER TABLE projects ADD COLUMN general_template TEXT;
ALTER TABLE projects ADD COLUMN business_template TEXT;
ALTER TABLE projects ADD COLUMN coding_template TEXT;
ALTER TABLE projects ADD COLUMN qa_template TEXT;
