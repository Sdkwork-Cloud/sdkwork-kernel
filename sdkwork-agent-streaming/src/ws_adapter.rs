use crate::{EventMapper, StreamManager, StreamType};
use sdkwork_agent_kernel::{
    AgentTask, KernelEvent, KernelResult, ProtocolAdapter, ProtocolAdapterAuthMode,
    ProtocolAdapterManifest, ProtocolAdapterRequest, ProtocolAdapterResponse,
    ProtocolAdapterStreamingSupport, ProtocolFamily, ProtocolObjectEnvelope, ProtocolObjectKind,
    ProtocolStreamUpdate, ProtocolTransport, ProviderHealth,
};

/// WebSocket protocol adapter for bidirectional kernel communication
pub struct WsProtocolAdapter {
    manifest: ProtocolAdapterManifest,
    event_mapper: EventMapper,
    stream_manager: StreamManager,
}

impl WsProtocolAdapter {
    pub fn new() -> Self {
        let manifest = ProtocolAdapterManifest::new(
            "protocol.adapter.websocket",
            ProtocolFamily::WebSocket,
            "1.0",
            ProtocolTransport::WebSocket,
            ProtocolAdapterAuthMode::None,
        )
        .with_exposed_capabilities(vec![
            "protocol.stream".to_string(),
            "protocol.map".to_string(),
            "protocol.handle_request".to_string(),
        ])
        .with_streaming_support(ProtocolAdapterStreamingSupport::Ordered)
        .with_trace_support(true);

        Self {
            manifest,
            event_mapper: EventMapper::new(ProtocolFamily::WebSocket, ProtocolTransport::WebSocket),
            stream_manager: StreamManager::new(),
        }
    }

    /// Create a new WebSocket connection
    pub fn create_connection(
        &self,
        connection_id: impl Into<String>,
        session_id: Option<String>,
    ) -> KernelResult<()> {
        self.stream_manager
            .connect(connection_id, StreamType::WebSocket, session_id)
    }

    /// Start streaming for a connection
    pub fn start_stream(&self, connection_id: &str) -> KernelResult<()> {
        self.stream_manager.start_stream(connection_id)
    }

    /// Push a kernel event to a connection's stream
    pub fn push_event(&self, connection_id: &str, event: &KernelEvent) -> KernelResult<()> {
        let update = self.event_mapper.map_event(event, 0);
        self.stream_manager.push_update(connection_id, update)
    }

    /// Get the next WebSocket message for a connection
    pub fn next_message(&self, connection_id: &str) -> KernelResult<Option<String>> {
        let update = self.stream_manager.pop_update(connection_id)?;
        Ok(update.map(|update| self.event_mapper.to_ws_message(&update)))
    }

    /// Get all pending messages as a JSON array
    pub fn drain_messages(&self, connection_id: &str) -> KernelResult<String> {
        let updates = self.stream_manager.drain_updates(connection_id)?;
        let messages: Vec<serde_json::Value> = updates
            .iter()
            .map(|u| {
                serde_json::json!({
                    "event_id": u.event_id,
                    "event_type": u.event_type,
                    "event_version": u.event_version,
                    "sequence": u.sequence,
                    "payload": u.payload,
                })
            })
            .collect();

        Ok(serde_json::to_string(&messages).unwrap_or_else(|_| "[]".to_string()))
    }

    /// Handle an incoming WebSocket message
    pub fn handle_message(
        &self,
        connection_id: &str,
        message: &str,
    ) -> KernelResult<Option<String>> {
        // Parse the incoming message
        let parsed: serde_json::Value = serde_json::from_str(message).map_err(|e| {
            sdkwork_agent_kernel::KernelError::validation(format!("invalid JSON: {}", e))
        })?;

        let msg_type = parsed
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        match msg_type {
            "ping" => Ok(Some(serde_json::json!({"type": "pong"}).to_string())),
            "subscribe" => {
                let session_id = parsed.get("session_id").and_then(|v| v.as_str());
                self.stream_manager.connect(
                    format!("{}.sub", connection_id),
                    StreamType::WebSocket,
                    session_id.map(|s| s.to_string()),
                )?;
                Ok(Some(serde_json::json!({"type": "subscribed"}).to_string()))
            }
            "unsubscribe" => {
                self.stream_manager
                    .disconnect(&format!("{}.sub", connection_id))?;
                Ok(Some(
                    serde_json::json!({"type": "unsubscribed"}).to_string(),
                ))
            }
            _ => Ok(Some(
                serde_json::json!({"type": "error", "message": "unknown message type"}).to_string(),
            )),
        }
    }

    /// Disconnect a connection
    pub fn disconnect(&self, connection_id: &str) -> KernelResult<()> {
        self.stream_manager.disconnect(connection_id)
    }

    /// Get the number of active connections
    pub fn connection_count(&self) -> usize {
        self.stream_manager.connection_count()
    }
}

impl Default for WsProtocolAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolAdapter for WsProtocolAdapter {
    fn manifest(&self) -> ProtocolAdapterManifest {
        self.manifest.clone()
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn handle_request(
        &self,
        _runtime: &sdkwork_agent_kernel::AgentRuntime,
        request: ProtocolAdapterRequest,
    ) -> KernelResult<ProtocolObjectEnvelope> {
        // Map the request to a task
        let task = self.map_request_to_task(request.clone())?;

        // Create a response envelope
        Ok(ProtocolObjectEnvelope::new(
            ProtocolFamily::WebSocket,
            ProtocolObjectKind::AgentTask,
            &task.task_id,
            serde_json::json!({
                "task_id": task.task_id,
                "session_id": task.session_id,
                "instruction": task.instruction,
                "state": format!("{:?}", task.state).to_lowercase()
            })
            .to_string(),
        ))
    }

    fn map_request_to_task(&self, request: ProtocolAdapterRequest) -> KernelResult<AgentTask> {
        let operation = request.operation.clone();
        let session_id = request
            .metadata_value("sdkwork.agent.session_id")
            .unwrap_or("session.default");

        Ok(AgentTask::new(
            format!("task.ws.{}", request.protocol_request_id),
            session_id,
            operation,
        ))
    }

    fn map_event_to_stream_update(&self, event: KernelEvent) -> KernelResult<ProtocolStreamUpdate> {
        Ok(self.event_mapper.map_event(&event, 0))
    }

    fn map_response(&self, task: AgentTask) -> KernelResult<ProtocolAdapterResponse> {
        Ok(ProtocolAdapterResponse::accepted(
            format!("resp.ws.{}", task.task_id),
            &task.task_id,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_kernel::KernelEventSeverity;

    #[test]
    fn create_adapter() {
        let adapter = WsProtocolAdapter::new();
        assert_eq!(adapter.manifest().adapter_id, "protocol.adapter.websocket");
        assert_eq!(adapter.manifest().protocol, ProtocolFamily::WebSocket);
    }

    #[test]
    fn create_and_disconnect_connection() {
        let adapter = WsProtocolAdapter::new();
        adapter
            .create_connection("conn.1", Some("session.1".to_string()))
            .expect("created");
        assert_eq!(adapter.connection_count(), 1);

        adapter.disconnect("conn.1").expect("disconnected");
        assert_eq!(adapter.connection_count(), 0);
    }

    #[test]
    fn push_event_and_get_message() {
        let adapter = WsProtocolAdapter::new();
        adapter
            .create_connection("conn.1", Some("session.1".to_string()))
            .expect("created");

        let event = KernelEvent::new(
            "evt.1",
            "test.event",
            KernelEventSeverity::Info,
            "test payload",
        );
        adapter.push_event("conn.1", &event).expect("pushed");

        let msg = adapter.next_message("conn.1").expect("message");
        assert!(msg.is_some());
        let msg = msg.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&msg).expect("valid json");
        assert_eq!(parsed["event_id"], "evt.1");
    }

    #[test]
    fn next_message_preserves_buffered_updates() {
        let adapter = WsProtocolAdapter::new();
        adapter
            .create_connection("conn.1", Some("session.1".to_string()))
            .expect("created");

        let event1 = KernelEvent::new("evt.1", "test.event", KernelEventSeverity::Info, "payload1");
        let event2 = KernelEvent::new("evt.2", "test.event", KernelEventSeverity::Info, "payload2");
        adapter.push_event("conn.1", &event1).expect("pushed");
        adapter.push_event("conn.1", &event2).expect("pushed");

        let first = adapter.next_message("conn.1").expect("first message");
        let second = adapter.next_message("conn.1").expect("second message");
        assert!(first.is_some());
        assert!(second.is_some());
        let first_id: serde_json::Value = serde_json::from_str(&first.unwrap()).expect("valid json");
        let second_id: serde_json::Value =
            serde_json::from_str(&second.unwrap()).expect("valid json");
        assert_eq!(first_id["event_id"], "evt.1");
        assert_eq!(second_id["event_id"], "evt.2");
    }

    #[test]
    fn handle_ping_message() {
        let adapter = WsProtocolAdapter::new();
        let response = adapter
            .handle_message("conn.1", r#"{"type": "ping"}"#)
            .expect("handled");
        assert!(response.is_some());
        let response = response.unwrap();
        assert!(response.contains("pong"));
    }

    #[test]
    fn handle_unknown_message() {
        let adapter = WsProtocolAdapter::new();
        let response = adapter
            .handle_message("conn.1", r#"{"type": "unknown"}"#)
            .expect("handled");
        assert!(response.is_some());
        let response = response.unwrap();
        assert!(response.contains("error"));
    }

    #[test]
    fn map_request_to_task() {
        let adapter = WsProtocolAdapter::new();
        let request =
            ProtocolAdapterRequest::new("req.1", ProtocolFamily::WebSocket, "ws.chat", "{}")
                .with_metadata("sdkwork.agent.session_id", "session.1");

        let task = adapter.map_request_to_task(request).expect("mapped");
        assert_eq!(task.session_id, "session.1");
    }

    #[test]
    fn drain_messages_as_json_array() {
        let adapter = WsProtocolAdapter::new();
        adapter
            .create_connection("conn.1", Some("session.1".to_string()))
            .expect("created");

        let event1 = KernelEvent::new("evt.1", "test.event", KernelEventSeverity::Info, "payload1");
        let event2 = KernelEvent::new("evt.2", "test.event", KernelEventSeverity::Info, "payload2");
        adapter.push_event("conn.1", &event1).expect("pushed");
        adapter.push_event("conn.1", &event2).expect("pushed");

        let messages = adapter.drain_messages("conn.1").expect("drained");
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&messages).expect("valid json");
        assert_eq!(parsed.len(), 2);
    }
}
