use crate::error::DatabaseResult;
use crate::postgres_pool::BlockingPostgresPool;
use crate::schema_migrations::apply_postgres_pool;
use crate::traits::{AgentDatabase, DatabaseParam, DatabaseRow};
use sqlx::{Column, Row};
use std::collections::HashMap;

/// PostgreSQL-backed agent session persistence.
#[derive(Clone)]
pub struct PostgresDatabase {
    pub(crate) pool: BlockingPostgresPool,
}

impl PostgresDatabase {
    pub async fn connect_migrated_async(connection_uri: &str) -> DatabaseResult<Self> {
        let pool = BlockingPostgresPool::connect_async(connection_uri).await?;
        let db = Self { pool };
        db.migrate_async().await?;
        Ok(db)
    }

    pub fn connect_migrated(connection_uri: &str) -> DatabaseResult<Self> {
        let pool = BlockingPostgresPool::connect(connection_uri)?;
        let db = Self { pool };
        db.migrate()?;
        Ok(db)
    }

    pub async fn connect_from_sdkwork_env_async(service_name: &str) -> DatabaseResult<Self> {
        let pool = BlockingPostgresPool::connect_from_sdkwork_env_async(service_name).await?;
        let db = Self { pool };
        db.migrate_async().await?;
        Ok(db)
    }

    pub fn connect_from_sdkwork_env(service_name: &str) -> DatabaseResult<Self> {
        let pool = BlockingPostgresPool::connect_from_sdkwork_env(service_name)?;
        let db = Self { pool };
        db.migrate()?;
        Ok(db)
    }

    pub fn migrate(&self) -> DatabaseResult<()> {
        let pool = self.pool.pool().clone();
        self.pool
            .run_db(async move { apply_postgres_pool(&pool).await })
    }

    pub async fn migrate_async(&self) -> DatabaseResult<()> {
        apply_postgres_pool(self.pool.pool()).await
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
            let mut query = sqlx::query(sqlx::AssertSqlSafe(sql.clone()));
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
            let mut query = sqlx::query(sqlx::AssertSqlSafe(sql.clone()));
            for value in &bindings {
                query = query.bind(value);
            }
            let rows = query.fetch_all(&pool).await?;
            let mut result = Vec::new();
            for row in rows {
                let mut values = HashMap::new();
                for (index, column) in row.columns().iter().enumerate() {
                    let name = column.name().to_string();
                    let value: Option<String> = row.try_get(index)?;
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

    fn migrate_schema(&self) -> DatabaseResult<()> {
        self.migrate()
    }
}
