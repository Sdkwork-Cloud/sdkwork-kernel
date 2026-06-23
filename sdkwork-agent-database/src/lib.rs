mod error;
mod schema;
mod traits;
mod types;

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
    SessionRepository, TaskRepository,
};
pub use types::{
    AgentRow, EventQuery, EventRow, MessageQuery, MessageRow, SessionQuery, SessionRow, TaskRow,
};

#[cfg(feature = "sqlite")]
pub use sqlite::SqliteDatabase;

#[cfg(feature = "postgres-sync")]
pub use postgres::PostgresDatabase;
