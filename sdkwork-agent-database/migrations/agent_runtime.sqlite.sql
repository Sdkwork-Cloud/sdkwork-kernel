-- Agent runtime transient state schema (SQLite).
-- Authority: sdkwork-agent-database/migrations/ (keep agent_runtime.postgres.sql in sync).

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

ALTER TABLE sessions ADD COLUMN owner_tenant_id TEXT;
ALTER TABLE sessions ADD COLUMN owner_user_ref TEXT;

CREATE INDEX IF NOT EXISTS idx_sessions_updated_at ON sessions(COALESCE(updated_at, created_at) DESC);
CREATE INDEX IF NOT EXISTS idx_sessions_owner_tenant ON sessions(owner_tenant_id);
CREATE INDEX IF NOT EXISTS idx_sessions_owner_user ON sessions(owner_user_ref);

CREATE TABLE IF NOT EXISTS messages (
    message_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL,
    metadata_json TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_messages_session_id ON messages(session_id);
CREATE INDEX IF NOT EXISTS idx_messages_session_created_at ON messages(session_id, created_at ASC);

CREATE TABLE IF NOT EXISTS tasks (
    task_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    instruction TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'created',
    created_at TEXT NOT NULL,
    updated_at TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_tasks_session_id ON tasks(session_id);
CREATE INDEX IF NOT EXISTS idx_tasks_session_created_at ON tasks(session_id, created_at ASC);

CREATE TABLE IF NOT EXISTS events (
    event_id TEXT PRIMARY KEY,
    session_id TEXT,
    event_type TEXT NOT NULL,
    severity TEXT NOT NULL,
    payload TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_events_session_id ON events(session_id);
CREATE INDEX IF NOT EXISTS idx_events_session_created_at ON events(session_id, created_at ASC);
CREATE INDEX IF NOT EXISTS idx_events_created_at ON events(created_at DESC);

DROP TABLE IF EXISTS agents;

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

CREATE INDEX IF NOT EXISTS idx_permissions_session_id ON permissions(session_id);
CREATE INDEX IF NOT EXISTS idx_permissions_status ON permissions(status);
CREATE INDEX IF NOT EXISTS idx_permissions_status_created_at ON permissions(status, created_at DESC);
