use sdkwork_agent_provider_spi::{ProviderTransportKind, SdkBackendKind, SdkDriverHealth};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderTransportStatus {
    Ready,
    Starting,
    Degraded,
    Unavailable,
}

/// Deprecated alias retained for one release cycle.
pub type SdkBackendStatus = ProviderTransportStatus;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderTransportHealth {
    pub status: ProviderTransportStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Deprecated alias retained for one release cycle.
pub type SdkBackendHealth = ProviderTransportHealth;

impl ProviderTransportHealth {
    pub fn ready() -> Self {
        Self {
            status: ProviderTransportStatus::Ready,
            message: None,
        }
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            status: ProviderTransportStatus::Unavailable,
            message: Some(message.into()),
        }
    }

    pub fn is_usable(&self) -> bool {
        matches!(
            self.status,
            ProviderTransportStatus::Ready
                | ProviderTransportStatus::Degraded
                | ProviderTransportStatus::Starting
        )
    }

    pub fn to_driver_health(&self) -> SdkDriverHealth {
        match self.status {
            ProviderTransportStatus::Ready => SdkDriverHealth::healthy(),
            ProviderTransportStatus::Starting | ProviderTransportStatus::Degraded => {
                SdkDriverHealth::degraded(
                    self.message
                        .clone()
                        .unwrap_or_else(|| "transport degraded".to_string()),
                )
            }
            ProviderTransportStatus::Unavailable => SdkDriverHealth::unhealthy(
                self.message
                    .clone()
                    .unwrap_or_else(|| "transport unavailable".to_string()),
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderTransportError {
    pub message: String,
}

/// Deprecated alias retained for one release cycle.
pub type SdkBackendError = ProviderTransportError;

impl ProviderTransportError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ProviderTransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ProviderTransportError {}

/// Host-side contract for a provider transport surface.
pub trait ProviderTransportHost: Send + Sync {
    fn transport_kind(&self) -> ProviderTransportKind;
    fn host_id(&self) -> &str;
    fn health(&self) -> ProviderTransportHealth;
    fn prepare(&self) -> Result<(), ProviderTransportError>;
    fn shutdown(&self) -> Result<(), ProviderTransportError> {
        Ok(())
    }
}

macro_rules! declare_transport_host {
    ($name:ident, $kind:expr, $host_id:expr) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name {
            package_ref: String,
            health: ProviderTransportHealth,
        }

        impl $name {
            pub fn new(package_ref: impl Into<String>) -> Self {
                Self {
                    package_ref: package_ref.into(),
                    health: ProviderTransportHealth::ready(),
                }
            }

            pub fn package_ref(&self) -> &str {
                &self.package_ref
            }
        }

        impl ProviderTransportHost for $name {
            fn transport_kind(&self) -> ProviderTransportKind {
                $kind
            }

            fn host_id(&self) -> &str {
                $host_id
            }

            fn health(&self) -> ProviderTransportHealth {
                self.health.clone()
            }

            fn prepare(&self) -> Result<(), ProviderTransportError> {
                if self.package_ref.is_empty() {
                    return Err(ProviderTransportError::new(format!(
                        "{} transport requires a package reference",
                        self.host_id()
                    )));
                }
                Ok(())
            }
        }
    };
}

declare_transport_host!(
    RustNativeTransportHost,
    ProviderTransportKind::RustNative,
    "transport.host.rust-native"
);
declare_transport_host!(
    TypeScriptNodeTransportHost,
    ProviderTransportKind::TypeScriptNode,
    "transport.host.typescript-node"
);
declare_transport_host!(
    PythonProcessTransportHost,
    ProviderTransportKind::PythonProcess,
    "transport.host.python-process"
);
declare_transport_host!(
    HttpOpenApiTransportHost,
    ProviderTransportKind::HttpOpenApi,
    "transport.host.http-openapi"
);
declare_transport_host!(
    IpcProtocolTransportHost,
    ProviderTransportKind::IpcProtocol,
    "transport.host.ipc-protocol"
);

/// Deprecated aliases retained for one release cycle.
pub type RustNativeBackendHost = RustNativeTransportHost;
pub type TypeScriptNodeBackendHost = TypeScriptNodeTransportHost;
pub type PythonProcessBackendHost = PythonProcessTransportHost;
pub type HttpOpenApiBackendHost = HttpOpenApiTransportHost;
pub type IpcProtocolBackendHost = IpcProtocolTransportHost;

/// Deprecated trait alias for hosts registered before provider-transport rename.
pub trait SdkBackendHost: ProviderTransportHost {
    fn backend_kind(&self) -> SdkBackendKind {
        self.transport_kind()
    }
}

impl<T: ProviderTransportHost + ?Sized> SdkBackendHost for T {}
