use sdkwork_agent_kernel::{
    AgentPackageSource, AgentProviderFamily, ModelProvider, RuntimeBuilder,
};
use sdkwork_agent_plugin_core::SdkworkKernelPlugin;
use sdkwork_agent_provider_claude_code::{
    claude_code_agent_definition, claude_code_agent_installer, claude_code_kernel_plugin_manifest,
    claude_code_package_manifest, claude_code_provider_manifests, ClaudeCodeKernelPlugin,
    ClaudeModelProvider, ANTHROPIC_SDK_PACKAGE, ANTHROPIC_SDK_VERSION, CLAUDE_AGENT_SDK_PACKAGE,
    CLAUDE_AGENT_SDK_VERSION, MCP_SDK_PACKAGE, MCP_SDK_VERSION, ZOD_PACKAGE, ZOD_VERSION,
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
fn kernel_plugin_manifest_declares_runtime_providers() {
    let manifest = claude_code_kernel_plugin_manifest();
    assert_eq!(manifest.plugin_id, "plugin.intelligence.claude-code");
    assert_eq!(
        manifest.agent_id.as_deref(),
        Some("agent.intelligence.claude-code")
    );
    assert!(manifest
        .provider_ids
        .contains(&"provider.model.claude-code".to_string()));
    assert!(!manifest
        .provider_ids
        .contains(&"provider.tool.claude-code".to_string()));
    assert!(manifest.supports_profile("provider-model"));
    assert!(manifest.supports_profile("agent-installation"));
    assert!(!manifest.supports_profile("provider-tool"));
}

#[test]
fn provider_manifests_exclude_agent_internal_tools() {
    let provider_ids: Vec<String> = claude_code_provider_manifests()
        .into_iter()
        .map(|manifest| manifest.provider_id)
        .collect();
    assert!(provider_ids.contains(&"provider.model.claude-code".to_string()));
    assert!(!provider_ids.contains(&"provider.tool.claude-code".to_string()));
    assert!(provider_ids.contains(&"provider.policy.sdk-standard".to_string()));
    assert!(provider_ids.contains(&"provider.agent.installer.claude-code".to_string()));
}

#[test]
fn installer_descriptor_uses_the_latest_exact_sdk_version() {
    let installer = claude_code_agent_installer();
    assert_eq!(
        installer.provider_id(),
        "provider.agent.installer.claude-code"
    );
    assert_eq!(installer.packages()[0].package_id, CLAUDE_AGENT_SDK_PACKAGE);
    assert_eq!(installer.packages()[0].version, CLAUDE_AGENT_SDK_VERSION);
    assert_eq!(installer.packages()[1].package_id, ANTHROPIC_SDK_PACKAGE);
    assert_eq!(installer.packages()[1].version, ANTHROPIC_SDK_VERSION);
    assert_eq!(installer.packages()[2].package_id, MCP_SDK_PACKAGE);
    assert_eq!(installer.packages()[2].version, MCP_SDK_VERSION);
    assert_eq!(installer.packages()[3].package_id, ZOD_PACKAGE);
    assert_eq!(installer.packages()[3].version, ZOD_VERSION);
    assert_eq!(
        claude_code_package_manifest().source,
        AgentPackageSource::registry("npm", CLAUDE_AGENT_SDK_PACKAGE, CLAUDE_AGENT_SDK_VERSION,)
    );
}

#[test]
fn agent_definition_does_not_bind_agent_internal_tools() {
    let definition = claude_code_agent_definition();
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
    let plugin = ClaudeCodeKernelPlugin::new();
    assert_eq!(
        plugin.plugin_manifest().plugin_id,
        "plugin.intelligence.claude-code"
    );
    assert_eq!(
        plugin.agent_manifest().agent_id,
        "agent.intelligence.claude-code"
    );
    let report = plugin
        .configure_runtime(RuntimeBuilder::new(
            "runtime.claude-code.installer",
            plugin.agent_manifest(),
        ))
        .bootstrap()
        .expect("claude-code runtime bootstraps");
    assert_eq!(
        report
            .runtime
            .agent_installer()
            .expect("typed installer")
            .provider_manifest()
            .provider_id,
        "provider.agent.installer.claude-code"
    );
}
