use sdkwork_agent_provider_spi::SdkBackendKind;
use sdkwork_agent_provider_transport_core::{
    BackendHostRegistry, HttpOpenApiBackendHost, RustNativeBackendHost, TypeScriptNodeBackendHost,
};
use std::sync::Arc;

#[test]
fn registry_prepares_registered_backends() {
    let mut registry = BackendHostRegistry::new();
    registry.register(Arc::new(RustNativeBackendHost::new("codex-core")));
    registry.register(Arc::new(TypeScriptNodeBackendHost::new("openclaw")));
    registry.register(Arc::new(HttpOpenApiBackendHost::new(
        "openclaw-gateway-open-api",
    )));

    registry
        .prepare_all()
        .expect("backend hosts should prepare");
    assert_eq!(
        registry
            .health(SdkBackendKind::RustNative)
            .expect("rust host")
            .status,
        sdkwork_agent_provider_transport_core::SdkBackendStatus::Ready
    );
}
