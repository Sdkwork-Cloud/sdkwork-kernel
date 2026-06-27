use crate::host::{ProviderTransportError, ProviderTransportHost};
use crate::registry::ProviderTransportRegistry;
use sdkwork_agent_provider_spi::{
    ProviderTransportKind, SdkBackendRuntime, SdkCapabilityNegotiation, SdkNegotiationError,
    SdkRuntimeRouter,
};
use std::collections::HashMap;
use std::sync::Arc;

/// Builds a provider transport registry and attaches healthy runtimes to the router.
#[derive(Default)]
pub struct ProviderTransportBootstrap {
    registry: ProviderTransportRegistry,
    runtimes: HashMap<ProviderTransportKind, Arc<dyn SdkBackendRuntime>>,
}

impl ProviderTransportBootstrap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_host(&mut self, host: Arc<dyn ProviderTransportHost>) -> &mut Self {
        self.registry.register(host);
        self
    }

    pub fn with_runtime(
        &mut self,
        kind: ProviderTransportKind,
        runtime: Arc<dyn SdkBackendRuntime>,
    ) -> &mut Self {
        self.runtimes.insert(kind, runtime);
        self
    }

    pub fn with_typescript_runtime(&mut self, runtime: Arc<dyn SdkBackendRuntime>) -> &mut Self {
        self.with_runtime(ProviderTransportKind::TypeScriptNode, runtime)
    }

    pub fn with_rust_runtime(&mut self, runtime: Arc<dyn SdkBackendRuntime>) -> &mut Self {
        self.with_runtime(ProviderTransportKind::RustNative, runtime)
    }

    pub fn with_python_runtime(&mut self, runtime: Arc<dyn SdkBackendRuntime>) -> &mut Self {
        self.with_runtime(ProviderTransportKind::PythonProcess, runtime)
    }

    pub fn with_http_runtime(&mut self, runtime: Arc<dyn SdkBackendRuntime>) -> &mut Self {
        self.with_runtime(ProviderTransportKind::HttpOpenApi, runtime)
    }

    pub fn with_ipc_runtime(&mut self, runtime: Arc<dyn SdkBackendRuntime>) -> &mut Self {
        self.with_runtime(ProviderTransportKind::IpcProtocol, runtime)
    }

    pub fn finalize_router(
        self,
        negotiation: SdkCapabilityNegotiation,
    ) -> Result<Arc<SdkRuntimeRouter>, SdkNegotiationError> {
        self.finalize_pair(negotiation)
            .map(|(_registry, router)| router)
    }

    pub fn finalize_pair(
        self,
        negotiation: SdkCapabilityNegotiation,
    ) -> Result<(ProviderTransportRegistry, Arc<SdkRuntimeRouter>), SdkNegotiationError> {
        self.registry.prepare_all().map_err(|error| {
            SdkNegotiationError::missing_required_capabilities(
                negotiation.agent_id.clone(),
                vec![format!("transport.prepare: {error}")],
            )
        })?;
        let router = Arc::new(
            self.registry
                .attach_runtimes(SdkRuntimeRouter::new(negotiation), &self.runtimes),
        );
        Ok((self.registry, router))
    }

    pub fn into_registry(self) -> ProviderTransportRegistry {
        self.registry
    }
}

impl ProviderTransportError {
    pub fn to_negotiation_error(self, agent_id: impl Into<String>) -> SdkNegotiationError {
        SdkNegotiationError::missing_required_capabilities(
            agent_id.into(),
            vec![format!("transport.prepare: {self}")],
        )
    }
}
