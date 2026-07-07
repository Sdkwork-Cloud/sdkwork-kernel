use crate::error::DatabaseResult;
use crate::schema_migrations::migrate_sqlite;
use crate::traits::AgentDatabase;

/// Schema manager for SQLite database migrations.
pub struct SchemaManager {
    db: Box<dyn AgentDatabase>,
}

impl SchemaManager {
    pub fn new(db: Box<dyn AgentDatabase>) -> Self {
        Self { db }
    }

    /// Run all migrations from the canonical SQLite migration file.
    pub fn migrate(&self) -> DatabaseResult<()> {
        migrate_sqlite(self.db.as_ref())
    }
}
