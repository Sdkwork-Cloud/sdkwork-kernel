use axum::http::{HeaderMap, StatusCode};

use crate::config::ServerConfig;
use crate::middleware::RequestContext;

pub const IDENTITY_MAC_HEADER: &str = "x-sdkwork-identity-mac";

/// How caller tenant/user identity is resolved for internal-api runtime routes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngressIdentityMode {
    /// Loopback-only open ingress; session scope is not enforced.
    OpenLocal,
    /// Server-configured fixed tenant/user; client headers are ignored.
    Bound,
    /// Token ingress without bound identity; identity headers must carry an HMAC proof.
    Signed,
}

impl ServerConfig {
    pub fn is_loopback_bind(&self) -> bool {
        matches!(
            self.bind_address.as_str(),
            "127.0.0.1" | "::1" | "localhost"
        )
    }

    pub fn has_bound_identity(&self) -> bool {
        self.ingress_bound_tenant_id
            .as_deref()
            .is_some_and(|value| !value.is_empty())
            && self
                .ingress_bound_user_id
                .as_deref()
                .is_some_and(|value| !value.is_empty())
    }

    pub fn ingress_identity_mode(&self) -> IngressIdentityMode {
        if self.ingress_auth_mode.eq_ignore_ascii_case("open") {
            return IngressIdentityMode::OpenLocal;
        }
        if self.has_bound_identity() {
            return IngressIdentityMode::Bound;
        }
        IngressIdentityMode::Signed
    }
}

pub fn identity_mac_payload(tenant_id: &str, user_id: &str) -> String {
    format!("{tenant_id}\n{user_id}")
}

pub fn compute_identity_mac(token: &str, tenant_id: &str, user_id: &str) -> Option<String> {
    // Delegate to sdkwork-utils-rust to avoid duplicating HMAC-SHA256 + hex
    // encoding logic that already exists in the cross-language utility crate.
    // The utils function returns the lowercase hex digest directly.
    Some(sdkwork_utils_rust::hmac_sha256(
        identity_mac_payload(tenant_id, user_id).as_bytes(),
        token.as_bytes(),
    ))
}

pub fn verify_identity_mac(token: &str, tenant_id: &str, user_id: &str, presented: &str) -> bool {
    let Some(expected) = compute_identity_mac(token, tenant_id, user_id) else {
        return false;
    };
    constant_time_eq(&expected, presented.trim())
}

pub fn resolve_request_identity(
    config: &ServerConfig,
    headers: &HeaderMap,
    mut ctx: RequestContext,
) -> Result<RequestContext, StatusCode> {
    match config.ingress_identity_mode() {
        IngressIdentityMode::OpenLocal => Ok(ctx),
        IngressIdentityMode::Bound => {
            ctx.tenant_id = config.ingress_bound_tenant_id.clone();
            ctx.user_id = config.ingress_bound_user_id.clone();
            ctx.subject_id = None;
            Ok(ctx)
        }
        IngressIdentityMode::Signed => {
            let token = config
                .ingress_token
                .as_deref()
                .filter(|value| !value.is_empty())
                .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
            let tenant_id = ctx
                .tenant_id
                .as_deref()
                .filter(|value| !value.is_empty())
                .ok_or(StatusCode::FORBIDDEN)?;
            let user_id = ctx
                .user_id
                .as_deref()
                .or(ctx.subject_id.as_deref())
                .filter(|value| !value.is_empty())
                .ok_or(StatusCode::FORBIDDEN)?;
            let mac = extract_header(headers, IDENTITY_MAC_HEADER).ok_or(StatusCode::FORBIDDEN)?;
            if !verify_identity_mac(token, tenant_id, user_id, &mac) {
                return Err(StatusCode::FORBIDDEN);
            }
            Ok(ctx)
        }
    }
}

fn extract_header(headers: &HeaderMap, key: &str) -> Option<String> {
    headers
        .get(key)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

/// Constant-time string equality backed by `subtle::ConstantTimeEq`.
///
/// Exposed as `pub(crate)` so `middleware::authorize_request` can reuse the
/// same implementation instead of redefining a private copy. Using `subtle`
/// here (rather than `sdkwork_utils_rust::secure_compare`) preserves the
/// stricter constant-time guarantees expected for token comparison: the
/// utils helper has an explicit early-return on length mismatch, while
/// `subtle::ConstantTimeEq` avoids that timing side-channel.
pub(crate) fn constant_time_eq(left: &str, right: &str) -> bool {
    use subtle::ConstantTimeEq;
    left.as_bytes().ct_eq(right.as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_token_mode_uses_signed_identity() {
        let config = ServerConfig {
            bind_address: "127.0.0.1".to_string(),
            ingress_auth_mode: "token".to_string(),
            ingress_token: Some("secret-token".to_string()),
            ..ServerConfig::default()
        };
        assert_eq!(config.ingress_identity_mode(), IngressIdentityMode::Signed);
    }

    #[test]
    fn signed_mode_requires_matching_mac() {
        let config = ServerConfig {
            bind_address: "0.0.0.0".to_string(),
            ingress_auth_mode: "token".to_string(),
            ingress_token: Some("secret-token".to_string()),
            ..ServerConfig::default()
        };
        assert_eq!(config.ingress_identity_mode(), IngressIdentityMode::Signed);

        let mac = compute_identity_mac("secret-token", "tenant.1", "user.1").expect("mac");
        let mut headers = HeaderMap::new();
        headers.insert(
            IDENTITY_MAC_HEADER,
            mac.parse().expect("valid header value"),
        );
        let ctx = RequestContext {
            request_id: "req.1".to_string(),
            trace_id: None,
            tenant_id: Some("tenant.1".to_string()),
            user_id: Some("user.1".to_string()),
            subject_id: None,
            api_surface: Some("internal-api"),
            route_template: "/internal/v3/api/intelligence/runtime/snapshot".to_string(),
        };
        assert!(resolve_request_identity(&config, &headers, ctx).is_ok());
    }

    #[test]
    fn bound_mode_overwrites_client_headers() {
        let config = ServerConfig {
            ingress_auth_mode: "token".to_string(),
            ingress_bound_tenant_id: Some("tenant.bound".to_string()),
            ingress_bound_user_id: Some("user.bound".to_string()),
            ..ServerConfig::default()
        };
        let ctx = RequestContext {
            request_id: "req.1".to_string(),
            trace_id: None,
            tenant_id: Some("tenant.spoof".to_string()),
            user_id: Some("user.spoof".to_string()),
            subject_id: None,
            api_surface: Some("internal-api"),
            route_template: "/internal/v3/api/intelligence/runtime/snapshot".to_string(),
        };
        let resolved = resolve_request_identity(&config, &HeaderMap::new(), ctx).expect("bound");
        assert_eq!(resolved.tenant_id.as_deref(), Some("tenant.bound"));
        assert_eq!(resolved.user_id.as_deref(), Some("user.bound"));
    }
}
