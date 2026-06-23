//! Contract tests: Rust client ingress auth matches server ingress_identity.

use sdkwork_agent_client::{compute_identity_mac, ingress_auth, AgentAuth, INGRESS_IDENTITY_MAC_HEADER};

#[test]
fn apply_ingress_auth_sets_canonical_headers() {
    let auth = AgentAuth::ingress_session("secret-token", "tenant.1", "user.1");
    let client = reqwest::Client::new();
    let request = ingress_auth::apply_ingress_auth(client.get("http://127.0.0.1/"), &auth)
        .build()
        .expect("request");
    let headers = request.headers();
    assert_eq!(
        headers.get("Authorization").and_then(|v| v.to_str().ok()),
        Some("Bearer secret-token")
    );
    assert_eq!(
        headers.get("x-api-key").and_then(|v| v.to_str().ok()),
        Some("secret-token")
    );
    assert_eq!(
        headers
            .get("x-sdkwork-tenant-id")
            .and_then(|v| v.to_str().ok()),
        Some("tenant.1")
    );
    assert_eq!(
        headers.get("x-sdkwork-user-id").and_then(|v| v.to_str().ok()),
        Some("user.1")
    );
    let mac = headers
        .get(INGRESS_IDENTITY_MAC_HEADER)
        .and_then(|v| v.to_str().ok())
        .expect("identity mac");
    let expected = compute_identity_mac("secret-token", "tenant.1", "user.1").expect("mac");
    assert_eq!(mac, expected);
}

#[test]
fn ingress_jwt_profile_sends_bearer_only() {
    let auth = AgentAuth::ingress_jwt("jwt-token-value");
    let client = reqwest::Client::new();
    let request = ingress_auth::apply_ingress_auth(client.get("http://127.0.0.1/"), &auth)
        .build()
        .expect("request");
    let headers = request.headers();
    assert_eq!(
        headers.get("Authorization").and_then(|v| v.to_str().ok()),
        Some("Bearer jwt-token-value")
    );
    assert!(headers.get("x-api-key").is_none());
    assert!(headers.get(INGRESS_IDENTITY_MAC_HEADER).is_none());
}

#[test]
fn signed_mac_vector_is_stable_hex_hmac_sha256() {
    let mac = compute_identity_mac("secret-token", "tenant.1", "user.1").expect("mac");
    assert_eq!(mac.len(), 64);
    assert!(mac.chars().all(|ch| ch.is_ascii_hexdigit() && !ch.is_uppercase()));
}
