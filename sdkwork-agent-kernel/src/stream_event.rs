//! Unified agent streaming event model.
//!
//! `AgentStreamEvent` is the discriminated-union event protocol consumed by
//! chat, execution, protocol adapters, and UI-facing streams. It aligns the
//! kernel's streaming vocabulary with the industry agent SDKs:
//!
//! - `@anthropic-ai/claude-agent-sdk` `SDKMessage` union: `system/init`,
//!   `assistant` (with `tool_use` blocks), `user` (with `tool_result`),
//!   `result` (cost/usage/num_turns), `stream_event` partial deltas, and
//!   `rate_limit_event`.
//! - OpenAI Codex app-server notifications: `item/started`,
//!   `item/agentMessage/delta`, `item/completed`, `turn/completed`,
//!   `thread/tokenUsage/updated` — every notification carries the
//!   thread/turn/item association keys.
//! - opencode part events with `state: pending/streaming/complete` and delta
//!   semantics.
//!
//! Every event carries `event_id` and optional `session_id`/`stream_id`
//! association keys so consumers can reconstruct message lineage and prevent
//! history forks (the `parent_message_id` chain on `MessageStartEvent`).
//! `AgentStreamEvent` is the single streaming channel; providers emit typed
//! chunks and the kernel maps them into this protocol.

use crate::{
    AgentMessageRole, KernelEvent, KernelEventRedaction, KernelEventSeverity, KernelEventSource,
    KernelResult, ModelStreamChunk, ToolCallStatus, TraceContext,
};

/// Stream event vocabulary prefix shared by all `AgentStreamEvent` types.
pub const AGENT_STREAM_EVENT_FAMILY: &str = "agent.stream";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageDeltaKind {
    /// Visible assistant text delta.
    Text,
    /// Reasoning/thinking delta, rendered separately when supported.
    Reasoning,
}

impl MessageDeltaKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Reasoning => "reasoning",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamStatusLevel {
    Info,
    Warn,
}

impl StreamStatusLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warn => "warn",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitStatus {
    Allowed,
    AllowedWarning,
    Rejected,
}

impl RateLimitStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::AllowedWarning => "allowed_warning",
            Self::Rejected => "rejected",
        }
    }
}

/// Session initialization payload, mirroring the `system/init` message of the
/// agent SDKs: the session identity and the capability surface (model, tools,
/// skills, permission mode) the consumer can render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionInitEvent {
    pub event_id: String,
    pub session_id: Option<String>,
    pub stream_id: Option<String>,
    pub provider_id: Option<String>,
    pub model: Option<String>,
    pub tools: Vec<String>,
    pub skills: Vec<String>,
    pub permission_mode: Option<String>,
}

impl SessionInitEvent {
    pub fn new(event_id: impl Into<String>) -> Self {
        Self {
            event_id: event_id.into(),
            session_id: None,
            stream_id: None,
            provider_id: None,
            model: None,
            tools: Vec::new(),
            skills: Vec::new(),
            permission_mode: None,
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.stream_id = Some(stream_id.into());
        self
    }

    pub fn with_model(mut self, provider_id: impl Into<String>, model: impl Into<String>) -> Self {
        self.provider_id = Some(provider_id.into());
        self.model = Some(model.into());
        self
    }

    pub fn with_tool(mut self, tool: impl Into<String>) -> Self {
        self.tools.push(tool.into());
        self
    }

    pub fn with_skill(mut self, skill: impl Into<String>) -> Self {
        self.skills.push(skill.into());
        self
    }

    pub fn with_permission_mode(mut self, permission_mode: impl Into<String>) -> Self {
        self.permission_mode = Some(permission_mode.into());
        self
    }
}

/// Start of a message part; `parent_message_id` links assistant messages back
/// to the tool call that produced them (sub-agent/task lineage) and chains
/// messages into a fork-safe history graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageStartEvent {
    pub event_id: String,
    pub session_id: Option<String>,
    pub stream_id: Option<String>,
    pub message_id: String,
    pub role: AgentMessageRole,
    pub parent_message_id: Option<String>,
    pub model: Option<String>,
}

impl MessageStartEvent {
    pub fn new(
        event_id: impl Into<String>,
        message_id: impl Into<String>,
        role: AgentMessageRole,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            session_id: None,
            stream_id: None,
            message_id: message_id.into(),
            role,
            parent_message_id: None,
            model: None,
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.stream_id = Some(stream_id.into());
        self
    }

    pub fn with_parent_message(mut self, parent_message_id: impl Into<String>) -> Self {
        self.parent_message_id = Some(parent_message_id.into());
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

/// Incremental content delta for a message (text or reasoning).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageDeltaEvent {
    pub event_id: String,
    pub session_id: Option<String>,
    pub stream_id: Option<String>,
    pub message_id: String,
    pub kind: MessageDeltaKind,
    pub delta: String,
}

impl MessageDeltaEvent {
    pub fn new(
        event_id: impl Into<String>,
        message_id: impl Into<String>,
        kind: MessageDeltaKind,
        delta: impl Into<String>,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            session_id: None,
            stream_id: None,
            message_id: message_id.into(),
            kind,
            delta: delta.into(),
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.stream_id = Some(stream_id.into());
        self
    }

    pub fn text(
        event_id: impl Into<String>,
        message_id: impl Into<String>,
        delta: impl Into<String>,
    ) -> Self {
        Self::new(event_id, message_id, MessageDeltaKind::Text, delta)
    }

    pub fn reasoning(
        event_id: impl Into<String>,
        message_id: impl Into<String>,
        delta: impl Into<String>,
    ) -> Self {
        Self::new(event_id, message_id, MessageDeltaKind::Reasoning, delta)
    }
}

/// End of a message: aggregated content and the model finish reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageStopEvent {
    pub event_id: String,
    pub session_id: Option<String>,
    pub stream_id: Option<String>,
    pub message_id: String,
    pub content: String,
    pub finish_reason: Option<String>,
}

impl MessageStopEvent {
    pub fn new(event_id: impl Into<String>, message_id: impl Into<String>) -> Self {
        Self {
            event_id: event_id.into(),
            session_id: None,
            stream_id: None,
            message_id: message_id.into(),
            content: String::new(),
            finish_reason: None,
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.stream_id = Some(stream_id.into());
        self
    }

    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self
    }

    pub fn with_finish_reason(mut self, finish_reason: impl Into<String>) -> Self {
        self.finish_reason = Some(finish_reason.into());
        self
    }
}

/// The model requested a tool call; the arguments stream as partial JSON
/// deltas until `ToolCallStopEvent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallStartEvent {
    pub event_id: String,
    pub session_id: Option<String>,
    pub stream_id: Option<String>,
    pub tool_call_id: String,
    pub tool_name: String,
    pub message_id: Option<String>,
}

impl ToolCallStartEvent {
    pub fn new(
        event_id: impl Into<String>,
        tool_call_id: impl Into<String>,
        tool_name: impl Into<String>,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            session_id: None,
            stream_id: None,
            tool_call_id: tool_call_id.into(),
            tool_name: tool_name.into(),
            message_id: None,
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.stream_id = Some(stream_id.into());
        self
    }

    pub fn with_message(mut self, message_id: impl Into<String>) -> Self {
        self.message_id = Some(message_id.into());
        self
    }
}

/// Partial JSON arguments delta for an in-flight tool call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallDeltaEvent {
    pub event_id: String,
    pub session_id: Option<String>,
    pub stream_id: Option<String>,
    pub tool_call_id: String,
    pub delta: String,
}

impl ToolCallDeltaEvent {
    pub fn new(
        event_id: impl Into<String>,
        tool_call_id: impl Into<String>,
        delta: impl Into<String>,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            session_id: None,
            stream_id: None,
            tool_call_id: tool_call_id.into(),
            delta: delta.into(),
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.stream_id = Some(stream_id.into());
        self
    }
}

/// The tool call arguments are complete; `arguments` holds the full JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallStopEvent {
    pub event_id: String,
    pub session_id: Option<String>,
    pub stream_id: Option<String>,
    pub tool_call_id: String,
    pub tool_name: String,
    pub arguments: String,
}

impl ToolCallStopEvent {
    pub fn new(
        event_id: impl Into<String>,
        tool_call_id: impl Into<String>,
        tool_name: impl Into<String>,
        arguments: impl Into<String>,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            session_id: None,
            stream_id: None,
            tool_call_id: tool_call_id.into(),
            tool_name: tool_name.into(),
            arguments: arguments.into(),
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.stream_id = Some(stream_id.into());
        self
    }
}

/// Strongly typed tool result paired with its `tool_call_id`, addressing the
/// historical encoding of tool output as an untyped text part plus metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResultEvent {
    pub event_id: String,
    pub session_id: Option<String>,
    pub stream_id: Option<String>,
    pub tool_call_id: String,
    pub tool_name: String,
    pub content: String,
    pub is_error: bool,
    pub status: ToolCallStatus,
    pub duration_ms: Option<u64>,
    pub metadata: Vec<(String, String)>,
}

impl ToolResultEvent {
    pub fn new(
        event_id: impl Into<String>,
        tool_call_id: impl Into<String>,
        tool_name: impl Into<String>,
        content: impl Into<String>,
        status: ToolCallStatus,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            session_id: None,
            stream_id: None,
            tool_call_id: tool_call_id.into(),
            tool_name: tool_name.into(),
            content: content.into(),
            is_error: false,
            status,
            duration_ms: None,
            metadata: Vec::new(),
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.stream_id = Some(stream_id.into());
        self
    }

    pub fn with_error(mut self, error: bool) -> Self {
        self.is_error = error;
        self
    }

    pub fn with_duration_ms(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }
}

/// Token accounting for a turn or message, including cached and reasoning
/// tokens that drive cost estimation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageEvent {
    pub event_id: String,
    pub session_id: Option<String>,
    pub stream_id: Option<String>,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cached_input_tokens: u32,
    pub reasoning_tokens: u32,
    pub total_tokens: u32,
}

impl UsageEvent {
    pub fn new(event_id: impl Into<String>, input_tokens: u32, output_tokens: u32) -> Self {
        Self {
            event_id: event_id.into(),
            session_id: None,
            stream_id: None,
            input_tokens,
            output_tokens,
            cached_input_tokens: 0,
            reasoning_tokens: 0,
            total_tokens: input_tokens + output_tokens,
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.stream_id = Some(stream_id.into());
        self
    }

    pub fn with_cached_input_tokens(mut self, cached_input_tokens: u32) -> Self {
        self.cached_input_tokens = cached_input_tokens;
        self.total_tokens = self.total_tokens.saturating_add(cached_input_tokens);
        self
    }

    pub fn with_reasoning_tokens(mut self, reasoning_tokens: u32) -> Self {
        self.reasoning_tokens = reasoning_tokens;
        self.total_tokens = self.total_tokens.saturating_add(reasoning_tokens);
        self
    }
}

/// Cost accounting for a stream, aligned with the agent SDK result payloads
/// (`total_cost_usd` plus a currency-denominated derived value).
#[derive(Debug, Clone, PartialEq)]
pub struct CostEvent {
    pub event_id: String,
    pub session_id: Option<String>,
    pub stream_id: Option<String>,
    pub cost_cents: u64,
    pub currency: String,
    pub total_cost_usd: Option<f64>,
}

impl CostEvent {
    pub fn new(event_id: impl Into<String>, cost_cents: u64, currency: impl Into<String>) -> Self {
        Self {
            event_id: event_id.into(),
            session_id: None,
            stream_id: None,
            cost_cents,
            currency: currency.into(),
            total_cost_usd: None,
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.stream_id = Some(stream_id.into());
        self
    }

    pub fn with_total_cost_usd(mut self, total_cost_usd: f64) -> Self {
        self.total_cost_usd = Some(total_cost_usd);
        self
    }
}

/// Lifecycle or progress status notice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusEvent {
    pub event_id: String,
    pub session_id: Option<String>,
    pub stream_id: Option<String>,
    pub level: StreamStatusLevel,
    pub message: String,
}

impl StatusEvent {
    pub fn new(
        event_id: impl Into<String>,
        level: StreamStatusLevel,
        message: impl Into<String>,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            session_id: None,
            stream_id: None,
            level,
            message: message.into(),
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.stream_id = Some(stream_id.into());
        self
    }

    pub fn info(event_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(event_id, StreamStatusLevel::Info, message)
    }

    pub fn warn(event_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(event_id, StreamStatusLevel::Warn, message)
    }
}

/// Terminal error event for a stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorEvent {
    pub event_id: String,
    pub session_id: Option<String>,
    pub stream_id: Option<String>,
    pub code: Option<String>,
    pub message: String,
}

impl ErrorEvent {
    pub fn new(event_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            event_id: event_id.into(),
            session_id: None,
            stream_id: None,
            code: None,
            message: message.into(),
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.stream_id = Some(stream_id.into());
        self
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }
}

/// Rate-limit status for a stream, mirroring the agent SDK `rate_limit_event`
/// payload (status, resets at, utilization).
#[derive(Debug, Clone, PartialEq)]
pub struct RateLimitEvent {
    pub event_id: String,
    pub session_id: Option<String>,
    pub stream_id: Option<String>,
    pub status: RateLimitStatus,
    pub resets_at: Option<String>,
    pub utilization: Option<f64>,
    pub limit_type: Option<String>,
}

impl RateLimitEvent {
    pub fn new(event_id: impl Into<String>, status: RateLimitStatus) -> Self {
        Self {
            event_id: event_id.into(),
            session_id: None,
            stream_id: None,
            status,
            resets_at: None,
            utilization: None,
            limit_type: None,
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.stream_id = Some(stream_id.into());
        self
    }

    pub fn with_resets_at(mut self, resets_at: impl Into<String>) -> Self {
        self.resets_at = Some(resets_at.into());
        self
    }

    pub fn with_utilization(mut self, utilization: f64) -> Self {
        self.utilization = Some(utilization);
        self
    }

    pub fn with_limit_type(mut self, limit_type: impl Into<String>) -> Self {
        self.limit_type = Some(limit_type.into());
        self
    }
}

/// Background or long-running operation progress (task panels, streaming
/// tool output).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressEvent {
    pub event_id: String,
    pub session_id: Option<String>,
    pub stream_id: Option<String>,
    pub label: String,
    pub detail: Option<String>,
}

impl ProgressEvent {
    pub fn new(event_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            event_id: event_id.into(),
            session_id: None,
            stream_id: None,
            label: label.into(),
            detail: None,
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.stream_id = Some(stream_id.into());
        self
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// Context compaction boundary: consumers may discard earlier context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactBoundaryEvent {
    pub event_id: String,
    pub session_id: Option<String>,
    pub stream_id: Option<String>,
    pub summary: Option<String>,
}

impl CompactBoundaryEvent {
    pub fn new(event_id: impl Into<String>) -> Self {
        Self {
            event_id: event_id.into(),
            session_id: None,
            stream_id: None,
            summary: None,
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.stream_id = Some(stream_id.into());
        self
    }

    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }
}

/// Sandbox lifecycle phase observed while the stream runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxEventPhase {
    /// The sandbox session is being prepared before execution.
    Pending,
    /// The sandbox session is active and hosting execution.
    Active,
    /// The sandbox session was stopped or torn down after execution.
    Completed,
    /// The sandbox session lifecycle failed.
    Failed,
}

impl SandboxEventPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "active" => Some(Self::Active),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// Sandbox lifecycle event: carries the bound sandbox session identity
/// and the observed lifecycle phase, correlating the stream with the
/// sandbox session lifecycle (pending -> active -> completed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxEvent {
    pub event_id: String,
    pub session_id: Option<String>,
    pub stream_id: Option<String>,
    pub sandbox_session_id: String,
    pub phase: SandboxEventPhase,
    pub message: Option<String>,
}

impl SandboxEvent {
    pub fn new(
        event_id: impl Into<String>,
        sandbox_session_id: impl Into<String>,
        phase: SandboxEventPhase,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            session_id: None,
            stream_id: None,
            sandbox_session_id: sandbox_session_id.into(),
            phase,
            message: None,
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.stream_id = Some(stream_id.into());
        self
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn pending(event_id: impl Into<String>, sandbox_session_id: impl Into<String>) -> Self {
        Self::new(event_id, sandbox_session_id, SandboxEventPhase::Pending)
    }

    pub fn active(event_id: impl Into<String>, sandbox_session_id: impl Into<String>) -> Self {
        Self::new(event_id, sandbox_session_id, SandboxEventPhase::Active)
    }

    pub fn completed(event_id: impl Into<String>, sandbox_session_id: impl Into<String>) -> Self {
        Self::new(event_id, sandbox_session_id, SandboxEventPhase::Completed)
    }

    pub fn failed(event_id: impl Into<String>, sandbox_session_id: impl Into<String>) -> Self {
        Self::new(event_id, sandbox_session_id, SandboxEventPhase::Failed)
    }
}

/// Terminal result for the stream, mirroring the agent SDK `result` message:
/// turn count, duration, cost, usage, and the final text.
#[derive(Debug, Clone, PartialEq)]
pub struct ResultEvent {
    pub event_id: String,
    pub session_id: Option<String>,
    pub stream_id: Option<String>,
    pub run_id: Option<String>,
    pub num_turns: u32,
    pub duration_ms: Option<u64>,
    pub is_error: bool,
    pub result: String,
    pub stop_reason: Option<String>,
    pub total_cost_usd: Option<f64>,
    pub usage: Option<UsageEvent>,
}

impl ResultEvent {
    pub fn new(event_id: impl Into<String>) -> Self {
        Self {
            event_id: event_id.into(),
            session_id: None,
            stream_id: None,
            run_id: None,
            num_turns: 0,
            duration_ms: None,
            is_error: false,
            result: String::new(),
            stop_reason: None,
            total_cost_usd: None,
            usage: None,
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.stream_id = Some(stream_id.into());
        self
    }

    pub fn with_run_id(mut self, run_id: impl Into<String>) -> Self {
        self.run_id = Some(run_id.into());
        self
    }

    pub fn with_num_turns(mut self, num_turns: u32) -> Self {
        self.num_turns = num_turns;
        self
    }

    pub fn with_duration_ms(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    pub fn with_error(mut self, is_error: bool) -> Self {
        self.is_error = is_error;
        self
    }

    pub fn with_result(mut self, result: impl Into<String>) -> Self {
        self.result = result.into();
        self
    }

    pub fn with_stop_reason(mut self, stop_reason: impl Into<String>) -> Self {
        self.stop_reason = Some(stop_reason.into());
        self
    }

    pub fn with_total_cost_usd(mut self, total_cost_usd: f64) -> Self {
        self.total_cost_usd = Some(total_cost_usd);
        self
    }

    pub fn with_usage(mut self, usage: UsageEvent) -> Self {
        self.usage = Some(usage);
        self
    }
}

/// The stream was cancelled before completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancelledEvent {
    pub event_id: String,
    pub session_id: Option<String>,
    pub stream_id: Option<String>,
    pub reason: Option<String>,
}

impl CancelledEvent {
    pub fn new(event_id: impl Into<String>) -> Self {
        Self {
            event_id: event_id.into(),
            session_id: None,
            stream_id: None,
            reason: None,
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.stream_id = Some(stream_id.into());
        self
    }

    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

/// The stream ended (terminal, after `ResultEvent` or `ErrorEvent`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndedEvent {
    pub event_id: String,
    pub session_id: Option<String>,
    pub stream_id: Option<String>,
    pub reason: Option<String>,
}

impl EndedEvent {
    pub fn new(event_id: impl Into<String>) -> Self {
        Self {
            event_id: event_id.into(),
            session_id: None,
            stream_id: None,
            reason: None,
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.stream_id = Some(stream_id.into());
        self
    }

    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

/// The unified agent streaming event protocol.
///
/// Ordering contract for a well-formed stream:
/// `SessionInit -> (MessageStart -> MessageDelta* -> MessageStop |
/// ToolCallStart -> ToolCallDelta* -> ToolCallStop -> ToolResult)* ->
/// Usage/Cost -> Result | Error | Cancelled -> Ended`.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentStreamEvent {
    SessionInit(SessionInitEvent),
    MessageStart(MessageStartEvent),
    MessageDelta(MessageDeltaEvent),
    MessageStop(MessageStopEvent),
    ToolCallStart(ToolCallStartEvent),
    ToolCallDelta(ToolCallDeltaEvent),
    ToolCallStop(ToolCallStopEvent),
    ToolResult(ToolResultEvent),
    Usage(UsageEvent),
    Cost(CostEvent),
    Status(StatusEvent),
    Error(ErrorEvent),
    RateLimit(RateLimitEvent),
    Progress(ProgressEvent),
    CompactBoundary(CompactBoundaryEvent),
    Sandbox(SandboxEvent),
    Result(ResultEvent),
    Cancelled(CancelledEvent),
    Ended(EndedEvent),
}

impl AgentStreamEvent {
    /// Stable, dot-delimited event type for this event, used by
    /// `KernelEvent::event_type` and external protocol mappings.
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::SessionInit(_) => "agent.stream.session.init",
            Self::MessageStart(_) => "agent.stream.message.start",
            Self::MessageDelta(_) => "agent.stream.message.delta",
            Self::MessageStop(_) => "agent.stream.message.stop",
            Self::ToolCallStart(_) => "agent.stream.tool.call.start",
            Self::ToolCallDelta(_) => "agent.stream.tool.call.delta",
            Self::ToolCallStop(_) => "agent.stream.tool.call.stop",
            Self::ToolResult(_) => "agent.stream.tool.result",
            Self::Usage(_) => "agent.stream.usage",
            Self::Cost(_) => "agent.stream.cost",
            Self::Status(_) => "agent.stream.status",
            Self::Error(_) => "agent.stream.error",
            Self::RateLimit(_) => "agent.stream.rate_limit",
            Self::Progress(_) => "agent.stream.progress",
            Self::CompactBoundary(_) => "agent.stream.compact_boundary",
            Self::Sandbox(_) => "agent.stream.sandbox",
            Self::Result(_) => "agent.stream.result",
            Self::Cancelled(_) => "agent.stream.cancelled",
            Self::Ended(_) => "agent.stream.ended",
        }
    }

    pub fn event_id(&self) -> &str {
        match self {
            Self::SessionInit(e) => &e.event_id,
            Self::MessageStart(e) => &e.event_id,
            Self::MessageDelta(e) => &e.event_id,
            Self::MessageStop(e) => &e.event_id,
            Self::ToolCallStart(e) => &e.event_id,
            Self::ToolCallDelta(e) => &e.event_id,
            Self::ToolCallStop(e) => &e.event_id,
            Self::ToolResult(e) => &e.event_id,
            Self::Usage(e) => &e.event_id,
            Self::Cost(e) => &e.event_id,
            Self::Status(e) => &e.event_id,
            Self::Error(e) => &e.event_id,
            Self::RateLimit(e) => &e.event_id,
            Self::Progress(e) => &e.event_id,
            Self::CompactBoundary(e) => &e.event_id,
            Self::Sandbox(e) => &e.event_id,
            Self::Result(e) => &e.event_id,
            Self::Cancelled(e) => &e.event_id,
            Self::Ended(e) => &e.event_id,
        }
    }

    pub fn session_id(&self) -> Option<&str> {
        match self {
            Self::SessionInit(e) => e.session_id.as_deref(),
            Self::MessageStart(e) => e.session_id.as_deref(),
            Self::MessageDelta(e) => e.session_id.as_deref(),
            Self::MessageStop(e) => e.session_id.as_deref(),
            Self::ToolCallStart(e) => e.session_id.as_deref(),
            Self::ToolCallDelta(e) => e.session_id.as_deref(),
            Self::ToolCallStop(e) => e.session_id.as_deref(),
            Self::ToolResult(e) => e.session_id.as_deref(),
            Self::Usage(e) => e.session_id.as_deref(),
            Self::Cost(e) => e.session_id.as_deref(),
            Self::Status(e) => e.session_id.as_deref(),
            Self::Error(e) => e.session_id.as_deref(),
            Self::RateLimit(e) => e.session_id.as_deref(),
            Self::Progress(e) => e.session_id.as_deref(),
            Self::CompactBoundary(e) => e.session_id.as_deref(),
            Self::Sandbox(e) => e.session_id.as_deref(),
            Self::Result(e) => e.session_id.as_deref(),
            Self::Cancelled(e) => e.session_id.as_deref(),
            Self::Ended(e) => e.session_id.as_deref(),
        }
    }

    pub fn stream_id(&self) -> Option<&str> {
        match self {
            Self::SessionInit(e) => e.stream_id.as_deref(),
            Self::MessageStart(e) => e.stream_id.as_deref(),
            Self::MessageDelta(e) => e.stream_id.as_deref(),
            Self::MessageStop(e) => e.stream_id.as_deref(),
            Self::ToolCallStart(e) => e.stream_id.as_deref(),
            Self::ToolCallDelta(e) => e.stream_id.as_deref(),
            Self::ToolCallStop(e) => e.stream_id.as_deref(),
            Self::ToolResult(e) => e.stream_id.as_deref(),
            Self::Usage(e) => e.stream_id.as_deref(),
            Self::Cost(e) => e.stream_id.as_deref(),
            Self::Status(e) => e.stream_id.as_deref(),
            Self::Error(e) => e.stream_id.as_deref(),
            Self::RateLimit(e) => e.stream_id.as_deref(),
            Self::Progress(e) => e.stream_id.as_deref(),
            Self::CompactBoundary(e) => e.stream_id.as_deref(),
            Self::Sandbox(e) => e.stream_id.as_deref(),
            Self::Result(e) => e.stream_id.as_deref(),
            Self::Cancelled(e) => e.stream_id.as_deref(),
            Self::Ended(e) => e.stream_id.as_deref(),
        }
    }

    /// Attach a session identity to any event, mirroring the agent SDK
    /// convention that every stream message carries `session_id`.
    pub fn with_session_id(self, session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();
        match self {
            Self::SessionInit(e) => Self::SessionInit(e.with_session_id(session_id)),
            Self::MessageStart(e) => Self::MessageStart(e.with_session_id(session_id)),
            Self::MessageDelta(e) => Self::MessageDelta(e.with_session_id(session_id)),
            Self::MessageStop(e) => Self::MessageStop(e.with_session_id(session_id)),
            Self::ToolCallStart(e) => Self::ToolCallStart(e.with_session_id(session_id)),
            Self::ToolCallDelta(e) => Self::ToolCallDelta(e.with_session_id(session_id)),
            Self::ToolCallStop(e) => Self::ToolCallStop(e.with_session_id(session_id)),
            Self::ToolResult(e) => Self::ToolResult(e.with_session_id(session_id)),
            Self::Usage(e) => Self::Usage(e.with_session_id(session_id)),
            Self::Cost(e) => Self::Cost(e.with_session_id(session_id)),
            Self::Status(e) => Self::Status(e.with_session_id(session_id)),
            Self::Error(e) => Self::Error(e.with_session_id(session_id)),
            Self::RateLimit(e) => Self::RateLimit(e.with_session_id(session_id)),
            Self::Progress(e) => Self::Progress(e.with_session_id(session_id)),
            Self::CompactBoundary(e) => Self::CompactBoundary(e.with_session_id(session_id)),
            Self::Sandbox(e) => Self::Sandbox(e.with_session_id(session_id)),
            Self::Result(e) => Self::Result(e.with_session_id(session_id)),
            Self::Cancelled(e) => Self::Cancelled(e.with_session_id(session_id)),
            Self::Ended(e) => Self::Ended(e.with_session_id(session_id)),
        }
    }

    /// Attach a session identity when present; used by stream builders that
    /// carry the session as an `Option`.
    pub fn with_session_id_optional(self, session_id: &Option<String>) -> Self {
        match session_id {
            Some(session_id) => self.with_session_id(session_id.clone()),
            None => self,
        }
    }

    /// Attach a stream correlation identity to any event.
    pub fn with_stream_id(self, stream_id: impl Into<String>) -> Self {
        let stream_id = stream_id.into();
        match self {
            Self::SessionInit(e) => Self::SessionInit(e.with_stream_id(stream_id)),
            Self::MessageStart(e) => Self::MessageStart(e.with_stream_id(stream_id)),
            Self::MessageDelta(e) => Self::MessageDelta(e.with_stream_id(stream_id)),
            Self::MessageStop(e) => Self::MessageStop(e.with_stream_id(stream_id)),
            Self::ToolCallStart(e) => Self::ToolCallStart(e.with_stream_id(stream_id)),
            Self::ToolCallDelta(e) => Self::ToolCallDelta(e.with_stream_id(stream_id)),
            Self::ToolCallStop(e) => Self::ToolCallStop(e.with_stream_id(stream_id)),
            Self::ToolResult(e) => Self::ToolResult(e.with_stream_id(stream_id)),
            Self::Usage(e) => Self::Usage(e.with_stream_id(stream_id)),
            Self::Cost(e) => Self::Cost(e.with_stream_id(stream_id)),
            Self::Status(e) => Self::Status(e.with_stream_id(stream_id)),
            Self::Error(e) => Self::Error(e.with_stream_id(stream_id)),
            Self::RateLimit(e) => Self::RateLimit(e.with_stream_id(stream_id)),
            Self::Progress(e) => Self::Progress(e.with_stream_id(stream_id)),
            Self::CompactBoundary(e) => Self::CompactBoundary(e.with_stream_id(stream_id)),
            Self::Sandbox(e) => Self::Sandbox(e.with_stream_id(stream_id)),
            Self::Result(e) => Self::Result(e.with_stream_id(stream_id)),
            Self::Cancelled(e) => Self::Cancelled(e.with_stream_id(stream_id)),
            Self::Ended(e) => Self::Ended(e.with_stream_id(stream_id)),
        }
    }

    /// Map the event into the generic `KernelEvent` envelope so existing
    /// event streams, replay cursors, and telemetry pipelines observe the
    /// stream without protocol changes.
    pub fn to_kernel_event(&self) -> KernelEvent {
        let session_id = self.session_id().map(str::to_string);
        let event_id = self.event_id().to_string();
        let event_type = self.event_type().to_string();
        let mut event = KernelEvent::new(
            event_id,
            event_type,
            KernelEventSeverity::Info,
            self.payload_json(),
        )
        .from_source(KernelEventSource::Model)
        .with_redaction(KernelEventRedaction::Public);
        if let Some(session_id) = session_id {
            event = event.for_session(session_id);
        }
        event
    }

    /// Compact JSON payload for the generic envelope. Envelope consumers that
    /// need the full payload should consume `AgentStreamEvent` directly.
    fn payload_json(&self) -> String {
        let event_type = self.event_type();
        match self {
            Self::SessionInit(e) => serde_json::json!({
                "event_type": event_type,
                "provider_id": e.provider_id,
                "model": e.model,
                "tools": e.tools,
                "skills": e.skills,
                "permission_mode": e.permission_mode,
            })
            .to_string(),
            Self::MessageStart(e) => serde_json::json!({
                "event_type": event_type,
                "message_id": e.message_id,
                "role": e.role.as_str(),
                "parent_message_id": e.parent_message_id,
                "model": e.model,
            })
            .to_string(),
            Self::MessageDelta(e) => serde_json::json!({
                "event_type": event_type,
                "message_id": e.message_id,
                "kind": e.kind.as_str(),
                "delta": e.delta,
            })
            .to_string(),
            Self::MessageStop(e) => serde_json::json!({
                "event_type": event_type,
                "message_id": e.message_id,
                "finish_reason": e.finish_reason,
                "content_length": e.content.chars().count(),
            })
            .to_string(),
            Self::ToolCallStart(e) => serde_json::json!({
                "event_type": event_type,
                "tool_call_id": e.tool_call_id,
                "tool_name": e.tool_name,
            })
            .to_string(),
            Self::ToolCallDelta(e) => serde_json::json!({
                "event_type": event_type,
                "tool_call_id": e.tool_call_id,
                "delta": e.delta,
            })
            .to_string(),
            Self::ToolCallStop(e) => serde_json::json!({
                "event_type": event_type,
                "tool_call_id": e.tool_call_id,
                "tool_name": e.tool_name,
            })
            .to_string(),
            Self::ToolResult(e) => serde_json::json!({
                "event_type": event_type,
                "tool_call_id": e.tool_call_id,
                "tool_name": e.tool_name,
                "is_error": e.is_error,
                "status": e.status.as_str(),
                "duration_ms": e.duration_ms,
            })
            .to_string(),
            Self::Usage(e) => serde_json::json!({
                "event_type": event_type,
                "input_tokens": e.input_tokens,
                "output_tokens": e.output_tokens,
                "cached_input_tokens": e.cached_input_tokens,
                "reasoning_tokens": e.reasoning_tokens,
                "total_tokens": e.total_tokens,
            })
            .to_string(),
            Self::Cost(e) => serde_json::json!({
                "event_type": event_type,
                "cost_cents": e.cost_cents,
                "currency": e.currency,
                "total_cost_usd": e.total_cost_usd,
            })
            .to_string(),
            Self::Status(e) => serde_json::json!({
                "event_type": event_type,
                "level": e.level.as_str(),
                "message": e.message,
            })
            .to_string(),
            Self::Error(e) => serde_json::json!({
                "event_type": event_type,
                "code": e.code,
                "message": e.message,
            })
            .to_string(),
            Self::RateLimit(e) => serde_json::json!({
                "event_type": event_type,
                "status": e.status.as_str(),
                "resets_at": e.resets_at,
                "utilization": e.utilization,
                "limit_type": e.limit_type,
            })
            .to_string(),
            Self::Progress(e) => serde_json::json!({
                "event_type": event_type,
                "label": e.label,
                "detail": e.detail,
            })
            .to_string(),
            Self::CompactBoundary(e) => serde_json::json!({
                "event_type": event_type,
                "summary": e.summary,
            })
            .to_string(),
            Self::Sandbox(e) => serde_json::json!({
                "event_type": event_type,
                "sandbox_session_id": e.sandbox_session_id,
                "phase": e.phase.as_str(),
                "message": e.message,
            })
            .to_string(),
            Self::Result(e) => serde_json::json!({
                "event_type": event_type,
                "num_turns": e.num_turns,
                "duration_ms": e.duration_ms,
                "is_error": e.is_error,
                "stop_reason": e.stop_reason,
                "total_cost_usd": e.total_cost_usd,
                "result_length": e.result.chars().count(),
            })
            .to_string(),
            Self::Cancelled(e) => serde_json::json!({
                "event_type": event_type,
                "reason": e.reason,
            })
            .to_string(),
            Self::Ended(e) => serde_json::json!({
                "event_type": event_type,
                "reason": e.reason,
            })
            .to_string(),
        }
    }
}

/// Streaming sink for `AgentStreamEvent`; the single delivery channel for
/// chat and execution streams. Implementations must preserve event order.
pub trait AgentStreamSink {
    fn push_event(&mut self, event: AgentStreamEvent) -> KernelResult<()>;
}

/// In-memory sink that collects events in arrival order. Used by buffered
/// chat/execution entrypoints, tests, and adapter composition.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct InMemoryAgentStreamSink {
    events: Vec<AgentStreamEvent>,
}

impl InMemoryAgentStreamSink {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Consume the collected events in arrival order.
    pub fn into_events(self) -> Vec<AgentStreamEvent> {
        self.events
    }

    /// Borrow the collected events.
    pub fn events(&self) -> &[AgentStreamEvent] {
        &self.events
    }

    pub fn count(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl AgentStreamSink for InMemoryAgentStreamSink {
    fn push_event(&mut self, event: AgentStreamEvent) -> KernelResult<()> {
        self.events.push(event);
        Ok(())
    }
}

/// Sink adapter that maps every `AgentStreamEvent` into the generic
/// `KernelEvent` envelope, so existing `EventStream` consumers observe
/// stream events through the same channel.
#[derive(Debug, Default, Clone)]
pub struct KernelEventStreamSink<S> {
    inner: S,
}

impl<S> KernelEventStreamSink<S> {
    pub fn new(inner: S) -> Self {
        Self { inner }
    }

    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S> AgentStreamSink for KernelEventStreamSink<S>
where
    S: FnMut(KernelEvent) -> KernelResult<()>,
{
    fn push_event(&mut self, event: AgentStreamEvent) -> KernelResult<()> {
        (self.inner)(event.to_kernel_event())
    }
}

impl From<&ModelStreamChunk> for AgentStreamEvent {
    /// Map a provider-neutral output chunk into the unified protocol.
    ///
    /// Text and reasoning chunks become `MessageDelta` events; chunks
    /// carrying a `tool_call_id` become `ToolCallDelta` events so tool
    /// argument streaming round-trips through the same channel; usage and
    /// status chunks surface as status notices.
    fn from(chunk: &ModelStreamChunk) -> Self {
        match chunk.chunk_kind {
            crate::ModelChunkKind::Reasoning => AgentStreamEvent::MessageDelta(
                MessageDeltaEvent::reasoning(
                    format!("stream.delta.{}", chunk.sequence),
                    chunk.model_request_id.clone(),
                    &chunk.content,
                )
                .with_stream_id(chunk.model_request_id.clone()),
            ),
            crate::ModelChunkKind::ToolCallArguments => AgentStreamEvent::ToolCallDelta(
                ToolCallDeltaEvent::new(
                    format!("stream.delta.{}", chunk.sequence),
                    chunk.tool_call_id.clone().unwrap_or_default(),
                    &chunk.content,
                )
                .with_stream_id(chunk.model_request_id.clone()),
            ),
            crate::ModelChunkKind::Usage | crate::ModelChunkKind::Status => {
                AgentStreamEvent::Status(
                    StatusEvent::info(format!("stream.status.{}", chunk.sequence), &chunk.content)
                        .with_stream_id(chunk.model_request_id.clone()),
                )
            }
            crate::ModelChunkKind::Text => AgentStreamEvent::MessageDelta(
                MessageDeltaEvent::text(
                    format!("stream.delta.{}", chunk.sequence),
                    chunk.model_request_id.clone(),
                    &chunk.content,
                )
                .with_stream_id(chunk.model_request_id.clone()),
            ),
        }
    }
}

/// Build a `TraceContext`-carrying stream if the caller supplies one; kept
/// separate so event construction stays dependency-light.
pub fn stream_event_with_trace(
    event: AgentStreamEvent,
    trace: Option<&TraceContext>,
) -> KernelEvent {
    let mut kernel_event = event.to_kernel_event();
    if let Some(trace) = trace {
        kernel_event = kernel_event.with_trace_context(trace.clone());
    }
    kernel_event
}
