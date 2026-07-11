-- Agent runtime transient state baseline (PostgreSQL, migration v1).
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
