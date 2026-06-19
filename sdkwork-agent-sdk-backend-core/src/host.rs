use sdkwork_agent_sdk_spi::{SdkBackendKind, SdkDriverHealth};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SdkBackendStatus {
    Ready,
    Starting,
    Degraded,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdkBackendHealth {
    pub status: SdkBackendStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl SdkBackendHealth {
    pub fn ready() -> Self {
        Self {
            status: SdkBackendStatus::Ready,
            message: None,
        }
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            status: SdkBackendStatus::Unavailable,
            message: Some(message.into()),
        }
    }

    pub fn is_usable(&self) -> bool {
        matches!(
            self.status,
            SdkBackendStatus::Ready | SdkBackendStatus::Degraded | SdkBackendStatus::Starting
        )
    }

    pub fn to_driver_health(&self) -> SdkDriverHealth {
        match self.status {
            SdkBackendStatus::Ready => SdkDriverHealth::healthy(),
            SdkBackendStatus::Starting | SdkBackendStatus::Degraded => SdkDriverHealth::degraded(
                self.message
                    .clone()
                    .unwrap_or_else(|| "backend degraded".to_string()),
            ),
            SdkBackendStatus::Unavailable => SdkDriverHealth::unhealthy(
                self.message
                    .clone()
                    .unwrap_or_else(|| "backend unavailable".to_string()),
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkBackendError {
    pub message: String,
}

impl SdkBackendError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for SdkBackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SdkBackendError {}

/// Host-side contract for invoking an external SDK through a specific backend kind.
pub trait SdkBackendHost: Send + Sync {
    fn backend_kind(&self) -> SdkBackendKind;
    fn host_id(&self) -> &str;
    fn health(&self) -> SdkBackendHealth;
    fn prepare(&self) -> Result<(), SdkBackendError>;
    fn shutdown(&self) -> Result<(), SdkBackendError> {
        Ok(())
    }
}

macro_rules! declare_backend_host {
    ($name:ident, $kind:expr, $host_id:expr) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name {
            package_ref: String,
            health: SdkBackendHealth,
        }

        impl $name {
            pub fn new(package_ref: impl Into<String>) -> Self {
                Self {
                    package_ref: package_ref.into(),
                    health: SdkBackendHealth::ready(),
                }
            }

            pub fn package_ref(&self) -> &str {
                &self.package_ref
            }
        }

        impl SdkBackendHost for $name {
            fn backend_kind(&self) -> SdkBackendKind {
                $kind
            }

            fn host_id(&self) -> &str {
                $host_id
            }

            fn health(&self) -> SdkBackendHealth {
                self.health.clone()
            }

            fn prepare(&self) -> Result<(), SdkBackendError> {
                if self.package_ref.is_empty() {
                    return Err(SdkBackendError::new(format!(
                        "{} backend requires a package reference",
                        self.host_id()
                    )));
                }
                Ok(())
            }
        }
    };
}

declare_backend_host!(
    RustNativeBackendHost,
    SdkBackendKind::RustNative,
    "backend.host.rust-native"
);
declare_backend_host!(
    TypeScriptNodeBackendHost,
    SdkBackendKind::TypeScriptNode,
    "backend.host.typescript-node"
);
declare_backend_host!(
    PythonProcessBackendHost,
    SdkBackendKind::PythonProcess,
    "backend.host.python-process"
);
declare_backend_host!(
    HttpOpenApiBackendHost,
    SdkBackendKind::HttpOpenApi,
    "backend.host.http-openapi"
);
declare_backend_host!(
    IpcProtocolBackendHost,
    SdkBackendKind::IpcProtocol,
    "backend.host.ipc-protocol"
);
