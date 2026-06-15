mod error;
mod traits;
mod types;
mod schema;

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "postgres")]
pub mod postgres;

pub mod memory;

pub use error::{DatabaseError, DatabaseResult};
pub use traits::{AgentDatabase, SessionRepository, MessageRepository, TaskRepository, EventRepository, DatabaseParam, DatabaseRow};
pub use types::{SessionRow, MessageRow, TaskRow, EventRow, AgentRow, SessionQuery, MessageQuery, EventQuery};
pub use schema::SchemaManager;
pub use memory::InMemoryDatabase;
