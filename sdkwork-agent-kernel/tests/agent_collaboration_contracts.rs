use sdkwork_agent_kernel::{
    AgentCard, AgentCollaborationProvider, AgentDelegation, AgentDelegationRequest,
    AgentDelegationResult, AgentHandoffRequest, AgentHandoffResult, AgentManifest, AgentMessage,
    AgentMessageRole, AgentPart, KernelResult, ProviderHealth, ProviderManifest,
    RedactionClassification, RuntimeBuilder, RuntimeState, TraceContext, TrustLevel,
};

const COLLABORATIVE_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.code.orchestrator",
  "name": "sdkwork-code-orchestrator",
  "display_name": "SDKWork Code Orchestrator",
  "description": "Agent used to prove collaboration and handoff SPI contracts.",
  "version": "0.1.0",
  "domain": "code",
  "required_capabilities": [
    {
      "capability_id": "agent.discover",
      "min_version": "0.1.0"
    },
    {
      "capability_id": "agent.handoff",
      "min_version": "0.1.0"
    },
    {
      "capability_id": "agent.delegate",
      "min_version": "0.1.0"
    }
  ],
  "optional_capabilities": [],
  "event_families": ["agent.runtime.*", "agent.collaboration.*"],
  "owner": {
    "name": "sdkwork-platform"
  },
  "status": "candidate"
}
"#;

#[test]
fn agent_card_declares_discovery_endpoint_capabilities_and_trust_boundary() {
    let card = AgentCard::new(
        "agent.code.review",
        "Code Review Agent",
        "Reviews patches and returns risks.",
        "0.1.0",
    )
    .with_endpoint("a2a://agents/code-review")
    .with_capability("code.review")
    .with_capability("artifact.patch.read")
    .with_input_mode("text")
    .with_output_mode("artifact")
    .with_provider_id("provider.collaboration.local")
    .with_trust_level(TrustLevel::TrustedHost)
    .with_metadata("sdkwork.agent.routing.priority", "5");

    assert_eq!(card.agent_id, "agent.code.review");
    assert_eq!(card.endpoint.as_deref(), Some("a2a://agents/code-review"));
    assert_eq!(card.capabilities, ["code.review", "artifact.patch.read"]);
    assert_eq!(card.input_modes, ["text"]);
    assert_eq!(card.output_modes, ["artifact"]);
    assert_eq!(
        card.metadata_value("sdkwork.agent.routing.priority"),
        Some("5")
    );
    assert!(!card.is_untrusted());
}

#[test]
fn handoff_request_preserves_context_policy_trace_and_input_filtering() {
    let request = AgentHandoffRequest::new(
        "handoff.1",
        "agent.code.orchestrator",
        "agent.code.review",
        "Review this patch.",
    )
    .for_session("session.1")
    .for_task("task.1")
    .for_run("run.1")
    .for_step("step.1")
    .with_message(AgentMessage::new(
        "message.1",
        AgentMessageRole::User,
        vec![AgentPart::text("part.1", "Please review diff")],
    ))
    .with_context_frame("context.diff.1")
    .with_artifact("artifact.patch.1")
    .with_policy_context("policy.handoff.1")
    .with_trace_context(TraceContext::new("trace.1", "span.handoff"))
    .with_input_filter("drop_untrusted_tool_output")
    .with_metadata("sdkwork.handoff.reason", "specialist");

    assert_eq!(request.session_id.as_deref(), Some("session.1"));
    assert_eq!(request.task_id.as_deref(), Some("task.1"));
    assert_eq!(request.run_id.as_deref(), Some("run.1"));
    assert_eq!(request.step_id.as_deref(), Some("step.1"));
    assert_eq!(request.messages[0].message_id, "message.1");
    assert_eq!(request.context_frame_ids, ["context.diff.1"]);
    assert_eq!(request.artifact_ids, ["artifact.patch.1"]);
    assert_eq!(
        request.policy_context_id.as_deref(),
        Some("policy.handoff.1")
    );
    assert_eq!(
        request.trace_context.as_ref().expect("trace").span_id,
        "span.handoff"
    );
    assert_eq!(
        request.input_filter.as_deref(),
        Some("drop_untrusted_tool_output")
    );
    assert_eq!(
        request.metadata_value("sdkwork.handoff.reason"),
        Some("specialist")
    );
}

#[test]
fn collaboration_provider_lists_agents_hands_off_and_records_delegation() {
    let provider = FakeCollaborationProvider::new(
        "provider.collaboration.local",
        "agent.code.review",
        "Code Review Agent",
        "code.review",
    );

    let agents = provider.list_agents();
    assert_eq!(agents[0].agent_id, "agent.code.review");
    assert_eq!(agents[0].capabilities, ["code.review"]);

    let card = provider
        .describe_agent("agent.code.review")
        .expect("agent card exists");
    assert_eq!(card.display_name, "Code Review Agent");

    let result = provider
        .handoff(
            AgentHandoffRequest::new(
                "handoff.1",
                "agent.code.orchestrator",
                "agent.code.review",
                "Review this patch.",
            )
            .with_trace_context(TraceContext::new("trace.1", "span.handoff")),
        )
        .expect("handoff succeeds");
    assert_eq!(result.handoff_id, "handoff.1");
    assert_eq!(result.delegation.target_agent_id, "agent.code.review");
    assert_eq!(result.status, "accepted");
    assert_eq!(result.messages[0].role, AgentMessageRole::Agent);
    assert_eq!(
        result.trace_context.as_ref().expect("trace").span_id,
        "span.handoff.accepted"
    );
}

#[test]
fn runtime_registry_supports_multiple_collaboration_providers() {
    let manifest = AgentManifest::from_json(COLLABORATIVE_AGENT_MANIFEST_JSON)
        .expect("collaborative agent manifest parses");
    let report = RuntimeBuilder::new("runtime.collaboration", manifest)
        .with_generated_at("2026-05-30T00:00:00Z")
        .register_collaboration_provider(
            "provider.collaboration.local",
            "0.1.0",
            FakeCollaborationProvider::new(
                "provider.collaboration.local",
                "agent.code.review",
                "Code Review Agent",
                "code.review",
            ),
        )
        .register_collaboration_provider(
            "provider.collaboration.remote",
            "0.1.0",
            FakeCollaborationProvider::new(
                "provider.collaboration.remote",
                "agent.security.review",
                "Security Review Agent",
                "security.review",
            ),
        )
        .bootstrap()
        .expect("runtime bootstraps");

    assert_eq!(report.runtime.state(), RuntimeState::Ready);
    assert_eq!(
        report.runtime.collaboration_provider_ids(),
        [
            "provider.collaboration.local",
            "provider.collaboration.remote"
        ]
    );
    assert_eq!(
        report
            .runtime
            .collaboration_provider()
            .expect("default collaboration provider")
            .list_agents()[0]
            .agent_id,
        "agent.code.review"
    );
    assert_eq!(
        report
            .runtime
            .collaboration_provider_by_id("provider.collaboration.remote")
            .expect("collaboration provider by id")
            .list_agents()[0]
            .agent_id,
        "agent.security.review"
    );

    let manifest = report.runtime.capability_manifest();
    assert!(manifest
        .providers
        .iter()
        .any(|provider| provider.provider_family == "collaboration"
            && provider.provider_id == "provider.collaboration.local"));
    assert!(manifest
        .providers
        .iter()
        .any(|provider| provider.provider_family == "collaboration"
            && provider.provider_id == "provider.collaboration.remote"));
    assert_eq!(
        manifest
            .providers
            .iter()
            .find(|provider| provider.provider_id == "provider.collaboration.remote")
            .expect("remote collaboration provider manifest exists")
            .capabilities,
        ["agent.discover"]
    );
    assert!(manifest
        .capabilities
        .iter()
        .any(|capability| capability.capability_id == "agent.handoff"
            && capability.provider_id == "provider.collaboration.local"));
}

struct FakeCollaborationProvider {
    provider_id: &'static str,
    agent_id: &'static str,
    display_name: &'static str,
    capability: &'static str,
}

impl FakeCollaborationProvider {
    fn new(
        provider_id: &'static str,
        agent_id: &'static str,
        display_name: &'static str,
        capability: &'static str,
    ) -> Self {
        Self {
            provider_id,
            agent_id,
            display_name,
            capability,
        }
    }
}

impl AgentCollaborationProvider for FakeCollaborationProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        let capabilities = if self.provider_id == "provider.collaboration.remote" {
            vec!["agent.discover".to_string()]
        } else {
            vec![
                "agent.discover".to_string(),
                "agent.handoff".to_string(),
                "agent.delegate".to_string(),
            ]
        };

        ProviderManifest::new(
            self.provider_id,
            "collaboration",
            self.provider_id,
            "0.1.0",
            capabilities,
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn list_agents(&self) -> Vec<AgentCard> {
        vec![AgentCard::new(
            self.agent_id,
            self.display_name,
            "Reviews patches and returns risks.",
            "0.1.0",
        )
        .with_capability(self.capability)
        .with_provider_id(self.provider_id)]
    }

    fn handoff(&self, request: AgentHandoffRequest) -> KernelResult<AgentHandoffResult> {
        Ok(AgentHandoffResult::accepted(
            request.handoff_id.clone(),
            AgentDelegation::new(
                "delegation.1",
                request.source_agent_id,
                request.target_agent_id,
                "code.review",
            )
            .with_policy_context("policy.handoff.1")
            .with_redaction(RedactionClassification::Internal),
        )
        .with_message(AgentMessage::new(
            "message.review.accepted",
            AgentMessageRole::Agent,
            vec![AgentPart::text("part.accepted", "Review accepted")],
        ))
        .with_trace_context(TraceContext::new("trace.1", "span.handoff.accepted")))
    }

    fn delegate(&self, request: AgentDelegationRequest) -> KernelResult<AgentDelegationResult> {
        Ok(AgentDelegationResult::accepted(
            request.delegation_id.clone(),
            AgentDelegation::new(
                "delegation.1",
                request.source_agent_id,
                request.target_agent_id,
                "code.review",
            )
            .with_policy_context("policy.delegate.1")
            .with_redaction(RedactionClassification::Internal),
        ))
    }
}
