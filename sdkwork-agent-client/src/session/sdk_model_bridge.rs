use crate::bridge::AgentBridgeHealth;
use crate::session::{BridgeSessionQuery, BridgeSessionStore};
use crate::types::*;
use sdkwork_agent_kernel::{ModelRequest, ModelResponse, ModelStatus};
use std::sync::Arc;

pub type ModelInvokeFn =
    Arc<dyn Fn(ModelRequest) -> Result<ModelResponse, String> + Send + Sync>;
pub type HealthProbeFn = Arc<dyn Fn() -> AgentBridgeHealth + Send + Sync>;

/// Local bridge runtime that persists sessions in SQLite and routes chat through a kernel model provider.
pub struct SdkModelBridgeRuntime {
    store: BridgeSessionStore,
    invoke_model: ModelInvokeFn,
    health_probe: HealthProbeFn,
}

impl SdkModelBridgeRuntime {
    pub fn new(
        provider_id: &str,
        bridge_id: &str,
        invoke_model: ModelInvokeFn,
        health_probe: HealthProbeFn,
    ) -> Result<Self, String> {
        Ok(Self {
            store: BridgeSessionStore::open_default(provider_id, bridge_id)?,
            invoke_model,
            health_probe,
        })
    }

    pub fn memory(
        provider_id: &str,
        bridge_id: &str,
        invoke_model: ModelInvokeFn,
        health_probe: HealthProbeFn,
    ) -> Result<Self, String> {
        Ok(Self {
            store: BridgeSessionStore::memory(provider_id, bridge_id)?,
            invoke_model,
            health_probe,
        })
    }

    pub fn session_store(&self) -> &BridgeSessionStore {
        &self.store
    }

    pub fn send_message(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        if request.stream {
            return Err(
                "streaming is not supported on local SDK bridge runtimes; use AgentClientMode::Remote with HttpRestSse"
                    .to_string(),
            );
        }

        let user_message = ChatMessage {
            id: format!("msg.{}", uuid_simple()),
            role: MessageRole::User,
            content: request.content.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: None,
        };
        self.store
            .append_message(&request.session_id, user_message)?;

        let history = self.store.get_messages(&request.session_id, None)?;
        let model_messages = history
            .iter()
            .map(format_chat_message_for_model)
            .collect::<Vec<_>>();

        let mut model_request =
            ModelRequest::new(format!("bridge.{}", uuid_simple()), model_messages)
                .for_session(&request.session_id);
        if let Some(model_id) = request.model.clone() {
            model_request = model_request.with_model_id(model_id);
        }

        let model_response = (self.invoke_model)(model_request)?;
        let assistant_content = model_response
            .messages
            .last()
            .cloned()
            .unwrap_or_default();
        let assistant_message_id = format!("msg.{}", uuid_simple());
        let assistant_message = ChatMessage {
            id: assistant_message_id.clone(),
            role: MessageRole::Assistant,
            content: assistant_content.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: None,
        };
        self.store
            .append_message(&request.session_id, assistant_message)?;

        let status = match model_response.status {
            ModelStatus::Succeeded => ChatStatus::Completed,
            ModelStatus::Cancelled
            | ModelStatus::Failed
            | ModelStatus::TimedOut
            | ModelStatus::PolicyDenied => ChatStatus::Failed,
        };

        Ok(ChatResponse {
            message_id: assistant_message_id,
            session_id: request.session_id,
            content: assistant_content,
            status,
            usage: model_response.usage.map(|usage| TokenUsage {
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
                total_tokens: usage.total_tokens(),
            }),
        })
    }

    pub fn get_messages(
        &self,
        session_id: &str,
        limit: Option<u32>,
    ) -> Result<Vec<ChatMessage>, String> {
        self.store.get_messages(session_id, limit)
    }

    pub fn create_session(&self, config: SessionConfig) -> Result<SessionInfo, String> {
        self.store.create_session(config)
    }

    pub fn close_session(&self, session_id: &str) -> Result<(), String> {
        self.store.close_session(session_id)
    }

    pub fn list_sessions(&self, query: &BridgeSessionQuery) -> Result<Vec<SessionInfo>, String> {
        self.store.list_sessions(query)
    }

    pub fn health_check(&self) -> AgentBridgeHealth {
        (self.health_probe)()
    }
}

fn format_chat_message_for_model(message: &ChatMessage) -> String {
    let role = match message.role {
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
        MessageRole::System => "system",
        MessageRole::Tool => "tool",
    };
    format!("{role}: {}", message.content)
}

fn uuid_simple() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}
