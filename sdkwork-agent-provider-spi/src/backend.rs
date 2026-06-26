use serde::{Deserialize, Serialize};

/// External agent SDK runtime backend kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SdkBackendKind {
    #[serde(rename = "rust_native")]
    RustNative,
    #[serde(rename = "typescript_node")]
    TypeScriptNode,
    #[serde(rename = "python_process")]
    PythonProcess,
    #[serde(rename = "http_openapi")]
    HttpOpenApi,
    #[serde(rename = "ipc_protocol")]
    IpcProtocol,
}

impl SdkBackendKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RustNative => "rust_native",
            Self::TypeScriptNode => "typescript_node",
            Self::PythonProcess => "python_process",
            Self::HttpOpenApi => "http_openapi",
            Self::IpcProtocol => "ipc_protocol",
        }
    }
}

/// Canonical transport kind for external agent provider integration.
pub type ProviderTransportKind = SdkBackendKind;

/// Global default transport priority from `AGENT_PROVIDER_INTEGRATION_SPEC.md`.
pub fn default_transport_priority() -> &'static [ProviderTransportKind] {
    default_backend_priority()
}

/// Global default backend priority from `AGENT_PROVIDER_INTEGRATION_SPEC.md`.
pub fn default_backend_priority() -> &'static [SdkBackendKind] {
    &[
        SdkBackendKind::RustNative,
        SdkBackendKind::TypeScriptNode,
        SdkBackendKind::PythonProcess,
        SdkBackendKind::HttpOpenApi,
        SdkBackendKind::IpcProtocol,
    ]
}
