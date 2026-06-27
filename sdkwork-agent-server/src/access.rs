use axum::http::StatusCode;
use sdkwork_agent_database::SessionRow;
use std::collections::HashMap;

use crate::config::ServerConfig;
use crate::middleware::RequestContext;
use crate::security_audit;

/// Session ownership policy derived from ingress auth configuration.
#[derive(Debug, Clone, Copy)]
pub struct AccessPolicy {
    pub enforce_session_scope: bool,
}

impl AccessPolicy {
    pub fn from_config(config: &ServerConfig) -> Self {
        Self {
            enforce_session_scope: config.ingress_auth_secured(),
        }
    }
}

pub fn stamp_session_ownership(
    metadata: &mut HashMap<String, String>,
    ctx: &RequestContext,
    config: &ServerConfig,
) -> Result<(), StatusCode> {
    if !config.ingress_auth_secured() {
        return Ok(());
    }

    let tenant = ctx.tenant_id.as_deref().ok_or(StatusCode::BAD_REQUEST)?;
    let user = ctx
        .user_id
        .as_deref()
        .or_else(|| ctx.subject_id.as_deref())
        .ok_or(StatusCode::BAD_REQUEST)?;

    metadata.insert("ownerTenantId".to_string(), tenant.to_string());
    metadata.insert("ownerUserRef".to_string(), user.to_string());
    Ok(())
}

pub fn assert_session_access(
    policy: AccessPolicy,
    ctx: &RequestContext,
    row: &SessionRow,
) -> Result<(), StatusCode> {
    if !policy.enforce_session_scope {
        return Ok(());
    }

    let metadata = parse_metadata(row);
    let owner_tenant = metadata.get("ownerTenantId");
    let owner_user = metadata.get("ownerUserRef");
    if owner_tenant.is_none() || owner_user.is_none() {
        return Err(StatusCode::FORBIDDEN);
    }

    let caller_tenant = ctx.tenant_id.as_deref().ok_or(StatusCode::FORBIDDEN)?;
    if owner_tenant.is_some_and(|owner| owner != caller_tenant) {
        security_audit::log_access_denied(
            "session.owner_tenant_mismatch",
            Some(&ctx.request_id),
            &row.session_id,
            ctx.tenant_id.as_deref(),
            ctx.user_id.as_deref(),
            "session tenant owner mismatch",
        );
        return Err(StatusCode::FORBIDDEN);
    }

    let caller_user = ctx
        .user_id
        .as_deref()
        .or_else(|| ctx.subject_id.as_deref())
        .ok_or(StatusCode::FORBIDDEN)?;
    if owner_user.is_some_and(|owner| owner != caller_user) {
        security_audit::log_access_denied(
            "session.owner_user_mismatch",
            Some(&ctx.request_id),
            &row.session_id,
            ctx.tenant_id.as_deref(),
            ctx.user_id.as_deref(),
            "session user owner mismatch",
        );
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(())
}

pub fn assert_permission_access(
    policy: AccessPolicy,
    ctx: &RequestContext,
    owner_tenant_id: Option<&str>,
    owner_user_ref: Option<&str>,
    permission_request_id: &str,
) -> Result<(), StatusCode> {
    if !policy.enforce_session_scope {
        return Ok(());
    }

    let caller_tenant = ctx.tenant_id.as_deref().ok_or(StatusCode::FORBIDDEN)?;
    let caller_user = ctx
        .user_id
        .as_deref()
        .or_else(|| ctx.subject_id.as_deref())
        .ok_or(StatusCode::FORBIDDEN)?;

    if owner_tenant_id.is_some_and(|owner| owner != caller_tenant)
        || owner_user_ref.is_some_and(|owner| owner != caller_user)
    {
        security_audit::log_access_denied(
            "permission.owner_mismatch",
            Some(&ctx.request_id),
            permission_request_id,
            ctx.tenant_id.as_deref(),
            ctx.user_id.as_deref(),
            "permission request owned by another caller",
        );
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(())
}

fn parse_metadata(row: &SessionRow) -> HashMap<String, String> {
    row.metadata_json
        .as_deref()
        .and_then(|value| serde_json::from_str::<HashMap<String, String>>(value).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session_with_owner(tenant: &str, user: &str) -> SessionRow {
        SessionRow {
            session_id: "session.1".to_string(),
            agent_id: "agent.1".to_string(),
            model: None,
            title: None,
            state: "active".to_string(),
            kind: "chat".to_string(),
            source: "kernel".to_string(),
            provider_id: None,
            bridge_id: None,
            token_usage_json: None,
            message_count: 0,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: None,
            cwd: None,
            metadata_json: Some(
                serde_json::json!({
                    "ownerTenantId": tenant,
                    "ownerUserRef": user
                })
                .to_string(),
            ),
        }
    }

    #[test]
    fn open_policy_allows_any_caller() {
        let policy = AccessPolicy {
            enforce_session_scope: false,
        };
        let ctx = RequestContext {
            request_id: "req.1".to_string(),
            trace_id: None,
            tenant_id: Some("other".to_string()),
            user_id: Some("other".to_string()),
            subject_id: None,
            api_surface: None,
            route_template: String::new(),
        };
        assert!(
            assert_session_access(policy, &ctx, &session_with_owner("tenant.1", "user.1")).is_ok()
        );
    }

    fn session_without_owner() -> SessionRow {
        SessionRow {
            session_id: "session.legacy".to_string(),
            agent_id: "agent.1".to_string(),
            model: None,
            title: None,
            state: "active".to_string(),
            kind: "chat".to_string(),
            source: "kernel".to_string(),
            provider_id: None,
            bridge_id: None,
            token_usage_json: None,
            message_count: 0,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: None,
            cwd: None,
            metadata_json: None,
        }
    }

    #[test]
    fn token_policy_rejects_session_without_owner_metadata() {
        let policy = AccessPolicy {
            enforce_session_scope: true,
        };
        let ctx = RequestContext {
            request_id: "req.1".to_string(),
            trace_id: None,
            tenant_id: Some("tenant.1".to_string()),
            user_id: Some("user.1".to_string()),
            subject_id: None,
            api_surface: None,
            route_template: String::new(),
        };
        assert_eq!(
            assert_session_access(policy, &ctx, &session_without_owner()),
            Err(StatusCode::FORBIDDEN)
        );
    }

    #[test]
    fn token_policy_rejects_mismatched_owner() {
        let policy = AccessPolicy {
            enforce_session_scope: true,
        };
        let ctx = RequestContext {
            request_id: "req.1".to_string(),
            trace_id: None,
            tenant_id: Some("tenant.2".to_string()),
            user_id: Some("user.1".to_string()),
            subject_id: None,
            api_surface: None,
            route_template: String::new(),
        };
        assert_eq!(
            assert_session_access(policy, &ctx, &session_with_owner("tenant.1", "user.1")),
            Err(StatusCode::FORBIDDEN)
        );
    }

    #[test]
    fn token_policy_rejects_missing_caller_identity() {
        let policy = AccessPolicy {
            enforce_session_scope: true,
        };
        let ctx = RequestContext {
            request_id: "req.1".to_string(),
            trace_id: None,
            tenant_id: None,
            user_id: None,
            subject_id: None,
            api_surface: None,
            route_template: String::new(),
        };
        assert_eq!(
            assert_session_access(policy, &ctx, &session_with_owner("tenant.1", "user.1")),
            Err(StatusCode::FORBIDDEN)
        );
    }

    #[test]
    fn stamp_requires_tenant_and_user_in_token_mode() {
        let config = ServerConfig {
            ingress_auth_mode: "token".to_string(),
            ..ServerConfig::default()
        };
        let mut metadata = HashMap::new();
        let ctx = RequestContext {
            request_id: "req.1".to_string(),
            trace_id: None,
            tenant_id: None,
            user_id: Some("user.1".to_string()),
            subject_id: None,
            api_surface: None,
            route_template: String::new(),
        };
        assert_eq!(
            stamp_session_ownership(&mut metadata, &ctx, &config),
            Err(StatusCode::BAD_REQUEST)
        );
    }
}
