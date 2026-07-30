use sdkwork_agent_kernel::{
    AgentPackageSource, AgentProviderFamily, ModelProvider, RuntimeBuilder,
};
use sdkwork_agent_plugin_core::SdkworkKernelPlugin;
use sdkwork_agent_provider_opencode::{
    opencode_agent_definition, opencode_agent_installer, opencode_kernel_plugin_manifest,
    opencode_package_manifest, opencode_provider_manifests, OpenCodeKernelPlugin,
    OpenCodeModelProvider, OPENCODE_SDK_PACKAGE, OPENCODE_SDK_VERSION,
};
use serde_json::Value;

const COMPONENT_SPEC: &str = include_str!("../specs/component.spec.json");

#[test]
fn component_spec_matches_crate_identity() {
    let spec: Value = serde_json::from_str(COMPONENT_SPEC).expect("valid component spec");
    assert_eq!(spec["component"]["name"], "sdkwork-agent-provider-opencode");
    assert_eq!(spec["kind"], "sdkwork.component.spec");
}

#[test]
fn model_provider_manifest_uses_canonical_provider_id() {
    let provider = OpenCodeModelProvider::new();
    assert_eq!(
        provider.provider_manifest().provider_id,
        "provider.model.opencode"
    );
}

#[test]
fn kernel_plugin_manifest_declares_runtime_providers() {
    let manifest = opencode_kernel_plugin_manifest();
    assert_eq!(manifest.plugin_id, "plugin.intelligence.opencode");
    assert_eq!(
        manifest.agent_id.as_deref(),
        Some("agent.intelligence.opencode")
    );
    assert!(manifest
        .provider_ids
        .contains(&"provider.model.opencode".to_string()));
    assert!(!manifest
        .provider_ids
        .contains(&"provider.tool.opencode".to_string()));
    assert!(manifest.supports_profile("provider-model"));
    assert!(manifest.supports_profile("agent-installation"));
    assert!(!manifest.supports_profile("provider-tool"));
}

#[test]
fn provider_manifests_exclude_agent_internal_tools() {
    let provider_ids: Vec<String> = opencode_provider_manifests()
        .into_iter()
        .map(|manifest| manifest.provider_id)
        .collect();
    assert!(provider_ids.contains(&"provider.model.opencode".to_string()));
    assert!(!provider_ids.contains(&"provider.tool.opencode".to_string()));
    assert!(provider_ids.contains(&"provider.policy.sdk-standard".to_string()));
    assert!(provider_ids.contains(&"provider.agent.installer.opencode".to_string()));
}

#[test]
fn installer_descriptor_uses_the_latest_exact_sdk_version() {
    let installer = opencode_agent_installer();
    assert_eq!(installer.provider_id(), "provider.agent.installer.opencode");
    assert_eq!(installer.packages()[0].package_id, OPENCODE_SDK_PACKAGE);
    assert_eq!(installer.packages()[0].version, OPENCODE_SDK_VERSION);
    assert_eq!(
        opencode_package_manifest().source,
        AgentPackageSource::registry("npm", OPENCODE_SDK_PACKAGE, OPENCODE_SDK_VERSION)
    );
}

#[test]
fn agent_definition_does_not_bind_agent_internal_tools() {
    let definition = opencode_agent_definition();
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
    let plugin = OpenCodeKernelPlugin::new();
    assert_eq!(
        plugin.plugin_manifest().plugin_id,
        "plugin.intelligence.opencode"
    );
    assert_eq!(
        plugin.agent_manifest().agent_id,
        "agent.intelligence.opencode"
    );
    let report = plugin
        .configure_runtime(RuntimeBuilder::new(
            "runtime.opencode.installer",
            plugin.agent_manifest(),
        ))
        .bootstrap()
        .expect("opencode runtime bootstraps");
    assert_eq!(
        report
            .runtime
            .agent_installer()
            .expect("typed installer")
            .provider_manifest()
            .provider_id,
        "provider.agent.installer.opencode"
    );
}
