use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Message state shared across handlers
#[derive(Debug, Clone)]
pub struct MessageState {
    pub messages: Arc<tokio::sync::Mutex<Vec<MessageResponse>>>,
}

impl MessageState {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }
}

impl Default for MessageState {
    fn default() -> Self {
        Self::new()
    }
}

/// Send message request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
    pub role: Option<String>,
}

/// Message response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageResponse {
    pub message_id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

/// List messages query
#[derive(Debug, Deserialize)]
pub struct ListMessagesQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Send a message
pub async fn send_message(
    State(state): State<Arc<MessageState>>,
    Path(session_id): Path<String>,
    Json(request): Json<SendMessageRequest>,
) -> (StatusCode, Json<MessageResponse>) {
    let now = chrono::Utc::now().to_rfc3339();
    let message_id = format!("msg.{}", generate_id());

    let response = MessageResponse {
        message_id: message_id.clone(),
        session_id: session_id.clone(),
        role: request.role.unwrap_or_else(|| "user".to_string()),
        content: request.content,
        created_at: now,
    };

    let mut messages = state.messages.lock().await;
    messages.push(response.clone());

    (StatusCode::CREATED, Json(response))
}

/// Get messages for a session
pub async fn get_messages(
    State(state): State<Arc<MessageState>>,
    Path(session_id): Path<String>,
    Query(query): Query<ListMessagesQuery>,
) -> Json<Vec<MessageResponse>> {
    let messages = state.messages.lock().await;
    let mut results: Vec<MessageResponse> = messages
        .iter()
        .filter(|m| m.session_id == session_id)
        .cloned()
        .collect();

    if let Some(offset) = query.offset {
        let offset = offset as usize;
        if offset < results.len() {
            results = results[offset..].to_vec();
        } else {
            results.clear();
        }
    }

    if let Some(limit) = query.limit {
        results.truncate(limit as usize);
    }

    Json(results)
}

/// Get message count for a session
pub async fn message_count(
    State(state): State<Arc<MessageState>>,
    Path(session_id): Path<String>,
) -> Json<serde_json::Value> {
    let messages = state.messages.lock().await;
    let count = messages
        .iter()
        .filter(|m| m.session_id == session_id)
        .count();

    Json(serde_json::json!({
        "session_id": session_id,
        "count": count
    }))
}

/// Delete messages for a session
pub async fn delete_messages(
    State(state): State<Arc<MessageState>>,
    Path(session_id): Path<String>,
) -> StatusCode {
    let mut messages = state.messages.lock().await;
    messages.retain(|m| m.session_id != session_id);
    StatusCode::NO_CONTENT
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
    async fn send_and_get_messages() {
        let state = Arc::new(MessageState::new());

        let request = SendMessageRequest {
            content: "Hello".to_string(),
            role: Some("user".to_string()),
        };

        let (status, Json(msg)) = send_message(
            State(state.clone()),
            Path("session.1".to_string()),
            Json(request),
        )
        .await;

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");

        let Json(messages) = get_messages(
            State(state.clone()),
            Path("session.1".to_string()),
            Query(ListMessagesQuery {
                limit: None,
                offset: None,
            }),
        )
        .await;

        assert_eq!(messages.len(), 1);
    }

    #[tokio::test]
    async fn message_count_test() {
        let state = Arc::new(MessageState::new());

        let request = SendMessageRequest {
            content: "Hello".to_string(),
            role: None,
        };

        send_message(
            State(state.clone()),
            Path("session.1".to_string()),
            Json(request),
        )
        .await;

        let Json(result) = message_count(State(state.clone()), Path("session.1".to_string())).await;

        assert_eq!(result["count"], 1);
    }
}
