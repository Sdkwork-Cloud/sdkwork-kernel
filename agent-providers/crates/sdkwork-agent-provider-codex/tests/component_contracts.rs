use sdkwork_agent_kernel::{
    AgentPackageSource, AgentProviderFamily, ModelProvider, RuntimeBuilder,
};
use sdkwork_agent_plugin_core::SdkworkKernelPlugin;
use sdkwork_agent_provider_codex::{
    codex_agent_definition, codex_agent_installer, codex_kernel_plugin_manifest,
    codex_package_manifest, codex_provider_manifests, CodexKernelPlugin, CodexModelProvider,
    CODEX_CLI_PACKAGE, CODEX_CLI_VERSION,
};
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
    assert_eq!(provider.provider_manifest().provider_id, "provider.codex");
}

#[test]
fn kernel_plugin_manifest_declares_runtime_providers() {
    let manifest = codex_kernel_plugin_manifest();
    assert_eq!(manifest.plugin_id, "plugin.intelligence.codex");
    assert_eq!(manifest.agent_id.as_deref(), Some("agent.codex"));
    assert!(manifest
        .provider_ids
        .contains(&"provider.codex".to_string()));
    assert!(manifest
        .provider_ids
        .contains(&"provider.session-control.codex".to_string()));
    assert!(!manifest
        .provider_ids
        .contains(&"provider.tool.codex".to_string()));
    assert!(manifest.supports_profile("provider-model"));
    assert!(manifest.supports_profile("agent-installation"));
    assert!(!manifest.supports_profile("provider-tool"));
}

#[test]
fn provider_manifests_exclude_agent_internal_tools() {
    let manifests = codex_provider_manifests();
    let provider_ids: Vec<String> = manifests
        .iter()
        .map(|manifest| manifest.provider_id.clone())
        .collect();
    assert!(provider_ids.contains(&"provider.codex".to_string()));
    assert!(provider_ids.contains(&"provider.session-control.codex".to_string()));
    assert!(!provider_ids.contains(&"provider.tool.codex".to_string()));
    assert!(provider_ids.contains(&"provider.policy.sdk-standard".to_string()));
    assert!(provider_ids.contains(&"provider.agent.installer.codex".to_string()));

    let session_control = manifests
        .iter()
        .find(|manifest| manifest.provider_id == "provider.session-control.codex")
        .expect("session control provider manifest");
    assert_eq!(session_control.provider_family, "session_control");
    assert_eq!(
        session_control.capabilities,
        vec![
            "session.control.interrupt".to_string(),
            "session.control.compact".to_string(),
            "session.control.fork".to_string(),
        ]
    );
}

#[test]
fn installer_descriptor_uses_the_latest_exact_sdk_version() {
    let installer = codex_agent_installer();
    assert_eq!(installer.provider_id(), "provider.agent.installer.codex");
    assert_eq!(installer.packages()[0].package_id, CODEX_CLI_PACKAGE);
    assert_eq!(installer.packages()[0].version, CODEX_CLI_VERSION);
    assert_eq!(
        codex_package_manifest().source,
        AgentPackageSource::registry("npm", CODEX_CLI_PACKAGE, CODEX_CLI_VERSION)
    );
}

#[test]
fn agent_definition_does_not_bind_agent_internal_tools() {
    let definition = codex_agent_definition();
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
    let plugin = CodexKernelPlugin::new();
    assert_eq!(
        plugin.plugin_manifest().plugin_id,
        "plugin.intelligence.codex"
    );
    assert_eq!(plugin.agent_manifest().agent_id, "agent.codex");
    let report = plugin
        .configure_runtime(RuntimeBuilder::new(
            "runtime.codex.installer",
            plugin.agent_manifest(),
        ))
        .bootstrap()
        .expect("codex runtime bootstraps");
    assert_eq!(
        report
            .runtime
            .agent_installer()
            .expect("typed installer")
            .provider_manifest()
            .provider_id,
        "provider.agent.installer.codex"
    );
    assert_eq!(
        report.runtime.provider_session_control_provider_ids(),
        ["provider.session-control.codex"]
    );
    let session_control = report
        .runtime
        .provider_session_control_provider_by_id("provider.session-control.codex")
        .expect("typed session control provider");
    assert_eq!(
        session_control.provider_manifest().capabilities,
        vec![
            "session.control.interrupt".to_string(),
            "session.control.compact".to_string(),
            "session.control.fork".to_string(),
        ]
    );
}
