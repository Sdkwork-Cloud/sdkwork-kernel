use sdkwork_agent_plugin_core::SdkworkKernelPlugin;
use sdkwork_agent_provider_rig::{ids, RigDeploymentSpec, RigKernelPlugin, RigProviderBindingSpec};

#[test]
fn rig_deployment_snapshot_preserves_binding_provider_and_capabilities() {
    let binding = RigProviderBindingSpec::default_local("binding.rig.default", "profile.rig.local")
        .activate();
    let deployment = RigDeploymentSpec::from_binding(
        "deployment.rig.1",
        "tenant.1",
        &binding,
        "2026-06-04T00:00:00Z",
    );

    let switched = binding
        .clone()
        .with_provider_id("provider.other")
        .deactivate();

    assert_eq!(deployment.agent_id, ids::AGENT_ID);
    assert_eq!(deployment.provider_id_snapshot, ids::MODEL_PROVIDER_ID);
    assert_eq!(deployment.binding_id, "binding.rig.default");
    assert!(!switched.active);
    assert_ne!(deployment.provider_id_snapshot, switched.provider_id);
}

#[test]
fn rig_deployment_contracts_reject_non_standard_ids() {
    let error = RigProviderBindingSpec::try_default_local("rig.default", "profile.rig.local")
        .expect_err("binding id without standard prefix should fail");
    assert!(error.contains("bindingId"));

    let error =
        RigProviderBindingSpec::try_default_local("binding.rig.default", "config.rig.local")
            .expect_err("profile id without standard prefix should fail");
    assert!(error.contains("configurationProfileId"));

    let binding =
        RigProviderBindingSpec::try_default_local("binding.rig.default", "profile.rig.local")
            .expect("standard Rig binding should be accepted");
    let error =
        RigDeploymentSpec::try_from_binding("rig.1", "tenant.1", &binding, "2026-06-04T00:00:00Z")
            .expect_err("deployment id without standard prefix should fail");
    assert!(error.contains("deploymentId"));
}

#[test]
fn rig_default_binding_capabilities_follow_standard_contract() {
    let binding =
        RigProviderBindingSpec::try_default_local("binding.rig.default", "profile.rig.local")
            .expect("standard Rig binding should be accepted");

    assert_eq!(binding.capabilities.len(), 6);
    assert_eq!(
        binding.capabilities,
        [
            "model.catalog",
            "model.chat",
            "knowledge.search",
            "knowledge.read",
            "knowledge.list",
            "planning.create"
        ]
    );
}

#[test]
fn rig_kernel_plugin_manifest_lists_all_provider_ids() {
    let plugin = RigKernelPlugin::fail_closed();
    let manifest = plugin.plugin_manifest();

    assert_eq!(manifest.plugin_id, ids::PLUGIN_ID);
    assert!(manifest
        .provider_ids
        .contains(&ids::MODEL_PROVIDER_ID.to_string()));
    assert!(!manifest
        .provider_ids
        .contains(&"provider.tool.rig-rust".to_string()));
    assert!(manifest
        .provider_ids
        .contains(&ids::KNOWLEDGE_PROVIDER_ID.to_string()));
    assert!(manifest
        .provider_ids
        .contains(&ids::PLANNING_PROVIDER_ID.to_string()));
    assert!(manifest.supports_profile("agent-installation"));
}
