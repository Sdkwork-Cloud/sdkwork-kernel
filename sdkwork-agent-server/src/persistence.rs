use sdkwork_agent_database::{EventRow, MessageRow, SessionRow, TaskRow, SqliteDatabase};
use sdkwork_agent_session::{MessageConfig, SessionConfig, SessionQuery, UnifiedSessionManager};
use std::sync::{Arc, Mutex};

pub type AppSessionManager = UnifiedSessionManager<
    SqliteDatabase,
    SqliteDatabase,
    SqliteDatabase,
    SqliteDatabase,
    SqliteDatabase,
>;

/// Shared SQLite-backed session persistence for server handlers.
#[derive(Clone)]
pub struct PersistenceState {
    manager: Arc<Mutex<AppSessionManager>>,
}

impl PersistenceState {
    pub fn open(database_path: &str) -> anyhow::Result<Self> {
        let db = SqliteDatabase::open_migrated(database_path)?;
        Ok(Self::from_database(db))
    }

    pub fn memory() -> anyhow::Result<Self> {
        let db = SqliteDatabase::memory_migrated()?;
        Ok(Self::from_database(db))
    }

    fn from_database(db: SqliteDatabase) -> Self {
        let manager =
            UnifiedSessionManager::new(db.clone(), db.clone(), db.clone(), db.clone(), db);
        Self {
            manager: Arc::new(Mutex::new(manager)),
        }
    }

    pub fn create_session(&self, config: SessionConfig) -> Result<SessionRow, String> {
        let manager = self
            .manager
            .lock()
            .map_err(|error| format!("lock poisoned: {error}"))?;
        manager.create_session(config)
    }

    pub fn get_session(&self, session_id: &str) -> Result<SessionRow, String> {
        let manager = self
            .manager
            .lock()
            .map_err(|error| format!("lock poisoned: {error}"))?;
        manager.get_session(session_id)
    }

    pub fn list_sessions(&self, query: SessionQuery) -> Result<Vec<SessionRow>, String> {
        let manager = self
            .manager
            .lock()
            .map_err(|error| format!("lock poisoned: {error}"))?;
        manager.list_sessions(query)
    }

    pub fn close_session(&self, session_id: &str) -> Result<SessionRow, String> {
        let manager = self
            .manager
            .lock()
            .map_err(|error| format!("lock poisoned: {error}"))?;
        manager.close_session(session_id)
    }

    pub fn delete_session(&self, session_id: &str) -> Result<(), String> {
        let manager = self
            .manager
            .lock()
            .map_err(|error| format!("lock poisoned: {error}"))?;
        manager.delete_session(session_id)
    }

    pub fn send_message(
        &self,
        session_id: &str,
        config: MessageConfig,
    ) -> Result<MessageRow, String> {
        let manager = self
            .manager
            .lock()
            .map_err(|error| format!("lock poisoned: {error}"))?;
        manager.send_message(session_id, config)
    }

    pub fn get_messages(
        &self,
        session_id: &str,
        limit: Option<i64>,
    ) -> Result<Vec<MessageRow>, String> {
        let manager = self
            .manager
            .lock()
            .map_err(|error| format!("lock poisoned: {error}"))?;
        manager.get_messages(session_id, limit)
    }

    pub fn message_count(&self, session_id: &str) -> Result<i64, String> {
        let manager = self
            .manager
            .lock()
            .map_err(|error| format!("lock poisoned: {error}"))?;
        manager.message_count(session_id)
    }

    pub fn delete_messages(&self, session_id: &str) -> Result<(), String> {
        let manager = self
            .manager
            .lock()
            .map_err(|error| format!("lock poisoned: {error}"))?;
        manager
            .conversation()
            .clear_history(session_id)
            .map_err(|error| format!("failed to delete messages: {error}"))
    }

    pub fn health(&self) -> Result<bool, String> {
        let manager = self
            .manager
            .lock()
            .map_err(|error| format!("lock poisoned: {error}"))?;
        manager.health()
    }

    pub fn create_task(&self, session_id: &str, instruction: &str) -> Result<TaskRow, String> {
        let manager = self
            .manager
            .lock()
            .map_err(|error| format!("lock poisoned: {error}"))?;
        manager.create_task(session_id, instruction)
    }

    pub fn get_task(&self, task_id: &str) -> Result<TaskRow, String> {
        let manager = self
            .manager
            .lock()
            .map_err(|error| format!("lock poisoned: {error}"))?;
        manager.get_task(task_id)
    }

    pub fn list_tasks(&self, session_id: &str) -> Result<Vec<TaskRow>, String> {
        let manager = self
            .manager
            .lock()
            .map_err(|error| format!("lock poisoned: {error}"))?;
        manager.list_tasks(session_id)
    }

    pub fn cancel_task(&self, task_id: &str) -> Result<TaskRow, String> {
        let manager = self
            .manager
            .lock()
            .map_err(|error| format!("lock poisoned: {error}"))?;
        manager.cancel_task(task_id)
    }

    pub fn load_session_events(
        &self,
        session_id: &str,
        limit: Option<i64>,
    ) -> Result<Vec<EventRow>, String> {
        let manager = self
            .manager
            .lock()
            .map_err(|error| format!("lock poisoned: {error}"))?;
        manager.load_session_events(session_id, limit)
    }
}
