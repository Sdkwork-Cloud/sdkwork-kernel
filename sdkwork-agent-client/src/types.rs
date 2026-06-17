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

/// WebSocket message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub payload: serde_json::Value,
}

/// Chat events
#[derive(Debug, Clone)]
pub enum ChatEvent {
    /// Message received
    MessageReceived(ChatMessage),
    /// Streaming chunk
    StreamChunk {
        message_id: String,
        content: String,
        sequence: u32,
    },
    /// Stream completed
    StreamCompleted {
        message_id: String,
        final_content: String,
    },
    /// Error occurred
    Error { error: String, recoverable: bool },
    /// Session state changed
    SessionStateChanged { session_id: String, state: String },
}

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
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
