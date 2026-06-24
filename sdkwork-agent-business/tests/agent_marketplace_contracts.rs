use sdkwork_agent_business::{
    AgentBusinessService, AgentMcpAuthKind, AgentMcpServerCreateCommand,
    AgentMcpServerUpdateCommand, AgentMcpTransportKind, AgentVisibility, AllowAllPolicyProvider,
    GetAgentMarketplaceItemCommand, InMemoryAgentAuditSink, InMemoryAgentRepository,
};
use sdkwork_agent_kernel::{KernelErrorKind, PolicySubject};

fn subject() -> PolicySubject {
    PolicySubject::new("user.market.admin", "tenant.1").with_role("agent.market.admin")
}

fn service(
) -> AgentBusinessService<InMemoryAgentRepository, InMemoryAgentAuditSink, AllowAllPolicyProvider> {
    AgentBusinessService::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("provider.policy.market"),
    )
}

fn mcp_create_command(server_id: &str, code: &str) -> AgentMcpServerCreateCommand {
    AgentMcpServerCreateCommand {
        tenant_id: 1,
        organization_id: 10,
        owner_user_id: 100,
        mcp_server_id: server_id.to_string(),
        code: code.to_string(),
        display_name: "Filesystem MCP".to_string(),
        description: Some("Managed MCP server listing".to_string()),
        protocol_version: "2025-06-18".to_string(),
        transport_kind: AgentMcpTransportKind::Http,
        endpoint_ref: Some("endpoint.mcp.filesystem".to_string()),
        command_ref: None,
        auth_kind: AgentMcpAuthKind::OAuth2,
        auth_profile_id: Some("profile.auth.mcp.filesystem".to_string()),
        capability_ids: vec![
            "mcp.tools".to_string(),
            "mcp.resources".to_string(),
            "mcp.prompts".to_string(),
        ],
        tool_count: 12,
        resource_count: 3,
        prompt_count: 2,
        capabilities_json: r#"{"tools":true,"resources":true,"prompts":true}"#.to_string(),
        categories: vec!["filesystem".to_string()],
        tags: vec!["mcp".to_string(), "local".to_string()],
        security_profile_id: Some("profile.security.mcp.filesystem".to_string()),
        visibility: AgentVisibility::Tenant,
        requested_by: subject(),
        requested_at: "2026-06-04T00:10:00Z".to_string(),
    }
}

#[test]
fn mcp_server_marketplace_crud_models_protocol_capabilities_without_plaintext_secrets() {
    let mut service = service();

    let created = service
        .create_mcp_server(mcp_create_command(
            "mcp.server.filesystem",
            "filesystem-mcp",
        ))
        .expect("mcp server should be created");
    assert_eq!(created.transport_kind, AgentMcpTransportKind::Http);
    assert_eq!(created.auth_kind, AgentMcpAuthKind::OAuth2);
    assert_eq!(created.tool_count, 12);

    let updated = service
        .update_mcp_server(AgentMcpServerUpdateCommand {
            tenant_id: 1,
            mcp_server_id: "mcp.server.filesystem".to_string(),
            expected_version: Some(created.version),
            display_name: Some("Filesystem MCP Server".to_string()),
            description: None,
            protocol_version: Some("2026-01-01".to_string()),
            transport_kind: Some(AgentMcpTransportKind::Stdio),
            endpoint_ref: Some(None),
            command_ref: Some(Some("command.mcp.filesystem".to_string())),
            auth_kind: Some(AgentMcpAuthKind::HostSecretRef),
            auth_profile_id: Some(Some("profile.auth.mcp.filesystem.local".to_string())),
            capability_ids: Some(vec!["mcp.tools".to_string(), "mcp.resources".to_string()]),
            tool_count: Some(14),
            resource_count: Some(4),
            prompt_count: Some(2),
            capabilities_json: Some(r#"{"tools":true,"resources":true}"#.to_string()),
            categories: Some(vec!["filesystem".to_string(), "automation".to_string()]),
            tags: Some(vec!["mcp".to_string()]),
            security_profile_id: Some(Some("profile.security.mcp.filesystem.local".to_string())),
            visibility: Some(AgentVisibility::Organization),
            requested_by: subject(),
            requested_at: "2026-06-04T00:11:00Z".to_string(),
        })
        .expect("mcp server should be updated");

    assert_eq!(updated.transport_kind, AgentMcpTransportKind::Stdio);
    assert_eq!(updated.endpoint_ref, None);
    assert_eq!(
        updated.command_ref.as_deref(),
        Some("command.mcp.filesystem")
    );
    assert_eq!(updated.tool_count, 14);

    let got = service
        .get_mcp_server(GetAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "mcp.server.filesystem".to_string(),
            requested_by: subject(),
        })
        .expect("mcp server should be retrievable");
    assert_eq!(got.mcp_server_id, "mcp.server.filesystem");

    let bad_secret_endpoint = service
        .create_mcp_server(AgentMcpServerCreateCommand {
            endpoint_ref: Some("https://example.test?token=plaintext".to_string()),
            ..mcp_create_command("mcp.server.badsecret", "bad-secret")
        })
        .expect_err("plaintext secret material in endpoint_ref should fail");
    assert_eq!(bad_secret_endpoint.kind(), KernelErrorKind::ValidationError);
    assert!(bad_secret_endpoint
        .safe_message()
        .contains("endpointRef must be a standard id reference"));
}

#[test]
fn marketplace_records_reject_invalid_ids_duplicate_capabilities_and_stale_versions() {
    let mut service = service();

    let invalid_mcp_id = service
        .create_mcp_server(mcp_create_command("agent.mcp.bad", "bad-mcp"))
        .expect_err("mcp server id must use mcp prefix");
    assert_eq!(invalid_mcp_id.kind(), KernelErrorKind::ValidationError);

    let duplicate_capability = service
        .create_mcp_server(AgentMcpServerCreateCommand {
            capability_ids: vec!["mcp.tools".to_string(), "mcp.tools".to_string()],
            ..mcp_create_command("mcp.server.dup", "mcp-dup")
        })
        .expect_err("duplicate capability ids should fail");
    assert_eq!(
        duplicate_capability.kind(),
        KernelErrorKind::ValidationError
    );

    let created = service
        .create_mcp_server(mcp_create_command("mcp.server.versioned", "versioned-mcp"))
        .expect("mcp server should be created");
    service
        .update_mcp_server(AgentMcpServerUpdateCommand {
            tenant_id: 1,
            mcp_server_id: "mcp.server.versioned".to_string(),
            expected_version: Some(created.version),
            display_name: Some("Versioned MCP v2".to_string()),
            description: None,
            protocol_version: None,
            transport_kind: None,
            endpoint_ref: None,
            command_ref: None,
            auth_kind: None,
            auth_profile_id: None,
            capability_ids: None,
            tool_count: None,
            resource_count: None,
            prompt_count: None,
            capabilities_json: None,
            categories: None,
            tags: None,
            security_profile_id: None,
            visibility: None,
            requested_by: subject(),
            requested_at: "2026-06-04T00:12:00Z".to_string(),
        })
        .expect("matching expected version should update");

    let stale = service
        .update_mcp_server(AgentMcpServerUpdateCommand {
            tenant_id: 1,
            mcp_server_id: "mcp.server.versioned".to_string(),
            expected_version: Some(created.version),
            display_name: Some("Versioned MCP stale".to_string()),
            description: None,
            protocol_version: None,
            transport_kind: None,
            endpoint_ref: None,
            command_ref: None,
            auth_kind: None,
            auth_profile_id: None,
            capability_ids: None,
            tool_count: None,
            resource_count: None,
            prompt_count: None,
            capabilities_json: None,
            categories: None,
            tags: None,
            security_profile_id: None,
            visibility: None,
            requested_by: subject(),
            requested_at: "2026-06-04T00:13:00Z".to_string(),
        })
        .expect_err("stale expected version should fail");
    assert_eq!(stale.kind(), KernelErrorKind::Conflict);
}
