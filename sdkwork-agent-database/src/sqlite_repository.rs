use crate::error::{DatabaseError, DatabaseResult};
use crate::pagination::{resolve_history_limit, resolve_list_limit, resolve_list_offset};
use crate::sqlite::SqliteDatabase;
use crate::traits::*;
use crate::types::*;
use crate::PermissionRow;
use rusqlite::{
    params, params_from_iter, OptionalExtension, Row, Transaction, TransactionBehavior,
};

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

fn sqlite_terminal_state_sql(column: &str) -> String {
    format!(
        "lower({column}) IN ('closed','completed','complete','failed','cancelled','canceled','terminated','expired','orphaned','rejected','denied','approved')"
    )
}

fn sqlite_in_clause(count: usize) -> String {
    std::iter::repeat_n("?", count)
        .collect::<Vec<_>>()
        .join(",")
}

fn sqlite_delete_ids(
    tx: &Transaction<'_>,
    table: &str,
    column: &str,
    ids: &[String],
) -> DatabaseResult<usize> {
    if ids.is_empty() {
        return Ok(0);
    }
    let sql = format!(
        "DELETE FROM {table} WHERE {column} IN ({})",
        sqlite_in_clause(ids.len())
    );
    tx.execute(&sql, params_from_iter(ids.iter()))
        .map_err(|error| DatabaseError::Query(format!("failed to purge {table}: {error}")))
}

impl RuntimeMaintenance for SqliteDatabase {
    fn purge_expired(&self, cutoff: &str, batch_size: i64) -> DatabaseResult<RuntimePurgeCounts> {
        if !(1..=10_000).contains(&batch_size) {
            return Err(DatabaseError::Query(
                "runtime purge batch_size must be between 1 and 10000".to_string(),
            ));
        }
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        let tx =
            Transaction::new_unchecked(&conn, TransactionBehavior::Immediate).map_err(|error| {
                DatabaseError::Transaction(format!("failed to begin runtime purge: {error}"))
            })?;
        let terminal_state = sqlite_terminal_state_sql("state");
        let mut counts = RuntimePurgeCounts::default();

        let session_ids: Vec<String> = {
            let mut statement = tx
                .prepare(&format!(
                    "SELECT session_id FROM sessions
                     WHERE COALESCE(updated_at, created_at) < ?1
                       AND {terminal_state}
                     ORDER BY COALESCE(updated_at, created_at), session_id
                     LIMIT ?2"
                ))
                .map_err(|error| {
                    DatabaseError::Query(format!("failed to select expired sessions: {error}"))
                })?;
            let rows = statement
                .query_map(params![cutoff, batch_size], |row| row.get::<_, String>(0))
                .map_err(|error| {
                    DatabaseError::Query(format!("failed to read expired sessions: {error}"))
                })?;
            rows.collect::<Result<Vec<_>, _>>().map_err(|error| {
                DatabaseError::Query(format!("failed to collect expired sessions: {error}"))
            })?
        };
        if !session_ids.is_empty() {
            let clause = sqlite_in_clause(session_ids.len());
            for (table, count) in [
                ("messages", &mut counts.messages),
                ("tasks", &mut counts.tasks),
                ("events", &mut counts.events),
                ("permissions", &mut counts.permissions),
            ] {
                let sql = format!("SELECT COUNT(*) FROM {table} WHERE session_id IN ({clause})");
                let value: i64 = tx
                    .query_row(&sql, params_from_iter(session_ids.iter()), |row| row.get(0))
                    .map_err(|error| {
                        DatabaseError::Query(format!("failed to count expired {table}: {error}"))
                    })?;
                *count = (*count).saturating_add(value.max(0) as u64);
            }
            let deleted = sqlite_delete_ids(&tx, "sessions", "session_id", &session_ids)?;
            counts.sessions = deleted as u64;
        }

        let message_rows: Vec<(String, String)> = {
            let mut statement = tx
                .prepare(
                    "SELECT message_id, session_id FROM messages
                          WHERE created_at < ?1
                          ORDER BY created_at, message_id LIMIT ?2",
                )
                .map_err(|error| {
                    DatabaseError::Query(format!("failed to select expired messages: {error}"))
                })?;
            let rows = statement
                .query_map(params![cutoff, batch_size], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|error| {
                    DatabaseError::Query(format!("failed to read expired messages: {error}"))
                })?;
            rows.collect::<Result<Vec<_>, _>>().map_err(|error| {
                DatabaseError::Query(format!("failed to collect expired messages: {error}"))
            })?
        };
        if !message_rows.is_empty() {
            let message_ids: Vec<String> = message_rows.iter().map(|(id, _)| id.clone()).collect();
            let mut affected_sessions = message_rows
                .iter()
                .map(|(_, session_id)| session_id.clone())
                .collect::<Vec<_>>();
            affected_sessions.sort_unstable();
            affected_sessions.dedup();
            counts.messages = counts.messages.saturating_add(sqlite_delete_ids(
                &tx,
                "messages",
                "message_id",
                &message_ids,
            )? as u64);
            for session_id in affected_sessions {
                tx.execute(
                    "UPDATE sessions SET message_count = (
                        SELECT COUNT(*) FROM messages WHERE messages.session_id = sessions.session_id
                    ) WHERE session_id = ?1",
                    params![session_id],
                )
                .map_err(|error| DatabaseError::Query(format!("failed to refresh message count: {error}")))?;
            }
        }

        let task_ids: Vec<String> = {
            let mut statement = tx
                .prepare(&format!(
                    "SELECT task_id FROM tasks
                     WHERE COALESCE(updated_at, created_at) < ?1
                       AND {}
                     ORDER BY COALESCE(updated_at, created_at), task_id LIMIT ?2",
                    sqlite_terminal_state_sql("state")
                ))
                .map_err(|error| {
                    DatabaseError::Query(format!("failed to select expired tasks: {error}"))
                })?;
            let rows = statement
                .query_map(params![cutoff, batch_size], |row| row.get::<_, String>(0))
                .map_err(|error| {
                    DatabaseError::Query(format!("failed to read expired tasks: {error}"))
                })?;
            rows.collect::<Result<Vec<_>, _>>().map_err(|error| {
                DatabaseError::Query(format!("failed to collect expired tasks: {error}"))
            })?
        };
        counts.tasks = counts
            .tasks
            .saturating_add(sqlite_delete_ids(&tx, "tasks", "task_id", &task_ids)? as u64);

        let event_ids: Vec<String> = {
            let mut statement = tx
                .prepare(
                    "SELECT event_id FROM events WHERE created_at < ?1
                          ORDER BY created_at, event_id LIMIT ?2",
                )
                .map_err(|error| {
                    DatabaseError::Query(format!("failed to select expired events: {error}"))
                })?;
            let rows = statement
                .query_map(params![cutoff, batch_size], |row| row.get::<_, String>(0))
                .map_err(|error| {
                    DatabaseError::Query(format!("failed to read expired events: {error}"))
                })?;
            rows.collect::<Result<Vec<_>, _>>().map_err(|error| {
                DatabaseError::Query(format!("failed to collect expired events: {error}"))
            })?
        };
        counts.events = counts
            .events
            .saturating_add(sqlite_delete_ids(&tx, "events", "event_id", &event_ids)? as u64);

        let permission_ids: Vec<String> = {
            let mut statement = tx
                .prepare(&format!(
                    "SELECT permission_request_id FROM permissions
                     WHERE COALESCE(updated_at, created_at) < ?1
                       AND {}
                     ORDER BY COALESCE(updated_at, created_at), permission_request_id LIMIT ?2",
                    sqlite_terminal_state_sql("status")
                ))
                .map_err(|error| {
                    DatabaseError::Query(format!("failed to select expired permissions: {error}"))
                })?;
            let rows = statement
                .query_map(params![cutoff, batch_size], |row| row.get::<_, String>(0))
                .map_err(|error| {
                    DatabaseError::Query(format!("failed to read expired permissions: {error}"))
                })?;
            rows.collect::<Result<Vec<_>, _>>().map_err(|error| {
                DatabaseError::Query(format!("failed to collect expired permissions: {error}"))
            })?
        };
        counts.permissions = counts.permissions.saturating_add(sqlite_delete_ids(
            &tx,
            "permissions",
            "permission_request_id",
            &permission_ids,
        )? as u64);

        tx.commit().map_err(|error| {
            DatabaseError::Transaction(format!("failed to commit runtime purge: {error}"))
        })?;
        Ok(counts)
    }

    fn schema_status(&self) -> DatabaseResult<RuntimeSchemaStatus> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        let (version, count): (i64, i64) = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0), COUNT(*) FROM agent_runtime_schema_migration_history",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|error| DatabaseError::Migration(format!("failed to read SQLite migration state: {error}")))?;
        let structural = crate::schema_migrations::validate_sqlite_schema(&conn).is_ok();
        Ok(RuntimeSchemaStatus {
            version,
            expected_version: CURRENT_SCHEMA_VERSION,
            drift_free: structural
                && version == CURRENT_SCHEMA_VERSION
                && count == CURRENT_SCHEMA_VERSION,
        })
    }

    fn run_maintenance(&self) -> DatabaseResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        conn.execute_batch("PRAGMA wal_checkpoint(PASSIVE); PRAGMA incremental_vacuum(1000);")
            .map_err(|error| {
                DatabaseError::Query(format!("failed to run SQLite maintenance: {error}"))
            })
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
        let changed = conn
            .execute(
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
        if changed == 0 {
            return Err(DatabaseError::ConstraintViolation(format!(
                "session {} update conflicts with provider ownership or terminal lifecycle",
                session.session_id
            )));
        }
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
            if let Some(after_sort_at) = query
                .after_session_sort_at
                .as_deref()
                .filter(|value| !value.is_empty())
            {
                sql.push_str(
                    " AND (
                        COALESCE(updated_at, created_at) < ?
                        OR (
                            COALESCE(updated_at, created_at) = ?
                            AND session_id < ?
                        )
                    )",
                );
                values.push(after_sort_at.to_string());
                values.push(after_sort_at.to_string());
                values.push(after_session_id.to_string());
            } else {
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
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        let changed = conn
            .execute(
                "UPDATE sessions SET
                    agent_id = ?2, kind = ?3, source = ?4, state = ?5,
                    title = ?6, model = ?7, cwd = ?8, provider_id = ?9,
                    bridge_id = ?10, token_usage_json = ?11, updated_at = ?12,
                    metadata_json = ?13
                 WHERE session_id = ?1
                   AND provider_id IS ?9
                   AND (
                     LOWER(state) NOT IN ('closed', 'failed', 'archived')
                     OR LOWER(?5) IN ('closed', 'failed', 'archived')
                   )",
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
                    session.updated_at,
                    session.metadata_json,
                ],
            )
            .map_err(|error| DatabaseError::Query(format!("failed to update session: {error}")))?;
        if changed != 1 {
            let exists: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sessions WHERE session_id = ?1)",
                    params![session.session_id],
                    |row| row.get(0),
                )
                .map_err(|error| {
                    DatabaseError::Query(format!("failed to check session existence: {error}"))
                })?;
            return if exists {
                Err(DatabaseError::ConstraintViolation(format!(
                    "session {} update conflicts with provider ownership or terminal lifecycle",
                    session.session_id
                )))
            } else {
                Err(DatabaseError::NotFound(format!(
                    "session not found: {}",
                    session.session_id
                )))
            };
        }
        Ok(())
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
            if let Some(after_created_at) = query
                .after_message_created_at
                .as_deref()
                .filter(|value| !value.is_empty())
            {
                sql.push_str(
                    " AND (
                        created_at > ?
                        OR (created_at = ? AND message_id > ?)
                    )",
                );
                values.push(after_created_at.to_string());
                values.push(after_created_at.to_string());
                values.push(after_message_id.to_string());
            } else {
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
        }
        sql.push_str(" ORDER BY created_at ASC, message_id ASC");
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

    fn load_recent_messages(
        &self,
        session_id: &str,
        limit: i64,
    ) -> DatabaseResult<Vec<MessageRow>> {
        let limit = resolve_history_limit(limit)?;
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        let mut statement = conn
            .prepare(
                "SELECT message_id, session_id, role, content, created_at, metadata_json
                 FROM messages
                 WHERE session_id = ?1
                 ORDER BY created_at DESC, message_id DESC
                 LIMIT ?2",
            )
            .map_err(|error| {
                DatabaseError::Query(format!("failed to prepare recent messages: {error}"))
            })?;
        let rows = statement
            .query_map(params![session_id, limit], map_message_row)
            .map_err(|error| {
                DatabaseError::Query(format!("failed to load recent messages: {error}"))
            })?;
        let mut messages = Vec::new();
        for row in rows {
            messages.push(row.map_err(|error| {
                DatabaseError::Query(format!("failed to read recent message row: {error}"))
            })?);
        }
        messages.reverse();
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
            if let Some(after_created_at) = query
                .after_task_created_at
                .as_deref()
                .filter(|value| !value.is_empty())
            {
                sql.push_str(
                    " AND (
                        created_at > ?
                        OR (created_at = ? AND task_id > ?)
                    )",
                );
                values.push(after_created_at.to_string());
                values.push(after_created_at.to_string());
                values.push(after_task_id.to_string());
            } else {
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
        append_sqlite_event_scope(&mut sql, &mut values, query);
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
        append_sqlite_event_scope(&mut sql, &mut values, query);
        sql.push_str(" ORDER BY created_at DESC, event_id DESC");
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

fn append_sqlite_event_scope(sql: &mut String, values: &mut Vec<String>, query: &EventQuery) {
    if query.owner_tenant_id.is_none() && query.owner_user_ref.is_none() {
        return;
    }
    sql.push_str(
        " AND events.session_id IS NOT NULL
          AND EXISTS (
              SELECT 1 FROM sessions AS event_session
              WHERE event_session.session_id = events.session_id",
    );
    if let Some(owner_tenant_id) = query.owner_tenant_id.as_deref() {
        sql.push_str(" AND event_session.owner_tenant_id = ?");
        values.push(owner_tenant_id.to_string());
    }
    if let Some(owner_user_ref) = query.owner_user_ref.as_deref() {
        sql.push_str(" AND event_session.owner_user_ref = ?");
        values.push(owner_user_ref.to_string());
    }
    sql.push(')');
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
    fn create_permission_if_absent(&self, permission: &PermissionRow) -> DatabaseResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        let changed = conn
            .execute(
                "INSERT INTO permissions (
                    permission_request_id, session_id, category, resource,
                    side_effect_level, reason, status, owner_tenant_id,
                    owner_user_ref, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT(permission_request_id) DO NOTHING",
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
            .map_err(|error| {
                DatabaseError::Query(format!("failed to create permission: {error}"))
            })?;
        Ok(changed == 1)
    }

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
        if let Some(owner_tenant_id) = query.owner_tenant_id.as_deref() {
            sql.push_str(" AND owner_tenant_id = ?");
            values.push(owner_tenant_id.to_string());
        }
        if let Some(owner_user_ref) = query.owner_user_ref.as_deref() {
            sql.push_str(" AND owner_user_ref = ?");
            values.push(owner_user_ref.to_string());
        }
        sql.push_str(" ORDER BY created_at DESC, permission_request_id DESC");
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
        if !matches!(status, "allow" | "deny") {
            return Err(DatabaseError::ConstraintViolation(
                "permission status must be allow or deny".to_string(),
            ));
        }
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        let now = chrono::Utc::now().to_rfc3339();
        let changed = conn
            .execute(
                "UPDATE permissions SET status = ?1, updated_at = ?2
             WHERE permission_request_id = ?3 AND (status = 'pending' OR status = ?1)",
                params![status, now, permission_request_id],
            )
            .map_err(|error| {
                DatabaseError::Query(format!("failed to update permission: {error}"))
            })?;
        if changed == 0 {
            return Err(DatabaseError::ConstraintViolation(
                "permission request state conflict or not found".to_string(),
            ));
        }
        Ok(())
    }
}

impl RuntimeSessionWrites for SqliteDatabase {
    fn save_session_with_event(
        &self,
        session: &SessionRow,
        event: &EventRow,
    ) -> DatabaseResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        let tx = conn.unchecked_transaction().map_err(|error| {
            DatabaseError::Transaction(format!("failed to begin transaction: {error}"))
        })?;
        let changed = tx
            .execute(
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
        if changed == 0 {
            return Err(DatabaseError::ConstraintViolation(format!(
                "session {} update conflicts with provider ownership or terminal lifecycle",
                session.session_id
            )));
        }
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
        tx.commit().map_err(|error| {
            DatabaseError::Transaction(format!("failed to commit session event: {error}"))
        })
    }

    fn save_session_with_event_if_newer(
        &self,
        session: &SessionRow,
        event: &EventRow,
    ) -> DatabaseResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        let tx =
            Transaction::new_unchecked(&conn, TransactionBehavior::Immediate).map_err(|error| {
                DatabaseError::Transaction(format!("failed to begin transaction: {error}"))
            })?;
        let existing: Option<(Option<String>, String, Option<String>)> = tx
            .query_row(
                "SELECT provider_id, state, updated_at FROM sessions WHERE session_id = ?1",
                params![session.session_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(|error| {
                DatabaseError::Query(format!("failed to load session timestamp: {error}"))
            })?;
        let provider_conflicts = existing
            .as_ref()
            .and_then(|(provider_id, _, _)| provider_id.as_deref())
            .zip(session.provider_id.as_deref())
            .is_some_and(|(existing, incoming)| existing != incoming);
        let terminal_regression = existing.as_ref().is_some_and(|(_, state, _)| {
            crate::types::session_state_is_terminal(state)
                && !crate::types::session_state_is_terminal(&session.state)
        });
        let stale = existing
            .as_ref()
            .is_some_and(|(_, _, existing_updated_at)| {
                session.updated_at.is_none()
                    || crate::types::timestamp_is_older(
                        session.updated_at.as_deref(),
                        existing_updated_at.as_deref(),
                    )
            });
        if provider_conflicts || terminal_regression || stale {
            tx.commit().map_err(|error| {
                DatabaseError::Transaction(format!("failed to commit stale session check: {error}"))
            })?;
            return Ok(false);
        }
        let applied = tx.execute(
            crate::upsert_sql::sqlite::SAVE_PROVIDER_SESSION,
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
        )? > 0;
        if applied {
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
            DatabaseError::Transaction(format!("failed to commit session event: {error}"))
        })?;
        Ok(applied)
    }

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
        let session_state: String = tx
            .query_row(
                "SELECT state FROM sessions WHERE session_id = ?1",
                params![message.session_id],
                |row| row.get(0),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    DatabaseError::NotFound(format!("session not found: {}", message.session_id))
                }
                other => DatabaseError::Query(format!(
                    "failed to lock session for message append: {other}"
                )),
            })?;
        if crate::types::session_state_is_terminal(&session_state) {
            return Err(DatabaseError::ConstraintViolation(format!(
                "session {} is terminal ({session_state})",
                message.session_id
            )));
        }
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

    fn append_message_turn_with_events(
        &self,
        turn_messages: &[MessageRow],
        turn_events: &[EventRow],
    ) -> DatabaseResult<i64> {
        let session_id =
            crate::message_identity::validate_message_turn(turn_messages, turn_events)?;
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        let tx = conn.unchecked_transaction().map_err(|error| {
            DatabaseError::Transaction(format!("failed to begin message turn: {error}"))
        })?;
        let (session_state, current_count): (String, i64) = tx
            .query_row(
                "SELECT state, message_count FROM sessions WHERE session_id = ?1",
                params![session_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    DatabaseError::NotFound(format!("session not found: {session_id}"))
                }
                other => DatabaseError::Query(format!(
                    "failed to lock session for message turn: {other}"
                )),
            })?;
        if !session_state.eq_ignore_ascii_case("active") {
            return Err(DatabaseError::ConstraintViolation(format!(
                "session {session_id} is not active"
            )));
        }

        let mut inserted_count = 0_i64;
        for message in turn_messages {
            let inserted = tx
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
                .map_err(|error| {
                    DatabaseError::Query(format!("failed to save message turn row: {error}"))
                })?;
            if inserted > 0 {
                inserted_count = inserted_count.checked_add(1).ok_or_else(|| {
                    DatabaseError::ConstraintViolation("message turn size overflow".to_string())
                })?;
            } else {
                let existing = tx
                    .query_row(
                        "SELECT message_id, session_id, role, content, created_at, metadata_json
                         FROM messages WHERE message_id = ?1",
                        params![message.message_id],
                        map_message_row,
                    )
                    .map_err(|error| {
                        DatabaseError::Query(format!(
                            "failed to load existing message turn row: {error}"
                        ))
                    })?;
                crate::message_identity::ensure_message_retry_matches(&existing, message)?;
            }
        }

        let count = if inserted_count > 0 {
            let updated_at = chrono::Utc::now().to_rfc3339();
            let count = tx
                .query_row(
                    "UPDATE sessions
                     SET message_count = message_count + ?2, updated_at = ?3
                     WHERE session_id = ?1 RETURNING message_count",
                    params![session_id, inserted_count, updated_at],
                    |row| row.get(0),
                )
                .map_err(|error| {
                    DatabaseError::Query(format!(
                        "failed to update completed turn message count: {error}"
                    ))
                })?;
            for event in turn_events {
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
                .map_err(|error| {
                    DatabaseError::Query(format!("failed to save completed turn event: {error}"))
                })?;
            }
            count
        } else {
            current_count
        };

        tx.commit().map_err(|error| {
            DatabaseError::Transaction(format!("failed to commit message turn: {error}"))
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

    fn save_task_with_event(&self, task: &TaskRow, event: &EventRow) -> DatabaseResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        let tx = conn.unchecked_transaction().map_err(|error| {
            DatabaseError::Transaction(format!("failed to begin transaction: {error}"))
        })?;
        tx.execute(
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
        tx.commit().map_err(|error| {
            DatabaseError::Transaction(format!("failed to commit task event: {error}"))
        })
    }

    fn cancel_task_with_event(
        &self,
        task_id: &str,
        updated_at: &str,
        event: &EventRow,
    ) -> DatabaseResult<(TaskRow, bool)> {
        let conn = self
            .conn
            .lock()
            .map_err(|error| DatabaseError::Internal(format!("failed to acquire lock: {error}")))?;
        let tx = conn.unchecked_transaction().map_err(|error| {
            DatabaseError::Transaction(format!("failed to begin transaction: {error}"))
        })?;
        let mut task = tx
            .query_row(
                "SELECT task_id, session_id, instruction, state, created_at, updated_at
                 FROM tasks WHERE task_id = ?1",
                params![task_id],
                map_task_row,
            )
            .optional()
            .map_err(|error| DatabaseError::Query(format!("failed to load task: {error}")))?
            .ok_or_else(|| DatabaseError::NotFound(format!("task not found: {task_id}")))?;
        if task.state == "cancelled" {
            return Ok((task, false));
        }
        if !matches!(task.state.as_str(), "created" | "pending" | "running") {
            return Err(DatabaseError::ConstraintViolation(format!(
                "task {task_id} is not active"
            )));
        }
        if event.session_id.as_deref() != Some(task.session_id.as_str()) {
            return Err(DatabaseError::ConstraintViolation(
                "task cancellation event session mismatch".to_string(),
            ));
        }
        let changed = tx
            .execute(
                "UPDATE tasks SET state = 'cancelled', updated_at = ?2
                 WHERE task_id = ?1 AND state IN ('created', 'pending', 'running')",
                params![task_id, updated_at],
            )
            .map_err(|error| DatabaseError::Query(format!("failed to cancel task: {error}")))?;
        if changed != 1 {
            return Err(DatabaseError::ConstraintViolation(format!(
                "task {task_id} state changed concurrently"
            )));
        }
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
        tx.commit().map_err(|error| {
            DatabaseError::Transaction(format!("failed to commit task cancellation: {error}"))
        })?;
        task.state = "cancelled".to_string();
        task.updated_at = Some(updated_at.to_string());
        Ok((task, true))
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
