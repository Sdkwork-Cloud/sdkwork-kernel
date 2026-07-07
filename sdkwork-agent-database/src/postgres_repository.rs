use sqlx::Row;

use crate::error::{DatabaseError, DatabaseResult};
use crate::pagination::{resolve_list_limit, resolve_list_offset};
use crate::postgres::PostgresDatabase;
use crate::traits::*;
use crate::types::*;

fn map_session_row(row: &sqlx::postgres::PgRow) -> DatabaseResult<SessionRow> {
    Ok(SessionRow {
        session_id: row.try_get("session_id").map_err(map_sqlx_error)?,
        agent_id: row.try_get("agent_id").map_err(map_sqlx_error)?,
        kind: row.try_get("kind").map_err(map_sqlx_error)?,
        source: row.try_get("source").map_err(map_sqlx_error)?,
        state: row.try_get("state").map_err(map_sqlx_error)?,
        title: row.try_get("title").ok(),
        model: row.try_get("model").ok(),
        cwd: row.try_get("cwd").ok(),
        provider_id: row.try_get("provider_id").ok(),
        bridge_id: row.try_get("bridge_id").ok(),
        token_usage_json: row.try_get("token_usage_json").ok(),
        message_count: row.try_get("message_count").map_err(map_sqlx_error)?,
        owner_tenant_id: row.try_get("owner_tenant_id").ok(),
        owner_user_ref: row.try_get("owner_user_ref").ok(),
        created_at: row.try_get("created_at").map_err(map_sqlx_error)?,
        updated_at: row.try_get("updated_at").ok(),
        metadata_json: row.try_get("metadata_json").ok(),
    })
}

fn map_message_row(row: &sqlx::postgres::PgRow) -> DatabaseResult<MessageRow> {
    Ok(MessageRow {
        message_id: row.try_get("message_id").map_err(map_sqlx_error)?,
        session_id: row.try_get("session_id").map_err(map_sqlx_error)?,
        role: row.try_get("role").map_err(map_sqlx_error)?,
        content: row.try_get("content").map_err(map_sqlx_error)?,
        created_at: row.try_get("created_at").map_err(map_sqlx_error)?,
        metadata_json: row.try_get("metadata_json").ok(),
    })
}

fn map_task_row(row: &sqlx::postgres::PgRow) -> DatabaseResult<TaskRow> {
    Ok(TaskRow {
        task_id: row.try_get("task_id").map_err(map_sqlx_error)?,
        session_id: row.try_get("session_id").map_err(map_sqlx_error)?,
        instruction: row.try_get("instruction").map_err(map_sqlx_error)?,
        state: row.try_get("state").map_err(map_sqlx_error)?,
        created_at: row.try_get("created_at").map_err(map_sqlx_error)?,
        updated_at: row.try_get("updated_at").ok(),
    })
}

fn map_event_row(row: &sqlx::postgres::PgRow) -> DatabaseResult<EventRow> {
    Ok(EventRow {
        event_id: row.try_get("event_id").map_err(map_sqlx_error)?,
        session_id: row.try_get("session_id").ok(),
        event_type: row.try_get("event_type").map_err(map_sqlx_error)?,
        severity: row.try_get("severity").map_err(map_sqlx_error)?,
        payload: row.try_get("payload").ok(),
        created_at: row.try_get("created_at").map_err(map_sqlx_error)?,
    })
}

fn map_sqlx_error(error: sqlx::Error) -> DatabaseError {
    DatabaseError::Query(error.to_string())
}

impl SessionRepository for PostgresDatabase {
    fn save_session(&self, session: &SessionRow) -> DatabaseResult<()> {
        let pool = self.pool.pool().clone();
        let session = session.clone();
        self.pool.run_db(async move {
            sqlx::query(
                "INSERT INTO sessions (
                    session_id, agent_id, kind, source, state, title, model, cwd,
                    provider_id, bridge_id, token_usage_json, message_count,
                    owner_tenant_id, owner_user_ref,
                    created_at, updated_at, metadata_json
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
                ON CONFLICT (session_id) DO UPDATE SET
                    agent_id = EXCLUDED.agent_id,
                    kind = EXCLUDED.kind,
                    source = EXCLUDED.source,
                    state = EXCLUDED.state,
                    title = EXCLUDED.title,
                    model = EXCLUDED.model,
                    cwd = EXCLUDED.cwd,
                    provider_id = EXCLUDED.provider_id,
                    bridge_id = EXCLUDED.bridge_id,
                    token_usage_json = EXCLUDED.token_usage_json,
                    message_count = EXCLUDED.message_count,
                    owner_tenant_id = EXCLUDED.owner_tenant_id,
                    owner_user_ref = EXCLUDED.owner_user_ref,
                    created_at = EXCLUDED.created_at,
                    updated_at = EXCLUDED.updated_at,
                    metadata_json = EXCLUDED.metadata_json",
            )
            .bind(&session.session_id)
            .bind(&session.agent_id)
            .bind(&session.kind)
            .bind(&session.source)
            .bind(&session.state)
            .bind(&session.title)
            .bind(&session.model)
            .bind(&session.cwd)
            .bind(&session.provider_id)
            .bind(&session.bridge_id)
            .bind(&session.token_usage_json)
            .bind(session.message_count)
            .bind(&session.owner_tenant_id)
            .bind(&session.owner_user_ref)
            .bind(&session.created_at)
            .bind(&session.updated_at)
            .bind(&session.metadata_json)
            .execute(&pool)
            .await?;
            Ok(())
        })
    }

    fn load_session(&self, session_id: &str) -> DatabaseResult<Option<SessionRow>> {
        let pool = self.pool.pool().clone();
        let session_id = session_id.to_owned();
        self.pool.run_db(async move {
            let row = sqlx::query(
                "SELECT session_id, agent_id, kind, source, state, title, model, cwd,
                        provider_id, bridge_id, token_usage_json, message_count,
                        owner_tenant_id, owner_user_ref,
                        created_at, updated_at, metadata_json
                 FROM sessions WHERE session_id = $1",
            )
            .bind(&session_id)
            .fetch_optional(&pool)
            .await?;
            row.map(|row| map_session_row(&row)).transpose()
        })
    }

    fn list_sessions(&self, query: &SessionQuery) -> DatabaseResult<Vec<SessionRow>> {
        let pool = self.pool.pool().clone();
        let query = query.clone();
        self.pool.run_db(async move {
            let mut builder = sqlx::QueryBuilder::new(
                "SELECT session_id, agent_id, kind, source, state, title, model, cwd,
                        provider_id, bridge_id, token_usage_json, message_count,
                        owner_tenant_id, owner_user_ref,
                        created_at, updated_at, metadata_json
                 FROM sessions WHERE 1 = 1",
            );
            if let Some(agent_id) = query.agent_id.as_deref() {
                builder.push(" AND agent_id = ");
                builder.push_bind(agent_id);
            }
            if let Some(state) = query.state.as_deref() {
                builder.push(" AND state = ");
                builder.push_bind(state);
            }
            if let Some(kind) = query.kind.as_deref() {
                builder.push(" AND kind = ");
                builder.push_bind(kind);
            }
            if let Some(provider_id) = query.provider_id.as_deref() {
                builder.push(" AND provider_id = ");
                builder.push_bind(provider_id);
            }
            if let Some(bridge_id) = query.bridge_id.as_deref() {
                builder.push(" AND bridge_id = ");
                builder.push_bind(bridge_id);
            }
            if let Some(owner_tenant_id) = query.owner_tenant_id.as_deref() {
                builder.push(" AND owner_tenant_id = ");
                builder.push_bind(owner_tenant_id);
            }
            if let Some(owner_user_ref) = query.owner_user_ref.as_deref() {
                builder.push(" AND owner_user_ref = ");
                builder.push_bind(owner_user_ref);
            }
            if let Some(after_session_id) = query
                .after_session_id
                .as_deref()
                .filter(|value| !value.is_empty())
            {
                builder.push(
                    " AND EXISTS (
                        SELECT 1 FROM sessions AS session_cursor
                        WHERE session_cursor.session_id = ",
                );
                builder.push_bind(after_session_id);
                builder.push(
                    " AND (
                        COALESCE(sessions.updated_at, sessions.created_at)
                          < COALESCE(session_cursor.updated_at, session_cursor.created_at)
                        OR (
                          COALESCE(sessions.updated_at, sessions.created_at)
                            = COALESCE(session_cursor.updated_at, session_cursor.created_at)
                          AND sessions.session_id < session_cursor.session_id
                        )
                      )
                    )",
                );
            }
            builder.push(" ORDER BY COALESCE(updated_at, created_at) DESC, session_id DESC");
            let limit = resolve_list_limit(query.limit);
            let offset = resolve_list_offset(query.offset);
            builder.push(" LIMIT ");
            builder.push_bind(limit);
            builder.push(" OFFSET ");
            builder.push_bind(offset);
            let rows = builder.build().fetch_all(&pool).await?;
            rows.iter().map(map_session_row).collect()
        })
    }

    fn update_session(&self, session: &SessionRow) -> DatabaseResult<()> {
        self.save_session(session)
    }

    fn delete_session(&self, session_id: &str) -> DatabaseResult<()> {
        let pool = self.pool.pool().clone();
        let session_id = session_id.to_owned();
        self.pool.run_db(async move {
            sqlx::query("DELETE FROM sessions WHERE session_id = $1")
                .bind(&session_id)
                .execute(&pool)
                .await?;
            Ok(())
        })
    }

    fn delete_session_cascade(&self, session_id: &str) -> DatabaseResult<()> {
        let pool = self.pool.pool().clone();
        let session_id = session_id.to_owned();
        self.pool.run_db(async move {
            let mut tx = pool.begin().await.map_err(map_sqlx_error)?;
            sqlx::query("DELETE FROM events WHERE session_id = $1")
                .bind(&session_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM messages WHERE session_id = $1")
                .bind(&session_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM tasks WHERE session_id = $1")
                .bind(&session_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM permissions WHERE session_id = $1")
                .bind(&session_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM sessions WHERE session_id = $1")
                .bind(&session_id)
                .execute(&mut *tx)
                .await?;
            tx.commit().await.map_err(map_sqlx_error)?;
            Ok(())
        })
    }

    fn increment_session_message_count(&self, session_id: &str) -> DatabaseResult<i64> {
        let pool = self.pool.pool().clone();
        let session_id = session_id.to_owned();
        let updated_at = chrono::Utc::now().to_rfc3339();
        self.pool.run_db(async move {
            let count = sqlx::query_scalar::<_, i64>(
                "UPDATE sessions SET message_count = message_count + 1, updated_at = $2 \
                 WHERE session_id = $1 RETURNING message_count",
            )
            .bind(&session_id)
            .bind(&updated_at)
            .fetch_optional(&pool)
            .await?;
            count.ok_or_else(|| DatabaseError::NotFound(format!("session not found: {session_id}")))
        })
    }
}

impl MessageRepository for PostgresDatabase {
    fn save_message(&self, message: &MessageRow) -> DatabaseResult<()> {
        let pool = self.pool.pool().clone();
        let message = message.clone();
        self.pool.run_db(async move {
            sqlx::query(
                "INSERT INTO messages (
                    message_id, session_id, role, content, created_at, metadata_json
                ) VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (message_id) DO NOTHING",
            )
            .bind(&message.message_id)
            .bind(&message.session_id)
            .bind(&message.role)
            .bind(&message.content)
            .bind(&message.created_at)
            .bind(&message.metadata_json)
            .execute(&pool)
            .await?;
            Ok(())
        })
    }

    fn load_messages(
        &self,
        session_id: &str,
        query: &MessageQuery,
    ) -> DatabaseResult<Vec<MessageRow>> {
        let pool = self.pool.pool().clone();
        let session_id = session_id.to_owned();
        let query = query.clone();
        self.pool.run_db(async move {
            let mut builder = sqlx::QueryBuilder::new(
                "SELECT message_id, session_id, role, content, created_at, metadata_json
                 FROM messages WHERE session_id = ",
            );
            builder.push_bind(&session_id);
            if let Some(after_message_id) = query
                .after_message_id
                .as_deref()
                .filter(|value| !value.is_empty())
            {
                builder.push(
                    " AND EXISTS (
                        SELECT 1 FROM messages AS message_cursor
                        WHERE message_cursor.message_id = ",
                );
                builder.push_bind(after_message_id);
                builder.push(
                    " AND message_cursor.session_id = messages.session_id
                      AND (
                        messages.created_at > message_cursor.created_at
                        OR (
                          messages.created_at = message_cursor.created_at
                          AND messages.message_id > message_cursor.message_id
                        )
                      )
                    )",
                );
            }
            builder.push(" ORDER BY created_at ASC");
            let limit = resolve_list_limit(query.limit);
            let offset = resolve_list_offset(query.offset);
            builder.push(" LIMIT ");
            builder.push_bind(limit);
            builder.push(" OFFSET ");
            builder.push_bind(offset);
            let rows = builder.build().fetch_all(&pool).await?;
            rows.iter().map(map_message_row).collect()
        })
    }

    fn message_count(&self, session_id: &str) -> DatabaseResult<i64> {
        let pool = self.pool.pool().clone();
        let session_id = session_id.to_owned();
        self.pool.run_db(async move {
            let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM messages WHERE session_id = $1")
                .bind(&session_id)
                .fetch_one(&pool)
                .await?;
            Ok(row.0)
        })
    }

    fn delete_messages(&self, session_id: &str) -> DatabaseResult<()> {
        let pool = self.pool.pool().clone();
        let session_id = session_id.to_owned();
        self.pool.run_db(async move {
            sqlx::query("DELETE FROM messages WHERE session_id = $1")
                .bind(&session_id)
                .execute(&pool)
                .await?;
            Ok(())
        })
    }
}

impl TaskRepository for PostgresDatabase {
    fn save_task(&self, task: &TaskRow) -> DatabaseResult<()> {
        let pool = self.pool.pool().clone();
        let task = task.clone();
        self.pool.run_db(async move {
            sqlx::query(
                "INSERT INTO tasks (
                    task_id, session_id, instruction, state, created_at, updated_at
                ) VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (task_id) DO UPDATE SET
                    session_id = EXCLUDED.session_id,
                    instruction = EXCLUDED.instruction,
                    state = EXCLUDED.state,
                    created_at = EXCLUDED.created_at,
                    updated_at = EXCLUDED.updated_at",
            )
            .bind(&task.task_id)
            .bind(&task.session_id)
            .bind(&task.instruction)
            .bind(&task.state)
            .bind(&task.created_at)
            .bind(&task.updated_at)
            .execute(&pool)
            .await?;
            Ok(())
        })
    }

    fn load_task(&self, task_id: &str) -> DatabaseResult<Option<TaskRow>> {
        let pool = self.pool.pool().clone();
        let task_id = task_id.to_owned();
        self.pool.run_db(async move {
            let row = sqlx::query(
                "SELECT task_id, session_id, instruction, state, created_at, updated_at
                 FROM tasks WHERE task_id = $1",
            )
            .bind(&task_id)
            .fetch_optional(&pool)
            .await?;
            row.map(|row| map_task_row(&row)).transpose()
        })
    }

    fn load_tasks(&self, session_id: &str, query: &TaskQuery) -> DatabaseResult<Vec<TaskRow>> {
        let pool = self.pool.pool().clone();
        let session_id = session_id.to_owned();
        let query = query.clone();
        self.pool.run_db(async move {
            let mut builder = sqlx::QueryBuilder::new(
                "SELECT task_id, session_id, instruction, state, created_at, updated_at
                 FROM tasks WHERE session_id = ",
            );
            builder.push_bind(&session_id);
            if let Some(after_task_id) = query
                .after_task_id
                .as_deref()
                .filter(|value| !value.is_empty())
            {
                builder.push(
                    " AND EXISTS (
                        SELECT 1 FROM tasks AS task_cursor
                        WHERE task_cursor.task_id = ",
                );
                builder.push_bind(after_task_id);
                builder.push(
                    " AND task_cursor.session_id = tasks.session_id
                      AND (
                        tasks.created_at > task_cursor.created_at
                        OR (
                          tasks.created_at = task_cursor.created_at
                          AND tasks.task_id > task_cursor.task_id
                        )
                      )
                    )",
                );
            }
            builder.push(" ORDER BY created_at ASC, task_id ASC");
            let limit = resolve_list_limit(query.limit);
            let offset = resolve_list_offset(query.offset);
            builder.push(" LIMIT ");
            builder.push_bind(limit);
            builder.push(" OFFSET ");
            builder.push_bind(offset);
            let rows = builder.build().fetch_all(&pool).await?;
            rows.iter().map(map_task_row).collect()
        })
    }

    fn update_task(&self, task: &TaskRow) -> DatabaseResult<()> {
        self.save_task(task)
    }

    fn delete_task(&self, task_id: &str) -> DatabaseResult<()> {
        let pool = self.pool.pool().clone();
        let task_id = task_id.to_owned();
        self.pool.run_db(async move {
            sqlx::query("DELETE FROM tasks WHERE task_id = $1")
                .bind(&task_id)
                .execute(&pool)
                .await?;
            Ok(())
        })
    }
}

impl EventRepository for PostgresDatabase {
    fn save_event(&self, event: &EventRow) -> DatabaseResult<()> {
        let pool = self.pool.pool().clone();
        let event = event.clone();
        self.pool.run_db(async move {
            sqlx::query(
                "INSERT INTO events (
                    event_id, session_id, event_type, severity, payload, created_at
                ) VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (event_id) DO UPDATE SET
                    session_id = EXCLUDED.session_id,
                    event_type = EXCLUDED.event_type,
                    severity = EXCLUDED.severity,
                    payload = EXCLUDED.payload,
                    created_at = EXCLUDED.created_at",
            )
            .bind(&event.event_id)
            .bind(&event.session_id)
            .bind(&event.event_type)
            .bind(&event.severity)
            .bind(&event.payload)
            .bind(&event.created_at)
            .execute(&pool)
            .await?;
            Ok(())
        })
    }

    fn load_events(&self, session_id: &str, query: &EventQuery) -> DatabaseResult<Vec<EventRow>> {
        let pool = self.pool.pool().clone();
        let session_id = session_id.to_owned();
        let query = query.clone();
        self.pool.run_db(async move {
            let mut builder = sqlx::QueryBuilder::new(
                "SELECT event_id, session_id, event_type, severity, payload, created_at
                 FROM events WHERE session_id = ",
            );
            builder.push_bind(&session_id);
            if let Some(event_type) = query.event_type.as_deref() {
                builder.push(" AND event_type = ");
                builder.push_bind(event_type);
            }
            if let Some(severity) = query.severity.as_deref() {
                builder.push(" AND severity = ");
                builder.push_bind(severity);
            }
            if let Some(after_event_id) = query
                .after_event_id
                .as_deref()
                .filter(|value| !value.is_empty())
            {
                builder.push(
                    " AND EXISTS (
                        SELECT 1 FROM events AS event_cursor
                        WHERE event_cursor.event_id = ",
                );
                builder.push_bind(after_event_id);
                builder.push(
                    " AND event_cursor.session_id = events.session_id
                      AND (
                        events.created_at > event_cursor.created_at
                        OR (
                          events.created_at = event_cursor.created_at
                          AND events.event_id > event_cursor.event_id
                        )
                      )
                    )",
                );
            }
            builder.push(" ORDER BY created_at ASC, event_id ASC");
            let limit = resolve_list_limit(query.limit);
            let offset = resolve_list_offset(query.offset);
            builder.push(" LIMIT ");
            builder.push_bind(limit);
            builder.push(" OFFSET ");
            builder.push_bind(offset);
            let rows = builder.build().fetch_all(&pool).await?;
            rows.iter().map(map_event_row).collect()
        })
    }

    fn list_recent_events(&self, query: &EventQuery) -> DatabaseResult<Vec<EventRow>> {
        let pool = self.pool.pool().clone();
        let query = query.clone();
        self.pool.run_db(async move {
            let mut builder = sqlx::QueryBuilder::new(
                "SELECT event_id, session_id, event_type, severity, payload, created_at
                 FROM events WHERE 1 = 1",
            );
            if let Some(event_type) = query.event_type.as_deref() {
                builder.push(" AND event_type = ");
                builder.push_bind(event_type);
            }
            if let Some(severity) = query.severity.as_deref() {
                builder.push(" AND severity = ");
                builder.push_bind(severity);
            }
            builder.push(" ORDER BY created_at DESC");
            let limit = resolve_list_limit(query.limit);
            let offset = resolve_list_offset(query.offset);
            builder.push(" LIMIT ");
            builder.push_bind(limit);
            builder.push(" OFFSET ");
            builder.push_bind(offset);
            let rows = builder.build().fetch_all(&pool).await?;
            rows.iter().map(map_event_row).collect()
        })
    }

    fn delete_events(&self, session_id: &str) -> DatabaseResult<()> {
        let pool = self.pool.pool().clone();
        let session_id = session_id.to_owned();
        self.pool.run_db(async move {
            sqlx::query("DELETE FROM events WHERE session_id = $1")
                .bind(&session_id)
                .execute(&pool)
                .await?;
            Ok(())
        })
    }
}

fn map_permission_row(row: &sqlx::postgres::PgRow) -> DatabaseResult<PermissionRow> {
    Ok(PermissionRow {
        permission_request_id: row
            .try_get("permission_request_id")
            .map_err(map_sqlx_error)?,
        session_id: row.try_get("session_id").ok(),
        category: row.try_get("category").map_err(map_sqlx_error)?,
        resource: row.try_get("resource").map_err(map_sqlx_error)?,
        side_effect_level: row.try_get("side_effect_level").map_err(map_sqlx_error)?,
        reason: row.try_get("reason").map_err(map_sqlx_error)?,
        status: row.try_get("status").map_err(map_sqlx_error)?,
        owner_tenant_id: row.try_get("owner_tenant_id").ok(),
        owner_user_ref: row.try_get("owner_user_ref").ok(),
        created_at: row.try_get("created_at").map_err(map_sqlx_error)?,
        updated_at: row.try_get("updated_at").ok(),
    })
}

impl PermissionRepository for PostgresDatabase {
    fn save_permission(&self, permission: &PermissionRow) -> DatabaseResult<()> {
        let pool = self.pool.pool().clone();
        let permission = permission.clone();
        self.pool.run_db(async move {
            sqlx::query(
                "INSERT INTO permissions (
                    permission_request_id, session_id, category, resource,
                    side_effect_level, reason, status, owner_tenant_id,
                    owner_user_ref, created_at, updated_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                ON CONFLICT (permission_request_id) DO UPDATE SET
                    session_id = EXCLUDED.session_id,
                    category = EXCLUDED.category,
                    resource = EXCLUDED.resource,
                    side_effect_level = EXCLUDED.side_effect_level,
                    reason = EXCLUDED.reason,
                    status = EXCLUDED.status,
                    owner_tenant_id = EXCLUDED.owner_tenant_id,
                    owner_user_ref = EXCLUDED.owner_user_ref,
                    created_at = EXCLUDED.created_at,
                    updated_at = EXCLUDED.updated_at",
            )
            .bind(&permission.permission_request_id)
            .bind(&permission.session_id)
            .bind(&permission.category)
            .bind(&permission.resource)
            .bind(&permission.side_effect_level)
            .bind(&permission.reason)
            .bind(&permission.status)
            .bind(&permission.owner_tenant_id)
            .bind(&permission.owner_user_ref)
            .bind(&permission.created_at)
            .bind(&permission.updated_at)
            .execute(&pool)
            .await?;
            Ok(())
        })
    }

    fn load_permission(
        &self,
        permission_request_id: &str,
    ) -> DatabaseResult<Option<PermissionRow>> {
        let pool = self.pool.pool().clone();
        let permission_request_id = permission_request_id.to_owned();
        self.pool.run_db(async move {
            let row = sqlx::query(
                "SELECT permission_request_id, session_id, category, resource,
                 side_effect_level, reason, status, owner_tenant_id,
                 owner_user_ref, created_at, updated_at
                 FROM permissions WHERE permission_request_id = $1",
            )
            .bind(&permission_request_id)
            .fetch_optional(&pool)
            .await?;
            row.map(|row| map_permission_row(&row)).transpose()
        })
    }

    fn list_permissions(&self, query: &PermissionQuery) -> DatabaseResult<Vec<PermissionRow>> {
        let pool = self.pool.pool().clone();
        let query = query.clone();
        self.pool.run_db(async move {
            let mut builder = sqlx::QueryBuilder::new(
                "SELECT permission_request_id, session_id, category, resource,
                 side_effect_level, reason, status, owner_tenant_id,
                 owner_user_ref, created_at, updated_at
                 FROM permissions WHERE 1 = 1",
            );
            if let Some(ref status) = query.status {
                builder.push(" AND status = ");
                builder.push_bind(status);
            }
            builder.push(" ORDER BY created_at DESC");
            let limit = resolve_list_limit(query.limit);
            let offset = resolve_list_offset(query.offset);
            builder.push(" LIMIT ");
            builder.push_bind(limit);
            builder.push(" OFFSET ");
            builder.push_bind(offset);
            let rows = builder.build().fetch_all(&pool).await?;
            rows.iter().map(map_permission_row).collect()
        })
    }

    fn update_permission_status(
        &self,
        permission_request_id: &str,
        status: &str,
    ) -> DatabaseResult<()> {
        let pool = self.pool.pool().clone();
        let permission_request_id = permission_request_id.to_owned();
        let status = status.to_owned();
        self.pool.run_db(async move {
            let now = chrono::Utc::now().to_rfc3339();
            sqlx::query(
                "UPDATE permissions SET status = $1, updated_at = $2 WHERE permission_request_id = $3",
            )
            .bind(&status)
            .bind(&now)
            .bind(&permission_request_id)
            .execute(&pool)
            .await?;
            Ok(())
        })
    }
}

impl RuntimeSessionWrites for PostgresDatabase {
    fn append_message_with_event(
        &self,
        message: &MessageRow,
        event: &EventRow,
    ) -> DatabaseResult<i64> {
        let pool = self.pool.pool().clone();
        let message = message.clone();
        let event = event.clone();
        self.pool.run_db(async move {
            let mut tx = pool.begin().await.map_err(map_sqlx_error)?;
            let insert_result = sqlx::query(
                "INSERT INTO messages (
                    message_id, session_id, role, content, created_at, metadata_json
                ) VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (message_id) DO NOTHING",
            )
            .bind(&message.message_id)
            .bind(&message.session_id)
            .bind(&message.role)
            .bind(&message.content)
            .bind(&message.created_at)
            .bind(&message.metadata_json)
            .execute(&mut *tx)
            .await?;
            let count = if insert_result.rows_affected() > 0 {
                let updated_at = chrono::Utc::now().to_rfc3339();
                let count = sqlx::query_scalar::<_, i64>(
                    "UPDATE sessions SET message_count = message_count + 1, updated_at = $2 \
                     WHERE session_id = $1 RETURNING message_count",
                )
                .bind(&message.session_id)
                .bind(&updated_at)
                .fetch_optional(&mut *tx)
                .await?;
                count.ok_or_else(|| {
                    DatabaseError::NotFound(format!("session not found: {}", message.session_id))
                })?
            } else {
                let existing_session_id = sqlx::query_scalar::<_, String>(
                    "SELECT session_id FROM messages WHERE message_id = $1",
                )
                .bind(&message.message_id)
                .fetch_one(&mut *tx)
                .await?;
                if existing_session_id != message.session_id {
                    return Err(DatabaseError::ConstraintViolation(format!(
                        "message {} already belongs to session {}",
                        message.message_id, existing_session_id
                    )));
                }
                let count = sqlx::query_scalar::<_, i64>(
                    "SELECT message_count FROM sessions WHERE session_id = $1",
                )
                .bind(&message.session_id)
                .fetch_optional(&mut *tx)
                .await?;
                count.ok_or_else(|| {
                    DatabaseError::NotFound(format!("session not found: {}", message.session_id))
                })?
            };
            sqlx::query(
                "INSERT INTO events (
                    event_id, session_id, event_type, severity, payload, created_at
                ) VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (event_id) DO UPDATE SET
                    session_id = EXCLUDED.session_id,
                    event_type = EXCLUDED.event_type,
                    severity = EXCLUDED.severity,
                    payload = EXCLUDED.payload,
                    created_at = EXCLUDED.created_at",
            )
            .bind(&event.event_id)
            .bind(&event.session_id)
            .bind(&event.event_type)
            .bind(&event.severity)
            .bind(&event.payload)
            .bind(&event.created_at)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            Ok(count)
        })
    }

    fn delete_messages_and_reset_count(
        &self,
        session_id: &str,
        updated_at: &str,
    ) -> DatabaseResult<()> {
        let pool = self.pool.pool().clone();
        let session_id = session_id.to_owned();
        let updated_at = updated_at.to_owned();
        self.pool.run_db(async move {
            let mut tx = pool.begin().await.map_err(map_sqlx_error)?;
            sqlx::query("DELETE FROM messages WHERE session_id = $1")
                .bind(&session_id)
                .execute(&mut *tx)
                .await?;
            let result = sqlx::query(
                "UPDATE sessions SET message_count = 0, updated_at = $2 WHERE session_id = $1",
            )
            .bind(&session_id)
            .bind(&updated_at)
            .execute(&mut *tx)
            .await?;
            if result.rows_affected() == 0 {
                return Err(DatabaseError::NotFound(format!(
                    "session not found: {session_id}"
                )));
            }
            tx.commit().await?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_session(id: &str) -> SessionRow {
        SessionRow {
            session_id: id.to_string(),
            agent_id: "agent.1".to_string(),
            kind: "main".to_string(),
            source: "api".to_string(),
            state: "active".to_string(),
            title: Some("Test".to_string()),
            model: Some("gpt-4".to_string()),
            cwd: None,
            provider_id: Some("codex".to_string()),
            bridge_id: Some("bridge.codex".to_string()),
            token_usage_json: None,
            message_count: 0,
            owner_tenant_id: None,
            owner_user_ref: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: Some("2026-01-02T00:00:00Z".to_string()),
            metadata_json: None,
        }
    }

    fn open_postgres_or_skip() -> Option<PostgresDatabase> {
        let uri = std::env::var("SDKWORK_AGENT_RUNTIME_POSTGRES_URI")
            .or_else(|_| std::env::var("SDKWORK_AGENT_BUSINESS_POSTGRES_URI"))
            .ok()?;
        let trimmed = uri.trim();
        if trimmed.is_empty() {
            return None;
        }
        PostgresDatabase::connect_migrated(trimmed).ok()
    }

    #[test]
    fn postgres_session_repository_roundtrip_when_uri_configured() {
        let Some(db) = open_postgres_or_skip() else {
            return;
        };
        db.save_session(&sample_session("session.pg.1"))
            .expect("saved");
        let loaded = db
            .load_session("session.pg.1")
            .expect("loaded")
            .expect("found");
        assert_eq!(loaded.agent_id, "agent.1");
        assert_eq!(loaded.provider_id.as_deref(), Some("codex"));
        let _ = db.delete_session("session.pg.1");
    }
}
