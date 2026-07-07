use crate::chat::ChatClient;
use crate::ingress_auth;
use crate::types::*;
use crate::AgentAuth;
use reqwest::Client;
use serde::Deserialize;
use std::future::Future;

/// Canonical internal-api runtime mount prefix on `application.public-ingress`.
pub const INTERNAL_RUNTIME_MOUNT_PREFIX: &str = "/internal/v3/api/intelligence/runtime";

/// SSE-based chat client for streaming responses
#[derive(Clone)]
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

    fn messages_url(&self, session_id: &str, limit: Option<u32>) -> String {
        let url = self.runtime_url(&format!("/sessions/{session_id}/messages"));
        match limit {
            Some(limit) => format!("{url}?page_size={limit}"),
            None => url,
        }
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

        let messages = self.get_messages_async(&request.session_id, None).await?;
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
        let url = self.messages_url(session_id, limit);

        let response = self
            .apply_auth(self.client.get(&url))
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;

        if !response.status().is_success() {
            return Err(format!("request failed with status: {}", response.status()));
        }

        let payload = response
            .json::<SdkWorkApiResponse<SdkWorkListData<InternalMessageResponse>>>()
            .await
            .map_err(|e| format!("failed to parse response: {e}"))?;
        ensure_success_code(payload.code)?;

        payload
            .data
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

        let payload = response
            .json::<SdkWorkApiResponse<SdkWorkItemData<InternalSessionResponse>>>()
            .await
            .map_err(|e| format!("failed to parse response: {e}"))?;
        ensure_success_code(payload.code)?;

        Ok(map_internal_session(payload.data.item))
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
        let url = format!("{}/healthz", self.base_url.trim_end_matches('/'));
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
        let client = self.clone();
        block_on_sync(async move { client.send_message_async(request).await })
    }

    fn get_messages(
        &self,
        session_id: &str,
        limit: Option<u32>,
    ) -> Result<Vec<ChatMessage>, String> {
        let client = self.clone();
        let session_id = session_id.to_string();
        block_on_sync(async move { client.get_messages_async(&session_id, limit).await })
    }

    fn create_session(&self, config: SessionConfig) -> Result<SessionInfo, String> {
        let client = self.clone();
        block_on_sync(async move { client.create_session_async(config).await })
    }

    fn close_session(&self, session_id: &str) -> Result<(), String> {
        let client = self.clone();
        let session_id = session_id.to_string();
        block_on_sync(async move { client.close_session_async(&session_id).await })
    }

    fn health(&self) -> Result<bool, String> {
        let client = self.clone();
        block_on_sync(async move { client.health_async().await })
    }
}

fn block_on_sync<T, F>(future: F) -> Result<T, String>
where
    T: Send + 'static,
    F: Future<Output = Result<T, String>> + Send + 'static,
{
    if tokio::runtime::Handle::try_current().is_ok() {
        let thread = std::thread::Builder::new()
            .name("sdkwork-agent-client-sync-runtime".to_string())
            .spawn(move || block_on_new_runtime(future))
            .map_err(|error| format!("runtime thread error: {error}"))?;
        return thread
            .join()
            .map_err(|_| "runtime thread panicked".to_string())?;
    }

    block_on_new_runtime(future)
}

fn block_on_new_runtime<T, F>(future: F) -> Result<T, String>
where
    F: Future<Output = Result<T, String>>,
{
    let runtime = tokio::runtime::Runtime::new().map_err(|e| format!("runtime error: {e}"))?;
    runtime.block_on(future)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SdkWorkApiResponse<T> {
    code: i32,
    data: T,
    #[allow(dead_code)]
    trace_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SdkWorkListData<T> {
    items: Vec<T>,
    #[allow(dead_code)]
    page_info: Option<SdkWorkPageInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SdkWorkItemData<T> {
    item: T,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SdkWorkPageInfo {
    #[allow(dead_code)]
    mode: String,
    #[allow(dead_code)]
    page: Option<i64>,
    #[allow(dead_code)]
    page_size: i64,
    #[allow(dead_code)]
    has_more: bool,
    #[allow(dead_code)]
    next_cursor: Option<String>,
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

fn ensure_success_code(code: i32) -> Result<(), String> {
    if code == 0 {
        Ok(())
    } else {
        Err(format!("sdkwork response returned non-zero code: {code}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_sdkwork_v3_message_list_envelope() {
        let payload = json!({
            "code": 0,
            "traceId": "018f4f0d-0000-7000-8000-000000000001",
            "data": {
                "items": [
                    {
                        "messageId": "msg.1",
                        "role": "assistant",
                        "parts": [{ "content": "hello" }],
                        "createdAt": "2026-07-07T00:00:00Z"
                    }
                ],
                "pageInfo": {
                    "mode": "offset",
                    "page": 1,
                    "pageSize": 20,
                    "hasMore": false
                }
            }
        });

        let envelope: SdkWorkApiResponse<SdkWorkListData<InternalMessageResponse>> =
            serde_json::from_value(payload).expect("sdkwork v3 list envelope");

        assert_eq!(envelope.code, 0);
        assert_eq!(envelope.data.items.len(), 1);
        assert_eq!(envelope.data.items[0].message_id, "msg.1");
    }

    #[test]
    fn parses_sdkwork_v3_session_item_envelope() {
        let payload = json!({
            "code": 0,
            "traceId": "018f4f0d-0000-7000-8000-000000000002",
            "data": {
                "item": {
                    "sessionId": "session.1",
                    "agentId": "agent.1",
                    "model": null,
                    "title": "Kernel",
                    "state": "active",
                    "messageCount": 0,
                    "createdAt": "2026-07-07T00:00:00Z",
                    "updatedAt": "2026-07-07T00:00:00Z"
                }
            }
        });

        let envelope: SdkWorkApiResponse<SdkWorkItemData<InternalSessionResponse>> =
            serde_json::from_value(payload).expect("sdkwork v3 item envelope");

        assert_eq!(envelope.code, 0);
        assert_eq!(envelope.data.item.session_id, "session.1");
        assert_eq!(envelope.data.item.agent_id, "agent.1");
    }

    #[test]
    fn message_list_url_uses_canonical_page_size_query() {
        let client = SseChatClient::new("http://localhost:18280");

        assert_eq!(
            client.messages_url("session.1", Some(10)),
            "http://localhost:18280/internal/v3/api/intelligence/runtime/sessions/session.1/messages?page_size=10"
        );
    }

    #[tokio::test]
    async fn sync_health_does_not_panic_inside_existing_tokio_runtime() {
        let client = SseChatClient::new("http://127.0.0.1:9");

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| client.health()));

        assert!(
            result.is_ok(),
            "sync ChatClient methods must not panic when called from an existing Tokio runtime"
        );
    }
}
