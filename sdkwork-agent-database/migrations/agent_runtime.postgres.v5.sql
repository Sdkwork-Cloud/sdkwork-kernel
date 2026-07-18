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
