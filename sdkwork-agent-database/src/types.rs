use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

pub const RUNTIME_TIMESTAMP_PATTERN: &str = "%Y-%m-%dT%H:%M:%S%.9fZ";

pub fn format_runtime_timestamp(value: chrono::DateTime<chrono::Utc>) -> String {
    sdkwork_utils_rust::format_datetime(value, Some(RUNTIME_TIMESTAMP_PATTERN))
}

pub fn runtime_now_timestamp() -> String {
    format_runtime_timestamp(sdkwork_utils_rust::now())
}

/// Latest runtime schema migration version required by all supported stores.
pub const CURRENT_SCHEMA_VERSION: i64 = 5;

/// Bounded result returned by one runtime retention pass.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimePurgeCounts {
    pub sessions: u64,
    pub messages: u64,
    pub tasks: u64,
    pub runs: u64,
    pub steps: u64,
    pub events: u64,
    pub permissions: u64,
    pub permission_operations: u64,
}

impl RuntimePurgeCounts {
    pub fn total(self) -> u64 {
        self.sessions
            .saturating_add(self.messages)
            .saturating_add(self.tasks)
            .saturating_add(self.runs)
            .saturating_add(self.steps)
            .saturating_add(self.events)
            .saturating_add(self.permissions)
            .saturating_add(self.permission_operations)
    }
}

/// Schema state used by readiness checks and maintenance diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSchemaStatus {
    pub version: i64,
    pub expected_version: i64,
    pub drift_free: bool,
}

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

/// Compares runtime session snapshots using parsed UTC timestamps where
/// possible. Unknown timestamp formats remain writable for compatibility.
pub fn session_snapshot_is_older(incoming: &SessionRow, existing: &SessionRow) -> bool {
    timestamp_is_older(
        incoming
            .updated_at
            .as_deref()
            .or(Some(incoming.created_at.as_str())),
        existing
            .updated_at
            .as_deref()
            .or(Some(existing.created_at.as_str())),
    )
}

/// Provider-owned rows may be claimed from an unowned runtime row, but one
/// provider must never overwrite a row already owned by another provider.
pub fn session_provider_conflicts(incoming: &SessionRow, existing: &SessionRow) -> bool {
    matches!(
        (incoming.provider_id.as_deref(), existing.provider_id.as_deref()),
        (Some(incoming), Some(existing)) if incoming != existing
    )
}

/// Ordinary CRUD writes must preserve exact provider ownership. Provider
/// synchronization uses `session_provider_conflicts` so it can claim an
/// unowned row through its dedicated conditional-write path.
pub fn session_provider_ownership_changes(incoming: &SessionRow, existing: &SessionRow) -> bool {
    incoming.provider_id != existing.provider_id
}

pub fn session_state_is_terminal(state: &str) -> bool {
    matches!(
        state.to_ascii_lowercase().as_str(),
        "closed" | "failed" | "archived"
    )
}

pub fn session_state_regresses_from_terminal(incoming: &SessionRow, existing: &SessionRow) -> bool {
    session_state_is_terminal(&existing.state) && !session_state_is_terminal(&incoming.state)
}

pub fn ordinary_session_update_conflicts(incoming: &SessionRow, existing: &SessionRow) -> bool {
    session_provider_ownership_changes(incoming, existing)
        || session_state_regresses_from_terminal(incoming, existing)
}

pub fn task_state_is_terminal(state: &str) -> bool {
    matches!(
        state.to_ascii_lowercase().as_str(),
        "completed" | "failed" | "cancelled" | "canceled"
    )
}

pub fn task_update_conflicts(incoming: &TaskRow, existing: &TaskRow) -> bool {
    incoming.session_id != existing.session_id
        || (task_state_is_terminal(&existing.state)
            && !incoming.state.eq_ignore_ascii_case(&existing.state))
}

pub fn timestamp_is_older(incoming: Option<&str>, existing: Option<&str>) -> bool {
    match (
        incoming.and_then(|value| sdkwork_utils_rust::parse_datetime(value, None)),
        existing.and_then(|value| sdkwork_utils_rust::parse_datetime(value, None)),
    ) {
        (Some(incoming), Some(existing)) => incoming < existing,
        _ => false,
    }
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

macro_rules! string_enum {
    ($name:ident { $($variant:ident => $wire:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $wire),+
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = String;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value {
                    $($wire => Ok(Self::$variant)),+,
                    _ => Err(format!("unknown {} value: {value}", stringify!($name))),
                }
            }
        }
    };
}

string_enum!(RunState {
    Created => "created",
    Planning => "planning",
    Executing => "executing",
    AwaitingPermission => "awaiting_permission",
    Paused => "paused",
    Completed => "completed",
    Failed => "failed",
    Cancelled => "cancelled",
});

string_enum!(StepState {
    Created => "created",
    Ready => "ready",
    Running => "running",
    AwaitingPermission => "awaiting_permission",
    Completed => "completed",
    Failed => "failed",
    Skipped => "skipped",
    Cancelled => "cancelled",
});

string_enum!(ActionKind {
    ModelCall => "model_call",
    ToolCall => "tool_call",
    MemoryRead => "memory_read",
    MemoryWrite => "memory_write",
    HostOperation => "host_operation",
    ProtocolSend => "protocol_send",
    Handoff => "handoff",
    WaitForUser => "wait_for_user",
    Internal => "internal",
});

string_enum!(PermissionOperationState {
    Pending => "pending",
    Decided => "decided",
    Claimable => "claimable",
    Executing => "executing",
    Completed => "completed",
    Failed => "failed",
    Expired => "expired",
    Cancelled => "cancelled",
});

string_enum!(PermissionPayloadKind {
    Ciphertext => "ciphertext",
    SecretRef => "secret_ref",
});

string_enum!(RunControlAction {
    Pause => "pause",
    Resume => "resume",
    Cancel => "cancel",
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunRow {
    pub run_id: String,
    pub task_id: String,
    pub session_id: String,
    pub attempt: i64,
    pub state: RunState,
    pub next_attempt_at: Option<String>,
    pub lease_owner: Option<String>,
    pub lease_expires_at: Option<String>,
    pub fencing_token: i64,
    pub cancel_requested_at: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub error_kind: Option<String>,
    pub error_code: Option<String>,
    pub error_detail: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepRow {
    pub step_id: String,
    pub run_id: String,
    pub sequence_no: i64,
    pub action_kind: ActionKind,
    pub state: StepState,
    pub provider_id: Option<String>,
    pub descriptor_revision: Option<String>,
    pub policy_revision: Option<String>,
    pub causation_step_id: Option<String>,
    pub idempotency_key_hash: Option<String>,
    pub result_json: Option<String>,
    pub error_kind: Option<String>,
    pub error_code: Option<String>,
    pub error_detail: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionOperationRow {
    pub permission_request_id: String,
    pub run_id: String,
    pub step_id: String,
    pub tool_call_id: String,
    pub provider_id: String,
    pub descriptor_revision: String,
    pub policy_revision: String,
    pub payload_kind: PermissionPayloadKind,
    pub payload_ref: String,
    pub payload_digest: String,
    pub encryption_key_id: Option<String>,
    pub state: PermissionOperationState,
    pub expires_at: String,
    pub lease_owner: Option<String>,
    pub lease_expires_at: Option<String>,
    pub fencing_token: i64,
    pub result_json: Option<String>,
    pub error_kind: Option<String>,
    pub error_code: Option<String>,
    pub error_detail: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimedRun {
    pub run: RunRow,
    pub step: StepRow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimedPermissionOperation {
    pub operation: PermissionOperationRow,
    pub run: RunRow,
    pub step: StepRow,
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
    /// Persisted `COALESCE(updated_at, created_at)` key carried by an opaque cursor.
    pub after_session_sort_at: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Message query parameters
#[derive(Debug, Clone, Default)]
pub struct MessageQuery {
    /// Return rows strictly after this message within the session (keyset continuation).
    pub after_message_id: Option<String>,
    /// Persisted `created_at` key carried by an opaque cursor.
    pub after_message_created_at: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Event query parameters
#[derive(Debug, Clone, Default)]
pub struct EventQuery {
    pub event_type: Option<String>,
    pub severity: Option<String>,
    /// Restrict events to sessions owned by this tenant. Global events are excluded.
    pub owner_tenant_id: Option<String>,
    /// Restrict events to sessions owned by this user. Global events are excluded.
    pub owner_user_ref: Option<String>,
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
    /// Persisted `created_at` key carried by an opaque cursor.
    pub after_task_created_at: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Permission list query parameters
#[derive(Debug, Clone, Default)]
pub struct PermissionQuery {
    pub status: Option<String>,
    pub owner_tenant_id: Option<String>,
    pub owner_user_ref: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_comparison_normalizes_timezone_offsets() {
        assert!(timestamp_is_older(
            Some("2026-07-15T08:01:00+08:00"),
            Some("2026-07-15T00:02:00Z")
        ));
    }

    #[test]
    fn runtime_timestamp_is_fixed_width_utc_and_lexically_ordered() {
        let earlier = sdkwork_utils_rust::parse_datetime("2026-07-15T00:00:00.000000001Z", None)
            .expect("earlier");
        let later = sdkwork_utils_rust::parse_datetime("2026-07-15T00:00:00.000000010Z", None)
            .expect("later");
        let earlier = format_runtime_timestamp(earlier);
        let later = format_runtime_timestamp(later);
        assert_eq!(earlier.len(), 30);
        assert_eq!(later.len(), 30);
        assert!(earlier.ends_with('Z'));
        assert!(earlier < later);
        assert_eq!(runtime_now_timestamp().len(), 30);
    }
}
