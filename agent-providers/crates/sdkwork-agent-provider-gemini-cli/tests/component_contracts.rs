use sdkwork_agent_kernel::{
    AgentPackageSource, AgentProviderFamily, ModelProvider, RuntimeBuilder,
};
use sdkwork_agent_plugin_core::SdkworkKernelPlugin;
use sdkwork_agent_provider_gemini_cli::{
    gemini_cli_agent_definition, gemini_cli_agent_installer, gemini_cli_kernel_plugin_manifest,
    gemini_cli_package_manifest, gemini_cli_provider_manifests, GeminiCliKernelPlugin,
    GeminiModelProvider, GEMINI_CLI_PACKAGE, GEMINI_CLI_VERSION,
};
use serde_json::Value;

const COMPONENT_SPEC: &str = include_str!("../specs/component.spec.json");

#[test]
fn component_spec_matches_crate_identity() {
    let spec: Value = serde_json::from_str(COMPONENT_SPEC).expect("valid component spec");
    assert_eq!(
        spec["component"]["name"],
        "sdkwork-agent-provider-gemini-cli"
    );
    assert_eq!(spec["kind"], "sdkwork.component.spec");
}

#[test]
fn model_provider_manifest_uses_canonical_provider_id() {
    let provider = GeminiModelProvider::new();
    assert_eq!(
        provider.provider_manifest().provider_id,
        "provider.model.gemini"
    );
}

#[test]
fn kernel_plugin_uses_established_gemini_identities() {
    let manifest = gemini_cli_kernel_plugin_manifest();
    assert_eq!(manifest.plugin_id, "plugin.intelligence.gemini-cli");
    assert_eq!(
        manifest.agent_id.as_deref(),
        Some("agent.intelligence.gemini")
    );
    assert!(manifest
        .provider_ids
        .contains(&"provider.model.gemini".to_string()));
    assert!(!manifest
        .provider_ids
        .contains(&"provider.model.gemini-cli".to_string()));
    assert!(manifest.supports_profile("agent-installation"));
}

#[test]
fn provider_manifests_and_definition_exclude_agent_internal_tools() {
    let provider_ids = gemini_cli_provider_manifests()
        .into_iter()
        .map(|manifest| manifest.provider_id)
        .collect::<Vec<_>>();
    assert!(provider_ids.contains(&"provider.model.gemini".to_string()));
    assert!(!provider_ids.contains(&"provider.tool.gemini".to_string()));
    assert!(provider_ids.contains(&"provider.policy.sdk-standard".to_string()));
    assert!(provider_ids.contains(&"provider.agent.installer.gemini-cli".to_string()));

    let families = gemini_cli_agent_definition()
        .provider_bindings
        .into_iter()
        .map(|binding| binding.family)
        .collect::<Vec<_>>();
    assert!(families.contains(&AgentProviderFamily::Model));
    assert!(!families.contains(&AgentProviderFamily::Tool));
    assert!(families.contains(&AgentProviderFamily::Policy));
}

#[test]
fn installer_descriptor_uses_the_latest_exact_cli_version() {
    let installer = gemini_cli_agent_installer();
    assert_eq!(
        installer.provider_id(),
        "provider.agent.installer.gemini-cli"
    );
    assert_eq!(installer.packages()[0].package_id, GEMINI_CLI_PACKAGE);
    assert_eq!(installer.packages()[0].version, GEMINI_CLI_VERSION);
    assert_eq!(
        gemini_cli_package_manifest().source,
        AgentPackageSource::registry("npm", GEMINI_CLI_PACKAGE, GEMINI_CLI_VERSION)
    );
}

#[test]
fn kernel_plugin_configures_canonical_agent() {
    let plugin = GeminiCliKernelPlugin::new();
    assert_eq!(
        plugin.plugin_manifest().plugin_id,
        "plugin.intelligence.gemini-cli"
    );
    assert_eq!(
        plugin.agent_manifest().agent_id,
        "agent.intelligence.gemini"
    );
    let report = plugin
        .configure_runtime(RuntimeBuilder::new(
            "runtime.gemini-cli.installer",
            plugin.agent_manifest(),
        ))
        .bootstrap()
        .expect("gemini-cli runtime bootstraps");
    assert_eq!(
        report
            .runtime
            .agent_installer()
            .expect("typed installer")
            .provider_manifest()
            .provider_id,
        "provider.agent.installer.gemini-cli"
    );
}
