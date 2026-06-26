use sdkwork_agent_provider_codex::CodexModelProvider;
use sdkwork_agent_kernel::ModelProvider;
use serde_json::Value;

const COMPONENT_SPEC: &str = include_str!("../specs/component.spec.json");

#[test]
fn component_spec_matches_crate_identity() {
    let spec: Value = serde_json::from_str(COMPONENT_SPEC).expect("valid component spec");
    assert_eq!(spec["component"]["name"], "sdkwork-agent-provider-codex");
    assert_eq!(spec["kind"], "sdkwork.component.spec");
}

#[test]
fn model_provider_manifest_uses_canonical_provider_id() {
    let provider = CodexModelProvider::new();
    assert_eq!(
        provider.provider_manifest().provider_id,
        "provider.model.codex"
    );
}
