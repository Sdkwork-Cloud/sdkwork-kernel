-- Agent runtime transient state baseline (SQLite, migration v1).
--
-- This file is an idempotent baseline only. Schema evolution is coordinated by
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
    message_count INTEGER DEFAULT 0,
    owner_tenant_id TEXT,
    owner_user_ref TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT,
    metadata_json TEXT
);

CREATE TABLE IF NOT EXISTS messages (
    message_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL,
    metadata_json TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS tasks (
    task_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    instruction TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'created',
    created_at TEXT NOT NULL,
    updated_at TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS events (
    event_id TEXT PRIMARY KEY,
    session_id TEXT,
    event_type TEXT NOT NULL,
    severity TEXT NOT NULL,
    payload TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS permissions (
    permission_request_id TEXT PRIMARY KEY,
    session_id TEXT,
    category TEXT NOT NULL,
    resource TEXT NOT NULL,
    side_effect_level TEXT NOT NULL,
    reason TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    owner_tenant_id TEXT,
    owner_user_ref TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE
);

-- Indexes are created by migration v3 after legacy columns/FKs are repaired:
-- idx_sessions_updated_at, idx_sessions_owner_tenant, idx_sessions_owner_user,
-- idx_messages_session_created_at_message_id,
-- idx_tasks_session_created_at_task_id,
-- idx_events_session_created_at_event_id, idx_events_created_at_event_id,
-- idx_permissions_owner_tenant_status_created_id,
-- idx_permissions_owner_user_status_created_id, idx_permissions_session_id,
-- idx_permissions_status_created_at.
