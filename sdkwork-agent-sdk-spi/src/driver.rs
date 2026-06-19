use crate::backend::SdkBackendKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SdkDriverStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdkDriverHealth {
    pub status: SdkDriverStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl SdkDriverHealth {
    pub fn healthy() -> Self {
        Self {
            status: SdkDriverStatus::Healthy,
            message: None,
        }
    }

    pub fn degraded(message: impl Into<String>) -> Self {
        Self {
            status: SdkDriverStatus::Degraded,
            message: Some(message.into()),
        }
    }

    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self {
            status: SdkDriverStatus::Unhealthy,
            message: Some(message.into()),
        }
    }

    pub fn is_usable(&self) -> bool {
        matches!(
            self.status,
            SdkDriverStatus::Healthy | SdkDriverStatus::Degraded
        )
    }
}

/// Base trait for all external agent SDK capability drivers.
pub trait AgentSdkCapabilityDriver: Send + Sync {
    fn driver_id(&self) -> &str;
    fn capability_id(&self) -> &str;
    fn backend_kind(&self) -> SdkBackendKind;
    fn health(&self) -> SdkDriverHealth;
}

/// Manifest-backed driver placeholder used during binding negotiation and adapter bootstrap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticCapabilityDriver {
    driver_id: String,
    capability_id: String,
    backend_kind: SdkBackendKind,
    health: SdkDriverHealth,
}

impl StaticCapabilityDriver {
    pub fn new(
        driver_id: impl Into<String>,
        capability_id: impl Into<String>,
        backend_kind: SdkBackendKind,
    ) -> Self {
        Self {
            driver_id: driver_id.into(),
            capability_id: capability_id.into(),
            backend_kind,
            health: SdkDriverHealth::healthy(),
        }
    }

    pub fn with_health(mut self, health: SdkDriverHealth) -> Self {
        self.health = health;
        self
    }
}

impl AgentSdkCapabilityDriver for StaticCapabilityDriver {
    fn driver_id(&self) -> &str {
        &self.driver_id
    }

    fn capability_id(&self) -> &str {
        &self.capability_id
    }

    fn backend_kind(&self) -> SdkBackendKind {
        self.backend_kind
    }

    fn health(&self) -> SdkDriverHealth {
        self.health.clone()
    }
}
