use sdkwork_agent_business::{
    ActivateAgentProviderBindingCommand, AgentBusinessService, AgentImplementationKind,
    AgentProviderBindingCommand, AgentProviderDeploymentCommand, AgentVisibility,
    AllowAllPolicyProvider, CreateAgentCommand, InMemoryAgentAuditSink, InMemoryAgentRepository,
};
use sdkwork_agent_kernel::{AgentManifest, PolicySubject};

fn sample_manifest(agent_id: &str) -> AgentManifest {
    AgentManifest {
        schema_version: "0.1.0".to_string(),
        manifest_type: "agent".to_string(),
        agent_id: agent_id.to_string(),
        name: "rig-general-agent".to_string(),
        display_name: "Rig General Agent".to_string(),
        description: "Rig".to_string(),
        version: "0.1.0".to_string(),
        domain: "intelligence".to_string(),
        required_capabilities: vec!["model.chat".to_string()],
        optional_capabilities: vec!["tool.invoke".to_string()],
        required_capability_requirements: vec![],
        optional_capability_requirements: vec![],
        event_families: vec!["agent.runtime.*".to_string()],
        owner_name: "sdkwork-platform".to_string(),
        status: "candidate".to_string(),
    }
}

fn subject() -> PolicySubject {
    PolicySubject::new("user.1", "tenant.1").with_role("owner")
}

fn service(
) -> AgentBusinessService<InMemoryAgentRepository, InMemoryAgentAuditSink, AllowAllPolicyProvider> {
    AgentBusinessService::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("provider.policy.test"),
    )
}

fn create_rig_agent(
    service: &mut AgentBusinessService<
        InMemoryAgentRepository,
        InMemoryAgentAuditSink,
        AllowAllPolicyProvider,
    >,
) {
    service
        .create_agent(CreateAgentCommand {
            agent_id: "agent.intelligence.rig-general".to_string(),
            tenant_id: 1,
            organization_id: 10,
            owner_user_id: 100,
            code: "rig-general".to_string(),
            display_name: "Rig General".to_string(),
            description: Some("Rig".to_string()),
            manifest: sample_manifest("agent.intelligence.rig-general"),
            visibility: AgentVisibility::Organization,
            tags: vec!["rig".to_string()],
            default_code_task_intent: None,
            implementation_provider_id: Some("provider.model.rig-rust".to_string()),
            implementation_kind: Some(AgentImplementationKind::TypedLocalProvider),
            requested_by: subject(),
            requested_at: "2026-06-04T00:00:00Z".to_string(),
        })
        .expect("agent is created");
}

fn provider_binding_command(
    binding_id: &str,
    provider_id: &str,
    configuration_profile_id: &str,
    capabilities: Vec<&str>,
) -> AgentProviderBindingCommand {
    AgentProviderBindingCommand {
        tenant_id: 1,
        agent_id: "agent.intelligence.rig-general".to_string(),
        binding_id: binding_id.to_string(),
        provider_id: provider_id.to_string(),
        implementation_kind: AgentImplementationKind::TypedLocalProvider,
        configuration_profile_id: configuration_profile_id.to_string(),
        capabilities: capabilities.into_iter().map(str::to_string).collect(),
        make_default: false,
        requested_by: subject(),
        requested_at: "2026-06-04T00:01:00Z".to_string(),
    }
}

fn deployment_command(deployment_id: &str, binding_id: &str) -> AgentProviderDeploymentCommand {
    AgentProviderDeploymentCommand {
        tenant_id: 1,
        agent_id: "agent.intelligence.rig-general".to_string(),
        deployment_id: deployment_id.to_string(),
        binding_id: binding_id.to_string(),
        requested_by: subject(),
        requested_at: "2026-06-04T00:02:00Z".to_string(),
    }
}

#[test]
fn create_agent_records_implementation_provider_metadata() {
    let mut service = service();
    let record = service
        .create_agent(CreateAgentCommand {
            agent_id: "agent.intelligence.rig-general".to_string(),
            tenant_id: 1,
            organization_id: 10,
            owner_user_id: 100,
            code: "rig-general".to_string(),
            display_name: "Rig General".to_string(),
            description: Some("Rig".to_string()),
            manifest: sample_manifest("agent.intelligence.rig-general"),
            visibility: AgentVisibility::Organization,
            tags: vec!["rig".to_string()],
            default_code_task_intent: None,
            implementation_provider_id: Some("provider.model.rig-rust".to_string()),
            implementation_kind: Some(AgentImplementationKind::TypedLocalProvider),
            requested_by: subject(),
            requested_at: "2026-06-04T00:00:00Z".to_string(),
        })
        .expect("agent is created");

    assert_eq!(
        record.implementation_provider_id.as_deref(),
        Some("provider.model.rig-rust")
    );
    assert_eq!(
        record.implementation_kind,
        Some(AgentImplementationKind::TypedLocalProvider)
    );
}

#[test]
fn create_agent_rejects_non_standard_implementation_provider_id() {
    let mut service = service();

    let error = service
        .create_agent(CreateAgentCommand {
            agent_id: "agent.intelligence.invalid-provider".to_string(),
            tenant_id: 1,
            organization_id: 10,
            owner_user_id: 100,
            code: "invalid-provider".to_string(),
            display_name: "Invalid Provider".to_string(),
            description: Some("Rig".to_string()),
            manifest: sample_manifest("agent.intelligence.invalid-provider"),
            visibility: AgentVisibility::Organization,
            tags: vec!["rig".to_string()],
            default_code_task_intent: None,
            implementation_provider_id: Some("model.rig-rust".to_string()),
            implementation_kind: Some(AgentImplementationKind::TypedLocalProvider),
            requested_by: subject(),
            requested_at: "2026-06-04T00:00:00Z".to_string(),
        })
        .expect_err("invalid implementation provider id should fail");

    assert!(
        error
            .safe_message()
            .contains("implementationProviderId must start with provider."),
        "unexpected create agent error: {}",
        error.safe_message()
    );
}

#[test]
fn activating_provider_binding_deactivates_previous_default() {
    let mut service = service();
    create_rig_agent(&mut service);

    let first = service
        .add_provider_binding(AgentProviderBindingCommand {
            tenant_id: 1,
            agent_id: "agent.intelligence.rig-general".to_string(),
            binding_id: "binding.rig.default".to_string(),
            provider_id: "provider.model.rig-rust".to_string(),
            implementation_kind: AgentImplementationKind::TypedLocalProvider,
            configuration_profile_id: "profile.rig.local".to_string(),
            capabilities: vec!["model.chat".to_string()],
            make_default: true,
            requested_by: subject(),
            requested_at: "2026-06-04T00:01:00Z".to_string(),
        })
        .expect("first binding is added");
    assert!(first.active);

    let second = service
        .add_provider_binding(AgentProviderBindingCommand {
            tenant_id: 1,
            agent_id: "agent.intelligence.rig-general".to_string(),
            binding_id: "binding.rig.alt".to_string(),
            provider_id: "provider.model.rig-alt".to_string(),
            implementation_kind: AgentImplementationKind::TypedLocalProvider,
            configuration_profile_id: "profile.rig.alt".to_string(),
            capabilities: vec!["model.chat".to_string(), "model.streaming".to_string()],
            make_default: true,
            requested_by: subject(),
            requested_at: "2026-06-04T00:02:00Z".to_string(),
        })
        .expect("second binding is added");
    assert!(second.active);

    let bindings = service
        .list_provider_bindings(1, "agent.intelligence.rig-general", subject())
        .expect("bindings list");
    assert_eq!(bindings.len(), 2);
    assert!(
        !bindings
            .iter()
            .find(|binding| binding.binding_id == "binding.rig.default")
            .expect("first binding exists")
            .active
    );
    assert!(
        bindings
            .iter()
            .find(|binding| binding.binding_id == "binding.rig.alt")
            .expect("second binding exists")
            .active
    );

    let activated = service
        .activate_provider_binding(ActivateAgentProviderBindingCommand {
            tenant_id: 1,
            agent_id: "agent.intelligence.rig-general".to_string(),
            binding_id: "binding.rig.default".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-04T00:03:00Z".to_string(),
        })
        .expect("binding can be activated");
    assert_eq!(activated.binding_id, "binding.rig.default");
    assert!(activated.active);
}

#[test]
fn provider_binding_rejects_non_standard_ids() {
    let mut service = service();
    create_rig_agent(&mut service);
    let long_id = format!("binding.{}", "a".repeat(128));

    for (binding_id, provider_id, profile_id, expected_message) in [
        (
            " binding.rig.default ",
            "provider.model.rig-rust",
            "profile.rig.local",
            "bindingId must not contain leading or trailing whitespace",
        ),
        (
            long_id.as_str(),
            "provider.model.rig-rust",
            "profile.rig.local",
            "bindingId must be at most 128 characters",
        ),
        (
            "binding.rig.default",
            "provider.model.Rig",
            "profile.rig.local",
            "providerId must use lowercase standard id characters",
        ),
        (
            "binding.rig.default",
            "model.rig-rust",
            "profile.rig.local",
            "providerId must start with provider.",
        ),
        (
            "binding.rig.default",
            "provider.model.rig-rust",
            "rig.local",
            "configurationProfileId must start with profile.",
        ),
    ] {
        let error = service
            .add_provider_binding(provider_binding_command(
                binding_id,
                provider_id,
                profile_id,
                vec!["model.chat"],
            ))
            .expect_err("invalid provider binding ids should fail");

        assert!(
            error.safe_message().contains(expected_message),
            "expected {expected_message}, got {}",
            error.safe_message()
        );
    }
}

#[test]
fn provider_binding_rejects_non_standard_capabilities() {
    let mut service = service();
    create_rig_agent(&mut service);

    for (capabilities, expected_message) in [
        (
            vec!["model.chat", ""],
            "capabilities must not contain empty capability ids",
        ),
        (
            vec!["model.chat", "model.chat"],
            "capabilities must not contain duplicate capability id: model.chat",
        ),
        (
            vec!["model.chat", "Tool.Invoke"],
            "capabilities must use lowercase namespaced capability ids",
        ),
        (
            vec!["model.chat", "chat"],
            "capabilities must use lowercase namespaced capability ids",
        ),
    ] {
        let error = service
            .add_provider_binding(provider_binding_command(
                "binding.rig.invalid-capability",
                "provider.model.rig-rust",
                "profile.rig.local",
                capabilities,
            ))
            .expect_err("invalid provider binding capabilities should fail");

        assert!(
            error.safe_message().contains(expected_message),
            "expected {expected_message}, got {}",
            error.safe_message()
        );
    }
}

#[test]
fn activating_current_default_binding_is_idempotent() {
    let mut service = service();
    create_rig_agent(&mut service);

    let created = service
        .add_provider_binding(AgentProviderBindingCommand {
            tenant_id: 1,
            agent_id: "agent.intelligence.rig-general".to_string(),
            binding_id: "binding.rig.default".to_string(),
            provider_id: "provider.model.rig-rust".to_string(),
            implementation_kind: AgentImplementationKind::TypedLocalProvider,
            configuration_profile_id: "profile.rig.local".to_string(),
            capabilities: vec!["model.chat".to_string()],
            make_default: true,
            requested_by: subject(),
            requested_at: "2026-06-04T00:01:00Z".to_string(),
        })
        .expect("default binding is added");

    let activated = service
        .activate_provider_binding(ActivateAgentProviderBindingCommand {
            tenant_id: 1,
            agent_id: "agent.intelligence.rig-general".to_string(),
            binding_id: "binding.rig.default".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-04T00:02:00Z".to_string(),
        })
        .expect("current default activation is idempotent");

    assert!(activated.active);
    assert_eq!(activated.version, created.version);
    assert_eq!(activated.updated_at, created.updated_at);
}

#[test]
fn listing_provider_bindings_and_deployments_requires_existing_agent() {
    let mut service = service();

    let binding_error = service
        .list_provider_bindings(1, "agent.missing", subject())
        .expect_err("missing agent binding list should fail");
    assert!(
        binding_error.safe_message().contains("agent not found"),
        "unexpected binding list error: {}",
        binding_error.safe_message()
    );

    let deployment_error = service
        .list_deployments(1, "agent.missing", subject())
        .expect_err("missing agent deployment list should fail");
    assert!(
        deployment_error.safe_message().contains("agent not found"),
        "unexpected deployment list error: {}",
        deployment_error.safe_message()
    );
}

#[test]
fn activating_provider_binding_requires_existing_agent() {
    let mut service = service();

    let error = service
        .activate_provider_binding(ActivateAgentProviderBindingCommand {
            tenant_id: 1,
            agent_id: "agent.missing".to_string(),
            binding_id: "binding.rig.default".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-04T00:03:00Z".to_string(),
        })
        .expect_err("missing agent binding activation should fail");

    assert!(
        error.safe_message().contains("agent not found"),
        "unexpected activation error: {}",
        error.safe_message()
    );
}

#[test]
fn provider_binding_activation_rejects_non_standard_binding_id() {
    let mut service = service();
    create_rig_agent(&mut service);

    let error = service
        .activate_provider_binding(ActivateAgentProviderBindingCommand {
            tenant_id: 1,
            agent_id: "agent.intelligence.rig-general".to_string(),
            binding_id: " binding.rig.default ".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-04T00:03:00Z".to_string(),
        })
        .expect_err("invalid activation binding id should fail");

    assert!(
        error
            .safe_message()
            .contains("bindingId must not contain leading or trailing whitespace"),
        "unexpected activation error: {}",
        error.safe_message()
    );
}

#[test]
fn duplicate_default_provider_binding_does_not_deactivate_existing_default() {
    let mut service = service();
    create_rig_agent(&mut service);
    service
        .add_provider_binding(AgentProviderBindingCommand {
            tenant_id: 1,
            agent_id: "agent.intelligence.rig-general".to_string(),
            binding_id: "binding.rig.default".to_string(),
            provider_id: "provider.model.rig-rust".to_string(),
            implementation_kind: AgentImplementationKind::TypedLocalProvider,
            configuration_profile_id: "profile.rig.local".to_string(),
            capabilities: vec!["model.chat".to_string()],
            make_default: true,
            requested_by: subject(),
            requested_at: "2026-06-04T00:01:00Z".to_string(),
        })
        .expect("default binding is added");

    let duplicate_error = service
        .add_provider_binding(AgentProviderBindingCommand {
            tenant_id: 1,
            agent_id: "agent.intelligence.rig-general".to_string(),
            binding_id: "binding.rig.default".to_string(),
            provider_id: "provider.model.rig-rust".to_string(),
            implementation_kind: AgentImplementationKind::TypedLocalProvider,
            configuration_profile_id: "profile.rig.local.duplicate".to_string(),
            capabilities: vec!["model.chat".to_string(), "tool.invoke".to_string()],
            make_default: true,
            requested_by: subject(),
            requested_at: "2026-06-04T00:02:00Z".to_string(),
        })
        .expect_err("duplicate binding should fail");
    assert!(
        duplicate_error
            .safe_message()
            .contains("agent provider binding already exists"),
        "unexpected duplicate error: {}",
        duplicate_error.safe_message()
    );

    let bindings = service
        .list_provider_bindings(1, "agent.intelligence.rig-general", subject())
        .expect("bindings list");
    assert_eq!(bindings.len(), 1);
    assert!(bindings[0].active);
    assert_eq!(bindings[0].binding_id, "binding.rig.default");
    assert_eq!(bindings[0].configuration_profile_id, "profile.rig.local");
}

#[test]
fn deployment_preserves_binding_snapshot_after_provider_switch() {
    let mut service = service();
    create_rig_agent(&mut service);
    service
        .add_provider_binding(AgentProviderBindingCommand {
            tenant_id: 1,
            agent_id: "agent.intelligence.rig-general".to_string(),
            binding_id: "binding.rig.default".to_string(),
            provider_id: "provider.model.rig-rust".to_string(),
            implementation_kind: AgentImplementationKind::TypedLocalProvider,
            configuration_profile_id: "profile.rig.local".to_string(),
            capabilities: vec!["model.chat".to_string()],
            make_default: true,
            requested_by: subject(),
            requested_at: "2026-06-04T00:01:00Z".to_string(),
        })
        .expect("binding is added");

    let deployment = service
        .create_deployment(AgentProviderDeploymentCommand {
            tenant_id: 1,
            agent_id: "agent.intelligence.rig-general".to_string(),
            deployment_id: "deployment.rig.1".to_string(),
            binding_id: "binding.rig.default".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-04T00:02:00Z".to_string(),
        })
        .expect("deployment is created");
    assert_eq!(deployment.provider_id_snapshot, "provider.model.rig-rust");

    service
        .add_provider_binding(AgentProviderBindingCommand {
            tenant_id: 1,
            agent_id: "agent.intelligence.rig-general".to_string(),
            binding_id: "binding.rig.alt".to_string(),
            provider_id: "provider.model.rig-alt".to_string(),
            implementation_kind: AgentImplementationKind::TypedLocalProvider,
            configuration_profile_id: "profile.rig.alt".to_string(),
            capabilities: vec!["model.chat".to_string()],
            make_default: true,
            requested_by: subject(),
            requested_at: "2026-06-04T00:03:00Z".to_string(),
        })
        .expect("alternate binding is added");

    let deployments = service
        .list_deployments(1, "agent.intelligence.rig-general", subject())
        .expect("deployments list");
    assert_eq!(
        deployments[0].provider_id_snapshot,
        "provider.model.rig-rust"
    );
    assert_eq!(
        deployments[0].configuration_profile_id_snapshot,
        "profile.rig.local"
    );
}

#[test]
fn deployment_rejects_non_standard_ids_before_snapshot_creation() {
    let mut service = service();
    create_rig_agent(&mut service);
    service
        .add_provider_binding(provider_binding_command(
            "binding.rig.default",
            "provider.model.rig-rust",
            "profile.rig.local",
            vec!["model.chat"],
        ))
        .expect("binding is added");

    for (deployment_id, binding_id, expected_message) in [
        (
            " deployment.rig.1 ",
            "binding.rig.default",
            "deploymentId must not contain leading or trailing whitespace",
        ),
        (
            "deployment.rig.1",
            " binding.rig.default ",
            "bindingId must not contain leading or trailing whitespace",
        ),
        (
            "deploy.rig.1",
            "binding.rig.default",
            "deploymentId must start with deployment.",
        ),
    ] {
        let error = service
            .create_deployment(deployment_command(deployment_id, binding_id))
            .expect_err("invalid deployment ids should fail");

        assert!(
            error.safe_message().contains(expected_message),
            "expected {expected_message}, got {}",
            error.safe_message()
        );
    }
}
