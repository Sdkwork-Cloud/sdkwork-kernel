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
    pub owner_tenant_id: Option<String>,
    pub owner_user_ref: Option<String>,
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

/// Extract indexed ownership fields from session metadata JSON.
pub fn session_owner_fields_from_metadata_json(
    metadata_json: &Option<String>,
) -> (Option<String>, Option<String>) {
    let Some(raw) = metadata_json.as_deref() else {
        return (None, None);
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return (None, None);
    };
    let tenant = value
        .get("ownerTenantId")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let user = value
        .get("ownerUserRef")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    (tenant, user)
}

/// Session query parameters
#[derive(Debug, Clone, Default)]
pub struct SessionQuery {
    pub agent_id: Option<String>,
    pub state: Option<String>,
    pub kind: Option<String>,
    pub provider_id: Option<String>,
    pub bridge_id: Option<String>,
    pub owner_tenant_id: Option<String>,
    pub owner_user_ref: Option<String>,
    /// Return rows strictly after this session in `updated_at DESC, session_id DESC` order.
    pub after_session_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Message query parameters
#[derive(Debug, Clone, Default)]
pub struct MessageQuery {
    /// Return rows strictly after this message within the session (keyset continuation).
    pub after_message_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Event query parameters
#[derive(Debug, Clone, Default)]
pub struct EventQuery {
    pub event_type: Option<String>,
    pub severity: Option<String>,
    /// Return rows strictly after this event within the session (keyset continuation).
    pub after_event_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Task query parameters
#[derive(Debug, Clone, Default)]
pub struct TaskQuery {
    /// Return rows strictly after this task within the session (keyset continuation).
    pub after_task_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Permission list query parameters
#[derive(Debug, Clone, Default)]
pub struct PermissionQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
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
