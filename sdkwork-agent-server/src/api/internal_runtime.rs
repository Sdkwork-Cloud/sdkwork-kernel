use axum::{
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        Response,
    },
    Json,
};
use futures::stream::{self, Stream};
use sdkwork_agent_api_bridge::BridgeSessionConfig;
use sdkwork_agent_database::{
    EventRow, MessageQuery, MessageRow, PermissionRow, SessionRow, TaskQuery, TaskRow,
};
use sdkwork_agent_kernel::{AgentMessage, AgentMessageRole, AgentPart};
use sdkwork_agent_session::{SessionConfig, SessionQuery};
use sdkwork_utils_rust::{
    base64url_decode, base64url_encode, hmac_sha256_base64url, secure_compare,
    DEFAULT_LIST_PAGE_SIZE, MAX_LIST_PAGE_SIZE,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tokio_stream::wrappers::ReceiverStream;

type SessionEventStream = Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>>;

use crate::access::{
    assert_permission_access, assert_session_access, stamp_session_ownership, AccessPolicy,
};
use crate::agent_registry::{apply_hosted_agent_defaults, validate_hosted_agent_id};
use crate::config::ServerConfig;
use crate::http_response::{
    api_created, api_item, api_no_content, catalog_list_response, cursor_list_response, ApiError,
};
use crate::metrics::MetricsRegistry;
use crate::middleware::RequestContext;
use crate::persistence::PersistenceState;
use crate::runtime::RuntimeState;
use crate::tenant_token_quota::{TenantTokenQuotaReservation, TenantTokenQuotaState};

/// Maximum concurrent SSE event stream connections per server instance.
/// Prevents resource exhaustion from too many long-lived connections.
const MAX_CONCURRENT_SSE_STREAMS: u32 = 256;
const MAX_MESSAGE_CONTENT_BYTES: usize = 256 * 1024;
const MAX_TASK_INSTRUCTION_BYTES: usize = 64 * 1024;
const MAX_TOOL_INPUT_BYTES: usize = 256 * 1024;
const MAX_SESSION_TEXT_BYTES: usize = 256 * 1024;
const MAX_MODEL_ID_BYTES: usize = 256;
const MAX_MODEL_OVERRIDE_MESSAGES: usize = 128;
const MAX_METADATA_ENTRIES: usize = 64;
const MAX_METADATA_KEY_BYTES: usize = 128;
const MAX_METADATA_VALUE_BYTES: usize = 4096;
const MAX_WORKSPACE_ROOTS: usize = 32;
const MAX_WORKSPACE_ROOT_BYTES: usize = 4096;

/// Shared internal-api runtime HTTP handler state.
#[derive(Clone)]
pub struct InternalRuntimeApiState {
    pub persistence: Arc<PersistenceState>,
    pub config: Arc<ServerConfig>,
    pub access_policy: AccessPolicy,
    pub runtime: RuntimeState,
    pub tenant_token_quota: Arc<TenantTokenQuotaState>,
    pub sse_event_counter: Arc<tokio::sync::Mutex<u64>>,
    pub sse_connection_count: Arc<AtomicU32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRequestJson {
    pub permission_request_id: String,
    pub category: String,
    pub resource: String,
    pub side_effect_level: String,
    pub reason: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PermissionDecisionBody {
    pub decision: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchOperationJson {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchSetJson {
    pub patch_id: String,
    pub workspace_id: String,
    pub summary: String,
    pub operations: Vec<PatchOperationJson>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResultJson {
    pub command_id: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub cancelled: bool,
    pub timed_out: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationReportJson {
    pub report_id: String,
    pub verification_id: String,
    pub command_results: Vec<CommandResultJson>,
    pub failures: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalCommandJson {
    pub command_id: String,
    pub command: String,
    pub args: Vec<String>,
    pub working_directory: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_categories: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalOutputChunkJson {
    pub command_id: String,
    pub sequence: u64,
    pub channel: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redaction_classification: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewFindingJson {
    pub finding_id: String,
    pub severity: String,
    pub file_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub missing_test: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSnapshotJson {
    pub runtime: KernelRuntimeJson,
    pub events: Vec<KernelEventJson>,
    pub permissions: Vec<PermissionRequestJson>,
    pub workspace: WorkspaceJson,
    pub patches: Vec<PatchSetJson>,
    pub verification_reports: Vec<VerificationReportJson>,
    pub terminal_commands: Vec<TerminalCommandJson>,
    pub terminal_output: Vec<TerminalOutputChunkJson>,
    pub review_findings: Vec<ReviewFindingJson>,
}

/// Runtime manifest view returned by `GET /runtime/manifest`.
///
/// Exposes the capability manifest (runtime id, agent id, kernel version,
/// capabilities, providers) without persistence-derived state. Suitable for
/// UI clients, CI gates, and conformance runners.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeManifestJson {
    pub runtime_id: String,
    pub agent_id: String,
    pub kernel_version: String,
    pub security_profile: String,
    pub capabilities: Vec<CapabilityJson>,
    pub providers: Vec<ProviderManifestJson>,
    pub missing_required_capabilities: Vec<String>,
    pub degraded_capabilities: Vec<String>,
}

/// Provider manifest view embedded in [`RuntimeManifestJson`].
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderManifestJson {
    pub provider_id: String,
    pub provider_family: String,
    pub name: String,
    pub version: String,
    pub capabilities: Vec<String>,
    pub health_status: Option<String>,
}

/// Runtime health view returned by `GET /internal/v3/api/intelligence/runtime/health`.
///
/// Lightweight liveness/readiness probe surface combining runtime state and
/// persistence health. Side-effect-free; safe for load-balancer polls.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeHealthJson {
    pub runtime_id: String,
    pub state: String,
    pub health: String,
    pub persistence_healthy: bool,
    pub degraded_capabilities: Vec<String>,
}

/// Runtime diagnostics view returned by `GET /runtime/diagnostics`.
///
/// Machine-readable report validating against
/// `schemas/agent-runtime-diagnostics.schema.json`. Suitable for support
/// bundles, registry validation, and conformance runners.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeDiagnosticsJson {
    pub runtime_id: String,
    pub agent_id: String,
    pub state: String,
    pub provider_count: usize,
    pub capability_count: usize,
    pub typed_provider_count: usize,
    pub manifest_only_provider_count: usize,
    pub missing_required_capabilities: Vec<String>,
    pub degraded_capabilities: Vec<String>,
    pub provider_diagnostics: Vec<ProviderDiagnosticJson>,
}

/// Per-provider diagnostic entry embedded in [`RuntimeDiagnosticsJson`].
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDiagnosticJson {
    pub provider_id: String,
    pub provider_family: String,
    pub provider_version: String,
    pub typed_registered: bool,
    pub health_status: Option<String>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelRuntimeJson {
    pub runtime_id: String,
    pub agent_id: String,
    pub kernel_version: String,
    pub state: String,
    pub health: String,
    pub capabilities: Vec<CapabilityJson>,
    pub missing_required_capabilities: Vec<String>,
    pub degraded_capabilities: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityJson {
    pub capability_id: String,
    pub provider_id: String,
    pub status: String,
    pub required: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelEventJson {
    pub event_id: String,
    pub event_type: String,
    pub severity: String,
    pub summary: String,
    pub sequence: u32,
    pub trace_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceJson {
    pub workspace_id: String,
    pub root: String,
    pub branch: String,
    pub dirty: bool,
    pub changed_files: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateKernelSessionRequest {
    pub agent_id: String,
    pub tenant_id: Option<String>,
    pub user_ref: Option<String>,
    pub model: Option<String>,
    pub model_provider: Option<String>,
    pub title: Option<String>,
    pub goal: Option<String>,
    pub instructions: Option<String>,
    pub cwd: Option<String>,
    pub workspace_roots: Option<Vec<String>>,
    pub source: Option<String>,
    pub kind: Option<String>,
    pub timeout_ms: Option<u64>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionViewJson {
    pub session_id: String,
    pub source: String,
    pub kind: String,
    pub agent_id: String,
    pub user_ref: Option<String>,
    pub tenant_id: Option<String>,
    pub title: Option<String>,
    pub goal: Option<String>,
    pub state: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub model: Option<String>,
    pub model_provider: Option<String>,
    pub cwd: Option<String>,
    pub workspace_roots: Vec<String>,
    pub instructions: Option<String>,
    pub token_usage: TokenUsageJson,
    pub message_count: u32,
    pub tool_call_count: u32,
    pub compression_count: u32,
    pub change_summary: ChangeSummaryJson,
    pub child_session_ids: Vec<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsageJson {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_tokens: u64,
    pub reasoning_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeSummaryJson {
    pub additions: u32,
    pub deletions: u32,
    pub files_changed: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SendKernelMessageRequest {
    pub content: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageViewJson {
    pub message_id: String,
    pub session_id: String,
    pub role: String,
    pub parts: Vec<MessagePartJson>,
    pub created_at: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagePartJson {
    pub part_id: String,
    pub kind: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageTurnViewJson {
    pub user_message: MessageViewJson,
    pub assistant_message: Option<MessageViewJson>,
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubmitTaskRequest {
    pub instruction: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskViewJson {
    pub task_id: String,
    pub session_id: String,
    pub instruction: String,
    pub state: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDescriptorJson {
    pub model_id: String,
    pub provider_id: String,
    pub display_name: String,
    pub family: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDescriptorJson {
    pub tool_id: String,
    pub provider_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub side_effect_level: String,
    pub policy_categories: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InvokeModelRequest {
    pub model_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelResponseJson {
    pub model_request_id: String,
    pub provider_id: String,
    pub status: String,
    pub messages: Vec<String>,
    pub tool_calls: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecuteToolRequest {
    pub input: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallJson {
    pub tool_call_id: String,
    pub tool_id: String,
    pub input: String,
    pub status: String,
    pub output: Option<String>,
}

/// Request body for `POST /sessions/{sessionId}/model/stream`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StreamModelRequest {
    pub model_id: Option<String>,
    pub messages: Option<Vec<String>>,
}

/// Request body for `POST /sessions/{sessionId}/model/cancel`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CancelModelRequest {
    pub model_request_id: String,
    pub provider_id: Option<String>,
}

/// SSE chunk for model streaming output.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelStreamChunkJson {
    pub model_request_id: String,
    pub sequence: u64,
    pub content: String,
    pub finish_reason: Option<String>,
}

/// Response for model cancellation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCancelResponseJson {
    pub model_request_id: String,
    pub provider_id: String,
    pub status: String,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamEventJson {
    pub event_id: String,
    pub event_type: String,
    pub sequence: u32,
    pub payload: String,
    pub timestamp: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ListMessagesQuery {
    pub page_size: Option<i64>,
    pub cursor: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ListSessionsQuery {
    pub page_size: Option<i64>,
    pub cursor: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ListTasksQuery {
    pub page_size: Option<i64>,
    pub cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamEventsQuery {
    pub last_event_id: Option<String>,
    pub live: Option<bool>,
}

impl InternalRuntimeApiState {
    pub async fn new_async(
        persistence: Arc<PersistenceState>,
        config: Arc<ServerConfig>,
    ) -> Result<Self, sdkwork_agent_kernel::KernelError> {
        let tenant_token_quota = TenantTokenQuotaState::try_from_config_async(&config)
            .await
            .map_err(|message| sdkwork_agent_kernel::KernelError::Internal { message })?;
        Self::with_tenant_token_quota(persistence, config, tenant_token_quota)
    }

    pub fn new(
        persistence: Arc<PersistenceState>,
        config: Arc<ServerConfig>,
    ) -> Result<Self, sdkwork_agent_kernel::KernelError> {
        let tenant_token_quota = TenantTokenQuotaState::try_from_config(&config)
            .map_err(|message| sdkwork_agent_kernel::KernelError::Internal { message })?;
        Self::with_tenant_token_quota(persistence, config, tenant_token_quota)
    }

    fn with_tenant_token_quota(
        persistence: Arc<PersistenceState>,
        config: Arc<ServerConfig>,
        tenant_token_quota: TenantTokenQuotaState,
    ) -> Result<Self, sdkwork_agent_kernel::KernelError> {
        Ok(Self {
            persistence,
            config: config.clone(),
            access_policy: AccessPolicy::from_config(&config),
            runtime: RuntimeState::try_for_config(&config)?,
            tenant_token_quota: Arc::new(tenant_token_quota),
            sse_event_counter: Arc::new(tokio::sync::Mutex::new(0)),
            sse_connection_count: Arc::new(AtomicU32::new(0)),
        })
    }

    pub(crate) async fn persist<F, T>(&self, operation: F) -> Result<T, String>
    where
        F: FnOnce(&PersistenceState) -> Result<T, String> + Send + 'static,
        T: Send + 'static,
    {
        self.persistence.run(operation).await
    }

    async fn build_snapshot(&self, ctx: &RequestContext) -> Result<RuntimeSnapshotJson, String> {
        let db_healthy = matches!(
            self.persist(|persistence| persistence.health()).await,
            Ok(true)
        );
        let diagnostics = self.runtime.agent_runtime().diagnostics();
        let manifest = self.runtime.agent_runtime().capability_manifest();
        let allow_mock = self.runtime.allow_mock_fallback();

        let runtime_state = diagnostics.state.clone();
        let runtime_health = if db_healthy && (runtime_state == "ready" || allow_mock) {
            "healthy"
        } else {
            "degraded"
        };

        let mut degraded_capabilities = manifest.degraded_capabilities.clone();
        if !db_healthy {
            degraded_capabilities.push("persistence.sqlite".to_string());
        }

        let capabilities = manifest
            .capabilities
            .iter()
            .map(|capability| CapabilityJson {
                capability_id: capability.capability_id.clone(),
                provider_id: capability.provider_id.clone(),
                status: capability.status.clone(),
                required: capability.required,
            })
            .collect();

        let (owner_tenant_id, owner_user_ref) = if self.access_policy.enforce_session_scope {
            (
                ctx.tenant_id.clone(),
                ctx.user_id.clone().or_else(|| ctx.subject_id.clone()),
            )
        } else {
            (None, None)
        };

        let permission_tenant = owner_tenant_id.clone();
        let permission_user = owner_user_ref.clone();
        let permissions = self
            .persist(move |persistence| {
                persistence.list_permissions(sdkwork_agent_database::PermissionQuery {
                    status: None,
                    owner_tenant_id: permission_tenant,
                    owner_user_ref: permission_user,
                    limit: Some(100),
                    offset: None,
                })
            })
            .await
            .map_err(|error| format!("failed to load permissions: {error}"))?
            .into_iter()
            .map(permission_row_to_view)
            .collect();

        let event_rows = self
            .persist(move |persistence| {
                persistence.list_recent_events(sdkwork_agent_database::EventQuery {
                    event_type: None,
                    severity: None,
                    after_event_id: None,
                    owner_tenant_id,
                    owner_user_ref,
                    limit: Some(100),
                    offset: None,
                })
            })
            .await
            .map_err(|error| format!("failed to load recent events: {error}"))?;
        let events = event_rows
            .iter()
            .enumerate()
            .map(|(index, row)| event_row_to_kernel_json(row, index as u32))
            .collect();

        Ok(RuntimeSnapshotJson {
            runtime: KernelRuntimeJson {
                runtime_id: diagnostics.runtime_id.clone(),
                agent_id: diagnostics.agent_id.clone(),
                kernel_version: env!("CARGO_PKG_VERSION").to_string(),
                state: runtime_state.clone(),
                health: runtime_health.to_string(),
                capabilities,
                missing_required_capabilities: manifest.missing_required_capabilities.clone(),
                degraded_capabilities,
            },
            events,
            permissions,
            workspace: WorkspaceJson {
                workspace_id: diagnostics.runtime_id.clone(),
                root: diagnostics.agent_id.clone(),
                branch: runtime_state,
                dirty: false,
                changed_files: Vec::new(),
            },
            patches: Vec::new(),
            verification_reports: Vec::new(),
            terminal_commands: Vec::new(),
            terminal_output: Vec::new(),
            review_findings: Vec::new(),
        })
    }

    pub(crate) async fn register_persisted_session(
        &self,
        row: &SessionRow,
        trace_id: &str,
    ) -> Result<(), ApiError> {
        const MAX_HYDRATED_MESSAGES: i64 = 512;
        let session_id = row.session_id.clone();
        let messages = self
            .persist(move |persistence| {
                persistence.load_recent_messages(&session_id, MAX_HYDRATED_MESSAGES)
            })
            .await
            .map_err(|error| ApiError::from_persistence(error, trace_id))?;
        let history = messages
            .into_iter()
            .map(message_row_to_agent_message)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| {
                tracing::error!(
                    session_id = %row.session_id,
                    error = %error,
                    trace_id,
                    "persisted message history is invalid"
                );
                ApiError::internal("persisted message history is invalid", trace_id)
            })?;
        self.runtime
            .register_session_with_history(&row.session_id, bridge_config_from_row(row), history)
            .map_err(|error| ApiError::from_kernel(error, trace_id))?;
        Ok(())
    }
}

fn validate_required_text(
    value: &str,
    field: &str,
    max_bytes: usize,
    trace_id: &str,
) -> Result<(), ApiError> {
    if value.trim().is_empty() {
        return Err(ApiError::invalid_parameter(
            format!("{field} must not be empty"),
            trace_id,
        ));
    }
    if value.len() > max_bytes {
        return Err(ApiError::invalid_parameter(
            format!("{field} exceeds {max_bytes} bytes"),
            trace_id,
        ));
    }
    Ok(())
}

fn validate_optional_text(
    value: Option<&str>,
    field: &str,
    max_bytes: usize,
    trace_id: &str,
) -> Result<(), ApiError> {
    if let Some(value) = value {
        validate_required_text(value, field, max_bytes, trace_id)?;
    }
    Ok(())
}

fn validate_create_session_request(
    request: &CreateKernelSessionRequest,
    trace_id: &str,
) -> Result<(), ApiError> {
    validate_required_text(&request.agent_id, "agentId", MAX_MODEL_ID_BYTES, trace_id)?;
    validate_optional_text(
        request.model.as_deref(),
        "model",
        MAX_MODEL_ID_BYTES,
        trace_id,
    )?;
    validate_optional_text(request.title.as_deref(), "title", 1024, trace_id)?;
    validate_optional_text(
        request.goal.as_deref(),
        "goal",
        MAX_SESSION_TEXT_BYTES,
        trace_id,
    )?;
    validate_optional_text(
        request.instructions.as_deref(),
        "instructions",
        MAX_SESSION_TEXT_BYTES,
        trace_id,
    )?;
    validate_optional_text(
        request.cwd.as_deref(),
        "cwd",
        MAX_WORKSPACE_ROOT_BYTES,
        trace_id,
    )?;
    if let Some(roots) = request.workspace_roots.as_ref() {
        if roots.len() > MAX_WORKSPACE_ROOTS {
            return Err(ApiError::invalid_parameter(
                "workspaceRoots exceeds 32 entries",
                trace_id,
            ));
        }
        for root in roots {
            validate_required_text(
                root,
                "workspaceRoots item",
                MAX_WORKSPACE_ROOT_BYTES,
                trace_id,
            )?;
        }
    }
    if let Some(metadata) = request.metadata.as_ref() {
        if metadata.len() > MAX_METADATA_ENTRIES {
            return Err(ApiError::invalid_parameter(
                "metadata exceeds 64 entries",
                trace_id,
            ));
        }
        for (key, value) in metadata {
            validate_required_text(key, "metadata key", MAX_METADATA_KEY_BYTES, trace_id)?;
            if value.len() > MAX_METADATA_VALUE_BYTES {
                return Err(ApiError::invalid_parameter(
                    "metadata value exceeds 4096 bytes",
                    trace_id,
                ));
            }
        }
    }
    Ok(())
}

fn validate_stream_model_request(
    request: &StreamModelRequest,
    trace_id: &str,
) -> Result<(), ApiError> {
    validate_optional_text(
        request.model_id.as_deref(),
        "modelId",
        MAX_MODEL_ID_BYTES,
        trace_id,
    )?;
    if let Some(messages) = request.messages.as_ref() {
        if messages.len() > MAX_MODEL_OVERRIDE_MESSAGES {
            return Err(ApiError::invalid_parameter(
                "messages exceeds 128 entries",
                trace_id,
            ));
        }
        for message in messages {
            validate_required_text(
                message,
                "messages item",
                MAX_MESSAGE_CONTENT_BYTES,
                trace_id,
            )?;
        }
    }
    Ok(())
}

fn session_row_to_view(row: SessionRow) -> SessionViewJson {
    let metadata = parse_metadata_map(row.metadata_json.as_deref());
    let token_usage = parse_token_usage(row.token_usage_json.as_deref());
    SessionViewJson {
        session_id: row.session_id,
        source: row.source,
        kind: row.kind,
        agent_id: row.agent_id,
        user_ref: metadata.get("userRef").cloned(),
        tenant_id: metadata.get("tenantId").cloned(),
        title: row.title,
        goal: metadata.get("goal").cloned(),
        state: row.state,
        created_at: Some(row.created_at),
        updated_at: row.updated_at,
        model: row.model,
        model_provider: metadata.get("modelProvider").cloned(),
        cwd: row.cwd,
        workspace_roots: metadata
            .get("workspaceRoots")
            .and_then(|value| serde_json::from_str::<Vec<String>>(value).ok())
            .unwrap_or_default(),
        instructions: metadata.get("instructions").cloned(),
        token_usage,
        message_count: row.message_count.max(0) as u32,
        tool_call_count: metadata
            .get("toolCallCount")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0),
        compression_count: metadata
            .get("compressionCount")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0),
        change_summary: parse_change_summary(metadata.get("changeSummary")),
        child_session_ids: metadata
            .get("childSessionIds")
            .and_then(|value| serde_json::from_str::<Vec<String>>(value).ok())
            .unwrap_or_default(),
        metadata,
    }
}

/// Parse token usage from the JSON column stored in the sessions table.
/// The expected format is: {"inputTokens":N,"outputTokens":N,...}
fn parse_token_usage(raw: Option<&str>) -> TokenUsageJson {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct UsageJson {
        #[serde(default)]
        input_tokens: u64,
        #[serde(default)]
        output_tokens: u64,
        #[serde(default)]
        cached_tokens: u64,
        #[serde(default)]
        reasoning_tokens: u64,
        #[serde(default)]
        total_tokens: u64,
    }

    let usage = raw
        .and_then(|value| serde_json::from_str::<UsageJson>(value).ok())
        .unwrap_or(UsageJson {
            input_tokens: 0,
            output_tokens: 0,
            cached_tokens: 0,
            reasoning_tokens: 0,
            total_tokens: 0,
        });

    TokenUsageJson {
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        cached_tokens: usage.cached_tokens,
        reasoning_tokens: usage.reasoning_tokens,
        total_tokens: usage.total_tokens,
    }
}

/// Parse change summary from metadata.
fn parse_change_summary(raw: Option<&String>) -> ChangeSummaryJson {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SummaryJson {
        #[serde(default)]
        additions: u32,
        #[serde(default)]
        deletions: u32,
        #[serde(default)]
        files_changed: u32,
    }

    let summary = raw
        .and_then(|value| serde_json::from_str::<SummaryJson>(value).ok())
        .unwrap_or(SummaryJson {
            additions: 0,
            deletions: 0,
            files_changed: 0,
        });

    ChangeSummaryJson {
        additions: summary.additions,
        deletions: summary.deletions,
        files_changed: summary.files_changed,
    }
}

/// Convert a persisted PermissionRow to the API view JSON.
fn permission_row_to_view(row: PermissionRow) -> PermissionRequestJson {
    PermissionRequestJson {
        permission_request_id: row.permission_request_id,
        category: row.category,
        resource: row.resource,
        side_effect_level: row.side_effect_level,
        reason: row.reason,
        status: row.status,
    }
}

fn message_row_to_view(row: MessageRow) -> MessageViewJson {
    MessageViewJson {
        message_id: row.message_id.clone(),
        session_id: row.session_id,
        role: row.role,
        parts: vec![MessagePartJson {
            part_id: format!("{}/part.0", row.message_id),
            kind: "text".to_string(),
            content: row.content,
        }],
        created_at: Some(row.created_at),
        metadata: parse_metadata_map(row.metadata_json.as_deref()),
    }
}

fn message_row_to_agent_message(row: MessageRow) -> Result<AgentMessage, String> {
    let role = match row.role.trim().to_ascii_lowercase().as_str() {
        "user" => AgentMessageRole::User,
        "assistant" | "agent" => AgentMessageRole::Agent,
        "system" => AgentMessageRole::System,
        "model" => AgentMessageRole::Model,
        "tool" => AgentMessageRole::Tool,
        "policy" => AgentMessageRole::Policy,
        "adapter" => AgentMessageRole::Adapter,
        other => return Err(format!("unsupported persisted message role: {other}")),
    };
    let part_id = format!("{}/part.0", row.message_id);
    let mut message = AgentMessage::new(
        row.message_id,
        role,
        vec![AgentPart::text(part_id, row.content)],
    )
    .for_session(row.session_id)
    .created_at(row.created_at);
    for (key, value) in parse_metadata_map(row.metadata_json.as_deref()) {
        message = message.with_metadata(key, value);
    }
    Ok(message)
}

fn task_row_to_view(row: TaskRow) -> TaskViewJson {
    TaskViewJson {
        task_id: row.task_id,
        session_id: row.session_id,
        instruction: row.instruction,
        state: row.state,
        created_at: Some(row.created_at),
        updated_at: row.updated_at,
    }
}

fn event_row_to_sse(row: &EventRow, sequence: u32) -> Event {
    let payload = event_row_to_stream(row, sequence);
    Event::default()
        .id(row.event_id.clone())
        .data(serde_json::to_string(&payload).unwrap_or_default())
}

/// Stream wrapper that decrements an atomic counter when dropped,
/// ensuring the SSE connection count is always accurate even if the
/// client disconnects abruptly or the stream is cancelled.
///
/// The inner stream is pre-boxed and pinned so that `CountedStream`
/// itself is `Unpin` — no unsafe pinning is required.
struct CountedStream {
    inner: Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>>,
    counter: Arc<AtomicU32>,
    decremented: bool,
}

impl CountedStream {
    fn new(
        inner: Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>>,
        counter: Arc<AtomicU32>,
    ) -> Self {
        Self {
            inner,
            counter,
            decremented: false,
        }
    }
}

impl Drop for CountedStream {
    fn drop(&mut self) {
        if !self.decremented {
            self.counter.fetch_sub(1, Ordering::Relaxed);
            crate::metrics::record_sse_connection_close();
            self.decremented = true;
        }
    }
}

impl Stream for CountedStream {
    type Item = Result<Event, Infallible>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        // CountedStream is Unpin because all its fields are Unpin
        // (Pin<Box<..>> and Arc<AtomicU32> are both Unpin).
        let this = self.get_mut();
        this.inner.as_mut().poll_next(cx)
    }
}

enum ModelStreamWorkerItem {
    Chunk(sdkwork_agent_kernel::ModelStreamChunk),
    Failed(String),
}

struct MpscModelStreamSink {
    tx: mpsc::Sender<ModelStreamWorkerItem>,
}

struct ModelSseStream {
    receiver: ReceiverStream<ModelStreamWorkerItem>,
    reservation: Option<TenantTokenQuotaReservation>,
    connection_count: Arc<AtomicU32>,
    estimated_tokens: u64,
    provider_failed: bool,
    terminal_sent: bool,
    connection_released: bool,
}

impl ModelSseStream {
    fn new(
        receiver: ReceiverStream<ModelStreamWorkerItem>,
        reservation: Option<TenantTokenQuotaReservation>,
        connection_count: Arc<AtomicU32>,
    ) -> Self {
        Self {
            receiver,
            reservation,
            connection_count,
            estimated_tokens: 0,
            provider_failed: false,
            terminal_sent: false,
            connection_released: false,
        }
    }

    fn reconcile_quota(&mut self, completed: bool) {
        let Some(reservation) = self.reservation.take() else {
            return;
        };
        let actual_tokens = if completed && !self.provider_failed {
            reservation.reserved_tokens()
        } else {
            self.estimated_tokens
        };
        tokio::spawn(async move {
            reservation.reconcile(actual_tokens).await;
        });
    }

    fn release_connection(&mut self) {
        if !self.connection_released {
            self.connection_count.fetch_sub(1, Ordering::Relaxed);
            crate::metrics::record_sse_connection_close();
            self.connection_released = true;
        }
    }
}

impl Drop for ModelSseStream {
    fn drop(&mut self) {
        self.reconcile_quota(false);
        self.release_connection();
    }
}

impl Stream for ModelSseStream {
    type Item = Result<Event, Infallible>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            match Pin::new(&mut this.receiver).poll_next(cx) {
                std::task::Poll::Ready(Some(ModelStreamWorkerItem::Chunk(chunk))) => {
                    this.estimated_tokens = this
                        .estimated_tokens
                        .saturating_add(estimated_tokens_for_text(&chunk.content));
                    return std::task::Poll::Ready(Some(Ok(model_chunk_to_event(&chunk))));
                }
                std::task::Poll::Ready(Some(ModelStreamWorkerItem::Failed(message))) => {
                    this.provider_failed = true;
                    tracing::warn!(error = %message, "model provider stream failed");
                    return std::task::Poll::Ready(Some(Ok(Event::default()
                        .event("model.error")
                        .data(
                            serde_json::json!({ "code": "PROVIDER_STREAM_FAILED" }).to_string(),
                        ))));
                }
                std::task::Poll::Ready(None) if !this.terminal_sent => {
                    this.terminal_sent = true;
                    this.reconcile_quota(true);
                    return std::task::Poll::Ready(Some(Ok(Event::default()
                        .event("model.done")
                        .data("{}"))));
                }
                std::task::Poll::Ready(None) => {
                    this.release_connection();
                    return std::task::Poll::Ready(None);
                }
                std::task::Poll::Pending => return std::task::Poll::Pending,
            }
        }
    }
}

fn estimated_tokens_for_text(content: &str) -> u64 {
    let bytes = content.len() as u64;
    if bytes == 0 {
        0
    } else {
        bytes.saturating_add(3) / 4
    }
}

impl sdkwork_agent_kernel::ModelStreamSink for MpscModelStreamSink {
    fn push_chunk(
        &mut self,
        chunk: sdkwork_agent_kernel::ModelStreamChunk,
    ) -> sdkwork_agent_kernel::KernelResult<()> {
        self.tx
            .blocking_send(ModelStreamWorkerItem::Chunk(chunk))
            .map_err(|_| sdkwork_agent_kernel::KernelError::Internal {
                message: "model stream consumer dropped".to_string(),
            })
    }
}

fn model_chunk_to_event(chunk: &sdkwork_agent_kernel::ModelStreamChunk) -> Event {
    let json = ModelStreamChunkJson {
        model_request_id: chunk.model_request_id.clone(),
        sequence: chunk.sequence,
        content: chunk.content.clone(),
        finish_reason: None,
    };
    Event::default()
        .event("model.chunk")
        .data(serde_json::to_string(&json).unwrap_or_default())
}

fn spawn_model_sse_stream(
    runtime: RuntimeState,
    session_id: String,
    model_id: Option<String>,
    override_messages: Option<Vec<String>>,
    connection_count: Arc<AtomicU32>,
    reservation: Option<TenantTokenQuotaReservation>,
) -> Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>> {
    let (tx, rx) = mpsc::channel::<ModelStreamWorkerItem>(32);
    tokio::task::spawn_blocking(move || {
        let mut sink = MpscModelStreamSink { tx };
        if let Err(error) = runtime.stream_model_for_session_into(
            &session_id,
            model_id,
            override_messages,
            &mut sink,
        ) {
            let _ = sink
                .tx
                .blocking_send(ModelStreamWorkerItem::Failed(error.to_string()));
        }
    });

    Box::pin(ModelSseStream::new(
        ReceiverStream::new(rx),
        reservation,
        connection_count,
    ))
}

fn live_event_stream(
    receiver: broadcast::Receiver<EventRow>,
    persistence: Arc<PersistenceState>,
    session_id: String,
    replay: Vec<EventRow>,
    sequence: u32,
    last_event_id: Option<String>,
) -> SessionEventStream {
    const EVENT_POLL_INTERVAL: Duration = Duration::from_secs(1);
    const EVENT_POLL_BATCH: i64 = 200;
    let (tx, rx) = mpsc::channel::<Result<Event, Infallible>>(64);

    tokio::spawn(async move {
        let mut receiver = Some(receiver);
        let mut receiver_open = true;
        let mut sequence = sequence;
        let mut cursor = last_event_id;

        for row in replay {
            if cursor.as_deref() == Some(row.event_id.as_str()) {
                continue;
            }
            cursor = Some(row.event_id.clone());
            if tx.send(Ok(event_row_to_sse(&row, sequence))).await.is_err() {
                return;
            }
            sequence = sequence.saturating_add(1);
        }

        let mut interval = tokio::time::interval(EVENT_POLL_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let session_for_poll = session_id.clone();
                    let after_event_id = cursor.clone();
                    match persistence.run(move |state| {
                        state.load_session_events(
                            &session_for_poll,
                            Some(EVENT_POLL_BATCH),
                            after_event_id.as_deref(),
                        )
                    }).await {
                        Ok(rows) => {
                            for row in rows {
                                if cursor.as_deref() == Some(row.event_id.as_str()) {
                                    continue;
                                }
                                cursor = Some(row.event_id.clone());
                                if tx.send(Ok(event_row_to_sse(&row, sequence))).await.is_err() {
                                    return;
                                }
                                sequence = sequence.saturating_add(1);
                            }
                        }
                        Err(error) => {
                            tracing::warn!(session_id = %session_id, error = %error, "durable SSE event poll failed");
                        }
                    }
                }
                received = async {
                    match receiver.as_mut() {
                        Some(receiver) => receiver.recv().await,
                        None => std::future::pending().await,
                    }
                }, if receiver_open => {
                    match received {
                        Ok(row) if row.session_id.as_deref() == Some(session_id.as_str()) => {
                            if cursor.as_deref() == Some(row.event_id.as_str()) {
                                continue;
                            }
                            cursor = Some(row.event_id.clone());
                            if tx.send(Ok(event_row_to_sse(&row, sequence))).await.is_err() {
                                return;
                            }
                            sequence = sequence.saturating_add(1);
                        }
                        Ok(_) => {}
                        Err(broadcast::error::RecvError::Lagged(skipped)) => {
                            tracing::warn!(session_id = %session_id, skipped, "SSE event bus lagged; durable poll will recover events");
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            receiver_open = false;
                            receiver = None;
                        }
                    }
                }
            }
        }
    });

    Box::pin(ReceiverStream::new(rx))
}
fn event_row_to_stream(row: &EventRow, sequence: u32) -> StreamEventJson {
    StreamEventJson {
        event_id: row.event_id.clone(),
        event_type: row.event_type.clone(),
        sequence,
        payload: row.payload.clone().unwrap_or_default(),
        timestamp: Some(row.created_at.clone()),
    }
}

fn event_row_to_kernel_json(row: &EventRow, sequence: u32) -> KernelEventJson {
    KernelEventJson {
        event_id: row.event_id.clone(),
        event_type: row.event_type.clone(),
        severity: row.severity.clone(),
        summary: row
            .payload
            .clone()
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| row.event_type.clone()),
        sequence,
        trace_id: None,
    }
}

fn last_event_id_from_request(headers: &HeaderMap, query: &StreamEventsQuery) -> Option<String> {
    headers
        .get("Last-Event-ID")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            query
                .last_event_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
}

fn parse_metadata_map(raw: Option<&str>) -> HashMap<String, String> {
    raw.and_then(|value| serde_json::from_str::<HashMap<String, String>>(value).ok())
        .unwrap_or_default()
}

fn apply_create_session_metadata(
    metadata: &mut HashMap<String, String>,
    request: &CreateKernelSessionRequest,
) {
    if let Some(tenant_id) = &request.tenant_id {
        metadata.insert("tenantId".to_string(), tenant_id.clone());
    }
    if let Some(user_ref) = &request.user_ref {
        metadata.insert("userRef".to_string(), user_ref.clone());
    }
    if let Some(goal) = &request.goal {
        metadata.insert("goal".to_string(), goal.clone());
    }
    if let Some(model_provider) = &request.model_provider {
        metadata.insert("modelProvider".to_string(), model_provider.clone());
    }
    if let Some(instructions) = &request.instructions {
        metadata.insert("instructions".to_string(), instructions.clone());
    }
    if let Some(roots) = &request.workspace_roots {
        if let Ok(encoded) = serde_json::to_string(roots) {
            metadata.insert("workspaceRoots".to_string(), encoded);
        }
    }
}

pub fn ensure_session_access(
    state: &InternalRuntimeApiState,
    ctx: &RequestContext,
    row: &SessionRow,
) -> Result<(), StatusCode> {
    assert_session_access(state.access_policy, ctx, row)
}

fn ensure_session_access_api(
    state: &InternalRuntimeApiState,
    ctx: &RequestContext,
    row: &SessionRow,
    trace_id: &str,
) -> Result<(), ApiError> {
    ensure_session_access(state, ctx, row)
        .map_err(|status| ApiError::from_status(status, "session access denied", trace_id))
}

fn ensure_session_active_api(row: &SessionRow, trace_id: &str) -> Result<(), ApiError> {
    if row.state.eq_ignore_ascii_case("closed") {
        return Err(ApiError::conflict(
            format!("session {} is closed", row.session_id),
            trace_id,
        ));
    }
    Ok(())
}

fn assert_permission_access_api(
    policy: AccessPolicy,
    ctx: &RequestContext,
    owner_tenant_id: Option<&str>,
    owner_user_ref: Option<&str>,
    permission_id: &str,
    trace_id: &str,
) -> Result<(), ApiError> {
    assert_permission_access(policy, ctx, owner_tenant_id, owner_user_ref, permission_id)
        .map_err(|status| ApiError::from_status(status, "permission access denied", trace_id))
}

pub fn bridge_config_from_row(row: &SessionRow) -> BridgeSessionConfig {
    let metadata = parse_metadata_map(row.metadata_json.as_deref());
    BridgeSessionConfig {
        agent_id: row.agent_id.clone(),
        tenant_id: metadata
            .get("tenantId")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0),
        user_ref: metadata.get("userRef").cloned(),
        model: row.model.clone(),
        instructions: metadata.get("instructions").cloned(),
        cwd: row.cwd.clone(),
        metadata: metadata.into_iter().collect(),
    }
}

/// `GET /runtime/manifest` — returns the runtime capability manifest.
///
/// Side-effect-free surface suitable for UI clients, CI gates, and
/// conformance runners. Aligns with `AGENT_RUNTIME_SPEC` §4
/// `get_runtime_manifest` and `get_capability_manifest`.
pub async fn get_runtime_manifest(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
) -> Result<Response, ApiError> {
    let trace_id = ctx.problem_trace_id();
    let manifest = state.runtime.agent_runtime().capability_manifest();
    let diagnostics = state.runtime.agent_runtime().diagnostics();

    let capabilities = manifest
        .capabilities
        .iter()
        .map(|capability| CapabilityJson {
            capability_id: capability.capability_id.clone(),
            provider_id: capability.provider_id.clone(),
            status: capability.status.clone(),
            required: capability.required,
        })
        .collect();

    let providers = manifest
        .providers
        .iter()
        .map(|provider| ProviderManifestJson {
            provider_id: provider.provider_id.clone(),
            provider_family: provider.provider_family.clone(),
            name: provider.name.clone(),
            version: provider.version.clone(),
            capabilities: provider.capabilities.clone(),
            health_status: diagnostics
                .provider(&provider.provider_id)
                .and_then(|diag| diag.health.as_ref().map(|h| h.status.clone())),
        })
        .collect();

    Ok(api_item(
        RuntimeManifestJson {
            runtime_id: manifest.runtime_id.clone(),
            agent_id: manifest.agent_id.clone(),
            kernel_version: manifest.kernel_version.clone(),
            security_profile: manifest.security_profile.clone(),
            capabilities,
            providers,
            missing_required_capabilities: manifest.missing_required_capabilities.clone(),
            degraded_capabilities: manifest.degraded_capabilities.clone(),
        },
        &trace_id,
    ))
}

/// `GET /internal/v3/api/intelligence/runtime/health` — runtime diagnostics probe.
///
/// Combines runtime state with persistence health. Safe for load-balancer
/// polls. Aligns with `AGENT_RUNTIME_SPEC` §4 `get_health`.
pub async fn get_runtime_health(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
) -> Result<Response, ApiError> {
    let trace_id = ctx.problem_trace_id();
    let diagnostics = state.runtime.agent_runtime().diagnostics();
    let db_healthy = matches!(
        state.persist(|persistence| persistence.health()).await,
        Ok(true)
    );
    let runtime_state = diagnostics.state.clone();
    let health = if db_healthy && runtime_state == "ready" {
        "healthy"
    } else {
        "degraded"
    };
    let mut degraded = diagnostics.degraded_capabilities.clone();
    if !db_healthy {
        degraded.push("persistence".to_string());
    }
    if runtime_state != "ready" {
        degraded.push(format!("runtime.state.{runtime_state}"));
    }

    let payload = RuntimeHealthJson {
        runtime_id: diagnostics.runtime_id,
        state: runtime_state,
        health: health.to_string(),
        persistence_healthy: db_healthy,
        degraded_capabilities: degraded.clone(),
    };

    if health == "degraded" {
        return Err(ApiError::service_unavailable(
            format!(
                "runtime health is degraded (state={}, persistence={}, capabilities={})",
                payload.state,
                payload.persistence_healthy,
                payload.degraded_capabilities.join(",")
            ),
            trace_id,
        ));
    }

    Ok(api_item(payload, &trace_id))
}

/// `GET /runtime/diagnostics` — machine-readable runtime diagnostic report.
///
/// Validates against `schemas/agent-runtime-diagnostics.schema.json`.
/// Aligns with `AGENT_RUNTIME_SPEC` §4.1 `get_diagnostics`.
pub async fn get_runtime_diagnostics(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
) -> Result<Response, ApiError> {
    let trace_id = ctx.problem_trace_id();
    let diagnostics = state.runtime.agent_runtime().diagnostics();

    let provider_diagnostics = diagnostics
        .provider_diagnostics
        .iter()
        .map(|provider| ProviderDiagnosticJson {
            provider_id: provider.provider_id.clone(),
            provider_family: provider.provider_family.clone(),
            provider_version: provider.provider_version.clone(),
            typed_registered: provider.typed_registered,
            health_status: provider.health.as_ref().map(|h| h.status.clone()),
            capabilities: provider.capabilities.clone(),
        })
        .collect();

    Ok(api_item(
        RuntimeDiagnosticsJson {
            runtime_id: diagnostics.runtime_id,
            agent_id: diagnostics.agent_id,
            state: diagnostics.state,
            provider_count: diagnostics.provider_count,
            capability_count: diagnostics.capability_count,
            typed_provider_count: diagnostics.typed_provider_count,
            manifest_only_provider_count: diagnostics.manifest_only_provider_count,
            missing_required_capabilities: diagnostics.missing_required_capabilities,
            degraded_capabilities: diagnostics.degraded_capabilities,
            provider_diagnostics,
        },
        &trace_id,
    ))
}

pub async fn load_snapshot(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
) -> Result<Response, ApiError> {
    let trace_id = ctx.problem_trace_id();
    let snapshot = state
        .build_snapshot(&ctx)
        .await
        .map_err(|error| ApiError::internal(error, &trace_id))?;
    Ok(api_item(snapshot, &trace_id))
}

pub async fn decide_permission(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(permission_request_id): Path<String>,
    Json(body): Json<PermissionDecisionBody>,
) -> Result<Response, ApiError> {
    let trace_id = ctx.problem_trace_id();
    if state.access_policy.enforce_session_scope {
        if ctx.tenant_id.as_deref().is_none_or(str::is_empty)
            || ctx
                .user_id
                .as_deref()
                .or_else(|| ctx.subject_id.as_deref())
                .is_none_or(str::is_empty)
        {
            return Err(ApiError::from_status(
                StatusCode::FORBIDDEN,
                "tenant and user identity required",
                &trace_id,
            ));
        }
    }

    if !matches!(body.decision.as_str(), "allow" | "deny") {
        return Err(ApiError::invalid_parameter(
            "decision must be allow or deny",
            &trace_id,
        ));
    }

    let permission_id = permission_request_id.clone();
    let permission = state
        .persist(move |persistence| persistence.load_permission(&permission_id))
        .await
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?
        .ok_or_else(|| ApiError::not_found("permission request not found", &trace_id))?;

    assert_permission_access_api(
        state.access_policy,
        &ctx,
        permission.owner_tenant_id.as_deref(),
        permission.owner_user_ref.as_deref(),
        &permission_request_id,
        &trace_id,
    )?;

    let decision = body.decision.clone();
    let permission_id = permission_request_id.clone();
    state
        .persist(move |persistence| persistence.update_permission_status(&permission_id, &decision))
        .await
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?;

    let permission_id = permission_request_id.clone();
    let updated = state
        .persist(move |persistence| persistence.load_permission(&permission_id))
        .await
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?
        .ok_or_else(|| ApiError::not_found("permission request not found", &trace_id))?;

    Ok(api_item(permission_row_to_view(updated), &trace_id))
}

pub async fn create_session(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Json(request): Json<CreateKernelSessionRequest>,
) -> Result<Response, ApiError> {
    let trace_id = ctx.problem_trace_id();
    validate_create_session_request(&request, &trace_id)?;
    let registered = validate_hosted_agent_id(&request.agent_id)
        .map_err(|_| ApiError::invalid_parameter("unknown hosted agent id", &trace_id))?;

    let mut metadata_map = request.metadata.clone().unwrap_or_default();
    apply_create_session_metadata(&mut metadata_map, &request);
    apply_hosted_agent_defaults(&mut metadata_map, registered);
    stamp_session_ownership(&mut metadata_map, &ctx, &state.config)
        .map_err(|status| ApiError::from_status(status, "session ownership denied", &trace_id))?;

    let mut config = SessionConfig::new(request.agent_id);
    if let Some(title) = request.title {
        config = config.with_title(title);
    }
    if let Some(model) = request.model {
        config = config.with_model(model);
    }
    if let Some(source) = request.source {
        config = config.with_source(source);
    }
    if let Some(kind) = request.kind {
        config = config.with_kind(kind);
    }
    if let Some(cwd) = request.cwd {
        config = config.with_cwd(cwd);
    }
    if let Some(instructions) = request.instructions {
        config = config.with_instructions(instructions);
    }
    config.metadata = if metadata_map.is_empty() {
        None
    } else {
        serde_json::to_value(metadata_map).ok()
    };

    let row = state
        .persist(move |persistence| persistence.create_session(config))
        .await
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?;

    let _ = state
        .runtime
        .register_session(&row.session_id, bridge_config_from_row(&row));

    Ok(api_created(session_row_to_view(row), &trace_id))
}

pub async fn get_session(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(session_id): Path<String>,
) -> Result<Response, ApiError> {
    let trace_id = ctx.problem_trace_id();
    let session_key = session_id.clone();
    let row = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?;
    ensure_session_access_api(&state, &ctx, &row, &trace_id)?;
    Ok(api_item(session_row_to_view(row), &trace_id))
}

pub async fn list_sessions(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Query(query): Query<ListSessionsQuery>,
) -> Result<Response, ApiError> {
    let trace_id = ctx.problem_trace_id();
    let (owner_tenant_id, owner_user_ref) = if state.access_policy.enforce_session_scope {
        (
            ctx.tenant_id.clone(),
            ctx.user_id.clone().or_else(|| ctx.subject_id.clone()),
        )
    } else {
        (None, None)
    };
    let after_session_id = query
        .cursor
        .as_deref()
        .map(|cursor| decode_resource_cursor(&state.config, "sessions", cursor))
        .transpose()
        .map_err(|_| ApiError::invalid_parameter("invalid session cursor", &trace_id))?;
    let page_size = resolved_cursor_page_size(query.page_size, &trace_id)?;
    let db_query = SessionQuery {
        owner_tenant_id,
        owner_user_ref,
        after_session_id,
        limit: Some(page_size + 1),
        offset: Some(0),
        ..SessionQuery::default()
    };
    let rows = state
        .persist(move |persistence| persistence.list_sessions(db_query))
        .await
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?;
    let views: Vec<_> = rows.into_iter().map(session_row_to_view).collect();
    Ok(cursor_list_response(
        views,
        page_size,
        |view| encode_resource_cursor(&state.config, "sessions", &view.session_id),
        &trace_id,
    ))
}

pub async fn close_session(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(session_id): Path<String>,
) -> Result<Response, ApiError> {
    let trace_id = ctx.problem_trace_id();
    let session_key = session_id.clone();
    let row = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?;
    ensure_session_access_api(&state, &ctx, &row, &trace_id)?;
    let session_for_close = session_id.clone();
    let view = state
        .persist(move |persistence| persistence.close_session(&session_id))
        .await
        .map(session_row_to_view)
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?;
    state
        .runtime
        .register_session(&session_for_close, bridge_config_from_row(&row))
        .map_err(|error| ApiError::from_kernel(error, &trace_id))?;
    state
        .runtime
        .close_session(&session_for_close)
        .map_err(|error| ApiError::from_kernel(error, &trace_id))?;
    Ok(api_item(view, &trace_id))
}

pub async fn delete_session(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(session_id): Path<String>,
) -> Result<Response, ApiError> {
    let trace_id = ctx.problem_trace_id();
    let session_key = session_id.clone();
    let row = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?;
    ensure_session_access_api(&state, &ctx, &row, &trace_id)?;
    let session_for_delete = session_id.clone();
    state
        .persist(move |persistence| persistence.delete_session(&session_id))
        .await
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?;
    state
        .runtime
        .release_session_state(&session_for_delete)
        .map_err(|error| ApiError::from_kernel(error, &trace_id))?;
    Ok(api_no_content(&trace_id))
}

pub async fn send_message(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(session_id): Path<String>,
    Json(request): Json<SendKernelMessageRequest>,
) -> Result<Response, ApiError> {
    let trace_id = ctx.problem_trace_id();
    validate_required_text(
        &request.content,
        "content",
        MAX_MESSAGE_CONTENT_BYTES,
        &trace_id,
    )?;
    let session_key = session_id.clone();
    let row = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?;
    ensure_session_access_api(&state, &ctx, &row, &trace_id)?;
    ensure_session_active_api(&row, &trace_id)?;
    let content = request.content.clone();
    let (user_row, assistant_row, _bridge_response) =
        crate::message_dispatch::dispatch_user_message(
            &state,
            &session_id,
            &content,
            &row,
            &trace_id,
        )
        .await?;
    Ok(api_created(
        MessageTurnViewJson {
            user_message: message_row_to_view(user_row),
            assistant_message: assistant_row.map(message_row_to_view),
            status: "completed".to_string(),
        },
        &trace_id,
    ))
}

pub async fn get_messages(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(session_id): Path<String>,
    Query(query): Query<ListMessagesQuery>,
) -> Result<Response, ApiError> {
    let trace_id = ctx.problem_trace_id();
    let session_key = session_id.clone();
    let row = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?;
    ensure_session_access_api(&state, &ctx, &row, &trace_id)?;

    let after_message_id = query
        .cursor
        .as_deref()
        .map(|cursor| decode_resource_cursor(&state.config, "messages", cursor))
        .transpose()
        .map_err(|_| ApiError::invalid_parameter("invalid message cursor", &trace_id))?;
    let page_size = resolved_cursor_page_size(query.page_size, &trace_id)?;
    let message_query = MessageQuery {
        after_message_id,
        limit: Some(page_size + 1),
        offset: Some(0),
    };
    let rows = state
        .persist(move |persistence| persistence.list_messages(&session_id, message_query))
        .await
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?;
    let views = rows.into_iter().map(message_row_to_view).collect();
    Ok(cursor_list_response(
        views,
        page_size,
        |view| encode_resource_cursor(&state.config, "messages", &view.message_id),
        &trace_id,
    ))
}

pub async fn submit_task(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(session_id): Path<String>,
    Json(request): Json<SubmitTaskRequest>,
) -> Result<Response, ApiError> {
    let trace_id = ctx.problem_trace_id();
    validate_required_text(
        &request.instruction,
        "instruction",
        MAX_TASK_INSTRUCTION_BYTES,
        &trace_id,
    )?;
    let session_key = session_id.clone();
    let session = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?;
    ensure_session_access_api(&state, &ctx, &session, &trace_id)?;
    ensure_session_active_api(&session, &trace_id)?;
    let instruction = request.instruction.clone();
    let row = state
        .persist(move |persistence| persistence.create_task(&session_id, &instruction))
        .await
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?;
    Ok(api_created(task_row_to_view(row), &trace_id))
}

pub async fn get_task(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(task_id): Path<String>,
) -> Result<Response, ApiError> {
    let trace_id = ctx.problem_trace_id();
    let task_key = task_id.clone();
    let task = state
        .persist(move |persistence| persistence.get_task(&task_key))
        .await
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?;
    let session_key = task.session_id.clone();
    let session = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?;
    ensure_session_access_api(&state, &ctx, &session, &trace_id)?;
    Ok(api_item(task_row_to_view(task), &trace_id))
}

pub async fn list_tasks(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(session_id): Path<String>,
    Query(query): Query<ListTasksQuery>,
) -> Result<Response, ApiError> {
    let trace_id = ctx.problem_trace_id();
    let session_key = session_id.clone();
    let session = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?;
    ensure_session_access_api(&state, &ctx, &session, &trace_id)?;

    let after_task_id = query
        .cursor
        .as_deref()
        .map(|cursor| decode_resource_cursor(&state.config, "tasks", cursor))
        .transpose()
        .map_err(|_| ApiError::invalid_parameter("invalid task cursor", &trace_id))?;
    let page_size = resolved_cursor_page_size(query.page_size, &trace_id)?;
    let task_query = TaskQuery {
        after_task_id,
        limit: Some(page_size + 1),
        offset: Some(0),
        ..Default::default()
    };
    let rows = state
        .persist(move |persistence| persistence.list_tasks(&session_id, task_query))
        .await
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?;
    let views = rows.into_iter().map(task_row_to_view).collect();
    Ok(cursor_list_response(
        views,
        page_size,
        |view| encode_resource_cursor(&state.config, "tasks", &view.task_id),
        &trace_id,
    ))
}

pub async fn cancel_task(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(task_id): Path<String>,
) -> Result<Response, ApiError> {
    let trace_id = ctx.problem_trace_id();
    let task_key = task_id.clone();
    let task = state
        .persist(move |persistence| persistence.get_task(&task_key))
        .await
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?;
    let session_key = task.session_id.clone();
    let session = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?;
    ensure_session_access_api(&state, &ctx, &session, &trace_id)?;
    let view = state
        .persist(move |persistence| persistence.cancel_task(&task_id))
        .await
        .map(task_row_to_view)
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?;
    Ok(api_item(view, &trace_id))
}

pub async fn list_models(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
) -> Result<Response, ApiError> {
    let trace_id = ctx.problem_trace_id();
    let models = state
        .runtime
        .list_models()
        .map_err(|error| ApiError::from_kernel(error, &trace_id))?;
    let items = models
        .into_iter()
        .map(|model| ModelDescriptorJson {
            model_id: model.model_id,
            provider_id: model.provider_id,
            display_name: model.display_name,
            family: model.family,
            capabilities: model.capabilities,
        })
        .collect();
    Ok(catalog_list_response(items, &trace_id))
}

pub async fn invoke_model(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Extension(metrics): Extension<Arc<MetricsRegistry>>,
    Path(session_id): Path<String>,
    Json(request): Json<InvokeModelRequest>,
) -> Result<Response, ApiError> {
    let trace_id = ctx.problem_trace_id();
    validate_optional_text(
        request.model_id.as_deref(),
        "modelId",
        MAX_MODEL_ID_BYTES,
        &trace_id,
    )?;
    let session_key = session_id.clone();
    let row = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?;
    ensure_session_access_api(&state, &ctx, &row, &trace_id)?;
    ensure_session_active_api(&row, &trace_id)?;
    state.register_persisted_session(&row, &trace_id).await?;

    let mut quota_reservation = if let Some(tenant_id) = ctx.tenant_id.as_deref() {
        match state.tenant_token_quota.clone().reserve(tenant_id).await {
            Ok(reservation) => reservation,
            Err(status) => {
                crate::security_audit::log_auth_failure(
                    "quota.token_rejected",
                    Some(&ctx.request_id),
                    "/internal/v3/api/intelligence/runtime/sessions/{session_id}/model/invoke",
                    Some(tenant_id),
                    ctx.user_id.as_deref(),
                    "tenant daily model token quota exhausted",
                );
                metrics.record_tenant_token_quota_rejection();
                return Err(ApiError::from_status(
                    status,
                    "tenant daily model token quota exhausted",
                    &trace_id,
                ));
            }
        }
    } else {
        None
    };

    let model_id = request.model_id.clone();
    let result = match state
        .runtime
        .invoke_model_for_session(&session_id, model_id)
    {
        Ok(result) => result,
        Err(error) => {
            if let Some(reservation) = quota_reservation.take() {
                reservation.release().await;
            }
            return Err(ApiError::from_kernel(error, &trace_id));
        }
    };

    let status_label = format!("{:?}", result.response.status).to_lowercase();
    metrics.record_model_invocation(&result.response.provider_id, &status_label);
    if let Some(usage) = result.response.usage.as_ref() {
        crate::usage_meter::record_model_token_usage(
            ctx.tenant_id.as_deref(),
            ctx.user_id.as_deref(),
            &session_id,
            &result.response.provider_id,
            usage,
        );
        metrics.record_model_token_usage(
            &result.response.provider_id,
            "input",
            u64::from(usage.input_tokens),
        );
        metrics.record_model_token_usage(
            &result.response.provider_id,
            "output",
            u64::from(usage.output_tokens),
        );
    }
    if let Some(reservation) = quota_reservation.take() {
        let actual_tokens = result
            .response
            .usage
            .as_ref()
            .map(|usage| u64::from(usage.total_tokens()))
            .unwrap_or_else(|| reservation.reserved_tokens());
        reservation.reconcile(actual_tokens).await;
    }

    Ok(api_item(
        ModelResponseJson {
            model_request_id: result.response.model_request_id,
            provider_id: result.response.provider_id,
            status: format!("{:?}", result.response.status).to_lowercase(),
            messages: result.response.messages,
            tool_calls: result
                .response
                .tool_calls
                .into_iter()
                .map(|call| {
                    serde_json::json!({
                        "toolCallId": call.tool_call_id,
                        "toolId": call.tool_id,
                        "input": call.arguments,
                    })
                })
                .collect(),
        },
        &trace_id,
    ))
}

pub async fn list_tools(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(session_id): Path<String>,
) -> Result<Response, ApiError> {
    let trace_id = ctx.problem_trace_id();
    let session_key = session_id.clone();
    let row = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?;
    ensure_session_access_api(&state, &ctx, &row, &trace_id)?;

    let tools = state
        .runtime
        .list_tools()
        .map_err(|error| ApiError::from_kernel(error, &trace_id))?;
    let items = tools
        .into_iter()
        .map(|tool| ToolDescriptorJson {
            tool_id: tool.tool_id,
            provider_id: tool.provider_id,
            name: tool.name,
            display_name: tool.display_name,
            description: tool.description,
            side_effect_level: tool.side_effect_level.as_str().to_string(),
            policy_categories: tool.policy_categories,
            timeout_ms: tool.timeout_ms,
        })
        .collect();
    Ok(catalog_list_response(items, &trace_id))
}

pub async fn execute_tool(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path((session_id, tool_name)): Path<(String, String)>,
    Json(request): Json<ExecuteToolRequest>,
) -> Result<Response, ApiError> {
    let trace_id = ctx.problem_trace_id();
    validate_required_text(&request.input, "input", MAX_TOOL_INPUT_BYTES, &trace_id)?;
    let session_key = session_id.clone();
    let row = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?;
    ensure_session_access_api(&state, &ctx, &row, &trace_id)?;
    ensure_session_active_api(&row, &trace_id)?;
    state.register_persisted_session(&row, &trace_id).await?;

    let result = state
        .runtime
        .execute_tool(&session_id, &tool_name, &request.input)
        .map_err(|error| ApiError::from_kernel(error, &trace_id))?;

    Ok(api_item(
        ToolCallJson {
            tool_call_id: result.call_id,
            tool_id: tool_name,
            input: request.input,
            status: result.result.status,
            output: Some(result.result.output),
        },
        &trace_id,
    ))
}

/// `POST /sessions/{sessionId}/model/stream` — stream a model response via SSE.
///
/// Returns a Server-Sent Events stream of model output chunks. Each chunk
/// contains the `modelRequestId`, `sequence`, `content`, and optional
/// `finishReason`. The stream terminates after the final chunk.
///
/// This endpoint enables real-time token-by-token display in chat UIs
/// without blocking the HTTP response thread.
pub async fn stream_model(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Extension(metrics): Extension<Arc<MetricsRegistry>>,
    Path(session_id): Path<String>,
    Json(request): Json<StreamModelRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let trace_id = ctx.problem_trace_id();
    validate_stream_model_request(&request, &trace_id)?;
    let session_key = session_id.clone();
    let row = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?;
    ensure_session_access_api(&state, &ctx, &row, &trace_id)?;
    ensure_session_active_api(&row, &trace_id)?;
    state.register_persisted_session(&row, &trace_id).await?;

    let current = state.sse_connection_count.fetch_add(1, Ordering::Relaxed);
    if current >= MAX_CONCURRENT_SSE_STREAMS {
        state.sse_connection_count.fetch_sub(1, Ordering::Relaxed);
        return Err(ApiError::service_unavailable(
            "too many concurrent SSE streams",
            &trace_id,
        ));
    }
    crate::metrics::record_sse_connection_open();

    let quota_reservation = if let Some(tenant_id) = ctx.tenant_id.as_deref() {
        match state.tenant_token_quota.clone().reserve(tenant_id).await {
            Ok(reservation) => reservation,
            Err(status) => {
                state.sse_connection_count.fetch_sub(1, Ordering::Relaxed);
                crate::metrics::record_sse_connection_close();
                crate::security_audit::log_auth_failure(
                    "quota.token_rejected",
                    Some(&ctx.request_id),
                    "/internal/v3/api/intelligence/runtime/sessions/{session_id}/model/stream",
                    Some(tenant_id),
                    ctx.user_id.as_deref(),
                    "tenant daily model token quota exhausted",
                );
                metrics.record_tenant_token_quota_rejection();
                return Err(ApiError::from_status(
                    status,
                    "tenant daily model token quota exhausted",
                    &trace_id,
                ));
            }
        }
    } else {
        None
    };

    let model_id = request.model_id.clone();
    let override_messages = request.messages;
    let connection_count = state.sse_connection_count.clone();
    let stream = spawn_model_sse_stream(
        state.runtime.clone(),
        session_id,
        model_id,
        override_messages,
        connection_count,
        quota_reservation,
    );

    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    ))
}

/// `POST /sessions/{sessionId}/model/cancel` — cancel an in-flight model invocation.
///
/// Signals the model provider to cancel the generation identified by
/// `modelRequestId`. Returns a terminal `ModelResponse` with
/// `status: "cancelled"` and `finishReason: "cancelled"`.
pub async fn cancel_model(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(session_id): Path<String>,
    Json(request): Json<CancelModelRequest>,
) -> Result<Response, ApiError> {
    let trace_id = ctx.problem_trace_id();
    let session_key = session_id.clone();
    let row = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?;
    ensure_session_access_api(&state, &ctx, &row, &trace_id)?;

    let response = state
        .runtime
        .cancel_model(&request.model_request_id, request.provider_id.as_deref())
        .map_err(|error| ApiError::from_kernel(error, &trace_id))?;

    Ok(api_item(
        ModelCancelResponseJson {
            model_request_id: response.model_request_id,
            provider_id: response.provider_id,
            status: format!("{:?}", response.status).to_lowercase(),
            finish_reason: response.finish_reason,
        },
        &trace_id,
    ))
}

/// SSE event stream for a session.
///
/// Sequence numbers are monotonically increasing within a single SSE
/// connection: replay events are assigned 0..N, and live events continue
/// from N onward. Clients should use the `event_id` (not the sequence)
/// for deduplication and reconnection via the `Last-Event-ID` header,
/// because sequence numbers reset to 0 on each new connection.
pub async fn stream_session_events(
    State(state): State<Arc<InternalRuntimeApiState>>,
    Extension(ctx): Extension<RequestContext>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Query(query): Query<StreamEventsQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let trace_id = ctx.problem_trace_id();
    let session_key = session_id.clone();
    let row = state
        .persist(move |persistence| persistence.get_session(&session_key))
        .await
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?;
    ensure_session_access_api(&state, &ctx, &row, &trace_id)?;

    let last_event_id = last_event_id_from_request(&headers, &query);
    let live = query.live.unwrap_or(true);
    // Subscribe before the persistence replay query so events committed during
    // the replay window are buffered locally. The live stream also polls the
    // durable store, which recovers cross-pod and lagged broadcast events.
    let live_receiver = live.then(|| state.persistence.event_bus().subscribe());
    let session_for_events = session_id.clone();
    let replay_after_event_id = last_event_id.clone();
    let events = state
        .persist(move |persistence| {
            persistence.load_session_events(
                &session_for_events,
                Some(i64::from(sdkwork_utils_rust::MAX_LIST_PAGE_SIZE)),
                replay_after_event_id.as_deref(),
            )
        })
        .await
        .map_err(|error| ApiError::from_persistence(error, &trace_id))?;

    let current = state.sse_connection_count.fetch_add(1, Ordering::Relaxed);
    if current >= MAX_CONCURRENT_SSE_STREAMS {
        state.sse_connection_count.fetch_sub(1, Ordering::Relaxed);
        return Err(ApiError::service_unavailable(
            "too many concurrent SSE streams",
            &trace_id,
        ));
    }
    crate::metrics::record_sse_connection_open();

    let connection_count = state.sse_connection_count.clone();
    let stream: SessionEventStream = if live {
        let live_stream = live_event_stream(
            live_receiver.expect("live receiver is present when live streaming is enabled"),
            state.persistence.clone(),
            session_id,
            events,
            0,
            last_event_id,
        );
        // Wrap the stream to decrement the connection counter on drop.
        let inner: Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>> = live_stream;
        Box::pin(CountedStream::new(inner, connection_count))
    } else {
        let replay = stream::iter(
            events
                .into_iter()
                .enumerate()
                .map(|(index, row)| Ok(event_row_to_sse(&row, index as u32))),
        );
        let inner: Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>> = Box::pin(replay);
        Box::pin(CountedStream::new(inner, connection_count))
    };

    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    ))
}

fn resolved_cursor_page_size(page_size: Option<i64>, trace_id: &str) -> Result<i64, ApiError> {
    let page_size = page_size.unwrap_or(i64::from(DEFAULT_LIST_PAGE_SIZE));
    if !(1..=i64::from(MAX_LIST_PAGE_SIZE)).contains(&page_size) {
        return Err(ApiError::invalid_parameter(
            "page_size must be between 1 and 200",
            trace_id,
        ));
    }
    Ok(page_size)
}

#[derive(Debug, Serialize, Deserialize)]
struct ResourceCursorPayload {
    version: u8,
    resource: String,
    id: String,
}

fn cursor_signing_secret(config: &ServerConfig) -> String {
    config
        .ingress_token
        .as_deref()
        .or(config.ingress_jwt_secret.as_deref())
        .or(config.metrics_token.as_deref())
        .map(|secret| format!("sdkwork-kernel-cursor-v1:{secret}"))
        .unwrap_or_else(|| "sdkwork-kernel-local-cursor-v1".to_string())
}

fn encode_resource_cursor(config: &ServerConfig, resource: &str, id: &str) -> String {
    let payload = ResourceCursorPayload {
        version: 1,
        resource: resource.to_string(),
        id: id.to_string(),
    };
    let raw = serde_json::to_vec(&payload).expect("resource cursor serialization");
    let encoded = base64url_encode(&raw);
    let signature =
        hmac_sha256_base64url(encoded.as_bytes(), cursor_signing_secret(config).as_bytes());
    format!("{encoded}.{signature}")
}

fn decode_resource_cursor(
    config: &ServerConfig,
    expected_resource: &str,
    cursor: &str,
) -> Result<String, ()> {
    let (encoded, signature) = cursor.trim().split_once('.').ok_or(())?;
    if encoded.is_empty() || signature.is_empty() {
        return Err(());
    }
    let expected_signature =
        hmac_sha256_base64url(encoded.as_bytes(), cursor_signing_secret(config).as_bytes());
    if !secure_compare(&expected_signature, signature) {
        return Err(());
    }
    let raw = base64url_decode(encoded).ok_or(())?;
    let payload: ResourceCursorPayload = serde_json::from_slice(&raw).map_err(|_| ())?;
    if payload.version != 1 || payload.resource != expected_resource || payload.id.trim().is_empty()
    {
        return Err(());
    }
    Ok(payload.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServerConfig;
    use crate::middleware::RequestContext;
    use axum::extract::{Extension, Path, State};

    fn test_state() -> Arc<InternalRuntimeApiState> {
        Arc::new(
            InternalRuntimeApiState::new(
                Arc::new(PersistenceState::memory().expect("persistence")),
                Arc::new(ServerConfig::default()),
            )
            .expect("runtime state should initialize for tests"),
        )
    }

    fn test_context() -> Extension<RequestContext> {
        Extension(RequestContext {
            request_id: "req.test".to_string(),
            trace_id: None,
            tenant_id: None,
            user_id: None,
            subject_id: None,
            api_surface: None,
            route_template: String::new(),
        })
    }

    #[test]
    fn resource_cursor_round_trip_is_resource_scoped() {
        let config = ServerConfig::default();
        let cursor = encode_resource_cursor(&config, "messages", "msg.1");
        assert_eq!(
            decode_resource_cursor(&config, "messages", &cursor),
            Ok("msg.1".to_string())
        );
        assert!(decode_resource_cursor(&config, "sessions", &cursor).is_err());
    }

    #[test]
    fn resource_cursor_rejects_tampering() {
        let config = ServerConfig::default();
        let cursor = encode_resource_cursor(&config, "tasks", "task.1");
        let mut tampered = cursor.into_bytes();
        let index = tampered
            .iter()
            .position(|byte| *byte != b'.')
            .expect("cursor contains payload bytes");
        tampered[index] = if tampered[index] == b'a' { b'b' } else { b'a' };
        let tampered = String::from_utf8(tampered).expect("ASCII cursor");
        assert!(decode_resource_cursor(&config, "tasks", &tampered).is_err());
    }

    #[test]
    fn cursor_page_size_is_strict() {
        assert_eq!(
            resolved_cursor_page_size(None, "trace").expect("default"),
            i64::from(DEFAULT_LIST_PAGE_SIZE)
        );
        assert!(resolved_cursor_page_size(Some(0), "trace").is_err());
        assert!(resolved_cursor_page_size(Some(201), "trace").is_err());
    }

    #[tokio::test]
    async fn session_event_stream_releases_connection_slot_when_session_lookup_fails() {
        let state = test_state();
        let result = stream_session_events(
            State(state.clone()),
            test_context(),
            HeaderMap::new(),
            Path("session.missing".to_string()),
            Query(StreamEventsQuery {
                last_event_id: None,
                live: Some(false),
            }),
        )
        .await;

        assert!(result.is_err());
        assert_eq!(
            state.sse_connection_count.load(Ordering::Relaxed),
            0,
            "failed session event stream requests must not consume SSE connection slots"
        );
    }

    #[tokio::test]
    async fn internal_runtime_snapshot_and_session_roundtrip() {
        let _lock = crate::testing::env::lock();
        let _plugin = crate::testing::env::VarGuard::set(
            crate::runtime_bootstrap::KERNEL_AGENT_PLUGIN_ENV,
            None,
        );
        let state = test_state();
        let ctx = test_context();
        let snapshot_response = load_snapshot(State(state.clone()), ctx.clone())
            .await
            .expect("snapshot");
        assert_eq!(snapshot_response.status(), StatusCode::OK);
        let snapshot_body: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(snapshot_response.into_body(), usize::MAX)
                .await
                .expect("body"),
        )
        .expect("json");
        assert_eq!(
            snapshot_body["data"]["item"]["runtime"]["health"],
            "healthy"
        );

        let create_response = create_session(
            State(state.clone()),
            ctx.clone(),
            Json(CreateKernelSessionRequest {
                agent_id: "agent.1".to_string(),
                tenant_id: Some("tenant.1".to_string()),
                user_ref: None,
                model: None,
                model_provider: None,
                title: Some("Kernel UI".to_string()),
                goal: None,
                instructions: None,
                cwd: None,
                workspace_roots: None,
                source: Some("web".to_string()),
                kind: None,
                timeout_ms: None,
                metadata: None,
            }),
        )
        .await
        .expect("created");
        assert_eq!(create_response.status(), StatusCode::CREATED);
        let create_body: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(create_response.into_body(), usize::MAX)
                .await
                .expect("body"),
        )
        .expect("json");
        assert_eq!(create_body["data"]["item"]["agentId"], "agent.1");

        let session_id = create_body["data"]["item"]["sessionId"]
            .as_str()
            .expect("sessionId")
            .to_string();

        let loaded_response = get_session(State(state.clone()), ctx, Path(session_id))
            .await
            .expect("loaded");
        let loaded_body: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(loaded_response.into_body(), usize::MAX)
                .await
                .expect("body"),
        )
        .expect("json");
        assert_eq!(loaded_body["data"]["item"]["tenantId"], "tenant.1");
    }
}
