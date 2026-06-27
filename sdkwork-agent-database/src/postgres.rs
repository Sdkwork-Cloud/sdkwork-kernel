use crate::error::DatabaseResult;
use crate::postgres_pool::BlockingPostgresPool;
use crate::traits::{AgentDatabase, DatabaseParam, DatabaseRow};
use sqlx::{Column, Row};
use std::collections::HashMap;

/// PostgreSQL-backed agent session persistence.
#[derive(Clone)]
pub struct PostgresDatabase {
    pub(crate) pool: BlockingPostgresPool,
}

const MIGRATION_SQL: &str = r#"
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
    message_count BIGINT DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT,
    metadata_json TEXT
);

ALTER TABLE sessions ADD COLUMN IF NOT EXISTS provider_id TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS bridge_id TEXT;

CREATE INDEX IF NOT EXISTS idx_sessions_updated_at
    ON sessions (COALESCE(updated_at, created_at) DESC);

CREATE TABLE IF NOT EXISTS messages (
    message_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL,
    metadata_json TEXT
);

CREATE INDEX IF NOT EXISTS idx_messages_session_id ON messages(session_id);

CREATE TABLE IF NOT EXISTS tasks (
    task_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    instruction TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'created',
    created_at TEXT NOT NULL,
    updated_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_tasks_session_id ON tasks(session_id);

CREATE TABLE IF NOT EXISTS events (
    event_id TEXT PRIMARY KEY,
    session_id TEXT,
    event_type TEXT NOT NULL,
    severity TEXT NOT NULL,
    payload TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_events_session_id ON events(session_id);

CREATE TABLE IF NOT EXISTS agents (
    agent_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    source TEXT NOT NULL,
    config_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT
);
"#;

impl PostgresDatabase {
    pub fn connect_migrated(connection_uri: &str) -> DatabaseResult<Self> {
        let pool = BlockingPostgresPool::connect(connection_uri)?;
        let db = Self { pool };
        db.migrate()?;
        Ok(db)
    }

    pub fn connect_from_sdkwork_env(service_name: &str) -> DatabaseResult<Self> {
        let pool = BlockingPostgresPool::connect_from_sdkwork_env(service_name)?;
        let db = Self { pool };
        db.migrate()?;
        Ok(db)
    }

    pub fn migrate(&self) -> DatabaseResult<()> {
        self.pool.execute_batch_sql(MIGRATION_SQL)
    }

    pub fn health(&self) -> DatabaseResult<bool> {
        let pool = self.pool.pool().clone();
        let row: (i32,) = self
            .pool
            .run(async move { sqlx::query_as("SELECT 1").fetch_one(&pool).await })?;
        Ok(row.0 == 1)
    }
}

struct PostgresRow {
    values: HashMap<String, String>,
}

impl DatabaseRow for PostgresRow {
    fn get_string(&self, column: &str) -> DatabaseResult<String> {
        self.values.get(column).cloned().ok_or_else(|| {
            crate::error::DatabaseError::Query(format!("column not found: {column}"))
        })
    }

    fn get_optional_string(&self, column: &str) -> DatabaseResult<Option<String>> {
        Ok(self.values.get(column).cloned())
    }

    fn get_i64(&self, column: &str) -> DatabaseResult<i64> {
        let value = self.get_string(column)?;
        value
            .parse::<i64>()
            .map_err(|error| crate::error::DatabaseError::Query(format!("invalid i64: {error}")))
    }
}

impl AgentDatabase for PostgresDatabase {
    fn execute(&self, sql: &str, params: &[&dyn DatabaseParam]) -> DatabaseResult<usize> {
        let pool = self.pool.pool().clone();
        let sql = sql.to_owned();
        let bindings: Vec<String> = params.iter().map(|param| param.as_sql_value()).collect();
        self.pool.run_db(async move {
            let mut query = sqlx::query(&sql);
            for value in &bindings {
                query = query.bind(value);
            }
            let result = query.execute(&pool).await?;
            Ok(result.rows_affected() as usize)
        })
    }

    fn query_many(
        &self,
        sql: &str,
        params: &[&dyn DatabaseParam],
    ) -> DatabaseResult<Vec<Box<dyn DatabaseRow>>> {
        let pool = self.pool.pool().clone();
        let sql = sql.to_owned();
        let bindings: Vec<String> = params.iter().map(|param| param.as_sql_value()).collect();
        self.pool.run_db(async move {
            let mut query = sqlx::query(&sql);
            for value in &bindings {
                query = query.bind(value);
            }
            let rows = query.fetch_all(&pool).await?;
            let mut result = Vec::new();
            for row in rows {
                let mut values = HashMap::new();
                for (index, column) in row.columns().iter().enumerate() {
                    let name = column.name().to_string();
                    let value: Option<String> = row.try_get(index).ok();
                    if let Some(value) = value {
                        values.insert(name, value);
                    }
                }
                result.push(Box::new(PostgresRow { values }) as Box<dyn DatabaseRow>);
            }
            Ok(result)
        })
    }

    fn health(&self) -> DatabaseResult<bool> {
        PostgresDatabase::health(self)
    }
}
