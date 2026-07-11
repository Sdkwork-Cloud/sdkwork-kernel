use sdkwork_agent_database::{
    EventRow, MessageRow, PermissionRepository, PermissionRow, PostgresDatabase,
    RuntimeMaintenance, RuntimePurgeCounts, RuntimeSchemaStatus, SessionRow, SqliteDatabase,
    TaskRow,
};
use sdkwork_agent_session::{MessageConfig, SessionConfig, SessionQuery, UnifiedSessionManager};
use std::sync::Arc;

use crate::config::ServerConfig;
use crate::event_bus::SessionEventBus;

type AppSessionManagerSqlite = UnifiedSessionManager<SqliteDatabase>;

type AppSessionManagerPostgres = UnifiedSessionManager<PostgresDatabase>;

#[derive(Clone)]
enum ManagerInner {
    Sqlite(Arc<AppSessionManagerSqlite>),
    Postgres(Arc<AppSessionManagerPostgres>),
}

/// Database handle for permission persistence operations.
#[derive(Clone)]
enum PermissionDb {
    Sqlite(SqliteDatabase),
    Postgres(PostgresDatabase),
}

#[derive(Clone)]
enum MaintenanceDb {
    Sqlite(SqliteDatabase),
    Postgres(PostgresDatabase),
}

impl RuntimeMaintenance for MaintenanceDb {
    fn purge_expired(
        &self,
        cutoff: &str,
        batch_size: i64,
    ) -> Result<RuntimePurgeCounts, sdkwork_agent_database::DatabaseError> {
        match self {
            MaintenanceDb::Sqlite(db) => db.purge_expired(cutoff, batch_size),
            MaintenanceDb::Postgres(db) => db.purge_expired(cutoff, batch_size),
        }
    }

    fn schema_status(&self) -> Result<RuntimeSchemaStatus, sdkwork_agent_database::DatabaseError> {
        match self {
            MaintenanceDb::Sqlite(db) => db.schema_status(),
            MaintenanceDb::Postgres(db) => db.schema_status(),
        }
    }

    fn run_maintenance(&self) -> Result<(), sdkwork_agent_database::DatabaseError> {
        match self {
            MaintenanceDb::Sqlite(db) => db.run_maintenance(),
            MaintenanceDb::Postgres(db) => db.run_maintenance(),
        }
    }
}

impl PermissionRepository for PermissionDb {
    fn save_permission(
        &self,
        permission: &PermissionRow,
    ) -> Result<(), sdkwork_agent_database::DatabaseError> {
        match self {
            PermissionDb::Sqlite(db) => db.save_permission(permission),
            PermissionDb::Postgres(db) => db.save_permission(permission),
        }
    }

    fn load_permission(
        &self,
        permission_request_id: &str,
    ) -> Result<Option<PermissionRow>, sdkwork_agent_database::DatabaseError> {
        match self {
            PermissionDb::Sqlite(db) => db.load_permission(permission_request_id),
            PermissionDb::Postgres(db) => db.load_permission(permission_request_id),
        }
    }

    fn list_permissions(
        &self,
        query: &sdkwork_agent_database::PermissionQuery,
    ) -> Result<Vec<PermissionRow>, sdkwork_agent_database::DatabaseError> {
        match self {
            PermissionDb::Sqlite(db) => db.list_permissions(query),
            PermissionDb::Postgres(db) => db.list_permissions(query),
        }
    }

    fn update_permission_status(
        &self,
        permission_request_id: &str,
        status: &str,
    ) -> Result<(), sdkwork_agent_database::DatabaseError> {
        match self {
            PermissionDb::Sqlite(db) => db.update_permission_status(permission_request_id, status),
            PermissionDb::Postgres(db) => {
                db.update_permission_status(permission_request_id, status)
            }
        }
    }
}

macro_rules! with_manager {
    ($self:expr, |$mgr:ident| $body:expr) => {{
        match &$self.manager {
            ManagerInner::Sqlite(inner) => {
                let $mgr = &**inner;
                $body
            }
            ManagerInner::Postgres(inner) => {
                let $mgr = &**inner;
                $body
            }
        }
    }};
}

/// Shared session persistence for server handlers (SQLite or PostgreSQL).
#[derive(Clone)]
pub struct PersistenceState {
    manager: ManagerInner,
    permission_db: PermissionDb,
    maintenance_db: MaintenanceDb,
    event_bus: SessionEventBus,
    backend: PersistenceBackend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistenceBackend {
    Sqlite,
    Postgres,
}

impl PersistenceState {
    pub async fn open_from_config_async(config: &ServerConfig) -> anyhow::Result<Self> {
        if config.uses_postgres_runtime_database() {
            Self::open_postgres_with_event_bus_async(SessionEventBus::new()).await
        } else {
            Self::open_with_event_bus(&config.database_path, SessionEventBus::new())
        }
    }

    pub fn open_from_config(config: &ServerConfig) -> anyhow::Result<Self> {
        if config.uses_postgres_runtime_database() {
            Self::open_postgres_with_event_bus(SessionEventBus::new())
        } else {
            Self::open_with_event_bus(&config.database_path, SessionEventBus::new())
        }
    }

    pub fn open(database_path: &str) -> anyhow::Result<Self> {
        Self::open_with_event_bus(database_path, SessionEventBus::new())
    }

    pub fn open_with_event_bus(
        database_path: &str,
        event_bus: SessionEventBus,
    ) -> anyhow::Result<Self> {
        let db = SqliteDatabase::open_migrated(database_path)?;
        Ok(Self::from_sqlite_database(db, event_bus))
    }

    pub fn open_postgres() -> anyhow::Result<Self> {
        Self::open_postgres_with_event_bus(SessionEventBus::new())
    }

    pub async fn open_postgres_with_event_bus_async(
        event_bus: SessionEventBus,
    ) -> anyhow::Result<Self> {
        let db = PostgresDatabase::connect_from_sdkwork_env_async("AGENT_RUNTIME").await?;
        Ok(Self::from_postgres_database(db, event_bus))
    }

    pub fn open_postgres_with_event_bus(event_bus: SessionEventBus) -> anyhow::Result<Self> {
        let db = PostgresDatabase::connect_from_sdkwork_env("AGENT_RUNTIME")?;
        Ok(Self::from_postgres_database(db, event_bus))
    }

    pub fn memory() -> anyhow::Result<Self> {
        Self::memory_with_event_bus(SessionEventBus::new())
    }

    pub fn memory_with_event_bus(event_bus: SessionEventBus) -> anyhow::Result<Self> {
        let db = SqliteDatabase::memory_migrated()?;
        Ok(Self::from_sqlite_database(db, event_bus))
    }

    pub fn backend(&self) -> PersistenceBackend {
        self.backend
    }

    pub fn persistence_backend_label(&self) -> &'static str {
        match self.backend {
            PersistenceBackend::Sqlite => "sqlite",
            PersistenceBackend::Postgres => "postgres",
        }
    }

    fn from_sqlite_database(db: SqliteDatabase, event_bus: SessionEventBus) -> Self {
        let permission_db = db.clone();
        let mut manager = UnifiedSessionManager::new(db.clone());
        let bus = event_bus.clone();
        manager.set_event_listener(Arc::new(move |event| bus.publish(event)));
        Self {
            manager: ManagerInner::Sqlite(Arc::new(manager)),
            permission_db: PermissionDb::Sqlite(permission_db),
            maintenance_db: MaintenanceDb::Sqlite(db),
            event_bus,
            backend: PersistenceBackend::Sqlite,
        }
    }

    fn from_postgres_database(db: PostgresDatabase, event_bus: SessionEventBus) -> Self {
        let permission_db = db.clone();
        let mut manager = UnifiedSessionManager::new(db.clone());
        let bus = event_bus.clone();
        manager.set_event_listener(Arc::new(move |event| bus.publish(event)));
        Self {
            manager: ManagerInner::Postgres(Arc::new(manager)),
            permission_db: PermissionDb::Postgres(permission_db),
            maintenance_db: MaintenanceDb::Postgres(db),
            event_bus,
            backend: PersistenceBackend::Postgres,
        }
    }

    pub fn event_bus(&self) -> &SessionEventBus {
        &self.event_bus
    }

    pub fn create_session(&self, config: SessionConfig) -> Result<SessionRow, String> {
        with_manager!(self, |manager| manager.create_session(config))
    }

    pub fn get_session(&self, session_id: &str) -> Result<SessionRow, String> {
        with_manager!(self, |manager| manager.get_session(session_id))
    }

    pub fn list_sessions(&self, query: SessionQuery) -> Result<Vec<SessionRow>, String> {
        with_manager!(self, |manager| manager.list_sessions(query))
    }

    pub fn close_session(&self, session_id: &str) -> Result<SessionRow, String> {
        with_manager!(self, |manager| manager.close_session(session_id))
    }

    pub fn delete_session(&self, session_id: &str) -> Result<(), String> {
        with_manager!(self, |manager| manager.delete_session(session_id))
    }

    pub fn send_message(
        &self,
        session_id: &str,
        config: MessageConfig,
    ) -> Result<MessageRow, String> {
        with_manager!(self, |manager| manager.send_message(session_id, config))
    }

    pub fn emit_session_event(
        &self,
        session_id: &str,
        event_type: &str,
        severity: &str,
        payload: Option<&str>,
    ) -> Result<(), String> {
        with_manager!(self, |manager| {
            manager.emit_session_event(session_id, event_type, severity, payload)
        })
    }

    pub fn list_messages(
        &self,
        session_id: &str,
        query: sdkwork_agent_database::MessageQuery,
    ) -> Result<Vec<MessageRow>, String> {
        with_manager!(self, |manager| manager.list_messages(session_id, query))
    }

    pub fn get_messages(
        &self,
        session_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<MessageRow>, String> {
        with_manager!(self, |manager| manager
            .get_messages(session_id, limit, offset))
    }

    pub fn load_recent_messages(
        &self,
        session_id: &str,
        limit: i64,
    ) -> Result<Vec<MessageRow>, String> {
        with_manager!(self, |manager| manager
            .load_recent_messages(session_id, limit))
    }

    pub fn message_count(&self, session_id: &str) -> Result<i64, String> {
        with_manager!(self, |manager| manager.message_count(session_id))
    }

    pub fn delete_messages(&self, session_id: &str) -> Result<(), String> {
        with_manager!(self, |manager| manager.delete_messages(session_id))
    }

    pub fn health(&self) -> Result<bool, String> {
        with_manager!(self, |manager| manager.health())
    }

    pub fn schema_status(&self) -> Result<RuntimeSchemaStatus, String> {
        self.maintenance_db
            .schema_status()
            .map_err(|error| format!("failed to inspect runtime schema: {error}"))
    }

    pub fn readiness(&self) -> Result<(), String> {
        if !self.health()? {
            return Err("runtime persistence health probe returned false".to_string());
        }
        let status = self.schema_status()?;
        if !status.drift_free {
            return Err(format!(
                "runtime persistence schema is not current (version={}, expected={})",
                status.version, status.expected_version
            ));
        }
        Ok(())
    }

    pub fn purge_expired(
        &self,
        cutoff: &str,
        batch_size: i64,
    ) -> Result<RuntimePurgeCounts, String> {
        self.maintenance_db
            .purge_expired(cutoff, batch_size)
            .map_err(|error| format!("failed to purge expired runtime state: {error}"))
    }

    pub fn run_maintenance(&self) -> Result<(), String> {
        self.maintenance_db
            .run_maintenance()
            .map_err(|error| format!("failed to run runtime database maintenance: {error}"))
    }

    pub fn create_task(&self, session_id: &str, instruction: &str) -> Result<TaskRow, String> {
        with_manager!(self, |manager| manager.create_task(session_id, instruction))
    }

    pub fn get_task(&self, task_id: &str) -> Result<TaskRow, String> {
        with_manager!(self, |manager| manager.get_task(task_id))
    }

    pub fn list_tasks(
        &self,
        session_id: &str,
        query: sdkwork_agent_database::TaskQuery,
    ) -> Result<Vec<TaskRow>, String> {
        with_manager!(self, |manager| manager.list_tasks(session_id, query))
    }

    pub fn cancel_task(&self, task_id: &str) -> Result<TaskRow, String> {
        with_manager!(self, |manager| manager.cancel_task(task_id))
    }

    pub fn load_session_events(
        &self,
        session_id: &str,
        limit: Option<i64>,
        after_event_id: Option<&str>,
    ) -> Result<Vec<EventRow>, String> {
        with_manager!(self, |manager| {
            manager.load_session_events(session_id, limit, after_event_id)
        })
    }

    pub fn list_recent_events(
        &self,
        query: sdkwork_agent_database::EventQuery,
    ) -> Result<Vec<EventRow>, String> {
        with_manager!(self, |manager| manager.list_recent_events(query))
    }

    // -- Permission persistence --

    pub fn save_permission(&self, permission: &PermissionRow) -> Result<(), String> {
        self.permission_db
            .save_permission(permission)
            .map_err(|error| format!("failed to save permission: {error}"))
    }

    pub fn load_permission(
        &self,
        permission_request_id: &str,
    ) -> Result<Option<PermissionRow>, String> {
        self.permission_db
            .load_permission(permission_request_id)
            .map_err(|error| format!("failed to load permission: {error}"))
    }

    pub fn list_permissions(
        &self,
        query: sdkwork_agent_database::PermissionQuery,
    ) -> Result<Vec<PermissionRow>, String> {
        self.permission_db
            .list_permissions(&query)
            .map_err(|error| format!("failed to list permissions: {error}"))
    }

    pub fn update_permission_status(
        &self,
        permission_request_id: &str,
        status: &str,
    ) -> Result<(), String> {
        self.permission_db
            .update_permission_status(permission_request_id, status)
            .map_err(|error| format!("failed to update permission: {error}"))
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
        assert_eq!(persistence.backend(), PersistenceBackend::Sqlite);
    }
}
