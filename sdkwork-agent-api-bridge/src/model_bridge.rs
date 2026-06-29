use std::sync::Arc;

use crate::types::{generate_id, BridgeEvent, BridgeEventSeverity, BridgeModelResult};
use sdkwork_agent_kernel::{
    AgentMessage, AgentRuntime, AgentSession, ContextFrame, KernelResult, ModelCancellationRequest,
    ModelDescriptor, ModelExecutionService, ModelRequest, ModelResponse, ModelStreamChunk,
    ModelUsage,
};

/// Handles model invocations and response processing
pub struct ModelBridge {
    default_model: String,
    agent_runtime: Option<Arc<AgentRuntime>>,
    allow_mock_fallback: bool,
}

impl ModelBridge {
    pub fn new() -> Self {
        Self {
            default_model: "gpt-4".to_string(),
            agent_runtime: None,
            allow_mock_fallback: true,
        }
    }

    pub fn with_agent_runtime(agent_runtime: Arc<AgentRuntime>, allow_mock_fallback: bool) -> Self {
        Self {
            default_model: "gpt-4".to_string(),
            agent_runtime: Some(agent_runtime),
            allow_mock_fallback,
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

        let mut messages = Vec::new();
        for msg in history {
            for part in &msg.parts {
                if let Some(text) = &part.text {
                    messages.push(text.clone());
                }
            }
        }

        for frame in context {
            messages.push(format!("[context:{}] {}", frame.source, frame.content));
        }

        ModelRequest::new(format!("req.{}", generate_id()), messages)
            .with_model_id(model_id)
            .for_session(session_id)
    }

    /// Invoke the typed provider when registered, otherwise use the mock bridge path.
    pub fn invoke(
        &self,
        request: &ModelRequest,
        model_provider_id: Option<&str>,
    ) -> KernelResult<BridgeModelResult> {
        if let Some(runtime) = &self.agent_runtime {
            match self.invoke_typed(runtime, request, model_provider_id) {
                Ok(result) => return Ok(result),
                Err(error) if self.allow_mock_fallback && error.retryable() => {}
                Err(error) => return Err(error),
            }
        }

        self.invoke_mock(request)
    }

    /// Stream model response (typed provider when registered, otherwise mock bridge path).
    pub fn stream(
        &self,
        request: &ModelRequest,
        model_provider_id: Option<&str>,
    ) -> KernelResult<Vec<ModelStreamChunk>> {
        if let Some(runtime) = &self.agent_runtime {
            match self.stream_typed(runtime, request, model_provider_id) {
                Ok(chunks) => {
                    if !chunks.is_empty() || !self.allow_mock_fallback {
                        return Ok(chunks);
                    }
                }
                Err(error) if self.allow_mock_fallback => {}
                Err(error) => return Err(error),
            }
        }

        self.stream_mock(request)
    }

    /// Get available model descriptors
    pub fn list_models(&self) -> Vec<ModelDescriptor> {
        if let Some(runtime) = &self.agent_runtime {
            if let Ok(provider) = runtime.model_provider() {
                let models = provider.list_models();
                if !models.is_empty() {
                    return models;
                }
            }
        }

        self.list_models_mock()
    }

    fn stream_typed(
        &self,
        runtime: &AgentRuntime,
        request: &ModelRequest,
        model_provider_id: Option<&str>,
    ) -> KernelResult<Vec<ModelStreamChunk>> {
        let provider = self.resolve_model_provider(runtime, model_provider_id)?;
        provider.stream(request.clone())
    }

    /// Cancel an in-flight model invocation (typed provider when registered,
    /// otherwise mock bridge path).
    pub fn cancel(
        &self,
        model_request_id: &str,
        model_provider_id: Option<&str>,
    ) -> KernelResult<ModelResponse> {
        if let Some(runtime) = &self.agent_runtime {
            match self.cancel_typed(runtime, model_request_id, model_provider_id) {
                Ok(response) => return Ok(response),
                Err(error) if self.allow_mock_fallback => {}
                Err(error) => return Err(error),
            }
        }

        self.cancel_mock(model_request_id)
    }

    fn cancel_typed(
        &self,
        runtime: &AgentRuntime,
        model_request_id: &str,
        model_provider_id: Option<&str>,
    ) -> KernelResult<ModelResponse> {
        let mut cancellation_request =
            ModelCancellationRequest::new("cancel.bridge", model_request_id.to_string());
        if let Some(provider_id) = model_provider_id.filter(|value| !value.is_empty()) {
            cancellation_request = cancellation_request.with_provider_id(provider_id.to_string());
        }
        let response = ModelExecutionService::new().cancel(runtime, cancellation_request)?;
        Ok(response.model_response)
    }

    fn cancel_mock(&self, model_request_id: &str) -> KernelResult<ModelResponse> {
        Ok(ModelResponse::cancelled(model_request_id, "provider.mock"))
    }

    fn stream_mock(&self, request: &ModelRequest) -> KernelResult<Vec<ModelStreamChunk>> {
        let text = format!(
            "This is a mock streamed response. Model: {}, Messages: {}",
            request.model_id.as_deref().unwrap_or("unknown"),
            request.messages.len()
        );
        Ok(text
            .split_whitespace()
            .enumerate()
            .map(|(index, word)| {
                ModelStreamChunk::output(
                    &request.model_request_id,
                    index as u64,
                    format!("{word} "),
                )
            })
            .collect())
    }

    fn resolve_model_provider<'a>(
        &self,
        runtime: &'a AgentRuntime,
        model_provider_id: Option<&str>,
    ) -> KernelResult<&'a (dyn sdkwork_agent_kernel::ModelProvider + Send + Sync)> {
        match model_provider_id.filter(|value| !value.is_empty()) {
            Some(provider_id) => runtime.model_provider_by_id(provider_id),
            None => runtime.model_provider(),
        }
    }

    fn invoke_typed(
        &self,
        runtime: &AgentRuntime,
        request: &ModelRequest,
        model_provider_id: Option<&str>,
    ) -> KernelResult<BridgeModelResult> {
        let provider = self.resolve_model_provider(runtime, model_provider_id)?;
        let response = provider.invoke(request.clone())?;
        let events = vec![BridgeEvent {
            event_type: "agent.model.invoked".to_string(),
            session_id: request.session_id.clone(),
            task_id: None,
            payload: format!(
                "provider={};model={}",
                response.provider_id,
                request.model_id.as_deref().unwrap_or("default")
            ),
            severity: BridgeEventSeverity::Info,
        }];

        Ok(BridgeModelResult {
            response,
            tool_calls: Vec::new(),
            events,
        })
    }

    fn invoke_mock(&self, request: &ModelRequest) -> KernelResult<BridgeModelResult> {
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

    fn list_models_mock(&self) -> Vec<ModelDescriptor> {
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

        let result = bridge.invoke(&request, None).expect("invoked");
        assert!(!result.response.messages.is_empty());
    }

    #[test]
    fn stream_returns_chunks() {
        let bridge = ModelBridge::new();
        let request = ModelRequest::new("req.1", vec!["Hello".to_string()]);

        let chunks = bridge.stream(&request, None).expect("streamed");
        assert!(!chunks.is_empty());
        assert!(chunks.iter().all(|chunk| !chunk.content.is_empty()));
    }

    #[test]
    fn list_models_returns_catalog() {
        let bridge = ModelBridge::new();
        let models = bridge.list_models();
        assert_eq!(models.len(), 3);
    }
}
