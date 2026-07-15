use sdkwork_agent_kernel::{AgentProviderFamily, ModelProvider};
use sdkwork_agent_plugin_core::SdkworkKernelPlugin;
use sdkwork_agent_provider_openclaw::{
    openclaw_agent_definition, openclaw_kernel_plugin_manifest, openclaw_provider_manifests,
    OpenClawKernelPlugin, OpenClawModelProvider,
};
use serde_json::Value;

const COMPONENT_SPEC: &str = include_str!("../specs/component.spec.json");

#[test]
fn component_spec_matches_crate_identity() {
    let spec: Value = serde_json::from_str(COMPONENT_SPEC).expect("valid component spec");
    assert_eq!(spec["component"]["name"], "sdkwork-agent-provider-openclaw");
    assert_eq!(spec["kind"], "sdkwork.component.spec");
}

#[test]
fn model_provider_manifest_uses_canonical_provider_id() {
    let provider = OpenClawModelProvider::new();
    assert_eq!(
        provider.provider_manifest().provider_id,
        "provider.model.openclaw"
    );
}

#[test]
fn kernel_plugin_manifest_declares_runtime_providers() {
    let manifest = openclaw_kernel_plugin_manifest();
    assert_eq!(manifest.plugin_id, "plugin.intelligence.openclaw");
    assert_eq!(
        manifest.agent_id.as_deref(),
        Some("agent.intelligence.openclaw")
    );
    assert!(manifest
        .provider_ids
        .contains(&"provider.model.openclaw".to_string()));
    assert!(!manifest
        .provider_ids
        .contains(&"provider.tool.openclaw".to_string()));
    assert!(manifest.supports_profile("provider-model"));
    assert!(!manifest.supports_profile("provider-tool"));
}

#[test]
fn provider_manifests_exclude_agent_internal_tools() {
    let provider_ids: Vec<String> = openclaw_provider_manifests()
        .into_iter()
        .map(|manifest| manifest.provider_id)
        .collect();
    assert!(provider_ids.contains(&"provider.model.openclaw".to_string()));
    assert!(!provider_ids.contains(&"provider.tool.openclaw".to_string()));
    assert!(provider_ids.contains(&"provider.policy.sdk-standard".to_string()));
}

#[test]
fn agent_definition_does_not_bind_agent_internal_tools() {
    let definition = openclaw_agent_definition();
    let families: Vec<AgentProviderFamily> = definition
        .provider_bindings
        .iter()
        .map(|binding| binding.family)
        .collect();
    assert!(families.contains(&AgentProviderFamily::Model));
    assert!(!families.contains(&AgentProviderFamily::Tool));
    assert!(families.contains(&AgentProviderFamily::Policy));
}

#[test]
fn kernel_plugin_configures_runtime() {
    let plugin = OpenClawKernelPlugin::new();
    assert_eq!(
        plugin.plugin_manifest().plugin_id,
        "plugin.intelligence.openclaw"
    );
    assert_eq!(
        plugin.agent_manifest().agent_id,
        "agent.intelligence.openclaw"
    );
}
