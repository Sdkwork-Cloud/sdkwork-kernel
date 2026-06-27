use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::persistence::PersistenceState;

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
    pub components: Vec<ComponentHealth>,
}

/// Component health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub name: String,
    pub status: String,
    pub message: Option<String>,
}

/// Health check state
#[derive(Debug, Clone)]
pub struct HealthState {
    pub start_time: std::time::Instant,
    pub version: String,
}

impl HealthState {
    pub fn new() -> Self {
        Self {
            start_time: std::time::Instant::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

impl Default for HealthState {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn persistence_component_for_metrics(health: Result<bool, String>) -> ComponentHealth {
    match health {
        Ok(true) => ComponentHealth {
            name: "persistence".to_string(),
            status: "available".to_string(),
            message: None,
        },
        Ok(false) => ComponentHealth {
            name: "persistence".to_string(),
            status: "degraded".to_string(),
            message: Some("database health probe returned false".to_string()),
        },
        Err(error) => ComponentHealth {
            name: "persistence".to_string(),
            status: "unavailable".to_string(),
            message: Some(error),
        },
    }
}

pub(crate) fn aggregate_component_status(components: &[ComponentHealth]) -> String {
    if components
        .iter()
        .any(|component| component.status == "unavailable")
    {
        "unhealthy".to_string()
    } else if components
        .iter()
        .any(|component| component.status == "degraded")
    {
        "degraded".to_string()
    } else {
        "healthy".to_string()
    }
}

/// Health check handler
pub async fn health_check(
    State((health_state, persistence)): State<(Arc<HealthState>, Arc<PersistenceState>)>,
) -> (StatusCode, Json<HealthResponse>) {
    let persistence_health = persistence.run(|state| state.health()).await;
    let components = vec![
        ComponentHealth {
            name: "kernel".to_string(),
            status: "available".to_string(),
            message: None,
        },
        persistence_component_for_metrics(persistence_health),
    ];
    let status = aggregate_component_status(&components);
    let code = if status == "unhealthy" {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    };

    let response = HealthResponse {
        status,
        version: health_state.version.clone(),
        uptime_secs: health_state.uptime_secs(),
        components,
    };

    (code, Json(response))
}

/// Readiness check handler
pub async fn readiness_check(
    State((health_state, persistence)): State<(Arc<HealthState>, Arc<PersistenceState>)>,
) -> (StatusCode, Json<HealthResponse>) {
    let persistence_health = persistence.run(|state| state.health()).await;
    let components = vec![persistence_component_for_metrics(persistence_health)];
    let persistence_ready = components
        .first()
        .is_some_and(|component| component.status == "available");
    let response = HealthResponse {
        status: if persistence_ready {
            "ready".to_string()
        } else {
            "not_ready".to_string()
        },
        version: health_state.version.clone(),
        uptime_secs: health_state.uptime_secs(),
        components,
    };
    let code = if persistence_ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (code, Json(response))
}

/// Liveness check handler
pub async fn liveness_check(
    State((health_state, _persistence)): State<(Arc<HealthState>, Arc<PersistenceState>)>,
) -> (StatusCode, Json<HealthResponse>) {
    let response = HealthResponse {
        status: "alive".to_string(),
        version: health_state.version.clone(),
        uptime_secs: health_state.uptime_secs(),
        components: Vec::new(),
    };

    (StatusCode::OK, Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::PersistenceState;

    #[test]
    fn health_state_uptime() {
        let state = HealthState::new();
        let _ = state.uptime_secs();
    }

    #[tokio::test]
    async fn readiness_reports_database_state() {
        let health_state = Arc::new(HealthState::new());
        let persistence = Arc::new(PersistenceState::memory().expect("persistence"));
        let (status, Json(response)) = readiness_check(State((health_state, persistence))).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(response.status, "ready");
    }
}
