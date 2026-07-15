use sdkwork_agent_plugin_core::KernelPluginConformanceProfile;

pub fn rig_conformance_profile() -> KernelPluginConformanceProfile {
    KernelPluginConformanceProfile::new("rig-local")
        .require_profile("runtime-manifest")
        .require_profile("runtime-local")
        .require_profile("agent-installation")
        .require_profile("provider-model")
        .require_profile("provider-mcp")
        .require_profile("provider-knowledge")
        .require_profile("security-baseline")
}
