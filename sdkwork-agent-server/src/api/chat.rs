use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Chat state shared across handlers
#[derive(Debug, Clone)]
pub struct ChatState {
    pub messages: Arc<tokio::sync::Mutex<Vec<ChatMessageResponse>>>,
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }
}

impl Default for ChatState {
    fn default() -> Self {
        Self::new()
    }
}

/// Chat request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub session_id: String,
    pub content: String,
    pub model: Option<String>,
    pub stream: bool,
}

/// Chat message response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageResponse {
    pub message_id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub status: String,
    pub created_at: String,
}

/// Chat response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub message_id: String,
    pub session_id: String,
    pub content: String,
    pub status: String,
    pub usage: Option<UsageResponse>,
}

/// Usage response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageResponse {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

/// Send a chat message
pub async fn send_chat(
    State(state): State<Arc<ChatState>>,
    Json(request): Json<ChatRequest>,
) -> (StatusCode, Json<ChatResponse>) {
    let now = chrono::Utc::now().to_rfc3339();
    let message_id = format!("msg.{}", generate_id());

    // Store user message
    let user_msg = ChatMessageResponse {
        message_id: format!("msg.{}", generate_id()),
        session_id: request.session_id.clone(),
        role: "user".to_string(),
        content: request.content.clone(),
        status: "completed".to_string(),
        created_at: now.clone(),
    };

    let mut messages = state.messages.lock().await;
    messages.push(user_msg);

    // Generate mock response
    let response_content = format!("This is a mock response to: {}", request.content);

    // Store assistant message
    let assistant_msg = ChatMessageResponse {
        message_id: message_id.clone(),
        session_id: request.session_id.clone(),
        role: "assistant".to_string(),
        content: response_content.clone(),
        status: "completed".to_string(),
        created_at: now,
    };
    messages.push(assistant_msg);

    let response = ChatResponse {
        message_id,
        session_id: request.session_id,
        content: response_content,
        status: "completed".to_string(),
        usage: Some(UsageResponse {
            input_tokens: 100,
            output_tokens: 50,
            total_tokens: 150,
        }),
    };

    (StatusCode::OK, Json(response))
}

/// Get chat history for a session
pub async fn get_chat_history(
    State(state): State<Arc<ChatState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Json<Vec<ChatMessageResponse>> {
    let messages = state.messages.lock().await;
    let results: Vec<ChatMessageResponse> = messages
        .iter()
        .filter(|m| m.session_id == session_id)
        .cloned()
        .collect();

    Json(results)
}

fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", nanos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn send_chat_message() {
        let state = Arc::new(ChatState::new());

        let request = ChatRequest {
            session_id: "session.1".to_string(),
            content: "Hello".to_string(),
            model: Some("gpt-4".to_string()),
            stream: false,
        };

        let (status, Json(response)) = send_chat(State(state.clone()), Json(request)).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(response.status, "completed");
        assert!(!response.content.is_empty());
    }

    #[tokio::test]
    async fn get_chat_history_test() {
        let state = Arc::new(ChatState::new());

        let request = ChatRequest {
            session_id: "session.1".to_string(),
            content: "Hello".to_string(),
            model: None,
            stream: false,
        };

        send_chat(State(state.clone()), Json(request)).await;

        let Json(history) = get_chat_history(
            State(state.clone()),
            axum::extract::Path("session.1".to_string()),
        )
        .await;

        assert_eq!(history.len(), 2); // user + assistant
    }
}
