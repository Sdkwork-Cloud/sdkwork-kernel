use std::sync::{Arc, Mutex};

use sdkwork_agent_business::{
    AgentAuditSink, AgentBusinessService, AgentBusinessStatus, AgentListQuery, AgentVisibility,
    AllowAllPolicyProvider, ChangeAgentStatusCommand, CreateAgentCommand,
    DEFAULT_AGENT_MANAGEMENT_POLICY_CATEGORY, DeleteAgentCommand, GetAgentCommand,
    InMemoryAgentRepository, ListAgentsCommand, PolicyMode, RestoreAgentCommand,
    UpdateAgentCommand,
};
use sdkwork_agent_kernel::{AgentManifest, KernelError, KernelEvent, KernelResult, PolicySubject};
use sdkwork_code_kernel::CodeTaskIntent;

#[derive(Clone, Default)]
struct RecordingAuditSink {
    events: Arc<Mutex<Vec<KernelEvent>>>,
}

impl RecordingAuditSink {
    fn new() -> (Self, Arc<Mutex<Vec<KernelEvent>>>) {
        let events = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                events: events.clone(),
            },
            events,
        )
    }
}

impl AgentAuditSink for RecordingAuditSink {
    fn record(&mut self, event: KernelEvent) -> KernelResult<()> {
        self.events
            .lock()
            .expect("recording audit mutex poisoned")
            .push(event);
        Ok(())
    }

    fn list_events(&self, tenant_id: u64, agent_id: &str) -> KernelResult<Vec<KernelEvent>> {
        let tenant_pattern = format!("tenant_id={tenant_id};");
        let agent_pattern = format!("agent_id={agent_id};");
        let mut events = self
            .events
            .lock()
            .expect("recording audit mutex poisoned")
            .iter()
            .filter(|event| {
                event.payload.contains(tenant_pattern.as_str())
                    && event.payload.contains(agent_pattern.as_str())
            })
            .cloned()
            .collect::<Vec<_>>();
        events.sort_by(|left, right| right.occurred_at.cmp(&left.occurred_at));
        Ok(events)
    }
}

fn sample_manifest(agent_id: &str) -> AgentManifest {
    AgentManifest {
        schema_version: "1.0.0".to_string(),
        manifest_type: "agent".to_string(),
        agent_id: agent_id.to_string(),
        name: "sample-agent".to_string(),
        display_name: "Sample Agent".to_string(),
        description: "sample".to_string(),
        version: "0.1.0".to_string(),
        domain: "intelligence".to_string(),
        required_capabilities: vec!["model.chat".to_string()],
        optional_capabilities: vec!["tool.invoke".to_string()],
        required_capability_requirements: vec![],
        optional_capability_requirements: vec![],
        event_families: vec!["agent.lifecycle".to_string()],
        owner_name: "sdkwork".to_string(),
        status: "active".to_string(),
    }
}

fn sample_subject() -> PolicySubject {
    PolicySubject::new("u-1", "t-1").with_role("agent.admin")
}

fn create_agent_cmd(
    agent_id: &str,
    tenant_id: u64,
    organization_id: u64,
    owner_user_id: u64,
    code: &str,
    display_name: &str,
    requested_at: &str,
) -> CreateAgentCommand {
    CreateAgentCommand {
        agent_id: agent_id.to_string(),
        tenant_id,
        organization_id,
        owner_user_id,
        code: code.to_string(),
        display_name: display_name.to_string(),
        description: Some("sample".to_string()),
        manifest: sample_manifest(agent_id),
        visibility: AgentVisibility::Organization,
        tags: vec!["starter".to_string()],
        default_code_task_intent: Some(CodeTaskIntent::new("Refactor runtime")),
        requested_by: sample_subject(),
        requested_at: requested_at.to_string(),
    }
}

fn assert_structured_kind(error: KernelError, expected_kind: &str) {
    match error {
        KernelError::Structured { info } => {
            assert_eq!(info.kind.as_str(), expected_kind);
        }
        _ => panic!("expected structured error"),
    }
}

#[test]
fn create_update_status_delete_restore_and_list_agents() {
    let repository = InMemoryAgentRepository::new();
    let (audit_sink, _events) = RecordingAuditSink::new();
    let policy_provider = AllowAllPolicyProvider::allow("policy.memory");
    let mut service = AgentBusinessService::new(repository, audit_sink, policy_provider);

    let create = service
        .create_agent(create_agent_cmd(
            "agent.alpha",
            1,
            10,
            100,
            "alpha",
            "Alpha",
            "2026-06-01T00:00:00Z",
        ))
        .expect("create should succeed");
    assert_eq!(create.status, AgentBusinessStatus::Draft);
    assert_eq!(create.visibility, AgentVisibility::Organization);
    assert_eq!(create.id, 1);

    let updated = service
        .update_agent(UpdateAgentCommand {
            tenant_id: 1,
            agent_id: "agent.alpha".to_string(),
            display_name: Some("Alpha v2".to_string()),
            description: Some("updated".to_string()),
            visibility: Some(AgentVisibility::Tenant),
            tags: Some(vec!["starter".to_string(), "v2".to_string()]),
            default_code_task_intent: Some(CodeTaskIntent::new("Write tests first")),
            requested_by: sample_subject(),
            requested_at: "2026-06-01T00:10:00Z".to_string(),
        })
        .expect("update should succeed");
    assert_eq!(updated.display_name, "Alpha v2");
    assert_eq!(updated.visibility, AgentVisibility::Tenant);

    let activated = service
        .change_status(ChangeAgentStatusCommand {
            tenant_id: 1,
            agent_id: "agent.alpha".to_string(),
            target_status: AgentBusinessStatus::Active,
            requested_by: sample_subject(),
            requested_at: "2026-06-01T00:15:00Z".to_string(),
        })
        .expect("status transition should succeed");
    assert_eq!(activated.status, AgentBusinessStatus::Active);

    let deleted = service
        .delete_agent(DeleteAgentCommand {
            tenant_id: 1,
            agent_id: "agent.alpha".to_string(),
            requested_by: sample_subject(),
            requested_at: "2026-06-01T00:16:00Z".to_string(),
        })
        .expect("delete should succeed");
    assert_eq!(deleted.status, AgentBusinessStatus::Deleted);
    assert!(deleted.deleted_at.is_some());

    let restored = service
        .restore_agent(RestoreAgentCommand {
            tenant_id: 1,
            agent_id: "agent.alpha".to_string(),
            requested_by: sample_subject(),
            requested_at: "2026-06-01T00:17:00Z".to_string(),
        })
        .expect("restore should succeed");
    assert_eq!(restored.status, AgentBusinessStatus::Active);
    assert!(restored.deleted_at.is_none());

    let got = service
        .get_agent(GetAgentCommand {
            tenant_id: 1,
            agent_id: "agent.alpha".to_string(),
            requested_by: sample_subject(),
        })
        .expect("retrieve should succeed");
    assert_eq!(got.agent_id, "agent.alpha");
    assert_eq!(got.version, restored.version);

    let listed = service
        .list_agents(ListAgentsCommand {
            query: AgentListQuery::for_tenant(1).for_organization(10),
            requested_by: sample_subject(),
        })
        .expect("list should succeed");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].status, AgentBusinessStatus::Active);
}

#[test]
fn duplicate_agent_id_and_code_are_rejected() {
    let repository = InMemoryAgentRepository::new();
    let (audit_sink, _events) = RecordingAuditSink::new();
    let policy_provider = AllowAllPolicyProvider::allow("policy.memory");
    let mut service = AgentBusinessService::new(repository, audit_sink, policy_provider);

    service
        .create_agent(create_agent_cmd(
            "agent.alpha",
            1,
            10,
            100,
            "alpha",
            "Alpha",
            "2026-06-01T01:00:00Z",
        ))
        .expect("first create should succeed");

    let duplicate_agent_id = service
        .create_agent(create_agent_cmd(
            "agent.alpha",
            1,
            10,
            100,
            "alpha-v2",
            "Alpha Dup",
            "2026-06-01T01:10:00Z",
        ))
        .expect_err("same agent_id in tenant must fail");
    assert_structured_kind(duplicate_agent_id, "conflict");

    let duplicate_code = service
        .create_agent(create_agent_cmd(
            "agent.beta",
            1,
            10,
            101,
            "alpha",
            "Beta",
            "2026-06-01T01:20:00Z",
        ))
        .expect_err("same code in tenant must fail");
    assert_structured_kind(duplicate_code, "conflict");
}

#[test]
fn deleted_agent_cannot_be_updated() {
    let repository = InMemoryAgentRepository::new();
    let (audit_sink, _events) = RecordingAuditSink::new();
    let policy_provider = AllowAllPolicyProvider::allow("policy.memory");
    let mut service = AgentBusinessService::new(repository, audit_sink, policy_provider);

    service
        .create_agent(create_agent_cmd(
            "agent.delta",
            1,
            10,
            100,
            "delta",
            "Delta",
            "2026-06-01T02:00:00Z",
        ))
        .expect("create should succeed");

    service
        .delete_agent(DeleteAgentCommand {
            tenant_id: 1,
            agent_id: "agent.delta".to_string(),
            requested_by: sample_subject(),
            requested_at: "2026-06-01T02:05:00Z".to_string(),
        })
        .expect("delete should succeed");

    let result = service.update_agent(UpdateAgentCommand {
        tenant_id: 1,
        agent_id: "agent.delta".to_string(),
        display_name: Some("Delta v2".to_string()),
        description: None,
        visibility: None,
        tags: None,
        default_code_task_intent: None,
        requested_by: sample_subject(),
        requested_at: "2026-06-01T02:06:00Z".to_string(),
    });

    let error = result.expect_err("deleted agent should not allow updates");
    match error {
        KernelError::Validation { message } => {
            assert!(message.contains("deleted agent cannot be updated"));
        }
        _ => panic!("expected validation error"),
    }
}

#[test]
fn restore_requires_deleted_status() {
    let repository = InMemoryAgentRepository::new();
    let (audit_sink, _events) = RecordingAuditSink::new();
    let policy_provider = AllowAllPolicyProvider::allow("policy.memory");
    let mut service = AgentBusinessService::new(repository, audit_sink, policy_provider);

    service
        .create_agent(create_agent_cmd(
            "agent.epsilon",
            1,
            10,
            100,
            "epsilon",
            "Epsilon",
            "2026-06-01T03:00:00Z",
        ))
        .expect("create should succeed");

    let result = service.restore_agent(RestoreAgentCommand {
        tenant_id: 1,
        agent_id: "agent.epsilon".to_string(),
        requested_by: sample_subject(),
        requested_at: "2026-06-01T03:01:00Z".to_string(),
    });

    let error = result.expect_err("restore without delete should fail");
    match error {
        KernelError::Validation { message } => {
            assert!(message.contains("agent is not deleted"));
        }
        _ => panic!("expected validation error"),
    }
}

#[test]
fn list_filters_by_owner_organization_and_deleted_flag() {
    let repository = InMemoryAgentRepository::new();
    let (audit_sink, _events) = RecordingAuditSink::new();
    let policy_provider = AllowAllPolicyProvider::allow("policy.memory");
    let mut service = AgentBusinessService::new(repository, audit_sink, policy_provider);

    service
        .create_agent(create_agent_cmd(
            "agent.owner.a",
            1,
            10,
            100,
            "owner-a",
            "Owner A",
            "2026-06-01T04:00:00Z",
        ))
        .expect("create owner a should succeed");
    service
        .create_agent(create_agent_cmd(
            "agent.owner.b",
            1,
            11,
            101,
            "owner-b",
            "Owner B",
            "2026-06-01T04:01:00Z",
        ))
        .expect("create owner b should succeed");

    service
        .delete_agent(DeleteAgentCommand {
            tenant_id: 1,
            agent_id: "agent.owner.b".to_string(),
            requested_by: sample_subject(),
            requested_at: "2026-06-01T04:02:00Z".to_string(),
        })
        .expect("delete owner b should succeed");

    let by_org = service
        .list_agents(ListAgentsCommand {
            query: AgentListQuery::for_tenant(1).for_organization(10),
            requested_by: sample_subject(),
        })
        .expect("list by org should succeed");
    assert_eq!(by_org.len(), 1);
    assert_eq!(by_org[0].agent_id, "agent.owner.a");

    let by_owner_without_deleted = service
        .list_agents(ListAgentsCommand {
            query: AgentListQuery::for_tenant(1).for_owner(101),
            requested_by: sample_subject(),
        })
        .expect("list by owner should succeed");
    assert!(by_owner_without_deleted.is_empty());

    let by_owner_with_deleted = service
        .list_agents(ListAgentsCommand {
            query: AgentListQuery::for_tenant(1).for_owner(101).with_deleted(),
            requested_by: sample_subject(),
        })
        .expect("list by owner with deleted should succeed");
    assert_eq!(by_owner_with_deleted.len(), 1);
    assert_eq!(by_owner_with_deleted[0].status, AgentBusinessStatus::Deleted);
}

#[test]
fn list_filters_by_search_query_across_code_name_and_description() {
    let repository = InMemoryAgentRepository::new();
    let (audit_sink, _events) = RecordingAuditSink::new();
    let policy_provider = AllowAllPolicyProvider::allow("policy.memory");
    let mut service = AgentBusinessService::new(repository, audit_sink, policy_provider);

    service
        .create_agent(create_agent_cmd(
            "agent.search.alpha",
            1,
            10,
            100,
            "alpha-code",
            "Alpha Worker",
            "2026-06-01T06:00:00Z",
        ))
        .expect("create alpha should succeed");

    service
        .create_agent(CreateAgentCommand {
            description: Some("handles retrieval workloads".to_string()),
            ..create_agent_cmd(
                "agent.search.beta",
                1,
                10,
                101,
                "beta-code",
                "Beta Agent",
                "2026-06-01T06:01:00Z",
            )
        })
        .expect("create beta should succeed");

    let by_code = service
        .list_agents(ListAgentsCommand {
            query: AgentListQuery::for_tenant(1).with_search("alpha-code"),
            requested_by: sample_subject(),
        })
        .expect("list by code search should succeed");
    assert_eq!(by_code.len(), 1);
    assert_eq!(by_code[0].agent_id, "agent.search.alpha");

    let by_name = service
        .list_agents(ListAgentsCommand {
            query: AgentListQuery::for_tenant(1).with_search("beta"),
            requested_by: sample_subject(),
        })
        .expect("list by display name search should succeed");
    assert_eq!(by_name.len(), 1);
    assert_eq!(by_name[0].agent_id, "agent.search.beta");

    let by_description = service
        .list_agents(ListAgentsCommand {
            query: AgentListQuery::for_tenant(1).with_search("retrieval"),
            requested_by: sample_subject(),
        })
        .expect("list by description search should succeed");
    assert_eq!(by_description.len(), 1);
    assert_eq!(by_description[0].agent_id, "agent.search.beta");
}

#[test]
fn audit_events_are_recorded_for_state_mutations() {
    let repository = InMemoryAgentRepository::new();
    let (audit_sink, events) = RecordingAuditSink::new();
    let policy_provider = AllowAllPolicyProvider::allow("policy.memory");
    let mut service = AgentBusinessService::new(repository, audit_sink, policy_provider);

    service
        .create_agent(create_agent_cmd(
            "agent.audit",
            1,
            10,
            100,
            "audit",
            "Audit",
            "2026-06-01T05:00:00Z",
        ))
        .expect("create should succeed");
    service
        .update_agent(UpdateAgentCommand {
            tenant_id: 1,
            agent_id: "agent.audit".to_string(),
            display_name: Some("Audit v2".to_string()),
            description: None,
            visibility: None,
            tags: None,
            default_code_task_intent: None,
            requested_by: sample_subject(),
            requested_at: "2026-06-01T05:01:00Z".to_string(),
        })
        .expect("update should succeed");
    service
        .change_status(ChangeAgentStatusCommand {
            tenant_id: 1,
            agent_id: "agent.audit".to_string(),
            target_status: AgentBusinessStatus::Active,
            requested_by: sample_subject(),
            requested_at: "2026-06-01T05:02:00Z".to_string(),
        })
        .expect("status update should succeed");
    service
        .delete_agent(DeleteAgentCommand {
            tenant_id: 1,
            agent_id: "agent.audit".to_string(),
            requested_by: sample_subject(),
            requested_at: "2026-06-01T05:03:00Z".to_string(),
        })
        .expect("delete should succeed");
    service
        .restore_agent(RestoreAgentCommand {
            tenant_id: 1,
            agent_id: "agent.audit".to_string(),
            requested_by: sample_subject(),
            requested_at: "2026-06-01T05:04:00Z".to_string(),
        })
        .expect("restore should succeed");

    let captured = events.lock().expect("events mutex poisoned");
    assert_eq!(captured.len(), 5);
    let event_types: Vec<&str> = captured.iter().map(|event| event.event_type.as_str()).collect();
    assert_eq!(
        event_types,
        vec![
            "agent.business.created",
            "agent.business.updated",
            "agent.business.status_changed",
            "agent.business.deleted",
            "agent.business.restored",
        ]
    );
}

#[test]
fn policy_deny_blocks_management_operations() {
    let repository = InMemoryAgentRepository::new();
    let (audit_sink, _events) = RecordingAuditSink::new();
    let policy_provider = AllowAllPolicyProvider {
        provider_id: "policy.memory".to_string(),
        mode: PolicyMode::Deny("agent.business.denied".to_string()),
    };
    let mut service = AgentBusinessService::new(repository, audit_sink, policy_provider);

    let result = service.create_agent(create_agent_cmd(
        "agent.beta",
        1,
        10,
        100,
        "beta",
        "Beta",
        "2026-06-01T01:00:00Z",
    ));

    let error = result.expect_err("denied policy should block create");
    match error {
        KernelError::Structured { info } => {
            assert_eq!(info.kind.as_str(), "permission_required");
        }
        KernelError::Internal { .. }
        | KernelError::Validation { .. }
        | KernelError::CapabilityMissing { .. }
        | KernelError::ProviderUnavailable { .. }
        | KernelError::PolicyDenied { .. } => {
            panic!("expected permission_required structured error")
        }
    }
}

#[test]
fn invalid_status_transition_is_rejected() {
    let repository = InMemoryAgentRepository::new();
    let (audit_sink, _events) = RecordingAuditSink::new();
    let policy_provider = AllowAllPolicyProvider::allow("policy.memory");
    let mut service = AgentBusinessService::new(repository, audit_sink, policy_provider);

    service
        .create_agent(create_agent_cmd(
            "agent.gamma",
            1,
            10,
            100,
            "gamma",
            "Gamma",
            "2026-06-01T02:00:00Z",
        ))
        .expect("create should succeed");

    let result = service.change_status(ChangeAgentStatusCommand {
        tenant_id: 1,
        agent_id: "agent.gamma".to_string(),
        target_status: AgentBusinessStatus::Disabled,
        requested_by: sample_subject(),
        requested_at: "2026-06-01T02:10:00Z".to_string(),
    });
    let error = result.expect_err("draft -> disabled should fail");
    match error {
        KernelError::Validation { message } => {
            assert!(message.contains("invalid agent status transition"));
        }
        _ => panic!("expected validation error"),
    }
}

#[test]
fn policy_category_constant_is_sdkwork_agent_business_manage() {
    assert_eq!(
        DEFAULT_AGENT_MANAGEMENT_POLICY_CATEGORY,
        "agent.business.manage"
    );
}

#[test]
fn list_agent_audit_events_returns_events_for_agent() {
    let repository = InMemoryAgentRepository::new();
    let (audit_sink, _events) = RecordingAuditSink::new();
    let policy_provider = AllowAllPolicyProvider::allow("policy.memory");
    let mut service = AgentBusinessService::new(repository, audit_sink, policy_provider);

    service
        .create_agent(create_agent_cmd(
            "agent.audit.list",
            1,
            10,
            100,
            "audit-list",
            "Audit List",
            "2026-06-01T04:00:00Z",
        ))
        .expect("create should succeed");

    service
        .change_status(ChangeAgentStatusCommand {
            tenant_id: 1,
            agent_id: "agent.audit.list".to_string(),
            target_status: AgentBusinessStatus::Active,
            requested_by: sample_subject(),
            requested_at: "2026-06-01T04:05:00Z".to_string(),
        })
        .expect("status transition should succeed");

    let events = service
        .list_agent_audit_events(1, "agent.audit.list", sample_subject())
        .expect("list audit events should succeed");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_type, "agent.business.status_changed");
    assert_eq!(events[1].event_type, "agent.business.created");
}
