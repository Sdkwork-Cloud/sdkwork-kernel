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
}

/// Message repository trait
pub trait MessageRepository: Send + Sync {
    fn save_message(&self, message: &MessageRow) -> DatabaseResult<()>;
    fn load_messages(
        &self,
        session_id: &str,
        query: &MessageQuery,
    ) -> DatabaseResult<Vec<MessageRow>>;
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
