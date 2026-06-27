//! Distributed token-bucket rate limiting with optional Redis backing.
//!
//! Fail-closed design: when the Redis backend encounters an error, the limiter
//! falls back to an in-process token bucket rather than allowing the request
//! unconditionally. This prevents abuse during transient Redis outages.

use redis::aio::ConnectionManager;
use redis::Script;
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::Instant;
use tracing::warn;

use crate::config::{ServerConfig, TenantRateLimitOverride};

const MAX_RATE_LIMIT_BUCKETS: usize = 4096;

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

#[derive(Debug)]
struct RateBucket {
    tokens: f64,
    last_refill: Instant,
}

/// In-process token-bucket store with O(1) LRU eviction via an insertion-order
/// queue tracking the oldest bucket key.
struct MemoryBuckets {
    buckets: HashMap<String, RateBucket>,
    eviction_order: VecDeque<String>,
}

impl MemoryBuckets {
    fn new() -> Self {
        Self {
            buckets: HashMap::new(),
            eviction_order: VecDeque::new(),
        }
    }

    fn acquire(&mut self, key: &str, rps: u32, burst: u32) -> bool {
        if !self.buckets.contains_key(key) && self.buckets.len() >= MAX_RATE_LIMIT_BUCKETS {
            // O(1) eviction: pop the oldest key from the front of the queue.
            while let Some(oldest_key) = self.eviction_order.pop_front() {
                if self.buckets.remove(&oldest_key).is_some() {
                    break;
                }
            }
        }

        let now = Instant::now();
        let was_present = self.buckets.contains_key(key);
        let bucket = self.buckets.entry(key.to_string()).or_insert(RateBucket {
            tokens: f64::from(burst),
            last_refill: now,
        });

        if !was_present {
            self.eviction_order.push_back(key.to_string());
        }

        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * f64::from(rps)).min(f64::from(burst));
        bucket.last_refill = now;

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

enum RateLimitBackend {
    Memory {
        buckets: Mutex<MemoryBuckets>,
    },
    Redis {
        connection: ConnectionManager,
        script: Script,
        fallback: Mutex<MemoryBuckets>,
    },
}

/// Shared token-bucket rate limiter keyed by tenant/user or client address.
pub struct RateLimitState {
    default_rps: u32,
    default_burst: u32,
    tenant_overrides: HashMap<String, TenantRateLimitOverride>,
    backend: RateLimitBackend,
}

impl RateLimitState {
    pub fn from_config(config: &ServerConfig) -> Self {
        let default_rps = config.rate_limit_rps;
        let default_burst = config.rate_limit_burst.max(1);
        let tenant_overrides = config.tenant_rate_limit_overrides.clone();
        if let Some(redis_url) = config.effective_rate_limit_redis_url() {
            match Self::connect_redis(redis_url) {
                Ok(connection) => {
                    return Self {
                        default_rps,
                        default_burst,
                        tenant_overrides,
                        backend: RateLimitBackend::Redis {
                            connection,
                            script: Script::new(REDIS_TOKEN_BUCKET_SCRIPT),
                            fallback: Mutex::new(MemoryBuckets::new()),
                        },
                    };
                }
                Err(error) => {
                    if config.requires_distributed_rate_limit() {
                        // Intentional fail-fast: production cloud deployments MUST
                        // have distributed rate limiting. Silently falling back to
                        // in-process buckets would leave the deployment vulnerable
                        // to DDoS and tenant-quota bypass. Operators must provision
                        // Redis or disable `requires_distributed_rate_limit()`.
                        panic!(
                            "failed to connect rate-limit redis at {redis_url}: {error}; production cloud deployments require redis_cache"
                        );
                    }
                    warn!(
                        redis_url = redis_url,
                        error = %error,
                        "rate-limit redis unavailable; falling back to in-process buckets"
                    );
                }
            }
        } else if config.requires_distributed_rate_limit() {
            panic!(
                "SDKWORK_RATE_LIMIT_REDIS_URL (or SDKWORK_REDIS_URL) is required for production cloud/server deployments with rate limiting enabled"
            );
        }

        Self {
            default_rps,
            default_burst,
            tenant_overrides,
            backend: RateLimitBackend::Memory {
                buckets: Mutex::new(MemoryBuckets::new()),
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
        self.default_rps > 0
    }

    pub fn uses_redis(&self) -> bool {
        matches!(self.backend, RateLimitBackend::Redis { .. })
    }

    fn limits_for_tenant(&self, tenant_id: Option<&str>) -> (u32, u32) {
        if let Some(tenant_id) = tenant_id.filter(|value| !value.is_empty()) {
            if let Some(override_limits) = self.tenant_overrides.get(tenant_id) {
                return (override_limits.rps, override_limits.burst.max(1));
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
            RateLimitBackend::Memory { buckets } => {
                let mut buckets = buckets.lock().unwrap_or_else(|error| error.into_inner());
                buckets.acquire(key, rps, burst)
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
                    None => {
                        // Redis error — fail-closed by falling back to the
                        // in-process token bucket instead of allowing the
                        // request unconditionally.
                        warn!(
                            rate_limit_key = key,
                            "redis rate-limit failed; using in-process fallback bucket"
                        );
                        let mut fallback =
                            fallback.lock().unwrap_or_else(|error| error.into_inner());
                        fallback.acquire(key, rps, burst)
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
                warn!(error = %error, "redis rate-limit script failed; falling back to in-process bucket");
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_rate_limit_rejects_when_burst_exhausted() {
        let state = RateLimitState {
            default_rps: 1,
            default_burst: 1,
            tenant_overrides: HashMap::new(),
            backend: RateLimitBackend::Memory {
                buckets: Mutex::new(MemoryBuckets::new()),
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
            tenant_overrides: overrides,
            backend: RateLimitBackend::Memory {
                buckets: Mutex::new(MemoryBuckets::new()),
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

    #[test]
    fn memory_buckets_evict_oldest_when_full() {
        let mut buckets = MemoryBuckets::new();
        // Fill to capacity minus one so the next insert triggers eviction.
        for i in 0..MAX_RATE_LIMIT_BUCKETS {
            buckets.acquire(&format!("key-{i}"), 100, 1);
        }
        // Inserting a new key should evict the oldest (key-0).
        buckets.acquire("key-new", 100, 1);
        assert!(!buckets.buckets.contains_key("key-0"));
        assert!(buckets.buckets.contains_key("key-new"));
    }
}
