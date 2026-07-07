//! HTTP ingress token-bucket rate limiting for the kernel `RateLimitProvider` SPI.
//!
//! `sdkwork-agent-server` uses this provider for in-process buckets and as the
//! Redis fail-over path. Distributed Redis token buckets remain server-owned
//! because they require async I/O and deployment topology wiring.

use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use crate::rate_limit::{
    QuotaUsage, RateLimitError, RateLimitPolicy, RateLimitProvider, RateLimitProviderHealth,
    RateLimitProviderManifest, RateLimitProviderStatus, RateLimitRequest, RateLimitResult,
    ResourceType,
};

/// Canonical policy id for default HTTP ingress limiting.
pub const INGRESS_HTTP_RATE_LIMIT_POLICY_ID: &str = "sdkwork.ingress.http";

const DEFAULT_MAX_TOKEN_BUCKETS: usize = 4096;
const INGRESS_WINDOW_MS: u64 = 1_000;

#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
}

/// Continuous token-bucket limiter implementing [`RateLimitProvider`].
#[derive(Debug)]
pub struct TokenBucketRateLimitProvider {
    buckets: HashMap<String, TokenBucket>,
    eviction_order: VecDeque<String>,
    policies: HashMap<String, RateLimitPolicy>,
    tenant_policy_ids: HashMap<String, String>,
    default_policy_id: String,
    max_buckets: usize,
    total_allowed: u64,
    total_denied: u64,
}

impl TokenBucketRateLimitProvider {
    pub fn new() -> Self {
        Self {
            buckets: HashMap::new(),
            eviction_order: VecDeque::new(),
            policies: HashMap::new(),
            tenant_policy_ids: HashMap::new(),
            default_policy_id: INGRESS_HTTP_RATE_LIMIT_POLICY_ID.to_string(),
            max_buckets: DEFAULT_MAX_TOKEN_BUCKETS,
            total_allowed: 0,
            total_denied: 0,
        }
    }

    pub fn with_max_buckets(mut self, max_buckets: usize) -> Self {
        self.max_buckets = max_buckets.max(1);
        self
    }

    /// Build ingress policies from server-style RPS/burst configuration.
    pub fn ingress_from_config(
        default_rps: u32,
        default_burst: u32,
        tenant_overrides: &HashMap<String, (u32, u32)>,
    ) -> Self {
        let mut provider = Self::new();
        let default_policy = ingress_policy_from_rps(
            INGRESS_HTTP_RATE_LIMIT_POLICY_ID,
            "HTTP ingress default",
            default_rps,
            default_burst,
        );
        provider
            .create_policy(default_policy)
            .expect("default ingress policy");
        for (tenant_id, (rps, burst)) in tenant_overrides {
            let policy_id = format!("sdkwork.ingress.http.tenant.{tenant_id}");
            let policy = ingress_policy_from_rps(
                &policy_id,
                &format!("HTTP ingress tenant {tenant_id}"),
                *rps,
                (*burst).max(1),
            );
            provider
                .create_policy(policy)
                .expect("tenant ingress policy");
            provider
                .tenant_policy_ids
                .insert(tenant_id.clone(), policy_id);
        }
        provider
    }

    /// Fast path for HTTP middleware — acquire one token for `key`.
    pub fn try_acquire_ingress(&mut self, key: &str, tenant_id: Option<&str>) -> bool {
        let policy_id = self.policy_id_for_tenant(tenant_id).to_string();
        let request = RateLimitRequest::new(policy_id, key, "ingress", 1);
        self.check_rate_limit(request)
            .map(|result| result.allowed)
            .unwrap_or(false)
    }

    fn policy_id_for_tenant(&self, tenant_id: Option<&str>) -> &str {
        if let Some(tenant_id) = tenant_id.filter(|value| !value.is_empty()) {
            if let Some(policy_id) = self.tenant_policy_ids.get(tenant_id) {
                return policy_id.as_str();
            }
        }
        self.default_policy_id.as_str()
    }

    fn limits_for_policy(&self, policy_id: &str) -> Result<(u32, u32), RateLimitError> {
        let policy = self.get_policy(policy_id)?;
        let rps = u32::try_from(policy.max_requests).unwrap_or(u32::MAX);
        let burst = u32::try_from(policy.burst_size).unwrap_or(1).max(1);
        Ok((rps, burst))
    }

    fn acquire_token(&mut self, key: &str, rps: u32, burst: u32) -> bool {
        if !self.buckets.contains_key(key) && self.buckets.len() >= self.max_buckets {
            while let Some(oldest_key) = self.eviction_order.pop_front() {
                if self.buckets.remove(&oldest_key).is_some() {
                    break;
                }
            }
        }

        let now = Instant::now();
        let was_present = self.buckets.contains_key(key);
        let bucket = self.buckets.entry(key.to_string()).or_insert(TokenBucket {
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

impl Default for TokenBucketRateLimitProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimitProvider for TokenBucketRateLimitProvider {
    fn create_policy(&mut self, policy: RateLimitPolicy) -> Result<(), RateLimitError> {
        self.policies.insert(policy.policy_id.clone(), policy);
        Ok(())
    }

    fn get_policy(&self, policy_id: &str) -> Result<RateLimitPolicy, RateLimitError> {
        self.policies
            .get(policy_id)
            .cloned()
            .ok_or_else(|| RateLimitError::PolicyNotFound(policy_id.to_string()))
    }

    fn check_rate_limit(
        &mut self,
        request: RateLimitRequest,
    ) -> Result<RateLimitResult, RateLimitError> {
        let (rps, burst) = self.limits_for_policy(&request.policy_id)?;
        let allowed = self.acquire_token(&request.resource_id, rps, burst);
        if allowed {
            self.total_allowed += 1;
            Ok(RateLimitResult::allowed(
                request.request_id,
                1,
                u64::from(burst),
            ))
        } else {
            self.total_denied += 1;
            Ok(RateLimitResult::denied(
                request.request_id,
                0,
                0,
                "ingress rate limit exceeded",
            ))
        }
    }

    fn record_usage(
        &mut self,
        resource_id: &str,
        policy_id: &str,
        amount: u64,
    ) -> Result<(), RateLimitError> {
        let (rps, burst) = self.limits_for_policy(policy_id)?;
        for _ in 0..amount {
            let _ = self.acquire_token(resource_id, rps, burst);
        }
        Ok(())
    }

    fn get_usage(&self, resource_id: &str, policy_id: &str) -> Result<QuotaUsage, RateLimitError> {
        self.get_policy(policy_id)?;
        let mut usage = QuotaUsage::new(resource_id, policy_id, INGRESS_WINDOW_MS);
        if self.buckets.contains_key(resource_id) {
            usage.current_usage = 1;
        }
        Ok(usage)
    }

    fn reset_quota(&mut self, resource_id: &str, _policy_id: &str) -> Result<(), RateLimitError> {
        self.buckets.remove(resource_id);
        Ok(())
    }

    fn list_policies(&self) -> Result<Vec<RateLimitPolicy>, RateLimitError> {
        Ok(self.policies.values().cloned().collect())
    }

    fn delete_policy(&mut self, policy_id: &str) -> Result<(), RateLimitError> {
        self.policies
            .remove(policy_id)
            .map(|_| ())
            .ok_or_else(|| RateLimitError::PolicyNotFound(policy_id.to_string()))
    }

    fn health_check(&self) -> Result<RateLimitProviderHealth, RateLimitError> {
        Ok(RateLimitProviderHealth {
            status: if self.buckets.len() >= self.max_buckets {
                RateLimitProviderStatus::Degraded
            } else {
                RateLimitProviderStatus::Healthy
            },
            active_policies: self.policies.len(),
            active_quotas: self.buckets.len(),
            total_requests_processed: self.total_allowed.saturating_add(self.total_denied),
            total_violations: self.total_denied,
            last_check_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        })
    }

    fn provider_manifest(&self) -> RateLimitProviderManifest {
        RateLimitProviderManifest {
            provider_id: "token-bucket-ingress-rate-limit-provider".to_string(),
            name: "Token Bucket Ingress Rate Limit Provider".to_string(),
            version: "1.0.0".to_string(),
            max_policies: self.policies.len().max(1),
            supports_distributed: false,
            supports_custom_strategies: false,
            supports_realtime_monitoring: true,
        }
    }
}

fn ingress_policy_from_rps(policy_id: &str, name: &str, rps: u32, burst: u32) -> RateLimitPolicy {
    RateLimitPolicy::new(
        policy_id,
        name,
        ResourceType::ApiRequest,
        u64::from(rps.max(1)),
        INGRESS_WINDOW_MS,
    )
    .with_burst(u64::from(burst.max(1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ingress_provider_rejects_when_burst_exhausted() {
        let mut provider = TokenBucketRateLimitProvider::ingress_from_config(1, 1, &HashMap::new());
        assert!(provider.try_acquire_ingress("client", None));
        assert!(!provider.try_acquire_ingress("client", None));
    }

    #[test]
    fn tenant_override_policy_applies_before_acquire() {
        let mut overrides = HashMap::new();
        overrides.insert("tenant.a".to_string(), (1, 1));
        let mut provider = TokenBucketRateLimitProvider::ingress_from_config(100, 100, &overrides);
        assert!(provider.try_acquire_ingress("identity:tenant.a:user", Some("tenant.a")));
        assert!(!provider.try_acquire_ingress("identity:tenant.a:user", Some("tenant.a")));
        assert!(provider.try_acquire_ingress("identity:tenant.b:user", Some("tenant.b")));
    }

    #[test]
    fn implements_rate_limit_provider_trait() {
        let mut provider = TokenBucketRateLimitProvider::ingress_from_config(5, 5, &HashMap::new());
        let request = RateLimitRequest::new(INGRESS_HTTP_RATE_LIMIT_POLICY_ID, "key", "agent", 1);
        let allowed = provider.check_rate_limit(request).expect("check");
        assert!(allowed.allowed);
    }
}
