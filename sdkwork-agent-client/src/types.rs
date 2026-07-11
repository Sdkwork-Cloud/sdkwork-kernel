use serde::{Deserialize, Serialize};

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: String,
    pub metadata: Option<serde_json::Value>,
}

/// One completed server-side message turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatTurn {
    pub user_message: ChatMessage,
    pub assistant_message: Option<ChatMessage>,
    pub status: ChatStatus,
}

/// Message role
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

/// Chat request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub session_id: String,
    pub content: String,
    pub model: Option<String>,
    pub stream: bool,
}

/// Chat response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub message_id: String,
    pub session_id: String,
    pub content: String,
    pub status: ChatStatus,
    pub usage: Option<TokenUsage>,
}

/// Chat status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChatStatus {
    Pending,
    Streaming,
    Completed,
    Failed,
}

/// Token usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

/// SSE event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseEvent {
    pub event: Option<String>,
    pub data: String,
    pub id: Option<String>,
}

/// Standard bounded list query for internal-api resources.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdkWorkListQuery {
    pub page_size: Option<u32>,
    pub cursor: Option<String>,
}

/// Standard list continuation metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct SdkWorkPageInfo {
    pub mode: String,
    pub page: Option<u64>,
    pub page_size: u32,
    pub total_items: Option<u64>,
    pub total_pages: Option<u64>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

/// One server-bounded page. This type never auto-fetches following pages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdkWorkPage<T> {
    pub items: Vec<T>,
    pub page_info: SdkWorkPageInfo,
}

/// Typed request for the internal model SSE endpoint.
#[derive(Debug, Clone)]
pub struct ModelStreamRequest {
    pub session_id: String,
    pub model_id: Option<String>,
    pub messages: Option<Vec<String>>,
}

/// Typed model output chunk carried by `model.chunk`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelStreamChunk {
    pub model_request_id: String,
    pub sequence: u64,
    pub content: String,
    pub finish_reason: Option<String>,
}

/// Named model SSE events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelStreamEvent {
    Chunk(ModelStreamChunk),
    Done,
    Error { message: String },
}

/// Request for replaying and optionally following session events.
#[derive(Debug, Clone)]
pub struct SessionEventStreamRequest {
    pub session_id: String,
    pub last_event_id: Option<String>,
    pub live: bool,
}

/// Typed payload emitted by the session event stream.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionRuntimeStreamData {
    pub event_id: String,
    pub event_type: String,
    pub sequence: u32,
    pub payload: String,
    pub timestamp: Option<String>,
}

/// Session event with SSE transport metadata preserved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRuntimeStreamEvent {
    pub event: String,
    pub id: Option<String>,
    pub data: SessionRuntimeStreamData,
}

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionConfig {
    pub agent_id: String,
    pub model: Option<String>,
    pub title: Option<String>,
    pub instructions: Option<String>,
}

impl SessionConfig {
    pub fn new(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            model: None,
            title: None,
            instructions: None,
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }
}

/// Session info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub session_id: String,
    pub agent_id: String,
    pub provider_id: String,
    pub bridge_id: String,
    pub model: Option<String>,
    pub title: Option<String>,
    pub state: String,
    pub message_count: u32,
    pub created_at: String,
    pub updated_at: String,
}
