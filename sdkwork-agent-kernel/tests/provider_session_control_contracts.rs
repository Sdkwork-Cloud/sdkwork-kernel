use sdkwork_agent_kernel::{
    AgentManifest, KernelError, ProviderHealth, ProviderManifest, ProviderSessionControlAction,
    ProviderSessionControlActionKind, ProviderSessionControlOutput, ProviderSessionControlProvider,
    ProviderSessionControlRequest, ProviderSessionControlResult, RuntimeBuilder, RuntimeState,
};

const SESSION_CONTROL_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.test.session-control",
  "name": "session-control-test",
  "display_name": "Session Control Test",
  "description": "Agent used to prove provider session control registration.",
  "version": "0.1.0",
  "domain": "intelligence",
  "required_capabilities": [],
  "optional_capabilities": [
    { "capability_id": "session.control.interrupt", "min_version": "0.1.0" },
    { "capability_id": "session.control.compact", "min_version": "0.1.0" },
    { "capability_id": "session.control.fork", "min_version": "0.1.0" }
  ],
  "event_families": ["agent.session.*"],
  "owner": { "name": "sdkwork-platform" },
  "status": "candidate"
}
"#;

struct FakeSessionControlProvider;

impl ProviderSessionControlProvider for FakeSessionControlProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.test.session-control",
            "session_control",
            "Test session control",
            "0.1.0",
            vec![
                "session.control.interrupt".to_string(),
                "session.control.compact".to_string(),
                "session.control.fork".to_string(),
            ],
        )
    }

    fn control(
        &self,
        request: ProviderSessionControlRequest,
    ) -> Result<ProviderSessionControlResult, KernelError> {
        request.validate()?;
        match &request.action {
            ProviderSessionControlAction::Fork { .. } => Ok(ProviderSessionControlResult::forked(
                &request,
                "provider-session-forked",
            )),
            _ => Ok(ProviderSessionControlResult::acknowledged(&request)),
        }
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

#[test]
fn provider_session_control_preserves_both_session_identities_and_policy_evidence() {
    let provider = FakeSessionControlProvider;
    let request = ProviderSessionControlRequest::new(
        "control-1",
        "session-kernel",
        "provider-session-source",
        "policy-decision-1",
        ProviderSessionControlAction::Fork {
            before_message_id: Some("message-9".to_string()),
        },
    )
    .with_timeout_ms(5_000);

    let result = provider.control(request).expect("valid control request");

    assert_eq!(result.control_request_id, "control-1");
    assert_eq!(result.session_id, "session-kernel");
    assert_eq!(result.provider_session_id, "provider-session-source");
    assert_eq!(result.action, ProviderSessionControlActionKind::Fork);
    assert_eq!(
        result.output,
        ProviderSessionControlOutput::Forked {
            provider_session_id: "provider-session-forked".to_string(),
        }
    );
}

#[test]
fn provider_session_control_rejects_missing_provider_identity_or_policy_evidence() {
    let valid = ProviderSessionControlRequest::new(
        "control-2",
        "session-kernel",
        "provider-session",
        "policy-decision-2",
        ProviderSessionControlAction::Interrupt { reason: None },
    );

    let mut missing_provider_session = valid.clone();
    missing_provider_session.provider_session_id.clear();
    assert!(missing_provider_session.validate().is_err());

    let mut missing_policy = valid;
    missing_policy.policy_decision_id.clear();
    assert!(missing_policy.validate().is_err());
}

#[test]
fn provider_session_control_action_kinds_are_stable() {
    assert_eq!(
        ProviderSessionControlAction::Interrupt { reason: None }
            .kind()
            .as_str(),
        "interrupt"
    );
    assert_eq!(
        ProviderSessionControlAction::Compact { focus: None }
            .kind()
            .as_str(),
        "compact"
    );
    assert_eq!(
        ProviderSessionControlAction::Fork {
            before_message_id: None,
        }
        .kind()
        .as_str(),
        "fork"
    );
}

#[test]
fn runtime_registers_multiple_typed_session_control_providers() {
    let manifest = AgentManifest::from_json(SESSION_CONTROL_AGENT_MANIFEST_JSON)
        .expect("session control agent manifest");
    let report = RuntimeBuilder::new("runtime.session-control", manifest)
        .register_provider_session_control_provider(
            "provider.session-control.first",
            "0.1.0",
            FakeSessionControlProvider,
        )
        .register_provider_session_control_provider(
            "provider.session-control.second",
            "0.1.0",
            FakeSessionControlProvider,
        )
        .bootstrap()
        .expect("runtime bootstraps");

    assert_eq!(report.runtime.state(), RuntimeState::Ready);
    assert_eq!(
        report.runtime.provider_session_control_provider_ids(),
        [
            "provider.session-control.first",
            "provider.session-control.second"
        ]
    );
    assert_eq!(
        report
            .runtime
            .provider_session_control_provider_by_id("provider.session-control.second")
            .expect("second provider")
            .provider_manifest()
            .provider_family,
        "session_control"
    );
    let diagnostic = report
        .runtime
        .diagnostics()
        .provider("provider.session-control.first")
        .expect("typed provider diagnostic")
        .clone();
    assert!(diagnostic.typed_registered);
    assert!(diagnostic.health.expect("provider health").is_available());
}
