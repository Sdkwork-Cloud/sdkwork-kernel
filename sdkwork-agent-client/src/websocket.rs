use crate::chat::{ChatClient, ChatEventCallback, SessionConfig, SessionInfo};
use crate::types::*;

const INTERNAL_API_WS_UNSUPPORTED: &str = "WebSocketChatClient is not supported for application.public-ingress internal-api runtime; use SseChatClient (AgentProtocol::HttpRestSse)";

/// Legacy WebSocket transport scaffold. Internal-api runtime clients must use [`crate::SseChatClient`].
pub struct WebSocketChatClient {
    url: String,
    event_callback: Option<ChatEventCallback>,
}

impl WebSocketChatClient {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            event_callback: None,
        }
    }

    /// Set event callback
    pub fn with_callback(mut self, callback: ChatEventCallback) -> Self {
        self.event_callback = Some(callback);
        self
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

impl ChatClient for WebSocketChatClient {
    fn send_message(&self, _request: ChatRequest) -> Result<ChatResponse, String> {
        Err(INTERNAL_API_WS_UNSUPPORTED.to_string())
    }

    fn send_message_stream(
        &self,
        _request: ChatRequest,
        _callback: ChatEventCallback,
    ) -> Result<String, String> {
        Err(INTERNAL_API_WS_UNSUPPORTED.to_string())
    }

    fn get_messages(&self, _session_id: &str, _limit: Option<u32>) -> Result<Vec<ChatMessage>, String> {
        Err(INTERNAL_API_WS_UNSUPPORTED.to_string())
    }

    fn create_session(&self, _config: SessionConfig) -> Result<SessionInfo, String> {
        Err(INTERNAL_API_WS_UNSUPPORTED.to_string())
    }

    fn close_session(&self, _session_id: &str) -> Result<(), String> {
        Err(INTERNAL_API_WS_UNSUPPORTED.to_string())
    }

    fn health(&self) -> Result<bool, String> {
        Err(INTERNAL_API_WS_UNSUPPORTED.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn websocket_client_creation() {
        let client = WebSocketChatClient::new("ws://localhost:8080/ws");
        assert_eq!(client.url(), "ws://localhost:8080/ws");
    }

    #[test]
    fn websocket_client_fails_closed_for_internal_api_runtime() {
        let client = WebSocketChatClient::new("ws://localhost:8080/ws");
        assert!(client.health().is_err());
        assert!(client
            .send_message(ChatRequest {
                session_id: "session.test".to_string(),
                content: "hello".to_string(),
                model: None,
                stream: false,
            })
            .is_err());
    }
}

