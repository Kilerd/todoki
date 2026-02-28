-- Insert Human Operator agent
-- This special agent represents human actions in the Event Bus

INSERT INTO agents (
    id,
    name,
    workdir,
    command,
    args,
    execution_mode,
    project_id,
    status,
    created_at,
    updated_at
) VALUES (
    '00000000-0000-0000-0000-000000000001',
    'Human Operator',
    '/dev/null',
    'human',
    '[]',
    'local',
    (SELECT id FROM projects WHERE name = 'Inbox'),
    'active',
    NOW(),
    NOW()
) ON CONFLICT (id) DO NOTHING;

COMMENT ON TABLE agents IS 'Agents table. Special agent 00000000-0000-0000-0000-000000000001 represents human operators.';
