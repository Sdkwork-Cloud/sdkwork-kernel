mod error;
mod schema;
mod traits;
mod types;

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "sqlite")]
mod sqlite_repository;

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
