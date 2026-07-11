-- Repair legacy PostgreSQL runtime schemas without deleting data (migration v2).

ALTER TABLE sessions ADD COLUMN IF NOT EXISTS provider_id TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS bridge_id TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS owner_tenant_id TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS owner_user_ref TEXT;

ALTER TABLE permissions ADD COLUMN IF NOT EXISTS owner_tenant_id TEXT;
ALTER TABLE permissions ADD COLUMN IF NOT EXISTS owner_user_ref TEXT;

ALTER TABLE messages ADD COLUMN IF NOT EXISTS metadata_json TEXT;
ALTER TABLE tasks ADD COLUMN IF NOT EXISTS updated_at TEXT;
ALTER TABLE events ADD COLUMN IF NOT EXISTS payload TEXT;
ALTER TABLE permissions ADD COLUMN IF NOT EXISTS updated_at TEXT;

-- Older SQLite-compatible schemas did not enforce child foreign keys. Preserve
-- any existing child rows by recovering their missing parent as an explicitly
-- quarantined runtime session before constraints are added.
WITH orphan_sessions AS (
    SELECT session_id, MIN(created_at) AS created_at
    FROM (
        SELECT session_id, created_at FROM messages
        UNION ALL
        SELECT session_id, created_at FROM tasks
        UNION ALL
        SELECT session_id, created_at FROM events WHERE session_id IS NOT NULL
        UNION ALL
        SELECT session_id, created_at FROM permissions WHERE session_id IS NOT NULL
    ) AS child_rows
    WHERE NOT EXISTS (
        SELECT 1 FROM sessions WHERE sessions.session_id = child_rows.session_id
    )
    GROUP BY session_id
)
INSERT INTO sessions (
    session_id, agent_id, kind, source, state, message_count,
    created_at, updated_at, metadata_json
)
SELECT
    session_id,
    'agent.runtime.migration',
    'main',
    'migration',
    'orphaned',
    0,
    created_at,
    created_at,
    '{"migrationReason":"recovered orphan child rows before foreign-key repair"}'
FROM orphan_sessions
ON CONFLICT (session_id) DO NOTHING;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conrelid = 'messages'::regclass
          AND contype = 'f'
          AND confrelid = 'sessions'::regclass
          AND pg_get_constraintdef(oid) LIKE
              'FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE%'
    ) THEN
        ALTER TABLE messages
            ADD CONSTRAINT fk_messages_session
            FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE;
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conrelid = 'tasks'::regclass
          AND contype = 'f'
          AND confrelid = 'sessions'::regclass
          AND pg_get_constraintdef(oid) LIKE
              'FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE%'
    ) THEN
        ALTER TABLE tasks
            ADD CONSTRAINT fk_tasks_session
            FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE;
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conrelid = 'events'::regclass
          AND contype = 'f'
          AND confrelid = 'sessions'::regclass
          AND pg_get_constraintdef(oid) LIKE
              'FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE%'
    ) THEN
        ALTER TABLE events
            ADD CONSTRAINT fk_events_session
            FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE;
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conrelid = 'permissions'::regclass
          AND contype = 'f'
          AND confrelid = 'sessions'::regclass
          AND pg_get_constraintdef(oid) LIKE
              'FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE%'
    ) THEN
        ALTER TABLE permissions
            ADD CONSTRAINT fk_permissions_session
            FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE;
    END IF;
END $$;
