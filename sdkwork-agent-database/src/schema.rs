use crate::error::DatabaseResult;
use crate::traits::AgentDatabase;

/// Schema manager for database migrations
pub struct SchemaManager {
    db: Box<dyn AgentDatabase>,
}

impl SchemaManager {
    pub fn new(db: Box<dyn AgentDatabase>) -> Self {
        Self { db }
    }

    /// Run all migrations
    pub fn migrate(&self) -> DatabaseResult<()> {
        self.create_sessions_table()?;
        self.create_messages_table()?;
        self.create_tasks_table()?;
        self.create_events_table()?;
        self.create_agents_table()?;
        Ok(())
    }

    /// Create sessions table
    fn create_sessions_table(&self) -> DatabaseResult<()> {
        let sql = "
            CREATE TABLE IF NOT EXISTS sessions (
                session_id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                kind TEXT NOT NULL DEFAULT 'main',
                source TEXT NOT NULL DEFAULT 'api',
                state TEXT NOT NULL DEFAULT 'created',
                title TEXT,
                model TEXT,
                cwd TEXT,
                provider_id TEXT,
                bridge_id TEXT,
                token_usage_json TEXT,
                message_count INTEGER DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT,
                metadata_json TEXT
            )
        ";
        self.db.execute(sql, &[])?;
        self.ensure_sessions_provider_columns()?;
        self.db.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_updated_at ON sessions(COALESCE(updated_at, created_at) DESC)",
            &[],
        )?;
        Ok(())
    }

    fn ensure_sessions_provider_columns(&self) -> DatabaseResult<()> {
        let _ = self.db.execute("ALTER TABLE sessions ADD COLUMN provider_id TEXT", &[]);
        let _ = self.db.execute("ALTER TABLE sessions ADD COLUMN bridge_id TEXT", &[]);
        Ok(())
    }

    /// Create messages table
    fn create_messages_table(&self) -> DatabaseResult<()> {
        let sql = "
            CREATE TABLE IF NOT EXISTS messages (
                message_id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                metadata_json TEXT
            )
        ";
        self.db.execute(sql, &[])?;

        // Create index
        self.db.execute(
            "CREATE INDEX IF NOT EXISTS idx_messages_session_id ON messages(session_id)",
            &[],
        )?;
        Ok(())
    }

    /// Create tasks table
    fn create_tasks_table(&self) -> DatabaseResult<()> {
        let sql = "
            CREATE TABLE IF NOT EXISTS tasks (
                task_id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                instruction TEXT NOT NULL,
                state TEXT NOT NULL DEFAULT 'created',
                created_at TEXT NOT NULL,
                updated_at TEXT
            )
        ";
        self.db.execute(sql, &[])?;

        // Create index
        self.db.execute(
            "CREATE INDEX IF NOT EXISTS idx_tasks_session_id ON tasks(session_id)",
            &[],
        )?;
        Ok(())
    }

    /// Create events table
    fn create_events_table(&self) -> DatabaseResult<()> {
        let sql = "
            CREATE TABLE IF NOT EXISTS events (
                event_id TEXT PRIMARY KEY,
                session_id TEXT,
                event_type TEXT NOT NULL,
                severity TEXT NOT NULL,
                payload TEXT,
                created_at TEXT NOT NULL
            )
        ";
        self.db.execute(sql, &[])?;

        // Create index
        self.db.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_session_id ON events(session_id)",
            &[],
        )?;
        Ok(())
    }

    /// Create agents table
    fn create_agents_table(&self) -> DatabaseResult<()> {
        let sql = "
            CREATE TABLE IF NOT EXISTS agents (
                agent_id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                kind TEXT NOT NULL,
                source TEXT NOT NULL,
                config_json TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT
            )
        ";
        self.db.execute(sql, &[])?;
        Ok(())
    }
}
