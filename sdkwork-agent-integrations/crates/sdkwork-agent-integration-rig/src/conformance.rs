use sdkwork_agent_integration_core::IntegrationConformanceProfile;

pub fn rig_conformance_profile() -> IntegrationConformanceProfile {
    IntegrationConformanceProfile::new("rig-local")
        .require_profile("runtime-manifest")
        .require_profile("runtime-local")
        .require_profile("agent-installation")
        .require_profile("provider-model")
        .require_profile("provider-tool")
        .require_profile("provider-knowledge")
        .require_profile("security-baseline")
}
