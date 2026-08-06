-- Agent runtime transient state baseline (PostgreSQL).
--
-- This file is the idempotent initialization baseline: the former v2-v5
-- evolution layers are folded in below. Schema evolution is coordinated by
-- `src/schema_migrations.rs`; do not add destructive DDL here.

CREATE TABLE IF NOT EXISTS sessions (
    session_id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL,
    kind TEXT NOT NULL DEFAULT 'main',
    source TEXT NOT NULL DEFAULT 'api',
    state TEXT NOT NULL DEFAULT 'created',
    title TEXT,
    model TEXT,
    cwd TEXT,
    provider_id TEXT,
    bridge_id TEXT,
    token_usage_json TEXT,
    message_count BIGINT DEFAULT 0,
    owner_tenant_id TEXT,
    owner_user_ref TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT,
    metadata_json TEXT
);

CREATE TABLE IF NOT EXISTS messages (
    message_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(session_id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL,
    metadata_json TEXT
);

CREATE TABLE IF NOT EXISTS tasks (
    task_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(session_id) ON DELETE CASCADE,
    instruction TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'created',
    created_at TEXT NOT NULL,
    updated_at TEXT
);

CREATE TABLE IF NOT EXISTS events (
    event_id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES sessions(session_id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    severity TEXT NOT NULL,
    payload TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS permissions (
    permission_request_id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES sessions(session_id) ON DELETE CASCADE,
    category TEXT NOT NULL,
    resource TEXT NOT NULL,
    side_effect_level TEXT NOT NULL,
    reason TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    owner_tenant_id TEXT,
    owner_user_ref TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT
);

-- Indexes are created by migration v3 after legacy columns/FKs are repaired.

-- Consolidated initialization section: repair_legacy_runtime_schema
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

-- Consolidated initialization section: add_stable_pagination_indexes
-- Stable keyset pagination and ownership indexes (PostgreSQL, migration v3).

CREATE INDEX IF NOT EXISTS idx_sessions_updated_at
    ON sessions(COALESCE(updated_at, created_at) DESC, session_id DESC);

DROP INDEX IF EXISTS idx_sessions_owner_tenant;
CREATE INDEX idx_sessions_owner_tenant
    ON sessions(owner_tenant_id, COALESCE(updated_at, created_at) DESC, session_id DESC);

DROP INDEX IF EXISTS idx_sessions_owner_user;
CREATE INDEX idx_sessions_owner_user
    ON sessions(owner_user_ref, COALESCE(updated_at, created_at) DESC, session_id DESC);

DROP INDEX IF EXISTS idx_messages_session_created_at;
CREATE INDEX IF NOT EXISTS idx_messages_session_created_at_message_id
    ON messages(session_id, created_at ASC, message_id ASC);

DROP INDEX IF EXISTS idx_tasks_session_created_at;
CREATE INDEX IF NOT EXISTS idx_tasks_session_created_at_task_id
    ON tasks(session_id, created_at ASC, task_id ASC);

DROP INDEX IF EXISTS idx_events_session_created_at;
CREATE INDEX IF NOT EXISTS idx_events_session_created_at_event_id
    ON events(session_id, created_at ASC, event_id ASC);

DROP INDEX IF EXISTS idx_events_created_at;
CREATE INDEX IF NOT EXISTS idx_events_created_at_event_id
    ON events(created_at DESC, event_id DESC);

CREATE INDEX IF NOT EXISTS idx_permissions_owner_tenant_status_created_id
    ON permissions(owner_tenant_id, status, created_at DESC, permission_request_id DESC);
CREATE INDEX IF NOT EXISTS idx_permissions_owner_user_status_created_id
    ON permissions(owner_user_ref, status, created_at DESC, permission_request_id DESC);
CREATE INDEX IF NOT EXISTS idx_permissions_session_id ON permissions(session_id);
CREATE INDEX IF NOT EXISTS idx_permissions_status_created_at
    ON permissions(status, created_at DESC, permission_request_id DESC);

-- Consolidated initialization section: add_runtime_retention_indexes
-- Bounded runtime retention access paths (PostgreSQL, migration v4).

CREATE INDEX IF NOT EXISTS idx_sessions_retention_state_time_id
    ON sessions(lower(state), COALESCE(updated_at, created_at), session_id);

CREATE INDEX IF NOT EXISTS idx_messages_retention_created_id
    ON messages(created_at, message_id);

CREATE INDEX IF NOT EXISTS idx_tasks_retention_state_time_id
    ON tasks(lower(state), COALESCE(updated_at, created_at), task_id);

CREATE INDEX IF NOT EXISTS idx_permissions_retention_time_id
    ON permissions(COALESCE(updated_at, created_at), permission_request_id);

-- Consolidated initialization section: add_durable_runtime_execution
-- Durable task/run/step execution and permission resume (PostgreSQL, migration v5).

CREATE TABLE IF NOT EXISTS runs (
    run_id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(task_id) ON DELETE CASCADE,
    session_id TEXT NOT NULL REFERENCES sessions(session_id) ON DELETE CASCADE,
    attempt BIGINT NOT NULL CHECK (attempt >= 1),
    state TEXT NOT NULL CHECK (
        state IN ('created', 'planning', 'executing', 'awaiting_permission',
                  'paused', 'completed', 'failed', 'cancelled')
    ),
    next_attempt_at TEXT,
    lease_owner TEXT,
    lease_expires_at TEXT,
    fencing_token BIGINT NOT NULL DEFAULT 0 CHECK (fencing_token >= 0),
    cancel_requested_at TEXT,
    started_at TEXT,
    finished_at TEXT,
    error_kind TEXT,
    error_code TEXT,
    error_detail TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (task_id, attempt)
);

CREATE TABLE IF NOT EXISTS steps (
    step_id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES runs(run_id) ON DELETE CASCADE,
    sequence_no BIGINT NOT NULL CHECK (sequence_no >= 0),
    action_kind TEXT NOT NULL CHECK (
        action_kind IN ('model_call', 'tool_call', 'memory_read', 'memory_write',
                        'host_operation', 'protocol_send', 'handoff',
                        'wait_for_user', 'internal')
    ),
    state TEXT NOT NULL CHECK (
        state IN ('created', 'ready', 'running', 'awaiting_permission',
                  'completed', 'failed', 'skipped', 'cancelled')
    ),
    provider_id TEXT,
    descriptor_revision TEXT,
    policy_revision TEXT,
    causation_step_id TEXT REFERENCES steps(step_id) ON DELETE SET NULL,
    idempotency_key_hash TEXT,
    result_json TEXT,
    error_kind TEXT,
    error_code TEXT,
    error_detail TEXT,
    started_at TEXT,
    finished_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (run_id, sequence_no)
);

CREATE TABLE IF NOT EXISTS permission_operations (
    permission_request_id TEXT PRIMARY KEY
        REFERENCES permissions(permission_request_id) ON DELETE CASCADE,
    run_id TEXT NOT NULL REFERENCES runs(run_id) ON DELETE CASCADE,
    step_id TEXT NOT NULL REFERENCES steps(step_id) ON DELETE CASCADE,
    tool_call_id TEXT NOT NULL UNIQUE,
    provider_id TEXT NOT NULL,
    descriptor_revision TEXT NOT NULL,
    policy_revision TEXT NOT NULL,
    payload_kind TEXT NOT NULL CHECK (payload_kind IN ('ciphertext', 'secret_ref')),
    payload_ref TEXT NOT NULL,
    payload_digest TEXT NOT NULL,
    encryption_key_id TEXT,
    state TEXT NOT NULL CHECK (
        state IN ('pending', 'decided', 'claimable', 'executing', 'completed',
                  'failed', 'expired', 'cancelled')
    ),
    expires_at TEXT NOT NULL,
    lease_owner TEXT,
    lease_expires_at TEXT,
    fencing_token BIGINT NOT NULL DEFAULT 0 CHECK (fencing_token >= 0),
    result_json TEXT,
    error_kind TEXT,
    error_code TEXT,
    error_detail TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_runs_claim
    ON runs(state, next_attempt_at, lease_expires_at, created_at, run_id);
CREATE INDEX IF NOT EXISTS idx_runs_task_created
    ON runs(task_id, created_at, run_id);
CREATE INDEX IF NOT EXISTS idx_runs_session_created
    ON runs(session_id, created_at, run_id);
CREATE INDEX IF NOT EXISTS idx_runs_lease_expiry
    ON runs(lease_expires_at, run_id);
CREATE INDEX IF NOT EXISTS idx_runs_retention_state_time_id
    ON runs(state, finished_at, updated_at, run_id);

CREATE INDEX IF NOT EXISTS idx_steps_run_sequence
    ON steps(run_id, sequence_no);
CREATE INDEX IF NOT EXISTS idx_steps_retention_state_time_id
    ON steps(state, finished_at, updated_at, step_id);

CREATE INDEX IF NOT EXISTS idx_permission_operations_claim
    ON permission_operations(state, expires_at, lease_expires_at,
                             created_at, permission_request_id);
CREATE INDEX IF NOT EXISTS idx_permission_operations_run_step
    ON permission_operations(run_id, step_id);
CREATE INDEX IF NOT EXISTS idx_permission_operations_retention_state_time_id
    ON permission_operations(state, updated_at, permission_request_id);
