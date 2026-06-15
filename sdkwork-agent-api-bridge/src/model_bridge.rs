use crate::types::{generate_id, BridgeEvent, BridgeEventSeverity, BridgeModelResult};
use sdkwork_agent_kernel::{
    AgentMessage, AgentSession, ContextFrame, KernelResult, ModelDescriptor, ModelRequest,
    ModelResponse, ModelStreamChunk, ModelUsage,
};

/// Handles model invocations and response processing
pub struct ModelBridge {
    default_model: String,
}

impl ModelBridge {
    pub fn new() -> Self {
        Self {
            default_model: "gpt-4".to_string(),
        }
    }

    /// Build a model request from session context
    pub fn build_request(
        &self,
        session_id: &str,
        session: &AgentSession,
        history: &[AgentMessage],
        context: &[ContextFrame],
    ) -> ModelRequest {
        let model_id = session
            .model
            .clone()
            .unwrap_or_else(|| self.default_model.clone());

        // Collect messages as strings
        let mut messages = Vec::new();
        for msg in history {
            for part in &msg.parts {
                if let Some(text) = &part.text {
                    messages.push(text.clone());
                }
            }
        }

        // Add context frames as messages
        for frame in context {
            messages.push(format!("[context:{}] {}", frame.source, frame.content));
        }

        ModelRequest::new(format!("req.{}", generate_id()), messages)
            .with_model_id(model_id)
            .for_session(session_id)
    }

    /// Invoke the model (mock implementation)
    pub fn invoke(&self, request: &ModelRequest) -> KernelResult<BridgeModelResult> {
        // Mock implementation - in production this would call the actual model provider
        let response = ModelResponse::text(
            &request.model_request_id,
            "provider.mock",
            format!(
                "This is a mock response to your message. Model: {}, Messages: {}",
                request.model_id.as_deref().unwrap_or("unknown"),
                request.messages.len()
            ),
        )
        .with_usage(ModelUsage::new(100, 50));

        let events = vec![BridgeEvent {
            event_type: "agent.model.invoked".to_string(),
            session_id: request.session_id.clone(),
            task_id: None,
            payload: format!(
                "model={};input_tokens=100;output_tokens=50",
                request.model_id.as_deref().unwrap_or("unknown")
            ),
            severity: BridgeEventSeverity::Info,
        }];

        Ok(BridgeModelResult {
            response,
            tool_calls: Vec::new(),
            events,
        })
    }

    /// Stream model response (mock implementation)
    pub fn stream(&self, request: &ModelRequest) -> KernelResult<Vec<ModelStreamChunk>> {
        // Mock implementation - in production this would stream from the model provider
        let chunks = vec![
            ModelStreamChunk::output(&request.model_request_id, 0, "This "),
            ModelStreamChunk::output(&request.model_request_id, 1, "is "),
            ModelStreamChunk::output(&request.model_request_id, 2, "a "),
            ModelStreamChunk::output(&request.model_request_id, 3, "streamed "),
            ModelStreamChunk::output(&request.model_request_id, 4, "response."),
        ];

        Ok(chunks)
    }

    /// Get available model descriptors
    pub fn list_models(&self) -> Vec<ModelDescriptor> {
        vec![
            ModelDescriptor::new("gpt-4", "provider.openai", "GPT-4", "gpt")
                .with_context_window_tokens(128000)
                .with_max_output_tokens(4096),
            ModelDescriptor::new("gpt-3.5-turbo", "provider.openai", "GPT-3.5 Turbo", "gpt")
                .with_context_window_tokens(16385)
                .with_max_output_tokens(4096),
            ModelDescriptor::new(
                "claude-3-opus",
                "provider.anthropic",
                "Claude 3 Opus",
                "claude",
            )
            .with_context_window_tokens(200000)
            .with_max_output_tokens(4096),
        ]
    }
}

impl Default for ModelBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_request_with_history() {
        let bridge = ModelBridge::new();
        let session = AgentSession::new("session.1").with_model("gpt-4");

        let history = vec![AgentMessage::new(
            "msg.1",
            sdkwork_agent_kernel::AgentMessageRole::User,
            vec![],
        )];

        let request = bridge.build_request("session.1", &session, &history, &[]);
        assert_eq!(request.model_id, Some("gpt-4".to_string()));
    }

    #[test]
    fn invoke_returns_mock_response() {
        let bridge = ModelBridge::new();
        let request = ModelRequest::new("req.1", vec!["Hello".to_string()]);

        let result = bridge.invoke(&request).expect("invoked");
        assert!(!result.response.messages.is_empty());
    }

    #[test]
    fn stream_returns_chunks() {
        let bridge = ModelBridge::new();
        let request = ModelRequest::new("req.1", vec!["Hello".to_string()]);

        let chunks = bridge.stream(&request).expect("streamed");
        assert_eq!(chunks.len(), 5);
    }

    #[test]
    fn list_models_returns_catalog() {
        let bridge = ModelBridge::new();
        let models = bridge.list_models();
        assert_eq!(models.len(), 3);
    }
}
