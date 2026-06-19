use sdkwork_agent_sdk_spi::{
    bootstrap_binding, AgentSdkBindingManifest, AgentSdkCapabilityDriver, BindingRegistry,
    DriverRegistry, SdkBackendKind, SdkDriverHealth, CODEX_BINDING_ID,
};
use std::sync::Arc;

struct FakeDriver {
    id: String,
    capability: String,
    backend: SdkBackendKind,
}

impl AgentSdkCapabilityDriver for FakeDriver {
    fn driver_id(&self) -> &str {
        &self.id
    }

    fn capability_id(&self) -> &str {
        &self.capability
    }

    fn backend_kind(&self) -> SdkBackendKind {
        self.backend
    }

    fn health(&self) -> SdkDriverHealth {
        SdkDriverHealth::healthy()
    }
}

#[test]
fn codex_binding_manifest_parses() {
    let json = include_str!("../../sdks/external-agent-sdks/codex/sdk-binding.manifest.json");
    let manifest = AgentSdkBindingManifest::from_json(json).expect("manifest should parse");
    assert_eq!(manifest.agent_id, "agent.intelligence.codex");
    assert!(manifest.capability_binding("sdk.model.chat").is_some());
}

#[test]
fn bootstrap_binding_loads_codex_manifest() {
    let json = include_str!("../../sdks/external-agent-sdks/codex/sdk-binding.manifest.json");
    let manifest = AgentSdkBindingManifest::from_json(json).expect("manifest should parse");
    let mut drivers = DriverRegistry::new();
    let mut bindings = BindingRegistry::new();
    let negotiation =
        bootstrap_binding(manifest, &mut drivers, &mut bindings).expect("bootstrap should succeed");

    assert_eq!(negotiation.binding_id, CODEX_BINDING_ID);
    assert!(drivers.len() >= 3);
}

#[test]
fn negotiation_selects_registered_drivers_manually() {
    let json = include_str!("../../sdks/external-agent-sdks/codex/sdk-binding.manifest.json");
    let manifest = AgentSdkBindingManifest::from_json(json).expect("manifest should parse");

    let mut bindings = BindingRegistry::new();
    bindings.register(manifest);

    let mut drivers = DriverRegistry::new();
    drivers.register(Arc::new(FakeDriver {
        id: "driver.codex.session.lifecycle.rust".to_string(),
        capability: "sdk.session.lifecycle".to_string(),
        backend: SdkBackendKind::RustNative,
    }));
    drivers.register(Arc::new(FakeDriver {
        id: "driver.codex.session.history.rust".to_string(),
        capability: "sdk.session.history".to_string(),
        backend: SdkBackendKind::RustNative,
    }));
    drivers.register(Arc::new(FakeDriver {
        id: "driver.codex.model.chat.rust".to_string(),
        capability: "sdk.model.chat".to_string(),
        backend: SdkBackendKind::RustNative,
    }));

    let negotiation = bindings
        .negotiate("binding.agent-sdk.codex", &drivers)
        .expect("negotiation should succeed");

    assert_eq!(negotiation.agent_id, "agent.intelligence.codex");
    assert!(negotiation.selected_driver("sdk.model.chat").is_some());
}

#[test]
fn negotiation_fails_closed_for_missing_required_driver() {
    let json = include_str!("../../sdks/external-agent-sdks/codex/sdk-binding.manifest.json");
    let manifest = AgentSdkBindingManifest::from_json(json).expect("manifest should parse");

    let mut bindings = BindingRegistry::new();
    bindings.register(manifest);

    let drivers = DriverRegistry::new();
    let error = bindings
        .negotiate("binding.agent-sdk.codex", &drivers)
        .expect_err("empty registry should fail");

    assert_eq!(error.code, "missing_required_capabilities");
    assert!(!error.missing_capabilities.is_empty());
}

#[test]
fn runtime_router_routes_to_registered_backend() {
    use sdkwork_agent_sdk_spi::{
        SdkBackendRuntime, SdkRuntimeRequest, SdkRuntimeResponse, SdkRuntimeRouter,
    };

    struct StubRuntime {
        kind: SdkBackendKind,
    }

    impl SdkBackendRuntime for StubRuntime {
        fn backend_kind(&self) -> SdkBackendKind {
            self.kind
        }

        fn health(&self) -> SdkDriverHealth {
            SdkDriverHealth::healthy()
        }

        fn invoke(
            &self,
            request: &SdkRuntimeRequest,
        ) -> Result<SdkRuntimeResponse, sdkwork_agent_sdk_spi::SdkRuntimeError> {
            Ok(SdkRuntimeResponse::success(
                self.kind,
                &request.capability_id,
                serde_json::json!({ "stub": true }),
            ))
        }
    }

    let json = include_str!("../../sdks/external-agent-sdks/codex/sdk-binding.manifest.json");
    let manifest = AgentSdkBindingManifest::from_json(json).expect("manifest should parse");
    let mut drivers = DriverRegistry::new();
    let mut bindings = BindingRegistry::new();
    let negotiation =
        bootstrap_binding(manifest, &mut drivers, &mut bindings).expect("bootstrap should succeed");

    let router = SdkRuntimeRouter::new(negotiation).with_rust_runtime(Arc::new(StubRuntime {
        kind: SdkBackendKind::RustNative,
    }));

    let response = router
        .invoke(&SdkRuntimeRequest::ping("sdk.model.chat"))
        .expect("router invoke should succeed");
    assert!(response.success);
    assert_eq!(response.backend_kind, SdkBackendKind::RustNative);
}

#[test]
fn hermes_and_openclaw_manifests_parse() {
    let hermes = include_str!("../../sdks/external-agent-sdks/hermes/sdk-binding.manifest.json");
    let openclaw =
        include_str!("../../sdks/external-agent-sdks/openclaw/sdk-binding.manifest.json");

    let hermes_manifest = AgentSdkBindingManifest::from_json(hermes).expect("hermes manifest");
    let openclaw_manifest =
        AgentSdkBindingManifest::from_json(openclaw).expect("openclaw manifest");

    assert_eq!(hermes_manifest.agent_id, "agent.intelligence.hermes");
    assert_eq!(openclaw_manifest.agent_id, "agent.intelligence.openclaw");
}
