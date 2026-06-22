use sdkwork_agent_adapter_core::SessionConfig;
use sdkwork_agent_kernel::{SessionKind, SessionSource};
use serde_json::Value;

const COMPONENT_SPEC: &str = include_str!("../specs/component.spec.json");

#[test]
fn component_spec_matches_crate_identity() {
    let spec: Value = serde_json::from_str(COMPONENT_SPEC).expect("valid component spec");
    assert_eq!(spec["component"]["name"], "sdkwork-agent-adapter-core");
    assert_eq!(spec["kind"], "sdkwork.component.spec");
}

#[test]
fn session_config_builder_preserves_kernel_session_fields() {
    let config = SessionConfig::new()
        .with_title("Adapter contract")
        .with_model("test-model")
        .with_source(SessionSource::Cli)
        .with_kind(SessionKind::Main);

    assert_eq!(config.title.as_deref(), Some("Adapter contract"));
    assert_eq!(config.model.as_deref(), Some("test-model"));
    assert_eq!(config.source, SessionSource::Cli);
    assert_eq!(config.kind, SessionKind::Main);
}
