//! Blocking facade over `sdkwork-database-sqlx` PostgreSQL pools for sync repositories.

use sdkwork_database_config::{DatabaseConfig, DatabaseEngine};
use sdkwork_database_sqlx::{create_pool_from_config, DatabasePool, PoolError};
use sdkwork_utils_rust::is_blank;
use sqlx::PgPool;
use std::future::Future;
use std::sync::Arc;
use std::thread;
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

    pub async fn connect_from_config_async(config: DatabaseConfig) -> DatabaseResult<Self> {
        let database_pool = create_pool_from_config(config)
            .await
            .map_err(map_pool_error)?;
        let runtime = build_runtime()?;
        Self::from_database_pool(database_pool, runtime).map_err(map_pool_error)
    }

    pub fn connect_from_config(config: DatabaseConfig) -> DatabaseResult<Self> {
        if tokio::runtime::Handle::try_current().is_ok() {
            return thread::spawn(move || Self::connect_from_config_on_current_thread(config))
                .join()
                .map_err(|_| {
                    DatabaseError::Connection("postgres pool initialization worker panicked".into())
                })?;
        }
        Self::connect_from_config_on_current_thread(config)
    }

    fn connect_from_config_on_current_thread(config: DatabaseConfig) -> DatabaseResult<Self> {
        let runtime = build_runtime()?;
        let database_pool = runtime
            .block_on(create_pool_from_config(config))
            .map_err(map_pool_error)?;
        Self::from_database_pool(database_pool, runtime).map_err(map_pool_error)
    }

    fn config_from_connection_uri(connection_uri: &str) -> DatabaseResult<DatabaseConfig> {
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
        Ok(DatabaseConfig {
            engine,
            url: connection_uri.to_owned(),
            ..DatabaseConfig::default()
        })
    }

    pub async fn connect_async(connection_uri: &str) -> DatabaseResult<Self> {
        Self::connect_from_config_async(Self::config_from_connection_uri(connection_uri)?).await
    }

    pub fn connect(connection_uri: &str) -> DatabaseResult<Self> {
        Self::connect_from_config(Self::config_from_connection_uri(connection_uri)?)
    }

    pub async fn connect_from_sdkwork_env_async(service_name: &str) -> DatabaseResult<Self> {
        let legacy_uri_key = format!("SDKWORK_{}_POSTGRES_URI", service_name.to_uppercase());
        if let Ok(uri) = std::env::var(&legacy_uri_key) {
            let trimmed = uri.trim();
            if !is_blank(Some(trimmed)) {
                return Self::connect_async(trimmed).await;
            }
        }

        let config = DatabaseConfig::from_env(service_name).map_err(map_database_config_error)?;
        match config.engine {
            DatabaseEngine::Postgres => Self::connect_from_config_async(config).await,
            other => Err(DatabaseError::Connection(format!(
                "service {service_name} resolved database engine {other:?}, expected Postgres"
            ))),
        }
    }

    pub fn connect_from_sdkwork_env(service_name: &str) -> DatabaseResult<Self> {
        let legacy_uri_key = format!("SDKWORK_{}_POSTGRES_URI", service_name.to_uppercase());
        if let Ok(uri) = std::env::var(&legacy_uri_key) {
            let trimmed = uri.trim();
            if !is_blank(Some(trimmed)) {
                return Self::connect(trimmed);
            }
        }

        let config = DatabaseConfig::from_env(service_name).map_err(map_database_config_error)?;
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
        self.runtime.block_on(future).map_err(map_sqlx_error)
    }
}

fn build_runtime() -> DatabaseResult<Arc<Runtime>> {
    Runtime::new()
        .map(Arc::new)
        .map_err(|error| DatabaseError::Connection(format!("tokio runtime: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn connect_inside_existing_tokio_runtime_returns_error_without_nested_runtime_panic() {
        let result = BlockingPostgresPool::connect(
            "postgres://sdkwork:sdkwork@127.0.0.1:1/sdkwork_agent_runtime",
        );

        assert!(
            result.is_err(),
            "unreachable local postgres should return a connection error"
        );
        let message = result.err().expect("connection error").to_string();
        assert!(
            !message.contains("Cannot start a runtime from within a runtime"),
            "postgres startup must not create a nested Tokio runtime panic: {message}"
        );
    }
}
