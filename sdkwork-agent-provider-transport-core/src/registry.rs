use crate::host::{ProviderTransportError, ProviderTransportHealth, ProviderTransportHost};
use sdkwork_agent_provider_spi::{
    ProviderTransportKind, ProviderTransportRouter, SdkBackendRuntime,
};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Default)]
pub struct ProviderTransportRegistry {
    hosts: HashMap<ProviderTransportKind, Arc<dyn ProviderTransportHost>>,
}

/// Deprecated alias retained for one release cycle.
pub type BackendHostRegistry = ProviderTransportRegistry;

impl ProviderTransportRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, host: Arc<dyn ProviderTransportHost>) {
        self.hosts.insert(host.transport_kind(), host);
    }

    pub fn get(&self, kind: ProviderTransportKind) -> Option<Arc<dyn ProviderTransportHost>> {
        self.hosts.get(&kind).cloned()
    }

    pub fn health(&self, kind: ProviderTransportKind) -> Option<ProviderTransportHealth> {
        self.get(kind).map(|host| host.health())
    }

    pub fn is_transport_usable(&self, kind: ProviderTransportKind) -> bool {
        self.health(kind)
            .map(|health| health.is_usable())
            .unwrap_or(false)
    }

    pub fn prepare_all(&self) -> Result<(), ProviderTransportError> {
        for host in self.hosts.values() {
            host.prepare()?;
        }
        Ok(())
    }

    /// Attach only healthy negotiated transports to the runtime router.
    pub fn attach_runtimes(
        &self,
        router: ProviderTransportRouter,
        runtimes: &HashMap<ProviderTransportKind, Arc<dyn SdkBackendRuntime>>,
    ) -> ProviderTransportRouter {
        let mut router = router;
        for (kind, runtime) in runtimes {
            if !self.is_transport_usable(*kind) {
                continue;
            }
            router = match kind {
                ProviderTransportKind::RustNative => router.with_rust_runtime(runtime.clone()),
                ProviderTransportKind::TypeScriptNode => {
                    router.with_typescript_runtime(runtime.clone())
                }
                ProviderTransportKind::PythonProcess => router.with_python_runtime(runtime.clone()),
                ProviderTransportKind::HttpOpenApi => router.with_http_runtime(runtime.clone()),
                ProviderTransportKind::IpcProtocol => router.with_ipc_runtime(runtime.clone()),
            };
        }
        router
    }
}
