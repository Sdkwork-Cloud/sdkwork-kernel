//! Blocking facade over `sdkwork-database-sqlx` PostgreSQL pools for sync repositories.

use sdkwork_database_config::{DatabaseConfig, DatabaseEngine};
use sdkwork_database_sqlx::{create_pool_from_config, DatabasePool, PoolError};
use sdkwork_utils_rust::is_blank;
use sqlx::PgPool;
use std::future::Future;
use std::sync::Arc;
use tokio::runtime::Runtime;

use crate::error::{DatabaseError, DatabaseResult};

pub fn map_sqlx_error(error: sqlx::Error) -> DatabaseError {
    DatabaseError::Query(error.to_string())
}

pub fn map_pool_error(error: PoolError) -> DatabaseError {
    DatabaseError::Connection(error.to_string())
}

pub fn map_database_config_error(error: sdkwork_database_config::ConfigError) -> DatabaseError {
    DatabaseError::Connection(error.to_string())
}

#[derive(Debug, Clone)]
pub struct BlockingPostgresPool {
    pool: PgPool,
    runtime: Arc<Runtime>,
    #[allow(dead_code)]
    database_pool: DatabasePool,
}

impl BlockingPostgresPool {
    pub fn from_database_pool(
        database_pool: DatabasePool,
        runtime: Arc<Runtime>,
    ) -> Result<Self, PoolError> {
        let pool = database_pool.as_postgres().cloned().ok_or_else(|| {
            PoolError::DatabaseConfig("expected postgres database pool".to_owned())
        })?;
        Ok(Self {
            pool,
            runtime,
            database_pool,
        })
    }

    pub fn connect_from_config(config: DatabaseConfig) -> DatabaseResult<Self> {
        let runtime = Arc::new(Runtime::new().map_err(|error| {
            DatabaseError::Connection(format!("tokio runtime: {error}"))
        })?);
        let database_pool = runtime
            .block_on(create_pool_from_config(config))
            .map_err(map_pool_error)?;
        Self::from_database_pool(database_pool, runtime).map_err(map_pool_error)
    }

    pub fn connect(connection_uri: &str) -> DatabaseResult<Self> {
        let engine = DatabaseEngine::from_url(connection_uri).ok_or_else(|| {
            DatabaseError::Connection(format!(
                "unsupported postgres connection url: {connection_uri}"
            ))
        })?;
        if engine != DatabaseEngine::Postgres {
            return Err(DatabaseError::Connection(format!(
                "expected postgres engine for url: {connection_uri}"
            )));
        }
        Self::connect_from_config(DatabaseConfig {
            engine,
            url: connection_uri.to_owned(),
            ..DatabaseConfig::default()
        })
    }

    pub fn connect_from_sdkwork_env(service_name: &str) -> DatabaseResult<Self> {
        let legacy_uri_key = format!("SDKWORK_{}_POSTGRES_URI", service_name.to_uppercase());
        if let Ok(uri) = std::env::var(&legacy_uri_key) {
            let trimmed = uri.trim();
            if !is_blank(Some(trimmed)) {
                return Self::connect(trimmed);
            }
        }

        let config =
            DatabaseConfig::from_env(service_name).map_err(map_database_config_error)?;
        match config.engine {
            DatabaseEngine::Postgres => Self::connect_from_config(config),
            other => Err(DatabaseError::Connection(format!(
                "service {service_name} resolved database engine {other:?}, expected Postgres"
            ))),
        }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    #[allow(dead_code)]
    pub fn database_pool(&self) -> &DatabasePool {
        &self.database_pool
    }

    #[allow(dead_code)]
    pub fn block_on<F, T>(&self, future: F) -> T
    where
        F: Future<Output = T>,
    {
        self.runtime.block_on(future)
    }

    pub fn run_db<F, T>(&self, future: F) -> DatabaseResult<T>
    where
        F: Future<Output = DatabaseResult<T>>,
    {
        self.runtime.block_on(future)
    }

    pub fn run<F, T>(&self, future: F) -> DatabaseResult<T>
    where
        F: Future<Output = Result<T, sqlx::Error>>,
    {
        self.runtime
            .block_on(future)
            .map_err(map_sqlx_error)
    }

    pub fn execute_batch_sql(&self, sql: &str) -> DatabaseResult<()> {
        let pool = self.pool.clone();
        let sql = sql.to_owned();
        self.run(async move { sqlx::raw_sql(&sql).execute(&pool).await.map(|_| ()) })
    }
}
