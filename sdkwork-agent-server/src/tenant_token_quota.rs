//! Per-tenant daily model token quotas for commercial usage enforcement.

use std::collections::HashMap;
use std::sync::Mutex;

use axum::http::StatusCode;
use chrono::Utc;
use redis::aio::ConnectionManager;
use tracing::warn;

use crate::config::ServerConfig;

const MAX_QUOTA_COUNTERS: usize = 4096;

enum QuotaBackend {
    Memory {
        counters: Mutex<HashMap<String, u64>>,
    },
    Redis {
        connection: ConnectionManager,
    },
}

/// Tracks and enforces per-tenant daily token consumption limits.
pub struct TenantTokenQuotaState {
    overrides: HashMap<String, u64>,
    backend: QuotaBackend,
}

impl TenantTokenQuotaState {
    pub fn from_config(config: &ServerConfig) -> Self {
        let overrides = config
            .tenant_token_quota_overrides
            .iter()
            .map(|(tenant_id, override_quota)| (tenant_id.clone(), override_quota.daily_tokens))
            .collect();

        if let Some(redis_url) = config.effective_rate_limit_redis_url() {
            match Self::connect_redis(redis_url) {
                Ok(connection) => {
                    return Self {
                        overrides,
                        backend: QuotaBackend::Redis { connection },
                    };
                }
                Err(error) => {
                    if config.requires_distributed_rate_limit() {
                        panic!(
                            "failed to connect tenant token quota redis at {redis_url}: {error}; production cloud deployments require redis_cache"
                        );
                    }
                    warn!(
                        redis_url = redis_url,
                        error = %error,
                        "tenant token quota redis unavailable; falling back to in-process counters"
                    );
                }
            }
        }

        Self {
            overrides,
            backend: QuotaBackend::Memory {
                counters: Mutex::new(HashMap::new()),
            },
        }
    }

    fn connect_redis(redis_url: &str) -> Result<ConnectionManager, redis::RedisError> {
        let runtime = tokio::runtime::Handle::current();
        runtime.block_on(async {
            let client = redis::Client::open(redis_url)?;
            ConnectionManager::new(client).await
        })
    }

    pub fn is_enabled(&self) -> bool {
        !self.overrides.is_empty()
    }

    pub fn uses_redis(&self) -> bool {
        matches!(self.backend, QuotaBackend::Redis { .. })
    }

    pub fn quota_for_tenant(&self, tenant_id: &str) -> Option<u64> {
        self.overrides.get(tenant_id).copied()
    }

    pub async fn check_allowed(&self, tenant_id: &str) -> Result<(), StatusCode> {
        let Some(limit) = self.quota_for_tenant(tenant_id) else {
            return Ok(());
        };
        let current = self.current_usage(tenant_id).await;
        if current >= limit {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
        Ok(())
    }

    pub async fn record_usage(&self, tenant_id: &str, tokens: u64) {
        if tokens == 0 || !self.overrides.contains_key(tenant_id) {
            return;
        }
        match &self.backend {
            QuotaBackend::Memory { counters } => {
                self.record_usage_memory(counters, tenant_id, tokens);
            }
            QuotaBackend::Redis { connection } => {
                self.record_usage_redis(connection.clone(), tenant_id, tokens)
                    .await;
            }
        }
    }

    async fn current_usage(&self, tenant_id: &str) -> u64 {
        match &self.backend {
            QuotaBackend::Memory { counters } => self
                .current_usage_memory(counters, tenant_id),
            QuotaBackend::Redis { connection } => {
                self.current_usage_redis(connection.clone(), tenant_id)
                    .await
            }
        }
    }

    fn counter_key(tenant_id: &str) -> String {
        format!("{}:{}", tenant_id, Utc::now().format("%Y-%m-%d"))
    }

    fn redis_key(tenant_id: &str) -> String {
        format!(
            "sdkwork:tenant_token_quota:{}:{}",
            tenant_id,
            Utc::now().format("%Y-%m-%d")
        )
    }

    fn current_usage_memory(&self, counters: &Mutex<HashMap<String, u64>>, tenant_id: &str) -> u64 {
        let key = Self::counter_key(tenant_id);
        counters
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get(&key)
            .copied()
            .unwrap_or(0)
    }

    fn record_usage_memory(
        &self,
        counters: &Mutex<HashMap<String, u64>>,
        tenant_id: &str,
        tokens: u64,
    ) {
        let key = Self::counter_key(tenant_id);
        let mut counters = counters.lock().unwrap_or_else(|error| error.into_inner());
        if !counters.contains_key(&key) && counters.len() >= MAX_QUOTA_COUNTERS {
            if let Some(oldest_key) = counters.keys().next().cloned() {
                counters.remove(&oldest_key);
            }
        }
        *counters.entry(key).or_insert(0) += tokens;
    }

    async fn current_usage_redis(
        &self,
        mut connection: ConnectionManager,
        tenant_id: &str,
    ) -> u64 {
        let key = Self::redis_key(tenant_id);
        match redis::cmd("GET")
            .arg(&key)
            .query_async::<Option<u64>>(&mut connection)
            .await
        {
            Ok(value) => value.unwrap_or(0),
            Err(error) => {
                warn!(tenant_id = tenant_id, error = %error, "tenant token quota redis read failed");
                0
            }
        }
    }

    async fn record_usage_redis(
        &self,
        mut connection: ConnectionManager,
        tenant_id: &str,
        tokens: u64,
    ) {
        let key = Self::redis_key(tenant_id);
        let result: Result<(), redis::RedisError> = redis::pipe()
            .atomic()
            .incr(&key, tokens)
            .expire(&key, 172_800)
            .query_async(&mut connection)
            .await;
        if let Err(error) = result {
            warn!(
                tenant_id = tenant_id,
                error = %error,
                "tenant token quota redis increment failed"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn quota_config(tenant: &str, daily_tokens: u64) -> ServerConfig {
        use crate::config::TenantTokenQuotaOverride;
        let mut overrides = HashMap::new();
        overrides.insert(
            tenant.to_string(),
            TenantTokenQuotaOverride { daily_tokens },
        );
        ServerConfig {
            tenant_token_quota_overrides: overrides,
            ..ServerConfig::default()
        }
    }

    #[tokio::test]
    async fn rejects_when_daily_quota_exhausted() {
        let state = TenantTokenQuotaState::from_config(&quota_config("100001", 100));
        state.record_usage("100001", 100).await;
        assert_eq!(
            state.check_allowed("100001").await,
            Err(StatusCode::TOO_MANY_REQUESTS)
        );
    }

    #[tokio::test]
    async fn tenants_without_overrides_remain_unlimited() {
        let state = TenantTokenQuotaState::from_config(&quota_config("100001", 50));
        assert!(state.check_allowed("100002").await.is_ok());
        state.record_usage("100002", 10_000).await;
        assert!(state.check_allowed("100002").await.is_ok());
    }
}
