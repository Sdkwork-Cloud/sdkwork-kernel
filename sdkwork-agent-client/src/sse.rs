use crate::chat::ChatClient;
use crate::ingress_auth;
use crate::session::BridgeSessionQuery;
use crate::types::*;
use crate::AgentAuth;
use futures::Stream;
use reqwest::header::CONTENT_TYPE;
use reqwest::{Client, Response, Url};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Canonical internal-api runtime mount prefix on `application.public-ingress`.
pub const INTERNAL_RUNTIME_MOUNT_PREFIX: &str = "/internal/v3/api/intelligence/runtime";

const DEFAULT_PAGE_SIZE: u32 = 20;
const MAX_PAGE_SIZE: u32 = 200;
const MAX_SSE_LINE_BYTES: usize = 1024 * 1024;
const MAX_SSE_EVENT_BYTES: usize = 1024 * 1024;
const SSE_CHANNEL_CAPACITY: usize = 32;
const IDEMPOTENCY_KEY_HEADER: &str = "Idempotency-Key";

pub type ModelEventStream =
    Pin<Box<dyn Stream<Item = Result<ModelStreamEvent, String>> + Send + 'static>>;
pub type SessionEventStream =
    Pin<Box<dyn Stream<Item = Result<SessionRuntimeStreamEvent, String>> + Send + 'static>>;

/// Typed HTTP and SSE client for the canonical internal runtime API.
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

    fn runtime_endpoint(&self, segments: &[&str]) -> Result<Url, String> {
        let mut url = Url::parse(self.base_url.trim_end_matches('/'))
            .map_err(|error| format!("invalid agent server URL: {error}"))?;
        {
            let mut path = url
                .path_segments_mut()
                .map_err(|_| "agent server URL cannot be a base URL".to_string())?;
            path.pop_if_empty();
            for segment in INTERNAL_RUNTIME_MOUNT_PREFIX.trim_matches('/').split('/') {
                path.push(segment);
            }
            for segment in segments {
                path.push(segment);
            }
        }
        Ok(url)
    }

    fn health_endpoint(&self) -> Result<Url, String> {
        let mut url = Url::parse(self.base_url.trim_end_matches('/'))
            .map_err(|error| format!("invalid agent server URL: {error}"))?;
        url.path_segments_mut()
            .map_err(|_| "agent server URL cannot be a base URL".to_string())?
            .pop_if_empty()
            .push("healthz");
        Ok(url)
    }

    fn apply_auth(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let Some(auth) = &self.auth else {
            return builder;
        };
        ingress_auth::apply_ingress_auth(builder, auth)
    }

    fn new_idempotency_key() -> String {
        format!("sdkwork-client-{}", Uuid::new_v4())
    }

    fn validate_idempotency_key(value: &str) -> Result<&str, String> {
        let value = value.trim();
        if value.is_empty() || value.len() > 255 {
            return Err("idempotency key must contain between 1 and 255 bytes".to_string());
        }
        if !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        }) {
            return Err("idempotency key contains unsupported characters".to_string());
        }
        Ok(value)
    }

    /// Execute and return one persisted user/assistant turn.
    pub async fn send_message_async(&self, request: ChatRequest) -> Result<ChatTurn, String> {
        let idempotency_key = Self::new_idempotency_key();
        self.send_message_with_idempotency_key_async(request, &idempotency_key)
            .await
    }

    /// Execute a message turn with a caller-controlled retry key. Reusing the
    /// same key for a byte-identical request safely replays the original turn.
    pub async fn send_message_with_idempotency_key_async(
        &self,
        request: ChatRequest,
        idempotency_key: impl AsRef<str>,
    ) -> Result<ChatTurn, String> {
        let idempotency_key = Self::validate_idempotency_key(idempotency_key.as_ref())?;
        let url = self.runtime_endpoint(&["sessions", &request.session_id, "messages"])?;
        let response = self
            .apply_auth(self.client.post(url))
            .header(IDEMPOTENCY_KEY_HEADER, idempotency_key)
            .json(&serde_json::json!({ "content": request.content }))
            .send()
            .await
            .map_err(|error| format!("request failed: {error}"))?;
        let response = require_success(response)?;
        let payload = response
            .json::<SdkWorkApiResponse<SdkWorkItemData<InternalMessageTurnResponse>>>()
            .await
            .map_err(|error| format!("failed to parse response: {error}"))?;
        ensure_success_code(payload.code)?;
        map_internal_message_turn(payload.data.item)
    }

    /// Request exactly one bounded page of messages.
    pub async fn get_messages_page_async(
        &self,
        session_id: &str,
        query: SdkWorkListQuery,
    ) -> Result<SdkWorkPage<ChatMessage>, String> {
        let url = apply_list_query(
            self.runtime_endpoint(&["sessions", session_id, "messages"])?,
            &query,
        )?;
        let response = self
            .apply_auth(self.client.get(url))
            .send()
            .await
            .map_err(|error| format!("request failed: {error}"))?;
        let response = require_success(response)?;
        let payload = response
            .json::<SdkWorkApiResponse<SdkWorkListData<InternalMessageResponse>>>()
            .await
            .map_err(|error| format!("failed to parse response: {error}"))?;
        ensure_success_code(payload.code)?;

        let items = payload
            .data
            .items
            .into_iter()
            .map(map_internal_message)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(SdkWorkPage {
            items,
            page_info: payload.data.page_info,
        })
    }

    /// Compatibility helper that returns only the first server-bounded page.
    pub async fn get_messages_async(
        &self,
        session_id: &str,
        limit: Option<u32>,
    ) -> Result<Vec<ChatMessage>, String> {
        self.get_messages_page_async(
            session_id,
            SdkWorkListQuery {
                page_size: limit,
                ..Default::default()
            },
        )
        .await
        .map(|page| page.items)
    }

    /// Request exactly one bounded page of sessions.
    pub async fn list_sessions_page_async(
        &self,
        query: SdkWorkListQuery,
    ) -> Result<SdkWorkPage<SessionInfo>, String> {
        let url = apply_list_query(self.runtime_endpoint(&["sessions"])?, &query)?;
        let response = self
            .apply_auth(self.client.get(url))
            .send()
            .await
            .map_err(|error| format!("request failed: {error}"))?;
        let response = require_success(response)?;
        let payload = response
            .json::<SdkWorkApiResponse<SdkWorkListData<InternalSessionResponse>>>()
            .await
            .map_err(|error| format!("failed to parse response: {error}"))?;
        ensure_success_code(payload.code)?;

        Ok(SdkWorkPage {
            items: payload
                .data
                .items
                .into_iter()
                .map(map_internal_session)
                .collect(),
            page_info: payload.data.page_info,
        })
    }

    pub async fn list_sessions_async(
        &self,
        query: &BridgeSessionQuery,
    ) -> Result<Vec<SessionInfo>, String> {
        if query.agent_id.is_some()
            || query.provider_id.is_some()
            || query.bridge_id.is_some()
            || query.active_only
        {
            return Err(
                "remote internal-api session listing does not support bridge-local filters"
                    .to_string(),
            );
        }
        self.list_sessions_page_async(SdkWorkListQuery {
            page_size: query.limit,
            ..Default::default()
        })
        .await
        .map(|page| page.items)
    }

    pub async fn create_session_async(&self, config: SessionConfig) -> Result<SessionInfo, String> {
        let idempotency_key = Self::new_idempotency_key();
        self.create_session_with_idempotency_key_async(config, &idempotency_key)
            .await
    }

    /// Create a session with a caller-controlled retry key.
    pub async fn create_session_with_idempotency_key_async(
        &self,
        config: SessionConfig,
        idempotency_key: impl AsRef<str>,
    ) -> Result<SessionInfo, String> {
        let idempotency_key = Self::validate_idempotency_key(idempotency_key.as_ref())?;
        let response = self
            .apply_auth(self.client.post(self.runtime_endpoint(&["sessions"])?))
            .header(IDEMPOTENCY_KEY_HEADER, idempotency_key)
            .json(&config)
            .send()
            .await
            .map_err(|error| format!("request failed: {error}"))?;
        let response = require_success(response)?;
        let payload = response
            .json::<SdkWorkApiResponse<SdkWorkItemData<InternalSessionResponse>>>()
            .await
            .map_err(|error| format!("failed to parse response: {error}"))?;
        ensure_success_code(payload.code)?;
        Ok(map_internal_session(payload.data.item))
    }

    pub async fn close_session_async(&self, session_id: &str) -> Result<(), String> {
        let idempotency_key = Self::new_idempotency_key();
        self.close_session_with_idempotency_key_async(session_id, &idempotency_key)
            .await
    }

    /// Close a session with a caller-controlled retry key.
    pub async fn close_session_with_idempotency_key_async(
        &self,
        session_id: &str,
        idempotency_key: impl AsRef<str>,
    ) -> Result<(), String> {
        let idempotency_key = Self::validate_idempotency_key(idempotency_key.as_ref())?;
        let response = self
            .apply_auth(
                self.client
                    .post(self.runtime_endpoint(&["sessions", session_id, "close"])?),
            )
            .header(IDEMPOTENCY_KEY_HEADER, idempotency_key)
            .send()
            .await
            .map_err(|error| format!("request failed: {error}"))?;
        require_success(response)?;
        Ok(())
    }

    /// Open a typed model SSE stream. The returned stream is lazy to consume,
    /// bounded by a 32-item channel, and closes the HTTP response when dropped.
    pub async fn stream_model_async(
        &self,
        request: ModelStreamRequest,
    ) -> Result<ModelEventStream, String> {
        let body = StreamModelBody {
            model_id: request.model_id,
            messages: request.messages,
        };
        let response = self
            .apply_auth(self.client.post(self.runtime_endpoint(&[
                "sessions",
                &request.session_id,
                "model",
                "stream",
            ])?))
            .json(&body)
            .send()
            .await
            .map_err(|error| format!("request failed: {error}"))?;
        let response = require_success(response)?;
        require_event_stream(&response)?;
        Ok(decode_response_stream(response, map_model_event))
    }

    /// Replay and optionally follow typed session events.
    pub async fn stream_session_events_async(
        &self,
        request: SessionEventStreamRequest,
    ) -> Result<SessionEventStream, String> {
        let mut url =
            self.runtime_endpoint(&["sessions", &request.session_id, "events", "stream"])?;
        {
            let mut query = url.query_pairs_mut();
            if let Some(last_event_id) = request.last_event_id {
                query.append_pair("lastEventId", &last_event_id);
            }
            query.append_pair("live", if request.live { "true" } else { "false" });
        }
        let response = self
            .apply_auth(self.client.get(url))
            .send()
            .await
            .map_err(|error| format!("request failed: {error}"))?;
        let response = require_success(response)?;
        require_event_stream(&response)?;
        Ok(decode_response_stream(response, map_session_event))
    }

    pub async fn health_async(&self) -> Result<bool, String> {
        let response = self
            .client
            .get(self.health_endpoint()?)
            .send()
            .await
            .map_err(|error| format!("request failed: {error}"))?;
        Ok(response.status().is_success())
    }
}

impl ChatClient for SseChatClient {
    fn send_message(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        let session_id = request.session_id.clone();
        let client = self.clone();
        let turn = block_on_sync(async move { client.send_message_async(request).await })?;
        let message = turn
            .assistant_message
            .unwrap_or_else(|| turn.user_message.clone());
        Ok(ChatResponse {
            message_id: message.id,
            session_id,
            content: message.content,
            status: turn.status,
            usage: None,
        })
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

    fn list_sessions(&self, query: &BridgeSessionQuery) -> Result<Vec<SessionInfo>, String> {
        let client = self.clone();
        let query = query.clone();
        block_on_sync(async move { client.list_sessions_async(&query).await })
    }

    fn health(&self) -> Result<bool, String> {
        let client = self.clone();
        block_on_sync(async move { client.health_async().await })
    }
}

fn apply_list_query(mut url: Url, query: &SdkWorkListQuery) -> Result<Url, String> {
    let page_size = query.page_size.unwrap_or(DEFAULT_PAGE_SIZE);
    if !(1..=MAX_PAGE_SIZE).contains(&page_size) {
        return Err(format!("page_size must be between 1 and {MAX_PAGE_SIZE}"));
    }

    let mut pairs = url.query_pairs_mut();
    if let Some(cursor) = query.cursor.as_deref() {
        if cursor.trim().is_empty() {
            return Err("cursor cannot be empty".to_string());
        }
        pairs.append_pair("cursor", cursor);
    }
    pairs.append_pair("page_size", &page_size.to_string());
    drop(pairs);
    Ok(url)
}

fn require_success(response: Response) -> Result<Response, String> {
    if response.status().is_success() {
        Ok(response)
    } else {
        Err(format!("request failed with status: {}", response.status()))
    }
}

fn require_event_stream(response: &Response) -> Result<(), String> {
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if content_type
        .split(';')
        .next()
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("text/event-stream"))
    {
        Ok(())
    } else {
        Err(format!(
            "expected text/event-stream response, received {content_type:?}"
        ))
    }
}

fn decode_response_stream<T, F>(
    mut response: Response,
    mut mapper: F,
) -> Pin<Box<dyn Stream<Item = Result<T, String>> + Send>>
where
    T: Send + 'static,
    F: FnMut(SseEvent) -> Result<T, String> + Send + 'static,
{
    let (sender, receiver) = mpsc::channel(SSE_CHANNEL_CAPACITY);
    tokio::spawn(async move {
        let mut decoder = SseDecoder::default();
        loop {
            let (events, finished) = match response.chunk().await {
                Ok(Some(chunk)) => (decoder.push(&chunk), false),
                Ok(None) => (decoder.finish(), true),
                Err(error) => (Err(format!("SSE response read failed: {error}")), true),
            };

            let events = match events {
                Ok(events) => events,
                Err(error) => {
                    let _ = sender.send(Err(error)).await;
                    return;
                }
            };
            for event in events {
                let item = mapper(event);
                if sender.send(item).await.is_err() {
                    return;
                }
            }
            if finished {
                return;
            }
        }
    });

    Box::pin(futures::stream::unfold(
        receiver,
        |mut receiver| async move { receiver.recv().await.map(|item| (item, receiver)) },
    ))
}

fn map_model_event(event: SseEvent) -> Result<ModelStreamEvent, String> {
    match event.event.as_deref() {
        Some("model.chunk") => parse_json(&event.data).map(ModelStreamEvent::Chunk),
        Some("model.done") => Ok(ModelStreamEvent::Done),
        Some("model.error") => {
            let error: ModelStreamErrorData = parse_json(&event.data)?;
            Ok(ModelStreamEvent::Error {
                message: error.message,
            })
        }
        other => Err(format!("unsupported model SSE event: {other:?}")),
    }
}

fn map_session_event(event: SseEvent) -> Result<SessionRuntimeStreamEvent, String> {
    let data: SessionRuntimeStreamData = parse_json(&event.data)?;
    Ok(SessionRuntimeStreamEvent {
        event: event.event.unwrap_or_else(|| data.event_type.clone()),
        id: event.id.or_else(|| Some(data.event_id.clone())),
        data,
    })
}

fn parse_json<T: DeserializeOwned>(data: &str) -> Result<T, String> {
    serde_json::from_str(data).map_err(|error| format!("invalid SSE JSON payload: {error}"))
}

#[derive(Default)]
struct SseDecoder {
    line: Vec<u8>,
    event: Option<String>,
    last_event_id: Option<String>,
    data_lines: Vec<String>,
    data_bytes: usize,
}

impl SseDecoder {
    fn push(&mut self, bytes: &[u8]) -> Result<Vec<SseEvent>, String> {
        let mut events = Vec::new();
        for byte in bytes {
            if *byte == b'\n' {
                if self.line.last() == Some(&b'\r') {
                    self.line.pop();
                }
                if let Some(event) = self.process_line()? {
                    events.push(event);
                }
                self.line.clear();
            } else {
                self.line.push(*byte);
                if self.line.len() > MAX_SSE_LINE_BYTES {
                    return Err(format!(
                        "SSE line exceeds the {MAX_SSE_LINE_BYTES} byte limit"
                    ));
                }
            }
        }
        Ok(events)
    }

    fn finish(&mut self) -> Result<Vec<SseEvent>, String> {
        let mut events = Vec::new();
        if !self.line.is_empty() {
            if let Some(event) = self.process_line()? {
                events.push(event);
            }
            self.line.clear();
        }
        if let Some(event) = self.dispatch_event() {
            events.push(event);
        }
        Ok(events)
    }

    fn process_line(&mut self) -> Result<Option<SseEvent>, String> {
        let line = std::str::from_utf8(&self.line)
            .map_err(|error| format!("SSE line is not valid UTF-8: {error}"))?
            .to_string();
        if line.is_empty() {
            return Ok(self.dispatch_event());
        }
        if line.starts_with(':') {
            return Ok(None);
        }

        let (field, value) = line
            .split_once(':')
            .map(|(field, value)| (field, value.strip_prefix(' ').unwrap_or(value)))
            .unwrap_or((line.as_str(), ""));
        match field {
            "event" => self.event = Some(value.to_string()),
            "id" if !value.contains('\0') => self.last_event_id = Some(value.to_string()),
            "data" => {
                self.data_bytes = self
                    .data_bytes
                    .saturating_add(value.len())
                    .saturating_add(usize::from(!self.data_lines.is_empty()));
                if self.data_bytes > MAX_SSE_EVENT_BYTES {
                    return Err(format!(
                        "SSE event exceeds the {MAX_SSE_EVENT_BYTES} byte limit"
                    ));
                }
                self.data_lines.push(value.to_string());
            }
            _ => {}
        }
        Ok(None)
    }

    fn dispatch_event(&mut self) -> Option<SseEvent> {
        if self.data_lines.is_empty() {
            self.event = None;
            return None;
        }
        let event = SseEvent {
            event: self.event.take(),
            data: self.data_lines.join("\n"),
            id: self.last_event_id.clone(),
        };
        self.data_lines.clear();
        self.data_bytes = 0;
        Some(event)
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
    let runtime =
        tokio::runtime::Runtime::new().map_err(|error| format!("runtime error: {error}"))?;
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
    #[serde(default)]
    page_info: SdkWorkPageInfo,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SdkWorkItemData<T> {
    item: T,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InternalMessageResponse {
    message_id: String,
    role: String,
    parts: Vec<InternalMessagePartResponse>,
    created_at: Option<String>,
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InternalMessageTurnResponse {
    user_message: InternalMessageResponse,
    assistant_message: Option<InternalMessageResponse>,
    status: String,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StreamModelBody {
    model_id: Option<String>,
    messages: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct ModelStreamErrorData {
    message: String,
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
        metadata: message.metadata,
    })
}

fn map_internal_message_turn(turn: InternalMessageTurnResponse) -> Result<ChatTurn, String> {
    let status = match turn.status.as_str() {
        "completed" => ChatStatus::Completed,
        "pending" => ChatStatus::Pending,
        "streaming" => ChatStatus::Streaming,
        "failed" => ChatStatus::Failed,
        other => return Err(format!("unsupported message turn status: {other}")),
    };
    Ok(ChatTurn {
        user_message: map_internal_message(turn.user_message)?,
        assistant_message: turn
            .assistant_message
            .map(map_internal_message)
            .transpose()?,
        status,
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

    #[test]
    fn decoder_preserves_named_events_across_fragmented_chunks() {
        let mut decoder = SseDecoder::default();
        assert!(decoder
            .push(b"event: model.chunk\ndata: {\"modelRequest")
            .expect("first fragment")
            .is_empty());
        let events = decoder
            .push(b"Id\":\"model.1\",\"sequence\":0,\"content\":\"hi\"}\n\n")
            .expect("second fragment");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event.as_deref(), Some("model.chunk"));
        assert_eq!(
            map_model_event(events[0].clone()).expect("mapped"),
            ModelStreamEvent::Chunk(ModelStreamChunk {
                model_request_id: "model.1".to_string(),
                sequence: 0,
                content: "hi".to_string(),
                finish_reason: None,
            })
        );
    }

    #[test]
    fn pagination_rejects_empty_cursor_and_out_of_range_page_size() {
        let url = Url::parse("http://localhost/sessions").expect("url");
        assert!(apply_list_query(
            url.clone(),
            &SdkWorkListQuery {
                cursor: Some(" ".to_string()),
                page_size: Some(20),
            }
        )
        .is_err());
        assert!(apply_list_query(
            url,
            &SdkWorkListQuery {
                page_size: Some(201),
                ..Default::default()
            }
        )
        .is_err());
    }

    #[test]
    fn idempotency_key_validation_is_bounded() {
        assert!(SseChatClient::validate_idempotency_key("request-1").is_ok());
        assert!(SseChatClient::validate_idempotency_key("").is_err());
        assert!(SseChatClient::validate_idempotency_key(&"x".repeat(256)).is_err());
        assert!(SseChatClient::validate_idempotency_key("request key").is_err());
    }

    #[tokio::test]
    async fn sync_health_does_not_panic_inside_existing_tokio_runtime() {
        let client = SseChatClient::new("http://127.0.0.1:9");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| client.health()));
        assert!(result.is_ok());
    }
}
