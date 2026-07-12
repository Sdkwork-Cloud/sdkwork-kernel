use crate::session::BridgeSessionQuery;
use crate::types::*;

/// Core chat client trait
pub trait ChatClient: Send + Sync {
    /// Send a message and get a response
    fn send_message(&self, request: ChatRequest) -> Result<ChatResponse, String>;

    /// Get message history
    fn get_messages(
        &self,
        session_id: &str,
        limit: Option<u32>,
    ) -> Result<Vec<ChatMessage>, String>;

    /// Create a new session
    fn create_session(&self, config: SessionConfig) -> Result<SessionInfo, String>;

    /// Close a session
    fn close_session(&self, session_id: &str) -> Result<(), String>;

    /// List persisted sessions for this bridge provider.
    fn list_sessions(&self, query: &BridgeSessionQuery) -> Result<Vec<SessionInfo>, String>;

    /// Health check
    fn health(&self) -> Result<bool, String>;
}

/// Mock chat client for testing
pub struct MockChatClient {
    responses: Vec<ChatResponse>,
}

impl MockChatClient {
    pub fn new() -> Self {
        Self {
            responses: Vec::new(),
        }
    }

    pub fn with_response(mut self, response: ChatResponse) -> Self {
        self.responses.push(response);
        self
    }
}

impl Default for MockChatClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatClient for MockChatClient {
    fn send_message(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        if let Some(response) = self.responses.first() {
            Ok(ChatResponse {
                message_id: response.message_id.clone(),
                session_id: request.session_id,
                content: response.content.clone(),
                status: ChatStatus::Completed,
                usage: response.usage.clone(),
            })
        } else {
            Ok(ChatResponse {
                message_id: format!("msg.{}", generate_id()),
                session_id: request.session_id,
                content: format!("Mock response to: {}", request.content),
                status: ChatStatus::Completed,
                usage: Some(TokenUsage {
                    input_tokens: 100,
                    output_tokens: 50,
                    total_tokens: 150,
                }),
            })
        }
    }

    fn get_messages(
        &self,
        _session_id: &str,
        _limit: Option<u32>,
    ) -> Result<Vec<ChatMessage>, String> {
        Ok(Vec::new())
    }

    fn create_session(&self, config: SessionConfig) -> Result<SessionInfo, String> {
        let now = chrono_now();
        Ok(SessionInfo {
            session_id: format!("session.{}", generate_id()),
            agent_id: config.agent_id,
            provider_id: "mock".to_string(),
            bridge_id: "mock".to_string(),
            model: config.model,
            title: config.title,
            state: "active".to_string(),
            message_count: 0,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    fn list_sessions(&self, _query: &BridgeSessionQuery) -> Result<Vec<SessionInfo>, String> {
        Ok(Vec::new())
    }

    fn close_session(&self, _session_id: &str) -> Result<(), String> {
        Ok(())
    }

    fn health(&self) -> Result<bool, String> {
        Ok(true)
    }
}

fn generate_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn chrono_now() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_client_send_message() {
        let client = MockChatClient::new();
        let request = ChatRequest {
            session_id: "session.1".to_string(),
            content: "Hello".to_string(),
            model: None,
            stream: false,
        };

        let response = client.send_message(request).expect("sent");
        assert_eq!(response.status, ChatStatus::Completed);
        assert!(!response.content.is_empty());
    }

    #[test]
    fn mock_client_create_session() {
        let client = MockChatClient::new();
        let config = SessionConfig::new("agent.1").with_title("Test");

        let session = client.create_session(config).expect("created");
        assert_eq!(session.agent_id, "agent.1");
        assert_eq!(session.state, "active");
    }

    #[test]
    fn mock_client_health() {
        let client = MockChatClient::new();
        assert!(client.health().expect("health"));
    }
}
