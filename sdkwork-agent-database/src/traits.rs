use crate::error::DatabaseResult;
use crate::types::*;

/// Core database trait for agent persistence
pub trait AgentDatabase: Send + Sync {
    /// Execute a SQL statement
    fn execute(&self, sql: &str, params: &[&dyn DatabaseParam]) -> DatabaseResult<usize>;

    /// Query for multiple rows
    fn query_many(
        &self,
        sql: &str,
        params: &[&dyn DatabaseParam],
    ) -> DatabaseResult<Vec<Box<dyn DatabaseRow>>>;

    /// Check if the database is healthy
    fn health(&self) -> DatabaseResult<bool>;

    /// Apply the backend's versioned schema migrations.
    fn migrate_schema(&self) -> DatabaseResult<()> {
        Err(crate::error::DatabaseError::Migration(
            "this database backend does not expose schema migration support".to_string(),
        ))
    }
}

/// Lifecycle operations for transient runtime state.
///
/// Implementations must perform filtering and batching in the store. The
/// server never loads a complete table into process memory for retention work.
pub trait RuntimeMaintenance: Send + Sync {
    /// Delete rows older than `cutoff` in bounded batches and return counts.
    fn purge_expired(&self, cutoff: &str, batch_size: i64) -> DatabaseResult<RuntimePurgeCounts>;

    /// Check migration history and structural schema invariants without
    /// applying migrations or mutating application data.
    fn schema_status(&self) -> DatabaseResult<RuntimeSchemaStatus>;

    /// Run backend-specific low-impact maintenance after a purge. SQLite uses
    /// a passive WAL checkpoint and incremental vacuum; PostgreSQL is a no-op
    /// because autovacuum owns table maintenance.
    fn run_maintenance(&self) -> DatabaseResult<()>;
}

/// Database parameter trait
pub trait DatabaseParam: Send + Sync {
    fn as_sql_value(&self) -> String;
}

/// Database row trait
pub trait DatabaseRow: Send + Sync {
    fn get_string(&self, column: &str) -> DatabaseResult<String>;
    fn get_optional_string(&self, column: &str) -> DatabaseResult<Option<String>>;
    fn get_i64(&self, column: &str) -> DatabaseResult<i64>;
}

/// Session repository trait
pub trait SessionRepository: Send + Sync {
    fn save_session(&self, session: &SessionRow) -> DatabaseResult<()>;
    fn load_session(&self, session_id: &str) -> DatabaseResult<Option<SessionRow>>;
    fn list_sessions(&self, query: &SessionQuery) -> DatabaseResult<Vec<SessionRow>>;
    fn update_session(&self, session: &SessionRow) -> DatabaseResult<()>;
    fn delete_session(&self, session_id: &str) -> DatabaseResult<()>;
    /// Atomically delete a session and all dependent rows.
    fn delete_session_cascade(&self, session_id: &str) -> DatabaseResult<()>;
    /// Atomically increment `message_count` and refresh `updated_at`.
    fn increment_session_message_count(&self, session_id: &str) -> DatabaseResult<i64>;
}

/// Message repository trait
pub trait MessageRepository: Send + Sync {
    fn save_message(&self, message: &MessageRow) -> DatabaseResult<()>;
    fn load_messages(
        &self,
        session_id: &str,
        query: &MessageQuery,
    ) -> DatabaseResult<Vec<MessageRow>>;
    /// Load a bounded tail of session messages and return it in chronological order.
    fn load_recent_messages(&self, session_id: &str, limit: i64)
        -> DatabaseResult<Vec<MessageRow>>;
    fn message_count(&self, session_id: &str) -> DatabaseResult<i64>;
    fn delete_messages(&self, session_id: &str) -> DatabaseResult<()>;
}

/// Task repository trait
pub trait TaskRepository: Send + Sync {
    fn save_task(&self, task: &TaskRow) -> DatabaseResult<()>;
    fn load_task(&self, task_id: &str) -> DatabaseResult<Option<TaskRow>>;
    fn load_tasks(&self, session_id: &str, query: &TaskQuery) -> DatabaseResult<Vec<TaskRow>>;
    fn update_task(&self, task: &TaskRow) -> DatabaseResult<()>;
    fn delete_task(&self, task_id: &str) -> DatabaseResult<()>;
}

/// Event repository trait
pub trait EventRepository: Send + Sync {
    fn save_event(&self, event: &EventRow) -> DatabaseResult<()>;
    fn load_events(&self, session_id: &str, query: &EventQuery) -> DatabaseResult<Vec<EventRow>>;
    /// List recent events across all sessions (newest first).
    fn list_recent_events(&self, query: &EventQuery) -> DatabaseResult<Vec<EventRow>>;
    fn delete_events(&self, session_id: &str) -> DatabaseResult<()>;
}

/// Cross-table atomic writes for session message lifecycle.
pub trait RuntimeSessionWrites: Send + Sync {
    /// Atomically persist a session state and its corresponding event.
    fn save_session_with_event(&self, session: &SessionRow, event: &EventRow)
        -> DatabaseResult<()>;

    /// Atomically persist a message, increment the session message count, and record an event.
    fn append_message_with_event(
        &self,
        message: &MessageRow,
        event: &EventRow,
    ) -> DatabaseResult<i64>;

    /// Atomically delete all session messages and reset the cached message count.
    fn delete_messages_and_reset_count(
        &self,
        session_id: &str,
        updated_at: &str,
    ) -> DatabaseResult<()>;

    /// Atomically persist a task state and its corresponding event.
    fn save_task_with_event(&self, task: &TaskRow, event: &EventRow) -> DatabaseResult<()>;
}

/// Permission repository trait for persisting permission request state.
pub trait PermissionRepository: Send + Sync {
    fn save_permission(&self, permission: &PermissionRow) -> DatabaseResult<()>;
    fn load_permission(&self, permission_request_id: &str)
        -> DatabaseResult<Option<PermissionRow>>;
    fn list_permissions(&self, query: &PermissionQuery) -> DatabaseResult<Vec<PermissionRow>>;
    fn update_permission_status(
        &self,
        permission_request_id: &str,
        status: &str,
    ) -> DatabaseResult<()>;
}

// Default implementations for DatabaseParam
impl DatabaseParam for String {
    fn as_sql_value(&self) -> String {
        self.clone()
    }
}

impl DatabaseParam for &str {
    fn as_sql_value(&self) -> String {
        self.to_string()
    }
}

impl DatabaseParam for i64 {
    fn as_sql_value(&self) -> String {
        self.to_string()
    }
}

impl DatabaseParam for i32 {
    fn as_sql_value(&self) -> String {
        self.to_string()
    }
}

impl DatabaseParam for bool {
    fn as_sql_value(&self) -> String {
        if *self {
            "1".to_string()
        } else {
            "0".to_string()
        }
    }
}
