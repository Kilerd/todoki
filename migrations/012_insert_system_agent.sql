-- Insert System agent for internal events
-- This special agent represents system-generated events (relay commands, etc.)

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
    '00000000-0000-0000-0000-000000000000',
    'System',
    '/dev/null',
    'system',
    '[]',
    'local',
    (SELECT id FROM projects WHERE name = 'Inbox'),
    'active',
    NOW(),
    NOW()
) ON CONFLICT (id) DO NOTHING;

COMMENT ON TABLE agents IS 'Agents table. Special agents: 00000000-0000-0000-0000-000000000000 for system events, 00000000-0000-0000-0000-000000000001 for human operators.';
