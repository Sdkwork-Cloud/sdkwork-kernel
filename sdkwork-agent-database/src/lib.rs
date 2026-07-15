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
mod sqlite_repository;

#[cfg(feature = "postgres-sync")]
pub mod postgres;

#[cfg(feature = "postgres-sync")]
mod postgres_pool;

#[cfg(feature = "postgres-sync")]
mod postgres_repository;

pub mod memory;

pub use error::{DatabaseError, DatabaseResult};
pub use memory::InMemoryDatabase;
pub use schema::SchemaManager;
pub use traits::{
    AgentDatabase, DatabaseParam, DatabaseRow, EventRepository, MessageRepository,
    PermissionRepository, RuntimeMaintenance, RuntimeSessionWrites, SessionRepository,
    TaskRepository,
};
pub use types::{
    ordinary_session_update_conflicts, session_owner_fields_from_metadata_json,
    session_provider_ownership_changes, session_state_is_terminal,
    session_state_regresses_from_terminal, EventQuery, EventRow, MessageQuery, MessageRow,
    PermissionQuery, PermissionRow, RuntimePurgeCounts, RuntimeSchemaStatus, SessionQuery,
    SessionRow, TaskQuery, TaskRow, CURRENT_SCHEMA_VERSION,
};

#[cfg(feature = "sqlite")]
pub use sqlite::SqliteDatabase;

#[cfg(feature = "postgres-sync")]
pub use postgres::PostgresDatabase;
