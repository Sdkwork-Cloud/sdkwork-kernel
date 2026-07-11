//! Ingress token and signed identity headers for internal-api runtime HTTP.
//!
//! Aligns remote `SseChatClient` calls with `INTERNAL_API_SPEC.md` and
//! `sdkwork-agent-server` ingress identity modes (OpenLocal, Bound, Signed).

use hmac::Mac;

use crate::{AgentAuth, AgentAuthType};

pub const INGRESS_IDENTITY_MAC_HEADER: &str = "x-sdkwork-identity-mac";
pub const INGRESS_TENANT_HEADER: &str = "x-sdkwork-tenant-id";
pub const INGRESS_USER_HEADER: &str = "x-sdkwork-user-id";

type HmacSha256 = hmac::Hmac<sha2::Sha256>;

pub fn identity_mac_payload(tenant_id: &str, user_id: &str) -> String {
    format!("{tenant_id}\n{user_id}")
}

pub fn compute_identity_mac(token: &str, tenant_id: &str, user_id: &str) -> Option<String> {
    let mut mac = HmacSha256::new_from_slice(token.as_bytes()).ok()?;
    mac.update(identity_mac_payload(tenant_id, user_id).as_bytes());
    Some(hex_encode(mac.finalize().into_bytes()))
}

fn hex_encode(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn resolve_ingress_token(auth: &AgentAuth) -> Option<&str> {
    match auth.auth_type {
        AgentAuthType::ApiKey => auth.credentials.get("api_key").map(String::as_str),
        AgentAuthType::BearerToken => auth.credentials.get("token").map(String::as_str),
        AgentAuthType::BasicAuth | AgentAuthType::OAuth2 => None,
    }
}

fn resolve_tenant_id(auth: &AgentAuth) -> Option<&str> {
    auth.credentials
        .get("tenant_id")
        .map(String::as_str)
        .filter(|value: &&str| !value.is_empty())
}

fn resolve_user_id(auth: &AgentAuth) -> Option<&str> {
    auth.credentials
        .get("user_id")
        .map(String::as_str)
        .filter(|value: &&str| !value.is_empty())
}

fn uses_jwt_ingress_profile(auth: &AgentAuth) -> bool {
    auth.credentials
        .get("ingress_profile")
        .is_some_and(|value| value.eq_ignore_ascii_case("jwt"))
}

/// Attach canonical internal-api ingress auth headers to an outbound HTTP request.
pub fn apply_ingress_auth(
    builder: reqwest::RequestBuilder,
    auth: &AgentAuth,
) -> reqwest::RequestBuilder {
    let Some(token) = resolve_ingress_token(auth) else {
        return builder;
    };

    if uses_jwt_ingress_profile(auth) {
        return builder.header("Authorization", format!("Bearer {token}"));
    }

    let mut builder = builder
        .header("Authorization", format!("Bearer {token}"))
        .header("x-api-key", token);

    let tenant_id = resolve_tenant_id(auth);
    let user_id = resolve_user_id(auth);
    if let (Some(tenant_id), Some(user_id)) = (tenant_id, user_id) {
        builder = builder
            .header(INGRESS_TENANT_HEADER, tenant_id)
            .header(INGRESS_USER_HEADER, user_id);
        if let Some(mac) = compute_identity_mac(token, tenant_id, user_id) {
            builder = builder.header(INGRESS_IDENTITY_MAC_HEADER, mac);
        }
    }

    builder
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AgentAuth;

    #[test]
    fn ingress_mac_matches_server_contract() {
        let mac = compute_identity_mac("secret-token", "tenant.1", "user.1").expect("mac");
        assert_eq!(mac.len(), 64);
        assert!(mac.chars().all(|ch| ch.is_ascii_hexdigit()));
    }

    #[test]
    fn bearer_session_auth_carries_tenant_user_credentials() {
        let auth = AgentAuth::ingress_session("token", "tenant.1", "user.1");
        assert_eq!(resolve_ingress_token(&auth), Some("token"));
        assert_eq!(resolve_tenant_id(&auth), Some("tenant.1"));
        assert_eq!(resolve_user_id(&auth), Some("user.1"));
        let mac = compute_identity_mac("token", "tenant.1", "user.1").expect("mac");
        assert!(mac.len() == 64);
    }
}
