mod error;
mod event_identity;
mod message_identity;
mod pagination;
mod schema;
mod schema_migrations;
mod traits;
mod types;
mod upsert_sql;

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "sqlite")]
mod sqlite_execution_repository;
#[cfg(feature = "sqlite")]
mod sqlite_permission_operation_repository;
#[cfg(feature = "sqlite")]
mod sqlite_repository;

#[cfg(feature = "postgres-sync")]
pub mod postgres;

#[cfg(feature = "postgres-sync")]
mod postgres_pool;

#[cfg(feature = "postgres-sync")]
mod postgres_execution_repository;
#[cfg(feature = "postgres-sync")]
mod postgres_permission_operation_repository;
#[cfg(feature = "postgres-sync")]
mod postgres_repository;

pub mod memory;

pub use error::{DatabaseError, DatabaseResult};
pub use memory::InMemoryDatabase;
pub use schema::SchemaManager;
pub use traits::{
    AgentDatabase, DatabaseParam, DatabaseRow, EventRepository, MessageRepository,
    PermissionOperationRepository, PermissionRepository, RuntimeExecutionRepository,
    RuntimeMaintenance, RuntimeSessionWrites, SessionRepository, TaskRepository,
};
pub use types::{
    format_runtime_timestamp, ordinary_session_update_conflicts, runtime_now_timestamp,
    session_owner_fields_from_metadata_json, session_provider_ownership_changes,
    session_state_is_terminal, session_state_regresses_from_terminal, task_state_is_terminal,
    task_update_conflicts, ActionKind, ClaimedPermissionOperation, ClaimedRun, EventQuery,
    EventRow, MessageQuery, MessageRow, PermissionOperationRow, PermissionOperationState,
    PermissionPayloadKind, PermissionQuery, PermissionRow, RunControlAction, RunRow, RunState,
    RuntimePurgeCounts, RuntimeSchemaStatus, SessionQuery, SessionRow, StepRow, StepState,
    TaskQuery, TaskRow, CURRENT_SCHEMA_VERSION, RUNTIME_TIMESTAMP_PATTERN,
};

#[cfg(feature = "sqlite")]
pub use sqlite::SqliteDatabase;

#[cfg(feature = "postgres-sync")]
pub use postgres::PostgresDatabase;
