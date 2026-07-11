-- Bounded runtime retention access paths (SQLite, migration v4).

CREATE INDEX IF NOT EXISTS idx_sessions_retention_state_time_id
    ON sessions(lower(state), COALESCE(updated_at, created_at), session_id);

CREATE INDEX IF NOT EXISTS idx_messages_retention_created_id
    ON messages(created_at, message_id);

CREATE INDEX IF NOT EXISTS idx_tasks_retention_state_time_id
    ON tasks(lower(state), COALESCE(updated_at, created_at), task_id);

CREATE INDEX IF NOT EXISTS idx_permissions_retention_time_id
    ON permissions(COALESCE(updated_at, created_at), permission_request_id);
