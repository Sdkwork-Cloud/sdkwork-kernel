use sdkwork_agent_sdk_backend_core::{
    BackendHostRegistry, HttpOpenApiBackendHost, RustNativeBackendHost, TypeScriptNodeBackendHost,
};
use sdkwork_agent_sdk_spi::SdkBackendKind;
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
        sdkwork_agent_sdk_backend_core::SdkBackendStatus::Ready
    );
}
