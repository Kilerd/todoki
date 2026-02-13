-- Add task_id to artifacts table to associate artifacts with tasks
-- task_id is required - artifacts must belong to a task
ALTER TABLE artifacts ADD COLUMN task_id UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE;

-- Create index for task_id lookups
CREATE INDEX idx_artifacts_task ON artifacts(task_id);
