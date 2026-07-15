//! Idempotent upsert SQL shared by SQLite and PostgreSQL runtime repositories.
//!
//! SQLite uses `ON CONFLICT … DO UPDATE` (never `INSERT OR REPLACE`) so child rows
//! referenced by foreign keys are not cascade-deleted on session updates.

pub mod sqlite {
    pub const SAVE_SESSION: &str = "INSERT INTO sessions (
                session_id, agent_id, kind, source, state, title, model, cwd,
                provider_id, bridge_id, token_usage_json, message_count,
                owner_tenant_id, owner_user_ref,
                created_at, updated_at, metadata_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
            ON CONFLICT(session_id) DO UPDATE SET
                agent_id = excluded.agent_id,
                kind = excluded.kind,
                source = excluded.source,
                state = excluded.state,
                title = excluded.title,
                model = excluded.model,
                cwd = excluded.cwd,
                provider_id = excluded.provider_id,
                bridge_id = excluded.bridge_id,
                token_usage_json = excluded.token_usage_json,
                updated_at = excluded.updated_at,
                metadata_json = excluded.metadata_json
            WHERE sessions.provider_id IS excluded.provider_id
              AND (
                LOWER(sessions.state) NOT IN ('closed', 'failed', 'archived')
                OR LOWER(excluded.state) IN ('closed', 'failed', 'archived')
              )";

    /// Provider snapshots own the aggregate counters and scoped owner fields;
    /// the caller has already merged them with newer local runtime values.
    pub const SAVE_PROVIDER_SESSION: &str = "INSERT INTO sessions (
                session_id, agent_id, kind, source, state, title, model, cwd,
                provider_id, bridge_id, token_usage_json, message_count,
                owner_tenant_id, owner_user_ref,
                created_at, updated_at, metadata_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
            ON CONFLICT(session_id) DO UPDATE SET
                agent_id = excluded.agent_id,
                kind = excluded.kind,
                source = excluded.source,
                state = excluded.state,
                title = excluded.title,
                model = excluded.model,
                cwd = excluded.cwd,
                provider_id = excluded.provider_id,
                bridge_id = excluded.bridge_id,
                token_usage_json = excluded.token_usage_json,
                message_count = excluded.message_count,
                owner_tenant_id = excluded.owner_tenant_id,
                owner_user_ref = excluded.owner_user_ref,
                updated_at = excluded.updated_at,
                metadata_json = excluded.metadata_json";

    pub const SAVE_MESSAGE: &str = "INSERT INTO messages (
                message_id, session_id, role, content, created_at, metadata_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(message_id) DO NOTHING";

    pub const SAVE_TASK: &str = "INSERT INTO tasks (
                task_id, session_id, instruction, state, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(task_id) DO UPDATE SET
                session_id = excluded.session_id,
                instruction = excluded.instruction,
                state = excluded.state,
                created_at = excluded.created_at,
                updated_at = excluded.updated_at";

    pub const SAVE_EVENT: &str = "INSERT INTO events (
                event_id, session_id, event_type, severity, payload, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(event_id) DO NOTHING";

    pub const SAVE_PERMISSION: &str = "INSERT INTO permissions (
                permission_request_id, session_id, category, resource,
                side_effect_level, reason, status, owner_tenant_id,
                owner_user_ref, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(permission_request_id) DO UPDATE SET
                session_id = excluded.session_id,
                category = excluded.category,
                resource = excluded.resource,
                side_effect_level = excluded.side_effect_level,
                reason = excluded.reason,
                status = excluded.status,
                owner_tenant_id = excluded.owner_tenant_id,
                owner_user_ref = excluded.owner_user_ref,
                created_at = excluded.created_at,
                updated_at = excluded.updated_at";
}
