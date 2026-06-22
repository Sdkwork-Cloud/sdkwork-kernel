use sdkwork_agent_database::{EventRow, MessageRow, SessionRow, TaskRow, SqliteDatabase};
use sdkwork_agent_session::{MessageConfig, SessionConfig, SessionQuery, UnifiedSessionManager};
use std::sync::{Arc, Mutex};

use crate::event_bus::SessionEventBus;

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
    event_bus: SessionEventBus,
}

impl PersistenceState {
    pub fn open(database_path: &str) -> anyhow::Result<Self> {
        Self::open_with_event_bus(database_path, SessionEventBus::new())
    }

    pub fn open_with_event_bus(
        database_path: &str,
        event_bus: SessionEventBus,
    ) -> anyhow::Result<Self> {
        let db = SqliteDatabase::open_migrated(database_path)?;
        Ok(Self::from_database(db, event_bus))
    }

    pub fn memory() -> anyhow::Result<Self> {
        Self::memory_with_event_bus(SessionEventBus::new())
    }

    pub fn memory_with_event_bus(event_bus: SessionEventBus) -> anyhow::Result<Self> {
        let db = SqliteDatabase::memory_migrated()?;
        Ok(Self::from_database(db, event_bus))
    }

    fn from_database(db: SqliteDatabase, event_bus: SessionEventBus) -> Self {
        let mut manager =
            UnifiedSessionManager::new(db.clone(), db.clone(), db.clone(), db.clone(), db);
        let bus = event_bus.clone();
        manager.set_event_listener(Arc::new(move |event| bus.publish(event)));
        Self {
            manager: Arc::new(Mutex::new(manager)),
            event_bus,
        }
    }

    pub fn event_bus(&self) -> &SessionEventBus {
        &self.event_bus
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

    pub fn emit_session_event(
        &self,
        session_id: &str,
        event_type: &str,
        severity: &str,
        payload: Option<&str>,
    ) -> Result<(), String> {
        let manager = self
            .manager
            .lock()
            .map_err(|error| format!("lock poisoned: {error}"))?;
        manager.emit_session_event(session_id, event_type, severity, payload)
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
        manager.delete_messages(session_id)
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

    /// Run a blocking persistence operation off the async runtime thread pool.
    pub async fn run<F, T>(&self, operation: F) -> Result<T, String>
    where
        F: FnOnce(&PersistenceState) -> Result<T, String> + Send + 'static,
        T: Send + 'static,
    {
        let state = self.clone();
        tokio::task::spawn_blocking(move || operation(&state))
            .await
            .map_err(|error| format!("persistence worker failed: {error}"))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_session_publishes_event() {
        let persistence = PersistenceState::memory().expect("persistence");
        let mut receiver = persistence.event_bus().subscribe();
        let row = persistence
            .create_session(SessionConfig::new("agent.1"))
            .expect("session");
        let event = receiver.try_recv().expect("published event");
        assert_eq!(event.session_id.as_deref(), Some(row.session_id.as_str()));
        assert_eq!(event.event_type, "session.created");
    }
}
