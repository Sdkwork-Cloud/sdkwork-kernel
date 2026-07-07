use crate::error::{DatabaseError, DatabaseResult};
use crate::pagination::{resolve_list_limit, resolve_list_offset};
use crate::sqlite::SqliteDatabase;
use crate::traits::*;
use crate::types::*;
use crate::PermissionRow;
use rusqlite::{params, OptionalExtension, Row};

impl SqliteDatabase {
    /// Open a SQLite database file and run schema migrations.
    pub fn open_migrated(path: &str) -> DatabaseResult<Self> {
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|error| {
                    DatabaseError::Connection(format!(
                        "failed to create database directory: {error}"
                    ))
                })?;
            }
        }
        let db = Self::new(path)?;
        db.migrate()?;
        Ok(db)
    }

    /// Open an in-memory SQLite database and run schema migrations.
    pub fn memory_migrated() -> DatabaseResult<Self> {
        let db = Self::memory()?;
        db.migrate()?;
        Ok(db)
    }

    /// Run schema migrations on this database.
    pub fn migrate(&self) -> DatabaseResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        crate::schema_migrations::apply_sqlite_connection(&conn)
    }
}

fn map_session_row(row: &Row<'_>) -> rusqlite::Result<SessionRow> {
    Ok(SessionRow {
        session_id: row.get("session_id")?,
        agent_id: row.get("agent_id")?,
        kind: row.get("kind")?,
        source: row.get("source")?,
        state: row.get("state")?,
        title: row.get("title")?,
        model: row.get("model")?,
        cwd: row.get("cwd")?,
        provider_id: row.get("provider_id")?,
        bridge_id: row.get("bridge_id")?,
        token_usage_json: row.get("token_usage_json")?,
        message_count: row.get("message_count")?,
        owner_tenant_id: row.get("owner_tenant_id").ok(),
        owner_user_ref: row.get("owner_user_ref").ok(),
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        metadata_json: row.get("metadata_json")?,
    })
}

fn map_message_row(row: &Row<'_>) -> rusqlite::Result<MessageRow> {
    Ok(MessageRow {
        message_id: row.get("message_id")?,
        session_id: row.get("session_id")?,
        role: row.get("role")?,
        content: row.get("content")?,
        created_at: row.get("created_at")?,
        metadata_json: row.get("metadata_json")?,
    })
}

fn map_task_row(row: &Row<'_>) -> rusqlite::Result<TaskRow> {
    Ok(TaskRow {
        task_id: row.get("task_id")?,
        session_id: row.get("session_id")?,
        instruction: row.get("instruction")?,
        state: row.get("state")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn map_event_row(row: &Row<'_>) -> rusqlite::Result<EventRow> {
    Ok(EventRow {
        event_id: row.get("event_id")?,
        session_id: row.get("session_id")?,
        event_type: row.get("event_type")?,
        severity: row.get("severity")?,
        payload: row.get("payload")?,
        created_at: row.get("created_at")?,
    })
}

impl SessionRepository for SqliteDatabase {
    fn save_session(&self, session: &SessionRow) -> DatabaseResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        conn.execute(
            crate::upsert_sql::sqlite::SAVE_SESSION,
            params![
                session.session_id,
                session.agent_id,
                session.kind,
                session.source,
                session.state,
                session.title,
                session.model,
                session.cwd,
                session.provider_id,
                session.bridge_id,
                session.token_usage_json,
                session.message_count,
                session.owner_tenant_id,
                session.owner_user_ref,
                session.created_at,
                session.updated_at,
                session.metadata_json,
            ],
        )
        .map_err(|error| DatabaseError::Query(format!("failed to save session: {error}")))?;
        Ok(())
    }

    fn load_session(&self, session_id: &str) -> DatabaseResult<Option<SessionRow>> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        conn.query_row(
            "SELECT session_id, agent_id, kind, source, state, title, model, cwd,
                    provider_id, bridge_id, token_usage_json, message_count,
                    owner_tenant_id, owner_user_ref,
                    created_at, updated_at, metadata_json
             FROM sessions WHERE session_id = ?1",
            params![session_id],
            map_session_row,
        )
        .optional()
        .map_err(|error| DatabaseError::Query(format!("failed to load session: {error}")))
    }

    fn list_sessions(&self, query: &SessionQuery) -> DatabaseResult<Vec<SessionRow>> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        let mut sql = String::from(
            "SELECT session_id, agent_id, kind, source, state, title, model, cwd,
                    provider_id, bridge_id, token_usage_json, message_count,
                    owner_tenant_id, owner_user_ref,
                    created_at, updated_at, metadata_json
             FROM sessions WHERE 1 = 1",
        );
        let mut values: Vec<String> = Vec::new();
        if let Some(agent_id) = query.agent_id.as_deref() {
            sql.push_str(" AND agent_id = ?");
            values.push(agent_id.to_string());
        }
        if let Some(state) = query.state.as_deref() {
            sql.push_str(" AND state = ?");
            values.push(state.to_string());
        }
        if let Some(kind) = query.kind.as_deref() {
            sql.push_str(" AND kind = ?");
            values.push(kind.to_string());
        }
        if let Some(provider_id) = query.provider_id.as_deref() {
            sql.push_str(" AND provider_id = ?");
            values.push(provider_id.to_string());
        }
        if let Some(bridge_id) = query.bridge_id.as_deref() {
            sql.push_str(" AND bridge_id = ?");
            values.push(bridge_id.to_string());
        }
        if let Some(owner_tenant_id) = query.owner_tenant_id.as_deref() {
            sql.push_str(" AND owner_tenant_id = ?");
            values.push(owner_tenant_id.to_string());
        }
        if let Some(owner_user_ref) = query.owner_user_ref.as_deref() {
            sql.push_str(" AND owner_user_ref = ?");
            values.push(owner_user_ref.to_string());
        }
        if let Some(after_session_id) = query
            .after_session_id
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            sql.push_str(
                " AND EXISTS (
                    SELECT 1 FROM sessions AS session_cursor
                    WHERE session_cursor.session_id = ?
                      AND (
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
            values.push(after_session_id.to_string());
        }
        sql.push_str(" ORDER BY COALESCE(updated_at, created_at) DESC, session_id DESC");
        let limit = resolve_list_limit(query.limit);
        let offset = resolve_list_offset(query.offset);
        sql.push_str(" LIMIT ? OFFSET ?");
        values.push(limit.to_string());
        values.push(offset.to_string());

        let mut stmt = conn.prepare(&sql).map_err(|error| {
            DatabaseError::Query(format!("failed to prepare session list: {error}"))
        })?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(values.iter()), map_session_row)
            .map_err(|error| DatabaseError::Query(format!("failed to list sessions: {error}")))?;
        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row.map_err(|error| {
                DatabaseError::Query(format!("failed to read session row: {error}"))
            })?);
        }
        Ok(sessions)
    }

    fn update_session(&self, session: &SessionRow) -> DatabaseResult<()> {
        self.save_session(session)
    }

    fn delete_session(&self, session_id: &str) -> DatabaseResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        conn.execute(
            "DELETE FROM sessions WHERE session_id = ?1",
            params![session_id],
        )
        .map_err(|error| DatabaseError::Query(format!("failed to delete session: {error}")))?;
        Ok(())
    }

    fn delete_session_cascade(&self, session_id: &str) -> DatabaseResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        let tx = conn.unchecked_transaction().map_err(|error| {
            DatabaseError::Transaction(format!("failed to begin transaction: {error}"))
        })?;
        tx.execute(
            "DELETE FROM events WHERE session_id = ?1",
            params![session_id],
        )
        .map_err(|error| DatabaseError::Query(format!("failed to delete events: {error}")))?;
        tx.execute(
            "DELETE FROM messages WHERE session_id = ?1",
            params![session_id],
        )
        .map_err(|error| DatabaseError::Query(format!("failed to delete messages: {error}")))?;
        tx.execute(
            "DELETE FROM tasks WHERE session_id = ?1",
            params![session_id],
        )
        .map_err(|error| DatabaseError::Query(format!("failed to delete tasks: {error}")))?;
        tx.execute(
            "DELETE FROM permissions WHERE session_id = ?1",
            params![session_id],
        )
        .map_err(|error| DatabaseError::Query(format!("failed to delete permissions: {error}")))?;
        tx.execute(
            "DELETE FROM sessions WHERE session_id = ?1",
            params![session_id],
        )
        .map_err(|error| DatabaseError::Query(format!("failed to delete session: {error}")))?;
        tx.commit().map_err(|error| {
            DatabaseError::Transaction(format!("failed to commit cascade delete: {error}"))
        })?;
        Ok(())
    }

    fn increment_session_message_count(&self, session_id: &str) -> DatabaseResult<i64> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        let updated_at = chrono::Utc::now().to_rfc3339();
        let count: i64 = conn
            .query_row(
                "UPDATE sessions SET message_count = message_count + 1, updated_at = ?2 \
                 WHERE session_id = ?1 RETURNING message_count",
                params![session_id, updated_at],
                |row| row.get(0),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    DatabaseError::NotFound(format!("session not found: {session_id}"))
                }
                other => {
                    DatabaseError::Query(format!("failed to increment message count: {other}"))
                }
            })?;
        Ok(count)
    }
}

impl MessageRepository for SqliteDatabase {
    fn save_message(&self, message: &MessageRow) -> DatabaseResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        let inserted_rows = conn
            .execute(
                crate::upsert_sql::sqlite::SAVE_MESSAGE,
                params![
                    message.message_id,
                    message.session_id,
                    message.role,
                    message.content,
                    message.created_at,
                    message.metadata_json,
                ],
            )
            .map_err(|error| DatabaseError::Query(format!("failed to save message: {error}")))?;
        if inserted_rows == 0 {
            let existing = conn
                .query_row(
                    "SELECT message_id, session_id, role, content, created_at, metadata_json
                     FROM messages WHERE message_id = ?1",
                    params![message.message_id],
                    map_message_row,
                )
                .map_err(|error| {
                    DatabaseError::Query(format!("failed to load existing message: {error}"))
                })?;
            crate::message_identity::ensure_message_retry_matches(&existing, message)?;
        }
        Ok(())
    }

    fn load_messages(
        &self,
        session_id: &str,
        query: &MessageQuery,
    ) -> DatabaseResult<Vec<MessageRow>> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        let mut sql = String::from(
            "SELECT message_id, session_id, role, content, created_at, metadata_json
             FROM messages WHERE session_id = ?1",
        );
        let mut values = vec![session_id.to_string()];
        if let Some(after_message_id) = query
            .after_message_id
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            sql.push_str(
                " AND EXISTS (
                    SELECT 1 FROM messages AS message_cursor
                    WHERE message_cursor.message_id = ?
                      AND message_cursor.session_id = messages.session_id
                      AND (
                        messages.created_at > message_cursor.created_at
                        OR (
                          messages.created_at = message_cursor.created_at
                          AND messages.message_id > message_cursor.message_id
                        )
                      )
                  )",
            );
            values.push(after_message_id.to_string());
        }
        sql.push_str(" ORDER BY created_at ASC");
        let limit = resolve_list_limit(query.limit);
        let offset = resolve_list_offset(query.offset);
        sql.push_str(" LIMIT ? OFFSET ?");
        values.push(limit.to_string());
        values.push(offset.to_string());
        let mut stmt = conn.prepare(&sql).map_err(|error| {
            DatabaseError::Query(format!("failed to prepare messages: {error}"))
        })?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(values.iter()), map_message_row)
            .map_err(|error| DatabaseError::Query(format!("failed to load messages: {error}")))?;
        let mut messages = Vec::new();
        for row in rows {
            messages.push(row.map_err(|error| {
                DatabaseError::Query(format!("failed to read message row: {error}"))
            })?);
        }
        Ok(messages)
    }

    fn message_count(&self, session_id: &str) -> DatabaseResult<i64> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        conn.query_row(
            "SELECT COUNT(*) FROM messages WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        )
        .map_err(|error| DatabaseError::Query(format!("failed to count messages: {error}")))
    }

    fn delete_messages(&self, session_id: &str) -> DatabaseResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        conn.execute(
            "DELETE FROM messages WHERE session_id = ?1",
            params![session_id],
        )
        .map_err(|error| DatabaseError::Query(format!("failed to delete messages: {error}")))?;
        Ok(())
    }
}

impl TaskRepository for SqliteDatabase {
    fn save_task(&self, task: &TaskRow) -> DatabaseResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        conn.execute(
            crate::upsert_sql::sqlite::SAVE_TASK,
            params![
                task.task_id,
                task.session_id,
                task.instruction,
                task.state,
                task.created_at,
                task.updated_at,
            ],
        )
        .map_err(|error| DatabaseError::Query(format!("failed to save task: {error}")))?;
        Ok(())
    }

    fn load_task(&self, task_id: &str) -> DatabaseResult<Option<TaskRow>> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        conn.query_row(
            "SELECT task_id, session_id, instruction, state, created_at, updated_at
             FROM tasks WHERE task_id = ?1",
            params![task_id],
            map_task_row,
        )
        .optional()
        .map_err(|error| DatabaseError::Query(format!("failed to load task: {error}")))
    }

    fn load_tasks(&self, session_id: &str, query: &TaskQuery) -> DatabaseResult<Vec<TaskRow>> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        let mut sql = String::from(
            "SELECT task_id, session_id, instruction, state, created_at, updated_at
             FROM tasks WHERE session_id = ?1",
        );
        let mut values = vec![session_id.to_string()];
        if let Some(after_task_id) = query
            .after_task_id
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            sql.push_str(
                " AND EXISTS (
                    SELECT 1 FROM tasks AS task_cursor
                    WHERE task_cursor.task_id = ?
                      AND task_cursor.session_id = tasks.session_id
                      AND (
                        tasks.created_at > task_cursor.created_at
                        OR (
                          tasks.created_at = task_cursor.created_at
                          AND tasks.task_id > task_cursor.task_id
                        )
                      )
                  )",
            );
            values.push(after_task_id.to_string());
        }
        sql.push_str(" ORDER BY created_at ASC, task_id ASC");
        let limit = resolve_list_limit(query.limit);
        let offset = resolve_list_offset(query.offset);
        sql.push_str(" LIMIT ? OFFSET ?");
        values.push(limit.to_string());
        values.push(offset.to_string());
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|error| DatabaseError::Query(format!("failed to prepare tasks: {error}")))?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(values.iter()), map_task_row)
            .map_err(|error| DatabaseError::Query(format!("failed to load tasks: {error}")))?;
        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(row.map_err(|error| {
                DatabaseError::Query(format!("failed to read task row: {error}"))
            })?);
        }
        Ok(tasks)
    }

    fn update_task(&self, task: &TaskRow) -> DatabaseResult<()> {
        self.save_task(task)
    }

    fn delete_task(&self, task_id: &str) -> DatabaseResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        conn.execute("DELETE FROM tasks WHERE task_id = ?1", params![task_id])
            .map_err(|error| DatabaseError::Query(format!("failed to delete task: {error}")))?;
        Ok(())
    }
}

impl EventRepository for SqliteDatabase {
    fn save_event(&self, event: &EventRow) -> DatabaseResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        conn.execute(
            crate::upsert_sql::sqlite::SAVE_EVENT,
            params![
                event.event_id,
                event.session_id,
                event.event_type,
                event.severity,
                event.payload,
                event.created_at,
            ],
        )
        .map_err(|error| DatabaseError::Query(format!("failed to save event: {error}")))?;
        Ok(())
    }

    fn load_events(&self, session_id: &str, query: &EventQuery) -> DatabaseResult<Vec<EventRow>> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        let mut sql = String::from(
            "SELECT event_id, session_id, event_type, severity, payload, created_at
             FROM events WHERE session_id = ?1",
        );
        let mut values = vec![session_id.to_string()];
        if let Some(event_type) = query.event_type.as_deref() {
            sql.push_str(" AND event_type = ?");
            values.push(event_type.to_string());
        }
        if let Some(severity) = query.severity.as_deref() {
            sql.push_str(" AND severity = ?");
            values.push(severity.to_string());
        }
        if let Some(after_event_id) = query
            .after_event_id
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            sql.push_str(
                " AND EXISTS (
                    SELECT 1 FROM events AS event_cursor
                    WHERE event_cursor.event_id = ?
                      AND event_cursor.session_id = events.session_id
                      AND (
                        events.created_at > event_cursor.created_at
                        OR (
                          events.created_at = event_cursor.created_at
                          AND events.event_id > event_cursor.event_id
                        )
                      )
                  )",
            );
            values.push(after_event_id.to_string());
        }
        sql.push_str(" ORDER BY created_at ASC, event_id ASC");
        let limit = resolve_list_limit(query.limit);
        let offset = resolve_list_offset(query.offset);
        sql.push_str(" LIMIT ? OFFSET ?");
        values.push(limit.to_string());
        values.push(offset.to_string());
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|error| DatabaseError::Query(format!("failed to prepare events: {error}")))?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(values.iter()), map_event_row)
            .map_err(|error| DatabaseError::Query(format!("failed to load events: {error}")))?;
        let mut events = Vec::new();
        for row in rows {
            events.push(row.map_err(|error| {
                DatabaseError::Query(format!("failed to read event row: {error}"))
            })?);
        }
        Ok(events)
    }

    fn list_recent_events(&self, query: &EventQuery) -> DatabaseResult<Vec<EventRow>> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        let mut sql = String::from(
            "SELECT event_id, session_id, event_type, severity, payload, created_at
             FROM events WHERE 1 = 1",
        );
        let mut values: Vec<String> = Vec::new();
        if let Some(event_type) = query.event_type.as_deref() {
            sql.push_str(" AND event_type = ?");
            values.push(event_type.to_string());
        }
        if let Some(severity) = query.severity.as_deref() {
            sql.push_str(" AND severity = ?");
            values.push(severity.to_string());
        }
        sql.push_str(" ORDER BY created_at DESC");
        let limit = resolve_list_limit(query.limit);
        let offset = resolve_list_offset(query.offset);
        sql.push_str(" LIMIT ? OFFSET ?");
        values.push(limit.to_string());
        values.push(offset.to_string());
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|error| DatabaseError::Query(format!("failed to prepare events: {error}")))?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(values.iter()), map_event_row)
            .map_err(|error| {
                DatabaseError::Query(format!("failed to list recent events: {error}"))
            })?;
        let mut events = Vec::new();
        for row in rows {
            events.push(row.map_err(|error| {
                DatabaseError::Query(format!("failed to read event row: {error}"))
            })?);
        }
        Ok(events)
    }

    fn delete_events(&self, session_id: &str) -> DatabaseResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        conn.execute(
            "DELETE FROM events WHERE session_id = ?1",
            params![session_id],
        )
        .map_err(|error| DatabaseError::Query(format!("failed to delete events: {error}")))?;
        Ok(())
    }
}

fn map_permission_row(row: &Row<'_>) -> rusqlite::Result<PermissionRow> {
    Ok(PermissionRow {
        permission_request_id: row.get("permission_request_id")?,
        session_id: row.get("session_id")?,
        category: row.get("category")?,
        resource: row.get("resource")?,
        side_effect_level: row.get("side_effect_level")?,
        reason: row.get("reason")?,
        status: row.get("status")?,
        owner_tenant_id: row.get("owner_tenant_id")?,
        owner_user_ref: row.get("owner_user_ref")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

impl PermissionRepository for SqliteDatabase {
    fn save_permission(&self, permission: &PermissionRow) -> DatabaseResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        conn.execute(
            crate::upsert_sql::sqlite::SAVE_PERMISSION,
            params![
                permission.permission_request_id,
                permission.session_id,
                permission.category,
                permission.resource,
                permission.side_effect_level,
                permission.reason,
                permission.status,
                permission.owner_tenant_id,
                permission.owner_user_ref,
                permission.created_at,
                permission.updated_at,
            ],
        )
        .map_err(|error| DatabaseError::Query(format!("failed to save permission: {error}")))?;
        Ok(())
    }

    fn load_permission(
        &self,
        permission_request_id: &str,
    ) -> DatabaseResult<Option<PermissionRow>> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        conn.query_row(
            "SELECT permission_request_id, session_id, category, resource,
             side_effect_level, reason, status, owner_tenant_id,
             owner_user_ref, created_at, updated_at
             FROM permissions WHERE permission_request_id = ?1",
            params![permission_request_id],
            map_permission_row,
        )
        .optional()
        .map_err(|error| DatabaseError::Query(format!("failed to load permission: {error}")))
    }

    fn list_permissions(&self, query: &PermissionQuery) -> DatabaseResult<Vec<PermissionRow>> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        let mut sql = String::from(
            "SELECT permission_request_id, session_id, category, resource,
             side_effect_level, reason, status, owner_tenant_id,
             owner_user_ref, created_at, updated_at
             FROM permissions WHERE 1 = 1",
        );
        let mut values: Vec<String> = Vec::new();
        if let Some(status) = query.status.as_deref() {
            sql.push_str(" AND status = ?");
            values.push(status.to_string());
        }
        sql.push_str(" ORDER BY created_at DESC");
        let limit = resolve_list_limit(query.limit);
        let offset = resolve_list_offset(query.offset);
        sql.push_str(" LIMIT ? OFFSET ?");
        values.push(limit.to_string());
        values.push(offset.to_string());
        let mut stmt = conn.prepare(&sql).map_err(|error| {
            DatabaseError::Query(format!("failed to prepare permissions: {error}"))
        })?;
        let rows = stmt
            .query_map(
                rusqlite::params_from_iter(values.iter()),
                map_permission_row,
            )
            .map_err(|error| {
                DatabaseError::Query(format!("failed to load permissions: {error}"))
            })?;
        let mut permissions = Vec::new();
        for row in rows {
            permissions.push(row.map_err(|error| {
                DatabaseError::Query(format!("failed to read permission row: {error}"))
            })?);
        }
        Ok(permissions)
    }

    fn update_permission_status(
        &self,
        permission_request_id: &str,
        status: &str,
    ) -> DatabaseResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE permissions SET status = ?1, updated_at = ?2 WHERE permission_request_id = ?3",
            params![status, now, permission_request_id],
        )
        .map_err(|error| DatabaseError::Query(format!("failed to update permission: {error}")))?;
        Ok(())
    }
}

impl RuntimeSessionWrites for SqliteDatabase {
    fn append_message_with_event(
        &self,
        message: &MessageRow,
        event: &EventRow,
    ) -> DatabaseResult<i64> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        let tx = conn.unchecked_transaction().map_err(|error| {
            DatabaseError::Transaction(format!("failed to begin transaction: {error}"))
        })?;
        let inserted_rows = tx
            .execute(
                "INSERT INTO messages (
                    message_id, session_id, role, content, created_at, metadata_json
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ON CONFLICT(message_id) DO NOTHING",
                params![
                    message.message_id,
                    message.session_id,
                    message.role,
                    message.content,
                    message.created_at,
                    message.metadata_json,
                ],
            )
            .map_err(|error| DatabaseError::Query(format!("failed to save message: {error}")))?;
        let count: i64 = if inserted_rows > 0 {
            let updated_at = chrono::Utc::now().to_rfc3339();
            tx.query_row(
                "UPDATE sessions SET message_count = message_count + 1, updated_at = ?2 \
                     WHERE session_id = ?1 RETURNING message_count",
                params![message.session_id, updated_at],
                |row| row.get(0),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    DatabaseError::NotFound(format!("session not found: {}", message.session_id))
                }
                other => {
                    DatabaseError::Query(format!("failed to increment message count: {other}"))
                }
            })?
        } else {
            let existing = tx
                .query_row(
                    "SELECT message_id, session_id, role, content, created_at, metadata_json
                     FROM messages WHERE message_id = ?1",
                    params![message.message_id],
                    map_message_row,
                )
                .map_err(|error| {
                    DatabaseError::Query(format!("failed to load existing message: {error}"))
                })?;
            crate::message_identity::ensure_message_retry_matches(&existing, message)?;
            tx.query_row(
                "SELECT message_count FROM sessions WHERE session_id = ?1",
                params![message.session_id],
                |row| row.get(0),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    DatabaseError::NotFound(format!("session not found: {}", message.session_id))
                }
                other => DatabaseError::Query(format!("failed to load message count: {other}")),
            })?
        };
        if inserted_rows > 0 {
            tx.execute(
                crate::upsert_sql::sqlite::SAVE_EVENT,
                params![
                    event.event_id,
                    event.session_id,
                    event.event_type,
                    event.severity,
                    event.payload,
                    event.created_at,
                ],
            )
            .map_err(|error| DatabaseError::Query(format!("failed to save event: {error}")))?;
        }
        tx.commit().map_err(|error| {
            DatabaseError::Transaction(format!("failed to commit transaction: {error}"))
        })?;
        Ok(count)
    }

    fn delete_messages_and_reset_count(
        &self,
        session_id: &str,
        updated_at: &str,
    ) -> DatabaseResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        let tx = conn.unchecked_transaction().map_err(|error| {
            DatabaseError::Transaction(format!("failed to begin transaction: {error}"))
        })?;
        tx.execute(
            "DELETE FROM messages WHERE session_id = ?1",
            params![session_id],
        )
        .map_err(|error| DatabaseError::Query(format!("failed to delete messages: {error}")))?;
        let rows = tx
            .execute(
                "UPDATE sessions SET message_count = 0, updated_at = ?2 WHERE session_id = ?1",
                params![session_id, updated_at],
            )
            .map_err(|error| {
                DatabaseError::Query(format!("failed to reset session message count: {error}"))
            })?;
        if rows == 0 {
            return Err(DatabaseError::NotFound(format!(
                "session not found: {session_id}"
            )));
        }
        tx.commit().map_err(|error| {
            DatabaseError::Transaction(format!("failed to commit transaction: {error}"))
        })?;
        Ok(())
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

    #[test]
    fn sqlite_session_repository_roundtrip() {
        let db = SqliteDatabase::memory_migrated().expect("db");
        db.save_session(&sample_session("session.1"))
            .expect("saved");
        let loaded = db
            .load_session("session.1")
            .expect("loaded")
            .expect("found");
        assert_eq!(loaded.agent_id, "agent.1");
        assert_eq!(loaded.provider_id.as_deref(), Some("codex"));
    }

    #[test]
    fn sqlite_message_repository_roundtrip() {
        let db = SqliteDatabase::memory_migrated().expect("db");
        db.save_session(&sample_session("session.1"))
            .expect("saved");
        db.save_message(&MessageRow {
            message_id: "msg.1".to_string(),
            session_id: "session.1".to_string(),
            role: "user".to_string(),
            content: "hello".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            metadata_json: None,
        })
        .expect("saved");
        let messages = db
            .load_messages("session.1", &MessageQuery::default())
            .expect("loaded");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "hello");
    }
}
