use sdkwork_agent_business::{
    AgentBusinessService, AgentBusinessStatus, AgentMarketplaceListQuery, AgentMcpAuthKind,
    AgentMcpServerCreateCommand, AgentMcpServerUpdateCommand, AgentMcpTransportKind,
    AgentPromptTemplateCreateCommand, AgentPromptTemplateFormat, AgentPromptTemplateKind,
    AgentPromptTemplateUpdateCommand, AgentSkillInvocationKind, AgentSkillPackageCreateCommand,
    AgentSkillPackageUpdateCommand, AgentVisibility, AllowAllPolicyProvider,
    DeleteAgentMarketplaceItemCommand, GetAgentMarketplaceItemCommand, InMemoryAgentAuditSink,
    InMemoryAgentRepository, RestoreAgentMarketplaceItemCommand,
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

fn skill_create_command(skill_id: &str, code: &str) -> AgentSkillPackageCreateCommand {
    AgentSkillPackageCreateCommand {
        tenant_id: 1,
        organization_id: 10,
        owner_user_id: 100,
        skill_id: skill_id.to_string(),
        code: code.to_string(),
        display_name: "Research Skill".to_string(),
        description: Some("Curated research workflow skill".to_string()),
        invocation_kind: AgentSkillInvocationKind::LocalWorkflow,
        package_ref: "oci://registry.sdkwork.dev/skills/research:1.0.0".to_string(),
        entrypoint: "skills.research.run".to_string(),
        input_schema_json: r#"{"type":"object","required":["query"]}"#.to_string(),
        output_schema_json: r#"{"type":"object","required":["answer"]}"#.to_string(),
        capability_ids: vec!["skill.invoke".to_string(), "tool.invoke".to_string()],
        categories: vec!["research".to_string()],
        tags: vec!["rag".to_string(), "knowledge".to_string()],
        security_profile_id: Some("profile.skill.research".to_string()),
        visibility: AgentVisibility::Organization,
        requested_by: subject(),
        requested_at: "2026-06-04T00:00:00Z".to_string(),
    }
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

fn prompt_create_command(prompt_id: &str, code: &str) -> AgentPromptTemplateCreateCommand {
    AgentPromptTemplateCreateCommand {
        tenant_id: 1,
        organization_id: 10,
        owner_user_id: 100,
        prompt_id: prompt_id.to_string(),
        code: code.to_string(),
        display_name: "Review Prompt".to_string(),
        description: Some("Structured review prompt".to_string()),
        prompt_kind: AgentPromptTemplateKind::Developer,
        template_format: AgentPromptTemplateFormat::Handlebars,
        template_body: "Review {{artifact}} against {{standard}}.".to_string(),
        variables_schema_json: r#"{"type":"object","required":["artifact","standard"]}"#
            .to_string(),
        model_constraints_json: r#"{"families":["reasoning","coding"]}"#.to_string(),
        capability_ids: vec!["prompt.render".to_string(), "agent.review".to_string()],
        categories: vec!["review".to_string()],
        tags: vec!["quality".to_string()],
        safety_profile_id: Some("profile.safety.prompt.review".to_string()),
        visibility: AgentVisibility::Public,
        requested_by: subject(),
        requested_at: "2026-06-04T00:20:00Z".to_string(),
    }
}

#[test]
fn skill_package_marketplace_crud_enforces_standard_metadata() {
    let mut service = service();

    let created = service
        .create_skill_package(skill_create_command("skill.research.deep", "research-deep"))
        .expect("skill package should be created");
    assert!(created.id > (1_u64 << 22));
    assert!(created.id <= i64::MAX as u64);
    assert_eq!(created.status, AgentBusinessStatus::Draft);
    assert_eq!(
        created.invocation_kind,
        AgentSkillInvocationKind::LocalWorkflow
    );
    assert_eq!(created.version, 1);

    let updated = service
        .update_skill_package(AgentSkillPackageUpdateCommand {
            tenant_id: 1,
            skill_id: "skill.research.deep".to_string(),
            expected_version: Some(created.version),
            display_name: Some("Deep Research Skill".to_string()),
            description: Some("Updated research workflow skill".to_string()),
            invocation_kind: Some(AgentSkillInvocationKind::McpTool),
            package_ref: Some("oci://registry.sdkwork.dev/skills/research:1.1.0".to_string()),
            entrypoint: Some("skills.research.v11.run".to_string()),
            input_schema_json: None,
            output_schema_json: None,
            capability_ids: Some(vec!["skill.invoke".to_string(), "mcp.tools".to_string()]),
            categories: Some(vec!["research".to_string(), "productivity".to_string()]),
            tags: Some(vec!["knowledge".to_string()]),
            security_profile_id: Some("profile.skill.research.v2".to_string()),
            visibility: Some(AgentVisibility::Tenant),
            requested_by: subject(),
            requested_at: "2026-06-04T00:01:00Z".to_string(),
        })
        .expect("skill package should be updated");

    assert_eq!(updated.display_name, "Deep Research Skill");
    assert_eq!(updated.invocation_kind, AgentSkillInvocationKind::McpTool);
    assert_eq!(updated.visibility, AgentVisibility::Tenant);
    assert_eq!(updated.version, 2);

    let listed = service
        .list_skill_packages(
            AgentMarketplaceListQuery::for_tenant(1)
                .with_search("deep")
                .with_category("research")
                .with_tag("knowledge"),
            subject(),
        )
        .expect("skill packages should list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].skill_id, "skill.research.deep");

    let deleted = service
        .delete_skill_package(DeleteAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "skill.research.deep".to_string(),
            expected_version: Some(updated.version),
            requested_by: subject(),
            requested_at: "2026-06-04T00:02:00Z".to_string(),
        })
        .expect("skill package should be deleted");
    assert_eq!(deleted.status, AgentBusinessStatus::Deleted);
    assert!(deleted.deleted_at.is_some());

    assert!(service
        .list_skill_packages(AgentMarketplaceListQuery::for_tenant(1), subject())
        .expect("non-deleted list should work")
        .is_empty());

    let restored = service
        .restore_skill_package(RestoreAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "skill.research.deep".to_string(),
            expected_version: Some(deleted.version),
            requested_by: subject(),
            requested_at: "2026-06-04T00:03:00Z".to_string(),
        })
        .expect("skill package should be restored");
    assert_eq!(restored.status, AgentBusinessStatus::Active);
    assert!(restored.deleted_at.is_none());
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
fn prompt_template_marketplace_crud_preserves_template_and_schema_contracts() {
    let mut service = service();

    let created = service
        .create_prompt_template(prompt_create_command("prompt.review.structured", "review"))
        .expect("prompt template should be created");
    assert_eq!(created.prompt_kind, AgentPromptTemplateKind::Developer);
    assert_eq!(
        created.template_format,
        AgentPromptTemplateFormat::Handlebars
    );
    assert!(created.template_body.contains("{{artifact}}"));

    let updated = service
        .update_prompt_template(AgentPromptTemplateUpdateCommand {
            tenant_id: 1,
            prompt_id: "prompt.review.structured".to_string(),
            expected_version: Some(created.version),
            display_name: Some("Structured Review Prompt".to_string()),
            description: None,
            prompt_kind: Some(AgentPromptTemplateKind::Workflow),
            template_format: Some(AgentPromptTemplateFormat::Liquid),
            template_body: Some("Review {{ artifact }} with {{ standard }}.".to_string()),
            variables_schema_json: Some(
                r#"{"type":"object","required":["artifact","standard"],"additionalProperties":false}"#
                    .to_string(),
            ),
            model_constraints_json: Some(r#"{"requires":["tool_use"]}"#.to_string()),
            capability_ids: Some(vec!["prompt.render".to_string(), "workflow.plan".to_string()]),
            categories: Some(vec!["review".to_string(), "quality".to_string()]),
            tags: Some(vec!["audit".to_string()]),
            safety_profile_id: Some("profile.safety.prompt.strict".to_string()),
            visibility: Some(AgentVisibility::Tenant),
            requested_by: subject(),
            requested_at: "2026-06-04T00:21:00Z".to_string(),
        })
        .expect("prompt template should be updated");

    assert_eq!(updated.prompt_kind, AgentPromptTemplateKind::Workflow);
    assert_eq!(updated.template_format, AgentPromptTemplateFormat::Liquid);
    assert_eq!(updated.version, 2);

    let listed = service
        .list_prompt_templates(
            AgentMarketplaceListQuery::for_tenant(1)
                .with_search("structured")
                .with_visibility(AgentVisibility::Tenant),
            subject(),
        )
        .expect("prompt templates should list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].prompt_id, "prompt.review.structured");
}

#[test]
fn marketplace_records_reject_invalid_ids_duplicate_capabilities_and_stale_versions() {
    let mut service = service();

    let invalid_skill_id = service
        .create_skill_package(skill_create_command("agent.skill.bad", "bad-skill"))
        .expect_err("skill id must use skill prefix");
    assert_eq!(invalid_skill_id.kind(), KernelErrorKind::ValidationError);

    let duplicate_capability = service
        .create_prompt_template(AgentPromptTemplateCreateCommand {
            capability_ids: vec!["prompt.render".to_string(), "prompt.render".to_string()],
            ..prompt_create_command("prompt.review.dup", "review-dup")
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
