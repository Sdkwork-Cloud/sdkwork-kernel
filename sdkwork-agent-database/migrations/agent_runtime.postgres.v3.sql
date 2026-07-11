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
