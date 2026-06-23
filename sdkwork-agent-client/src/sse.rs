use crate::ingress_auth;
use crate::AgentAuth;
use crate::chat::ChatClient;
use crate::types::*;
use reqwest::Client;
use serde::Deserialize;

/// Canonical internal-api runtime mount prefix on `application.public-ingress`.
pub const INTERNAL_RUNTIME_MOUNT_PREFIX: &str = "/internal/v3/api/intelligence/runtime";

/// SSE-based chat client for streaming responses
pub struct SseChatClient {
    base_url: String,
    client: Client,
    auth: Option<AgentAuth>,
}

impl SseChatClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self::with_auth(base_url, None)
    }

    pub fn with_auth(base_url: impl Into<String>, auth: Option<AgentAuth>) -> Self {
        Self {
            base_url: base_url.into(),
            client: Client::new(),
            auth,
        }
    }

    fn runtime_url(&self, relative: &str) -> String {
        format!(
            "{}{INTERNAL_RUNTIME_MOUNT_PREFIX}{relative}",
            self.base_url.trim_end_matches('/')
        )
    }

    fn apply_auth(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let Some(auth) = &self.auth else {
            return builder;
        };
        ingress_auth::apply_ingress_auth(builder, auth)
    }

    /// Send a message via internal-api runtime HTTP.
    pub async fn send_message_async(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        let url = self.runtime_url(&format!("/sessions/{}/messages", request.session_id));

        let response = self
            .apply_auth(self.client.post(&url))
            .json(&serde_json::json!({ "content": request.content }))
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;

        if !response.status().is_success() {
            return Err(format!("request failed with status: {}", response.status()));
        }

        let messages = self
            .get_messages_async(&request.session_id, None)
            .await?;
        let assistant = messages
            .iter()
            .rev()
            .find(|message| message.role == MessageRole::Assistant)
            .ok_or_else(|| "assistant response missing after send".to_string())?;

        Ok(ChatResponse {
            message_id: assistant.id.clone(),
            session_id: request.session_id,
            content: assistant.content.clone(),
            status: ChatStatus::Completed,
            usage: None,
        })
    }

    /// Get messages via internal-api runtime HTTP.
    pub async fn get_messages_async(
        &self,
        session_id: &str,
        limit: Option<u32>,
    ) -> Result<Vec<ChatMessage>, String> {
        let mut url = self.runtime_url(&format!("/sessions/{session_id}/messages"));
        if let Some(limit) = limit {
            url = format!("{url}?limit={limit}");
        }

        let response = self
            .apply_auth(self.client.get(&url))
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;

        if !response.status().is_success() {
            return Err(format!("request failed with status: {}", response.status()));
        }

        let payload = response
            .json::<InternalMessageListResponse>()
            .await
            .map_err(|e| format!("failed to parse response: {e}"))?;

        payload
            .items
            .into_iter()
            .map(map_internal_message)
            .collect()
    }

    /// Create session via internal-api runtime HTTP.
    pub async fn create_session_async(&self, config: SessionConfig) -> Result<SessionInfo, String> {
        let url = self.runtime_url("/sessions");
        let response = self
            .apply_auth(self.client.post(&url))
            .json(&config)
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;

        if !response.status().is_success() {
            return Err(format!("request failed with status: {}", response.status()));
        }

        let session = response
            .json::<InternalSessionResponse>()
            .await
            .map_err(|e| format!("failed to parse response: {e}"))?;

        Ok(map_internal_session(session))
    }

    /// Close session via internal-api runtime HTTP.
    pub async fn close_session_async(&self, session_id: &str) -> Result<(), String> {
        let url = self.runtime_url(&format!("/sessions/{session_id}/close"));
        let response = self
            .apply_auth(self.client.post(&url))
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;

        if !response.status().is_success() {
            return Err(format!("request failed with status: {}", response.status()));
        }

        Ok(())
    }

    /// Health check via HTTP
    pub async fn health_async(&self) -> Result<bool, String> {
        let url = format!("{}/health", self.base_url.trim_end_matches('/'));
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;

        Ok(response.status().is_success())
    }
}

impl ChatClient for SseChatClient {
    fn send_message(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| format!("runtime error: {e}"))?;
        rt.block_on(self.send_message_async(request))
    }

    fn get_messages(
        &self,
        session_id: &str,
        limit: Option<u32>,
    ) -> Result<Vec<ChatMessage>, String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| format!("runtime error: {e}"))?;
        rt.block_on(self.get_messages_async(session_id, limit))
    }

    fn create_session(&self, config: SessionConfig) -> Result<SessionInfo, String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| format!("runtime error: {e}"))?;
        rt.block_on(self.create_session_async(config))
    }

    fn close_session(&self, session_id: &str) -> Result<(), String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| format!("runtime error: {e}"))?;
        rt.block_on(self.close_session_async(session_id))
    }

    fn health(&self) -> Result<bool, String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| format!("runtime error: {e}"))?;
        rt.block_on(self.health_async())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InternalMessageListResponse {
    items: Vec<InternalMessageResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InternalMessageResponse {
    message_id: String,
    role: String,
    parts: Vec<InternalMessagePartResponse>,
    created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InternalMessagePartResponse {
    content: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InternalSessionResponse {
    session_id: String,
    agent_id: String,
    model: Option<String>,
    title: Option<String>,
    state: String,
    message_count: u32,
    created_at: Option<String>,
    updated_at: Option<String>,
}

fn map_internal_message(message: InternalMessageResponse) -> Result<ChatMessage, String> {
    let role = match message.role.as_str() {
        "user" => MessageRole::User,
        "assistant" => MessageRole::Assistant,
        "system" => MessageRole::System,
        "tool" => MessageRole::Tool,
        other => return Err(format!("unsupported message role: {other}")),
    };
    let content = message
        .parts
        .first()
        .map(|part| part.content.clone())
        .unwrap_or_default();

    Ok(ChatMessage {
        id: message.message_id,
        role,
        content,
        timestamp: message.created_at.unwrap_or_default(),
        metadata: None,
    })
}

fn map_internal_session(session: InternalSessionResponse) -> SessionInfo {
    SessionInfo {
        session_id: session.session_id,
        agent_id: session.agent_id,
        provider_id: String::new(),
        bridge_id: String::new(),
        model: session.model,
        title: session.title,
        state: session.state,
        message_count: session.message_count,
        created_at: session.created_at.unwrap_or_default(),
        updated_at: session.updated_at.unwrap_or_default(),
    }
}
