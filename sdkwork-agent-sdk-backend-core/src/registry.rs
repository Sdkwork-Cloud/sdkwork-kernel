use crate::host::{SdkBackendHealth, SdkBackendHost};
use sdkwork_agent_sdk_spi::SdkBackendKind;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Default)]
pub struct BackendHostRegistry {
    hosts: HashMap<SdkBackendKind, Arc<dyn SdkBackendHost>>,
}

impl BackendHostRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, host: Arc<dyn SdkBackendHost>) {
        self.hosts.insert(host.backend_kind(), host);
    }

    pub fn get(&self, kind: SdkBackendKind) -> Option<Arc<dyn SdkBackendHost>> {
        self.hosts.get(&kind).cloned()
    }

    pub fn health(&self, kind: SdkBackendKind) -> Option<SdkBackendHealth> {
        self.get(kind).map(|host| host.health())
    }

    pub fn prepare_all(&self) -> Result<(), crate::host::SdkBackendError> {
        for host in self.hosts.values() {
            host.prepare()?;
        }
        Ok(())
    }
}
