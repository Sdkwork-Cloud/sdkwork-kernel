//! Per-tenant daily model token quotas for commercial usage enforcement.
//!
//! Uses an atomic **reserve-and-adjust** pattern to eliminate the
//! Time-Of-Check-To-Time-Of-Use (TOCTOU) race that existed when
//! `check_allowed` and `record_usage` were separate operations.
//!
//! Fail-closed design: when the Redis backend encounters a read error and the
//! tenant has a configured quota, the quota check returns a 503 (Service
//! Unavailable) rather than silently allowing unlimited consumption. This
//! prevents billing abuse during transient Redis outages. Tenants without an
//! explicit quota override remain unlimited regardless of backend state.

use std::collections::HashMap;
use std::sync::Mutex;

use axum::http::StatusCode;
use chrono::Utc;
use redis::aio::ConnectionManager;
use tracing::warn;

use crate::config::ServerConfig;

const MAX_QUOTA_COUNTERS: usize = 4096;

/// Default token estimate reserved before a model invocation completes.
/// The actual usage is adjusted after the model returns.
const DEFAULT_RESERVE_TOKENS: u64 = 4096;

const REDIS_RESERVE_SCRIPT: &str = r#"
local key = KEYS[1]
local limit = tonumber(ARGV[1])
local reserve = tonumber(ARGV[2])
local ttl = tonumber(ARGV[3])
local current = tonumber(redis.call('GET', key) or '0')
if current + reserve > limit then
  return 0
end
redis.call('INCRBY', key, reserve)
redis.call('EXPIRE', key, ttl)
return 1
"#;

const REDIS_ADJUST_SCRIPT: &str = r#"
local key = KEYS[1]
local reserved = tonumber(ARGV[1])
local actual = tonumber(ARGV[2])
local diff = actual - reserved
if diff ~= 0 then
  redis.call('INCRBY', key, diff)
end
return 1
"#;

enum QuotaBackend {
    Memory {
        counters: Mutex<HashMap<String, u64>>,
    },
    Redis {
        connection: ConnectionManager,
        reserve_script: redis::Script,
        adjust_script: redis::Script,
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
                        backend: QuotaBackend::Redis {
                            connection,
                            reserve_script: redis::Script::new(REDIS_RESERVE_SCRIPT),
                            adjust_script: redis::Script::new(REDIS_ADJUST_SCRIPT),
                        },
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

    /// Atomically reserve token budget for a model invocation.
    ///
    /// This replaces the separate `check_allowed` + `record_usage` flow that
    /// had a TOCTOU race: two concurrent requests could both pass the check
    /// before either recorded usage, allowing both to exceed the quota.
    ///
    /// Returns `Ok(())` when the reservation succeeds. The caller MUST call
    /// `adjust_usage` after the model returns to reconcile the reserved
    /// estimate with the actual token consumption.
    ///
    /// Returns `Err(StatusCode::TOO_MANY_REQUESTS)` when the quota is
    /// exhausted. Returns `Err(StatusCode::SERVICE_UNAVAILABLE)` when the
    /// Redis backend is temporarily unable to verify usage (fail-closed).
    pub async fn try_consume(&self, tenant_id: &str) -> Result<(), StatusCode> {
        let Some(limit) = self.quota_for_tenant(tenant_id) else {
            return Ok(());
        };
        // A quota of 0 means the tenant is blocked from all model invocations.
        if limit == 0 {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
        let reserve = DEFAULT_RESERVE_TOKENS.min(limit);
        match &self.backend {
            QuotaBackend::Memory { counters } => {
                self.try_consume_memory(counters, tenant_id, limit, reserve)
            }
            QuotaBackend::Redis {
                connection,
                reserve_script,
                ..
            } => {
                self.try_consume_redis(
                    connection.clone(),
                    reserve_script,
                    tenant_id,
                    limit,
                    reserve,
                )
                .await
            }
        }
    }

    /// Adjust the reserved token count to the actual usage after the model
    /// invocation completes.
    pub async fn adjust_usage(&self, tenant_id: &str, actual_tokens: u64) {
        if !self.overrides.contains_key(tenant_id) {
            return;
        }
        let reserved = DEFAULT_RESERVE_TOKENS;
        match &self.backend {
            QuotaBackend::Memory { counters } => {
                self.adjust_usage_memory(counters, tenant_id, reserved, actual_tokens);
            }
            QuotaBackend::Redis {
                connection,
                adjust_script,
                ..
            } => {
                self.adjust_usage_redis(
                    connection.clone(),
                    adjust_script,
                    tenant_id,
                    reserved,
                    actual_tokens,
                )
                .await;
            }
        }
    }

    /// Check whether the tenant is within its daily token quota.
    ///
    /// **Deprecated**: prefer `try_consume` for atomic reservation. This
    /// method remains for backward compatibility but does not reserve tokens.
    pub async fn check_allowed(&self, tenant_id: &str) -> Result<(), StatusCode> {
        let Some(limit) = self.quota_for_tenant(tenant_id) else {
            return Ok(());
        };
        let current = self.current_usage(tenant_id).await?;
        if current >= limit {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
        Ok(())
    }

    /// Record token usage after the fact.
    ///
    /// **Deprecated**: prefer `try_consume` + `adjust_usage` for atomic
    /// reservation. This method remains for backward compatibility.
    pub async fn record_usage(&self, tenant_id: &str, tokens: u64) {
        if tokens == 0 || !self.overrides.contains_key(tenant_id) {
            return;
        }
        match &self.backend {
            QuotaBackend::Memory { counters } => {
                self.record_usage_memory(counters, tenant_id, tokens);
            }
            QuotaBackend::Redis { connection, .. } => {
                self.record_usage_redis(connection.clone(), tenant_id, tokens)
                    .await;
            }
        }
    }

    /// Returns `Ok(usage)` on success, or `Err(503)` when the Redis backend
    /// cannot verify usage for a tenant that has a configured quota.
    async fn current_usage(&self, tenant_id: &str) -> Result<u64, StatusCode> {
        match &self.backend {
            QuotaBackend::Memory { counters } => Ok(self.current_usage_memory(counters, tenant_id)),
            QuotaBackend::Redis { connection, .. } => {
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

    fn try_consume_memory(
        &self,
        counters: &Mutex<HashMap<String, u64>>,
        tenant_id: &str,
        limit: u64,
        reserve: u64,
    ) -> Result<(), StatusCode> {
        let key = Self::counter_key(tenant_id);
        let mut counters = counters.lock().unwrap_or_else(|error| error.into_inner());
        if !counters.contains_key(&key) && counters.len() >= MAX_QUOTA_COUNTERS {
            if let Some(oldest_key) = counters.keys().next().cloned() {
                counters.remove(&oldest_key);
            }
        }
        let current = counters.entry(key).or_insert(0);
        if *current + reserve > limit {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
        *current += reserve;
        Ok(())
    }

    fn adjust_usage_memory(
        &self,
        counters: &Mutex<HashMap<String, u64>>,
        tenant_id: &str,
        reserved: u64,
        actual: u64,
    ) {
        let key = Self::counter_key(tenant_id);
        let mut counters = counters.lock().unwrap_or_else(|error| error.into_inner());
        let current = counters.entry(key).or_insert(0);
        // Reconcile: current already includes `reserved`; adjust by delta.
        *current = current.saturating_sub(reserved).saturating_add(actual);
    }

    async fn try_consume_redis(
        &self,
        mut connection: ConnectionManager,
        script: &redis::Script,
        tenant_id: &str,
        limit: u64,
        reserve: u64,
    ) -> Result<(), StatusCode> {
        let key = Self::redis_key(tenant_id);
        let result: redis::RedisResult<i32> = script
            .key(&key)
            .arg(limit)
            .arg(reserve)
            .arg(172_800_i64)
            .invoke_async(&mut connection)
            .await;
        match result {
            Ok(1) => Ok(()),
            Ok(0) => Err(StatusCode::TOO_MANY_REQUESTS),
            Ok(_) => Err(StatusCode::TOO_MANY_REQUESTS),
            Err(error) => {
                warn!(tenant_id = tenant_id, error = %error, "tenant token quota redis reserve failed; failing closed to prevent billing abuse");
                Err(StatusCode::SERVICE_UNAVAILABLE)
            }
        }
    }

    async fn adjust_usage_redis(
        &self,
        mut connection: ConnectionManager,
        script: &redis::Script,
        tenant_id: &str,
        reserved: u64,
        actual: u64,
    ) {
        let key = Self::redis_key(tenant_id);
        let result: Result<(), redis::RedisError> = script
            .key(&key)
            .arg(reserved)
            .arg(actual)
            .invoke_async(&mut connection)
            .await;
        if let Err(error) = result {
            warn!(
                tenant_id = tenant_id,
                error = %error,
                "tenant token quota redis adjust failed"
            );
        }
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
    ) -> Result<u64, StatusCode> {
        let key = Self::redis_key(tenant_id);
        match redis::cmd("GET")
            .arg(&key)
            .query_async::<Option<u64>>(&mut connection)
            .await
        {
            Ok(value) => Ok(value.unwrap_or(0)),
            Err(error) => {
                warn!(tenant_id = tenant_id, error = %error, "tenant token quota redis read failed; failing closed to prevent billing abuse");
                Err(StatusCode::SERVICE_UNAVAILABLE)
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

    #[tokio::test]
    async fn try_consume_atomically_reserves_and_rejects() {
        let state = TenantTokenQuotaState::from_config(&quota_config("100001", 100));
        // First reservation should succeed.
        assert!(state.try_consume("100001").await.is_ok());
        // Exhaust the remaining quota.
        state.record_usage("100001", 100).await;
        // Subsequent reservation should fail atomically.
        assert_eq!(
            state.try_consume("100001").await,
            Err(StatusCode::TOO_MANY_REQUESTS)
        );
    }

    #[tokio::test]
    async fn adjust_usage_reconciles_reserved_estimate() {
        let state = TenantTokenQuotaState::from_config(&quota_config("100001", 10_000));
        // Reserve tokens.
        state.try_consume("100001").await.expect("reserve");
        // Actual usage was less than reserved.
        state.adjust_usage("100001", 100).await;
        // Verify the counter reflects actual usage, not the reserved estimate.
        assert_eq!(state.current_usage("100001").await.expect("usage"), 100);
    }
}
