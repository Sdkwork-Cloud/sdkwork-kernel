//! Provider transport host contracts for external agent framework integration.

mod bootstrap;
mod host;
mod registry;

pub use host::{
    HttpOpenApiBackendHost, HttpOpenApiTransportHost, IpcProtocolBackendHost,
    IpcProtocolTransportHost, ProviderTransportError, ProviderTransportHealth,
    ProviderTransportHost, ProviderTransportStatus, PythonProcessBackendHost,
    PythonProcessTransportHost, RustNativeBackendHost, RustNativeTransportHost,
    SdkBackendError, SdkBackendHealth, SdkBackendHost, SdkBackendStatus,
    TypeScriptNodeBackendHost, TypeScriptNodeTransportHost,
};
pub use registry::{BackendHostRegistry, ProviderTransportRegistry};
pub use bootstrap::ProviderTransportBootstrap;
