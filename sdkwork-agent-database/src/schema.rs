use crate::error::DatabaseResult;
use crate::traits::AgentDatabase;

/// Schema manager for SQLite database migrations.
pub struct SchemaManager {
    db: Box<dyn AgentDatabase>,
}

impl SchemaManager {
    pub fn new(db: Box<dyn AgentDatabase>) -> Self {
        Self { db }
    }

    /// Run the backend's versioned migration authority.
    pub fn migrate(&self) -> DatabaseResult<()> {
        self.db.migrate_schema()
    }
}
