use sdkwork_agent_kernel::{
    AgentPackageSource, AgentProviderFamily, ModelProvider, RuntimeBuilder,
};
use sdkwork_agent_plugin_core::SdkworkKernelPlugin;
use sdkwork_agent_provider_hermes::{
    hermes_agent_definition, hermes_agent_installer, hermes_kernel_plugin_manifest,
    hermes_package_manifest, hermes_provider_manifests, HermesKernelPlugin, HermesModelProvider,
    HERMES_PACKAGE, HERMES_PACKAGE_VERSION,
};
use serde_json::Value;

const COMPONENT_SPEC: &str = include_str!("../specs/component.spec.json");

#[test]
fn component_spec_matches_crate_identity() {
    let spec: Value = serde_json::from_str(COMPONENT_SPEC).expect("valid component spec");
    assert_eq!(spec["component"]["name"], "sdkwork-agent-provider-hermes");
    assert_eq!(spec["kind"], "sdkwork.component.spec");
}

#[test]
fn model_provider_manifest_uses_canonical_provider_id() {
    let provider = HermesModelProvider::new();
    assert_eq!(provider.provider_manifest().provider_id, "provider.hermes");
}

#[test]
fn kernel_plugin_manifest_declares_runtime_providers() {
    let manifest = hermes_kernel_plugin_manifest();
    assert_eq!(manifest.plugin_id, "plugin.intelligence.hermes");
    assert_eq!(manifest.agent_id.as_deref(), Some("agent.hermes"));
    assert!(manifest
        .provider_ids
        .contains(&"provider.hermes".to_string()));
    assert!(!manifest
        .provider_ids
        .contains(&"provider.tool.hermes".to_string()));
    assert!(manifest.supports_profile("provider-model"));
    assert!(manifest.supports_profile("agent-installation"));
    assert!(!manifest.supports_profile("provider-tool"));
}

#[test]
fn provider_manifests_exclude_agent_internal_tools() {
    let provider_ids: Vec<String> = hermes_provider_manifests()
        .into_iter()
        .map(|manifest| manifest.provider_id)
        .collect();
    assert!(provider_ids.contains(&"provider.hermes".to_string()));
    assert!(!provider_ids.contains(&"provider.tool.hermes".to_string()));
    assert!(provider_ids.contains(&"provider.policy.sdk-standard".to_string()));
    assert!(provider_ids.contains(&"provider.agent.installer.hermes".to_string()));
}

#[test]
fn installer_descriptor_uses_the_latest_exact_python_version() {
    let installer = hermes_agent_installer();
    assert_eq!(installer.provider_id(), "provider.agent.installer.hermes");
    assert_eq!(installer.packages()[0].package_id, HERMES_PACKAGE);
    assert_eq!(installer.packages()[0].version, HERMES_PACKAGE_VERSION);
    assert_eq!(
        hermes_package_manifest().source,
        AgentPackageSource::registry("pypi", HERMES_PACKAGE, HERMES_PACKAGE_VERSION)
    );
}

#[test]
fn agent_definition_does_not_bind_agent_internal_tools() {
    let definition = hermes_agent_definition();
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
    let plugin = HermesKernelPlugin::new();
    assert_eq!(
        plugin.plugin_manifest().plugin_id,
        "plugin.intelligence.hermes"
    );
    assert_eq!(plugin.agent_manifest().agent_id, "agent.hermes");
    let report = plugin
        .configure_runtime(RuntimeBuilder::new(
            "runtime.hermes.installer",
            plugin.agent_manifest(),
        ))
        .bootstrap()
        .expect("hermes runtime bootstraps");
    assert_eq!(
        report
            .runtime
            .agent_installer()
            .expect("typed installer")
            .provider_manifest()
            .provider_id,
        "provider.agent.installer.hermes"
    );
}
