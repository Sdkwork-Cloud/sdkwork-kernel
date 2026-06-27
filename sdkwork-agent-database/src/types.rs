use serde::{Deserialize, Serialize};

/// Session row for database persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRow {
    pub session_id: String,
    pub agent_id: String,
    pub kind: String,
    pub source: String,
    pub state: String,
    pub title: Option<String>,
    pub model: Option<String>,
    pub cwd: Option<String>,
    pub provider_id: Option<String>,
    pub bridge_id: Option<String>,
    pub token_usage_json: Option<String>,
    pub message_count: i64,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub metadata_json: Option<String>,
}

/// Message row for database persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRow {
    pub message_id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
    pub metadata_json: Option<String>,
}

/// Task row for database persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRow {
    pub task_id: String,
    pub session_id: String,
    pub instruction: String,
    pub state: String,
    pub created_at: String,
    pub updated_at: Option<String>,
}

/// Event row for database persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRow {
    pub event_id: String,
    pub session_id: Option<String>,
    pub event_type: String,
    pub severity: String,
    pub payload: Option<String>,
    pub created_at: String,
}

/// Agent row for database persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRow {
    pub agent_id: String,
    pub name: String,
    pub kind: String,
    pub source: String,
    pub config_json: Option<String>,
    pub created_at: String,
    pub updated_at: Option<String>,
}

/// Session query parameters
#[derive(Debug, Clone, Default)]
pub struct SessionQuery {
    pub agent_id: Option<String>,
    pub state: Option<String>,
    pub kind: Option<String>,
    pub provider_id: Option<String>,
    pub bridge_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Message query parameters
#[derive(Debug, Clone, Default)]
pub struct MessageQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Event query parameters
#[derive(Debug, Clone, Default)]
pub struct EventQuery {
    pub event_type: Option<String>,
    pub severity: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Permission row for persisting permission request state across restarts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRow {
    pub permission_request_id: String,
    pub session_id: Option<String>,
    pub category: String,
    pub resource: String,
    pub side_effect_level: String,
    pub reason: String,
    pub status: String,
    pub owner_tenant_id: Option<String>,
    pub owner_user_ref: Option<String>,
    pub created_at: String,
    pub updated_at: Option<String>,
}
