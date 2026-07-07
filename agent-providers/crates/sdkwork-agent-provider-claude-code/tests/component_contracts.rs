use sdkwork_agent_kernel::{AgentProviderFamily, ModelProvider, ToolProvider};
use sdkwork_agent_plugin_core::SdkworkKernelPlugin;
use sdkwork_agent_provider_claude_code::{
    claude_code_agent_definition, claude_code_kernel_plugin_manifest,
    claude_code_provider_manifests, ClaudeCodeKernelPlugin, ClaudeModelProvider,
    ClaudeToolProvider,
};
use serde_json::Value;

const COMPONENT_SPEC: &str = include_str!("../specs/component.spec.json");

#[test]
fn component_spec_matches_crate_identity() {
    let spec: Value = serde_json::from_str(COMPONENT_SPEC).expect("valid component spec");
    assert_eq!(
        spec["component"]["name"],
        "sdkwork-agent-provider-claude-code"
    );
    assert_eq!(spec["kind"], "sdkwork.component.spec");
}

#[test]
fn model_provider_manifest_uses_canonical_provider_id() {
    let provider = ClaudeModelProvider::new();
    assert_eq!(
        provider.provider_manifest().provider_id,
        "provider.model.claude-code"
    );
}

#[test]
fn tool_provider_manifest_uses_canonical_provider_id() {
    let provider = ClaudeToolProvider::new();
    assert_eq!(
        provider.provider_manifest().provider_id,
        "provider.tool.claude-code"
    );
}

#[test]
fn kernel_plugin_manifest_declares_runtime_providers() {
    let manifest = claude_code_kernel_plugin_manifest();
    assert_eq!(manifest.plugin_id, "plugin.intelligence.claude-code");
    assert_eq!(manifest.agent_id.as_deref(), Some("agent.intelligence.claude-code"));
    assert!(manifest
        .provider_ids
        .contains(&"provider.model.claude-code".to_string()));
    assert!(manifest
        .provider_ids
        .contains(&"provider.tool.claude-code".to_string()));
    assert!(manifest.supports_profile("provider-model"));
    assert!(manifest.supports_profile("provider-tool"));
}

#[test]
fn provider_manifests_include_model_tool_and_policy() {
    let provider_ids: Vec<String> = claude_code_provider_manifests()
        .into_iter()
        .map(|manifest| manifest.provider_id)
        .collect();
    assert!(provider_ids.contains(&"provider.model.claude-code".to_string()));
    assert!(provider_ids.contains(&"provider.tool.claude-code".to_string()));
    assert!(provider_ids.contains(&"provider.policy.sdk-standard".to_string()));
}

#[test]
fn agent_definition_binds_model_tool_and_policy() {
    let definition = claude_code_agent_definition();
    let families: Vec<AgentProviderFamily> = definition
        .provider_bindings
        .iter()
        .map(|binding| binding.provider_family.clone())
        .collect();
    assert!(families.contains(&AgentProviderFamily::Model));
    assert!(families.contains(&AgentProviderFamily::Tool));
    assert!(families.contains(&AgentProviderFamily::Policy));
}

#[test]
fn kernel_plugin_configures_runtime() {
    let plugin = ClaudeCodeKernelPlugin::new();
    assert_eq!(
        plugin.plugin_manifest().plugin_id,
        "plugin.intelligence.claude-code"
    );
    assert_eq!(
        plugin.agent_manifest().agent_id,
        "agent.intelligence.claude-code"
    );
}
