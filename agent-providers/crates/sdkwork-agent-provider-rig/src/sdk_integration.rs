use crate::{RigLifecycleProvider, RigModelProvider, RigSessionAdapter};
use sdkwork_agent_provider_spi::{
    bootstrap_binding, AgentSdkBindingManifest, AgentSdkIntegration, BindingRegistry,
    DriverRegistry, SdkNegotiationError, SdkRuntimeBackedModelProvider, SdkRuntimeRequest,
    SdkRuntimeResponse, SdkRuntimeRouter, RIG_BINDING_ID, SDK_CAPABILITY_MODEL_CHAT,
};
use sdkwork_agent_provider_transport_core::{
    ProviderTransportBootstrap, ProviderTransportRegistry, RustNativeTransportHost,
};
use sdkwork_agent_provider_transport_rust::{InProcessRustSdkRuntime, ProviderBackedRustHandler};
use std::sync::Arc;

const RIG_BINDING_MANIFEST_JSON: &str =
    include_str!("../../../../bindings/agent-providers/rig/provider-binding.manifest.json");

pub fn rig_binding_manifest() -> AgentSdkBindingManifest {
    AgentSdkBindingManifest::from_json(RIG_BINDING_MANIFEST_JSON)
        .expect("rig provider binding manifest must parse")
}

pub struct RigSdkIntegration {
    pub sdk: AgentSdkIntegration,
    pub transports: ProviderTransportRegistry,
    pub runtime: Arc<SdkRuntimeRouter>,
    pub lifecycle: RigLifecycleProvider,
    pub model: SdkRuntimeBackedModelProvider,
    pub session_adapter: RigSessionAdapter,
}

impl RigSdkIntegration {
    pub fn bootstrap() -> Result<Self, SdkNegotiationError> {
        let manifest = rig_binding_manifest();
        let mut drivers = DriverRegistry::new();
        let mut bindings = BindingRegistry::new();
        let negotiation = bootstrap_binding(manifest, &mut drivers, &mut bindings)?;

        let inner_model = Arc::new(RigModelProvider::fail_closed());
        let rust_handler = Arc::new(ProviderBackedRustHandler::model_only(
            inner_model.clone(),
            crate::ids::DEFAULT_MODEL_ID,
        ));
        let mut bootstrap = ProviderTransportBootstrap::new();
        bootstrap.register_host(Arc::new(RustNativeTransportHost::new("rig-core")));
        bootstrap.with_rust_runtime(Arc::new(InProcessRustSdkRuntime::new(rust_handler)));
        let (transports, runtime) = bootstrap.finalize_pair(negotiation.clone())?;
        let model = SdkRuntimeBackedModelProvider::new(
            runtime.clone(),
            inner_model,
            SDK_CAPABILITY_MODEL_CHAT,
            crate::ids::MODEL_PROVIDER_ID,
        );

        Ok(Self {
            sdk: AgentSdkIntegration::new(negotiation),
            transports,
            runtime,
            lifecycle: RigLifecycleProvider::new(),
            model,
            session_adapter: RigSessionAdapter::new(),
        })
    }

    pub fn binding_id(&self) -> &str {
        RIG_BINDING_ID
    }

    pub fn invoke_runtime(
        &self,
        request: &SdkRuntimeRequest,
    ) -> Result<SdkRuntimeResponse, sdkwork_agent_provider_spi::SdkRuntimeError> {
        self.runtime.invoke(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_provider_core::{SessionConfig, SessionLifecycleProvider};

    #[test]
    fn bootstrap_exposes_provider_local_session_lifecycle() {
        let integration = RigSdkIntegration::bootstrap().expect("bootstrap");
        assert_eq!(integration.binding_id(), RIG_BINDING_ID);
        assert_eq!(
            integration.sdk.selected_driver_id("sdk.session.lifecycle"),
            Some("driver.rig.session.lifecycle.rust")
        );
        let session = integration
            .lifecycle
            .create_session(crate::ids::AGENT_ID, None, SessionConfig::new())
            .expect("session");
        assert_eq!(
            integration
                .lifecycle
                .get_session(&session.session_id)
                .expect("loaded")
                .session_id,
            session.session_id
        );
    }
}
