-- Add role column to agents table
ALTER TABLE agents ADD COLUMN role VARCHAR(50) NOT NULL DEFAULT 'general';
