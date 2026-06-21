use axum::{
    extract::State,
    response::sse::{Event, Sse},
    Json,
};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use tokio_stream::StreamExt;

/// SSE state
#[derive(Debug, Clone)]
pub struct SseState {
    pub event_counter: Arc<tokio::sync::Mutex<u64>>,
}

impl SseState {
    pub fn new() -> Self {
        Self {
            event_counter: Arc::new(tokio::sync::Mutex::new(0)),
        }
    }
}

impl Default for SseState {
    fn default() -> Self {
        Self::new()
    }
}

/// SSE chat request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseChatRequest {
    pub session_id: String,
    pub content: String,
    pub model: Option<String>,
}

/// SSE event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseEventData {
    pub event_type: String,
    pub message_id: String,
    pub content: String,
    pub sequence: u32,
}

/// Stream chat response via SSE
pub async fn stream_chat(
    State(state): State<Arc<SseState>>,
    Json(request): Json<SseChatRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let message_id = format!("msg.{}", generate_id());
    let content = format!("This is a streaming response to: {}", request.content);
    let words: Vec<String> = content.split_whitespace().map(|s| s.to_string()).collect();
    let done_message_id = message_id.clone();
    let sequence_base = {
        let mut counter = state.event_counter.lock().await;
        let base = *counter;
        *counter += words.len() as u64 + 1;
        base as u32
    };

    let word_count = words.len() as u32;
    let stream = futures::stream::iter(words.into_iter().enumerate())
        .map(move |(i, word)| {
            let data = SseEventData {
                event_type: "chunk".to_string(),
                message_id: message_id.clone(),
                content: format!("{} ", word),
                sequence: sequence_base + i as u32,
            };

            let event = Event::default()
                .event("chunk")
                .data(serde_json::to_string(&data).unwrap_or_default());

            Ok(event)
        })
        .chain(futures::stream::once(async move {
            let data = SseEventData {
                event_type: "done".to_string(),
                message_id: done_message_id,
                content: String::new(),
                sequence: sequence_base + word_count,
            };

            let event = Event::default()
                .event("done")
                .data(serde_json::to_string(&data).unwrap_or_default());

            Ok(event)
        }));

    Sse::new(stream)
}

/// Stream session events via SSE
pub async fn stream_session_events(
    State(_state): State<Arc<SseState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = futures::stream::once(async move {
        let data = serde_json::json!({
            "event_type": "session.connected",
            "session_id": session_id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        let event = Event::default().event("connected").data(data.to_string());

        Ok(event)
    });

    Sse::new(stream)
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

    #[test]
    fn sse_state_creation() {
        let state = SseState::new();
        assert!(state.event_counter.try_lock().is_ok());
    }
}
