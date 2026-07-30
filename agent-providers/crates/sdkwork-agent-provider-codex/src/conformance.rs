use sdkwork_agent_plugin_core::KernelPluginConformanceProfile;

pub fn codex_conformance_profile() -> KernelPluginConformanceProfile {
    KernelPluginConformanceProfile::new("codex-sdk-runtime")
        .require_profile("runtime-manifest")
        .require_profile("agent-installation")
        .require_profile("provider-model")
        .require_profile("security-baseline")
}
