//! Backend host contracts for external agent SDK runtimes.

mod host;
mod registry;

pub use host::{
    HttpOpenApiBackendHost, IpcProtocolBackendHost, PythonProcessBackendHost,
    RustNativeBackendHost, SdkBackendError, SdkBackendHealth, SdkBackendHost, SdkBackendStatus,
    TypeScriptNodeBackendHost,
};
pub use registry::BackendHostRegistry;
