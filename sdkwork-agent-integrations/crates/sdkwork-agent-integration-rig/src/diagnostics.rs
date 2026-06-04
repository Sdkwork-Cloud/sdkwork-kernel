use crate::backend::RigBackendMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RigIntegrationDiagnostics {
    pub backend_mode: RigBackendMode,
    pub live_backend_configured: bool,
    pub fail_closed: bool,
    pub missing_secret_refs: Vec<String>,
}

impl RigIntegrationDiagnostics {
    pub fn fail_closed() -> Self {
        Self {
            backend_mode: RigBackendMode::FailClosed,
            live_backend_configured: false,
            fail_closed: true,
            missing_secret_refs: Vec::new(),
        }
    }
}
