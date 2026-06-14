use crate::chat::ChatClient;
use crate::types::*;
use reqwest::Client;

/// SSE-based chat client for streaming responses
pub struct SseChatClient {
    base_url: String,
    client: Client,
}

impl SseChatClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: Client::new(),
        }
    }

    /// Send a message via HTTP
    pub async fn send_message_async(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        let url = format!("{}/api/chat/send", self.base_url);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("request failed with status: {}", response.status()));
        }

        response
            .json::<ChatResponse>()
            .await
            .map_err(|e| format!("failed to parse response: {}", e))
    }

    /// Get messages via HTTP
    pub async fn get_messages_async(
        &self,
        session_id: &str,
        limit: Option<u32>,
    ) -> Result<Vec<ChatMessage>, String> {
        let mut url = format!("{}/api/sessions/{}/messages", self.base_url, session_id);
        if let Some(limit) = limit {
            url = format!("{}?limit={}", url, limit);
        }

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("request failed with status: {}", response.status()));
        }

        response
            .json::<Vec<ChatMessage>>()
            .await
            .map_err(|e| format!("failed to parse response: {}", e))
    }

    /// Create session via HTTP
    pub async fn create_session_async(&self, config: SessionConfig) -> Result<SessionInfo, String> {
        let url = format!("{}/api/sessions", self.base_url);
        let response = self
            .client
            .post(&url)
            .json(&config)
            .send()
            .await
            .map_err(|e| format!("request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("request failed with status: {}", response.status()));
        }

        response
            .json::<SessionInfo>()
            .await
            .map_err(|e| format!("failed to parse response: {}", e))
    }

    /// Close session via HTTP
    pub async fn close_session_async(&self, session_id: &str) -> Result<(), String> {
        let url = format!("{}/api/sessions/{}/close", self.base_url, session_id);
        let response = self
            .client
            .post(&url)
            .send()
            .await
            .map_err(|e| format!("request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("request failed with status: {}", response.status()));
        }

        Ok(())
    }

    /// Health check via HTTP
    pub async fn health_async(&self) -> Result<bool, String> {
        let url = format!("{}/health", self.base_url);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("request failed: {}", e))?;

        Ok(response.status().is_success())
    }
}

impl ChatClient for SseChatClient {
    fn send_message(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| format!("runtime error: {}", e))?;
        rt.block_on(self.send_message_async(request))
    }

    fn get_messages(
        &self,
        session_id: &str,
        limit: Option<u32>,
    ) -> Result<Vec<ChatMessage>, String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| format!("runtime error: {}", e))?;
        rt.block_on(self.get_messages_async(session_id, limit))
    }

    fn create_session(&self, config: SessionConfig) -> Result<SessionInfo, String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| format!("runtime error: {}", e))?;
        rt.block_on(self.create_session_async(config))
    }

    fn close_session(&self, session_id: &str) -> Result<(), String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| format!("runtime error: {}", e))?;
        rt.block_on(self.close_session_async(session_id))
    }

    fn health(&self) -> Result<bool, String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| format!("runtime error: {}", e))?;
        rt.block_on(self.health_async())
    }
}
