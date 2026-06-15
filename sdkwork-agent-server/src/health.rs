use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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

/// Health check handler
pub async fn health_check(
    State(state): State<Arc<HealthState>>,
) -> (StatusCode, Json<HealthResponse>) {
    let response = HealthResponse {
        status: "healthy".to_string(),
        version: state.version.clone(),
        uptime_secs: state.uptime_secs(),
        components: vec![
            ComponentHealth {
                name: "kernel".to_string(),
                status: "available".to_string(),
                message: None,
            },
            ComponentHealth {
                name: "business".to_string(),
                status: "available".to_string(),
                message: None,
            },
        ],
    };

    (StatusCode::OK, Json(response))
}

/// Readiness check handler
pub async fn readiness_check() -> (StatusCode, Json<HealthResponse>) {
    let response = HealthResponse {
        status: "ready".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: 0,
        components: Vec::new(),
    };

    (StatusCode::OK, Json(response))
}

/// Liveness check handler
pub async fn liveness_check() -> (StatusCode, Json<HealthResponse>) {
    let response = HealthResponse {
        status: "alive".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: 0,
        components: Vec::new(),
    };

    (StatusCode::OK, Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_state_uptime() {
        let state = HealthState::new();
        // Just verify it doesn't panic
        let _ = state.uptime_secs();
    }
}
