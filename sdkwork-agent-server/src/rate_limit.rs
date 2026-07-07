//! Distributed token-bucket rate limiting with optional Redis backing.
//!
//! In-process buckets and Redis fail-over paths delegate to the kernel
//! [`TokenBucketRateLimitProvider`] SPI. Redis remains server-owned for async
//! distributed enforcement across replicas.
//!
//! Fail-closed design: when the Redis backend encounters an error, the limiter
//! falls back to the kernel token-bucket provider rather than allowing the
//! request unconditionally.

use redis::aio::ConnectionManager;
use redis::Script;
use sdkwork_agent_kernel::TokenBucketRateLimitProvider;
use std::collections::HashMap;
use std::sync::Mutex;
use std::thread;
use tracing::warn;

use crate::config::ServerConfig;

const REDIS_TOKEN_BUCKET_SCRIPT: &str = r#"
local key = KEYS[1]
local rps = tonumber(ARGV[1])
local burst = tonumber(ARGV[2])
local now = tonumber(ARGV[3])
local data = redis.call('HMGET', key, 'tokens', 'last')
local tokens = tonumber(data[1])
local last = tonumber(data[2])
if tokens == nil then
  tokens = burst
end
if last == nil then
  last = now
end
local elapsed = math.max(0, now - last)
tokens = math.min(burst, tokens + elapsed * rps)
if tokens < 1 then
  redis.call('HMSET', key, 'tokens', tokens, 'last', now)
  redis.call('EXPIRE', key, 3600)
  return 0
end
tokens = tokens - 1
redis.call('HMSET', key, 'tokens', tokens, 'last', now)
redis.call('EXPIRE', key, 3600)
return 1
"#;

enum RateLimitBackend {
    Memory {
        provider: Mutex<TokenBucketRateLimitProvider>,
    },
    Redis {
        connection: ConnectionManager,
        script: Script,
        fallback: Mutex<TokenBucketRateLimitProvider>,
    },
}

/// Shared token-bucket rate limiter keyed by tenant/user or client address.
pub struct RateLimitState {
    default_rps: u32,
    default_burst: u32,
    tenant_overrides: HashMap<String, (u32, u32)>,
    backend: RateLimitBackend,
    /// When true, transient Redis script failures deny requests instead of SPI fallback.
    redis_fail_closed: bool,
}

impl RateLimitState {
    pub fn from_config(config: &ServerConfig) -> Self {
        Self::try_from_config(config).unwrap_or_else(|message| panic!("{message}"))
    }

    pub fn try_from_config(config: &ServerConfig) -> Result<Self, String> {
        let default_rps = config.rate_limit_rps;
        let default_burst = config.rate_limit_burst.max(1);
        let tenant_overrides = tenant_override_limits(&config.tenant_rate_limit_overrides);
        let ingress_provider = TokenBucketRateLimitProvider::ingress_from_config(
            default_rps,
            default_burst,
            &tenant_overrides,
        );
        let redis_fail_closed = config.requires_distributed_rate_limit();
        if let Some(redis_url) = config.effective_rate_limit_redis_url() {
            match connect_redis_blocking(redis_url) {
                Ok(connection) => {
                    return Ok(Self {
                        default_rps,
                        default_burst,
                        tenant_overrides,
                        redis_fail_closed,
                        backend: RateLimitBackend::Redis {
                            connection,
                            script: Script::new(REDIS_TOKEN_BUCKET_SCRIPT),
                            fallback: Mutex::new(ingress_provider),
                        },
                    });
                }
                Err(error) => {
                    if config.requires_distributed_rate_limit() {
                        return Err(format!(
                            "failed to connect rate-limit redis at {redis_url}: {error}; production cloud deployments require redis_cache"
                        ));
                    }
                    warn!(
                        redis_url = redis_url,
                        error = error.as_str(),
                        "rate-limit redis unavailable; falling back to in-process buckets"
                    );
                }
            }
        } else if config.requires_distributed_rate_limit() {
            return Err(
                "SDKWORK_RATE_LIMIT_REDIS_URL (or SDKWORK_REDIS_URL) is required for production cloud/server deployments with rate limiting enabled"
                    .to_string(),
            );
        }

        Ok(Self {
            default_rps,
            default_burst,
            tenant_overrides,
            redis_fail_closed,
            backend: RateLimitBackend::Memory {
                provider: Mutex::new(ingress_provider),
            },
        })
    }

    pub async fn try_from_config_async(config: &ServerConfig) -> Result<Self, String> {
        let default_rps = config.rate_limit_rps;
        let default_burst = config.rate_limit_burst.max(1);
        let tenant_overrides = tenant_override_limits(&config.tenant_rate_limit_overrides);
        let ingress_provider = TokenBucketRateLimitProvider::ingress_from_config(
            default_rps,
            default_burst,
            &tenant_overrides,
        );
        let redis_fail_closed = config.requires_distributed_rate_limit();
        if let Some(redis_url) = config.effective_rate_limit_redis_url() {
            match connect_redis_async(redis_url).await {
                Ok(connection) => {
                    return Ok(Self {
                        default_rps,
                        default_burst,
                        tenant_overrides,
                        redis_fail_closed,
                        backend: RateLimitBackend::Redis {
                            connection,
                            script: Script::new(REDIS_TOKEN_BUCKET_SCRIPT),
                            fallback: Mutex::new(ingress_provider),
                        },
                    });
                }
                Err(error) => {
                    if config.requires_distributed_rate_limit() {
                        return Err(format!(
                            "failed to connect rate-limit redis at {redis_url}: {error}; production cloud deployments require redis_cache"
                        ));
                    }
                    warn!(
                        redis_url = redis_url,
                        error = error.as_str(),
                        "rate-limit redis unavailable; falling back to in-process buckets"
                    );
                }
            }
        } else if config.requires_distributed_rate_limit() {
            return Err(
                "SDKWORK_RATE_LIMIT_REDIS_URL (or SDKWORK_REDIS_URL) is required for production cloud/server deployments with rate limiting enabled"
                    .to_string(),
            );
        }

        Ok(Self {
            default_rps,
            default_burst,
            tenant_overrides,
            redis_fail_closed,
            backend: RateLimitBackend::Memory {
                provider: Mutex::new(ingress_provider),
            },
        })
    }

    pub fn is_enabled(&self) -> bool {
        self.default_rps > 0
    }

    pub fn uses_redis(&self) -> bool {
        matches!(self.backend, RateLimitBackend::Redis { .. })
    }

    fn limits_for_tenant(&self, tenant_id: Option<&str>) -> (u32, u32) {
        if let Some(tenant_id) = tenant_id.filter(|value| !value.is_empty()) {
            if let Some((rps, burst)) = self.tenant_overrides.get(tenant_id) {
                return (*rps, (*burst).max(1));
            }
        }
        (self.default_rps, self.default_burst)
    }

    pub async fn try_acquire(&self, key: &str, tenant_id: Option<&str>) -> bool {
        if !self.is_enabled() {
            return true;
        }
        let (rps, burst) = self.limits_for_tenant(tenant_id);

        match &self.backend {
            RateLimitBackend::Memory { provider } => {
                let mut provider = provider.lock().unwrap_or_else(|error| error.into_inner());
                provider.try_acquire_ingress(key, tenant_id)
            }
            RateLimitBackend::Redis {
                connection,
                script,
                fallback,
            } => {
                let redis_result = self
                    .try_acquire_redis(connection.clone(), script, key, rps, burst)
                    .await;
                match redis_result {
                    Some(allowed) => allowed,
                    None if self.redis_fail_closed => {
                        warn!(
                            rate_limit_key = key,
                            "redis rate-limit failed; denying request in distributed profile"
                        );
                        false
                    }
                    None => {
                        warn!(
                            rate_limit_key = key,
                            "redis rate-limit failed; using kernel token-bucket fallback"
                        );
                        let mut provider =
                            fallback.lock().unwrap_or_else(|error| error.into_inner());
                        provider.try_acquire_ingress(key, tenant_id)
                    }
                }
            }
        }
    }

    /// Returns `Some(allowed)` on success, `None` when Redis itself errored.
    async fn try_acquire_redis(
        &self,
        mut connection: ConnectionManager,
        script: &Script,
        key: &str,
        rps: u32,
        burst: u32,
    ) -> Option<bool> {
        let redis_key = format!("sdkwork:ratelimit:{key}");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs_f64())
            .unwrap_or(0.0);
        let result: redis::RedisResult<i32> = script
            .key(redis_key)
            .arg(rps)
            .arg(burst)
            .arg(now)
            .invoke_async(&mut connection)
            .await;
        match result {
            Ok(allowed) => Some(allowed == 1),
            Err(error) => {
                warn!(error = %error, "redis rate-limit script failed; falling back to kernel token-bucket provider");
                None
            }
        }
    }
}

async fn connect_redis_async(redis_url: &str) -> Result<ConnectionManager, String> {
    let client = redis::Client::open(redis_url).map_err(|error| error.to_string())?;
    ConnectionManager::new(client)
        .await
        .map_err(|error| error.to_string())
}

fn connect_redis_blocking(redis_url: &str) -> Result<ConnectionManager, String> {
    let redis_url = redis_url.to_string();
    if tokio::runtime::Handle::try_current().is_ok() {
        return thread::spawn(move || connect_redis_on_current_thread(&redis_url))
            .join()
            .map_err(|_| "redis connection worker panicked".to_string())?;
    }
    connect_redis_on_current_thread(&redis_url)
}

fn connect_redis_on_current_thread(redis_url: &str) -> Result<ConnectionManager, String> {
    let runtime =
        tokio::runtime::Runtime::new().map_err(|error| format!("tokio runtime: {error}"))?;
    runtime.block_on(connect_redis_async(redis_url))
}

fn tenant_override_limits(
    overrides: &HashMap<String, crate::config::TenantRateLimitOverride>,
) -> HashMap<String, (u32, u32)> {
    overrides
        .iter()
        .map(|(tenant_id, limits)| (tenant_id.clone(), (limits.rps, limits.burst.max(1))))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TenantRateLimitOverride;

    #[test]
    fn memory_rate_limit_rejects_when_burst_exhausted() {
        let state = RateLimitState {
            default_rps: 1,
            default_burst: 1,
            tenant_overrides: HashMap::new(),
            redis_fail_closed: false,
            backend: RateLimitBackend::Memory {
                provider: Mutex::new(TokenBucketRateLimitProvider::ingress_from_config(
                    1,
                    1,
                    &HashMap::new(),
                )),
            },
        };
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        runtime.block_on(async {
            assert!(state.try_acquire("client", None).await);
            assert!(!state.try_acquire("client", None).await);
        });
    }

    #[test]
    fn tenant_override_limits_apply_before_acquire() {
        let mut overrides = HashMap::new();
        overrides.insert(
            "100001".to_string(),
            TenantRateLimitOverride { rps: 1, burst: 1 },
        );
        let state = RateLimitState {
            default_rps: 100,
            default_burst: 100,
            tenant_overrides: tenant_override_limits(&overrides),
            redis_fail_closed: false,
            backend: RateLimitBackend::Memory {
                provider: Mutex::new(TokenBucketRateLimitProvider::ingress_from_config(
                    100,
                    100,
                    &tenant_override_limits(&overrides),
                )),
            },
        };
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        runtime.block_on(async {
            assert!(
                state
                    .try_acquire("identity:100001:user-a", Some("100001"))
                    .await
            );
            assert!(
                !state
                    .try_acquire("identity:100001:user-a", Some("100001"))
                    .await
            );
            assert!(
                state
                    .try_acquire("identity:100002:user-b", Some("100002"))
                    .await
            );
        });
    }

    #[tokio::test]
    async fn redis_configuration_error_inside_runtime_returns_error_without_nested_runtime_panic() {
        let config = ServerConfig {
            environment: "production".to_string(),
            deployment_profile: Some("cloud".to_string()),
            kernel_runtime_target: Some("server".to_string()),
            rate_limit_rps: 1,
            rate_limit_burst: 1,
            rate_limit_redis_url: Some("not-a-redis-url".to_string()),
            ..Default::default()
        };

        let result = RateLimitState::try_from_config(&config);

        assert!(result.is_err(), "invalid redis URL should fail closed");
        let message = result.err().expect("redis error");
        assert!(
            !message.contains("Cannot start a runtime from within a runtime"),
            "redis rate-limit startup must not panic with nested Tokio runtime: {message}"
        );
    }
}
