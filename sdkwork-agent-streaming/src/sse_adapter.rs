use crate::{EventMapper, StreamManager, StreamType};
use sdkwork_agent_kernel::{
    AgentTask, KernelEvent, KernelResult, ProtocolAdapter, ProtocolAdapterAuthMode,
    ProtocolAdapterManifest, ProtocolAdapterResponse, ProtocolAdapterStreamingSupport,
    ProtocolFamily, ProtocolStreamUpdate, ProtocolTransport, ProviderHealth,
};

/// SSE (Server-Sent Events) protocol adapter for streaming kernel events
pub struct SseProtocolAdapter {
    manifest: ProtocolAdapterManifest,
    event_mapper: EventMapper,
    stream_manager: StreamManager,
}

impl SseProtocolAdapter {
    pub fn new() -> Self {
        let manifest = ProtocolAdapterManifest::new(
            "protocol.adapter.sse",
            ProtocolFamily::Http,
            "1.0",
            ProtocolTransport::Http,
            ProtocolAdapterAuthMode::None,
        )
        .with_exposed_capabilities(vec![
            "protocol.stream".to_string(),
            "protocol.map".to_string(),
        ])
        .with_streaming_support(ProtocolAdapterStreamingSupport::Ordered)
        .with_trace_support(true);

        Self {
            manifest,
            event_mapper: EventMapper::new(ProtocolFamily::Http, ProtocolTransport::Http),
            stream_manager: StreamManager::new(),
        }
    }

    /// Create a new SSE connection for a session
    pub fn create_connection(
        &self,
        connection_id: impl Into<String>,
        session_id: Option<String>,
    ) -> KernelResult<()> {
        self.stream_manager
            .connect(connection_id, StreamType::Sse, session_id)
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

    /// Get the next SSE frame for a connection
    pub fn next_frame(&self, connection_id: &str) -> KernelResult<Option<String>> {
        let updates = self.stream_manager.drain_updates(connection_id)?;
        if updates.is_empty() {
            return Ok(None);
        }

        let mut frames = String::new();
        for update in &updates {
            frames.push_str(&self.event_mapper.to_sse_frame(update));
        }

        Ok(Some(frames))
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

impl Default for SseProtocolAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolAdapter for SseProtocolAdapter {
    fn manifest(&self) -> ProtocolAdapterManifest {
        self.manifest.clone()
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn map_request_to_task(
        &self,
        request: sdkwork_agent_kernel::ProtocolAdapterRequest,
    ) -> KernelResult<AgentTask> {
        Ok(AgentTask::new(
            format!("task.sse.{}", request.protocol_request_id),
            request
                .metadata_value("sdkwork.agent.session_id")
                .unwrap_or("session.default"),
            "SSE stream request",
        ))
    }

    fn map_event_to_stream_update(&self, event: KernelEvent) -> KernelResult<ProtocolStreamUpdate> {
        Ok(self.event_mapper.map_event(&event, 0))
    }

    fn map_response(&self, task: AgentTask) -> KernelResult<ProtocolAdapterResponse> {
        Ok(ProtocolAdapterResponse::accepted(
            format!("resp.sse.{}", task.task_id),
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
        let adapter = SseProtocolAdapter::new();
        assert_eq!(adapter.manifest().adapter_id, "protocol.adapter.sse");
        assert_eq!(adapter.manifest().protocol, ProtocolFamily::Http);
    }

    #[test]
    fn create_and_disconnect_connection() {
        let adapter = SseProtocolAdapter::new();
        adapter
            .create_connection("conn.1", Some("session.1".to_string()))
            .expect("created");
        assert_eq!(adapter.connection_count(), 1);

        adapter.disconnect("conn.1").expect("disconnected");
        assert_eq!(adapter.connection_count(), 0);
    }

    #[test]
    fn push_event_and_get_frame() {
        let adapter = SseProtocolAdapter::new();
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

        let frame = adapter.next_frame("conn.1").expect("frame");
        assert!(frame.is_some());
        let frame = frame.unwrap();
        assert!(frame.contains("id: evt.1"));
        assert!(frame.contains("event: test.event"));
    }

    #[test]
    fn map_request_to_task() {
        let adapter = SseProtocolAdapter::new();
        let request = sdkwork_agent_kernel::ProtocolAdapterRequest::new(
            "req.1",
            sdkwork_agent_kernel::ProtocolFamily::Http,
            "sse.stream",
            "{}",
        )
        .with_metadata("sdkwork.agent.session_id", "session.1");

        let task = adapter.map_request_to_task(request).expect("mapped");
        assert_eq!(task.session_id, "session.1");
    }

    #[test]
    fn map_event_to_stream_update() {
        let adapter = SseProtocolAdapter::new();
        let event = KernelEvent::new("evt.1", "test.event", KernelEventSeverity::Info, "payload");

        let update = adapter.map_event_to_stream_update(event).expect("mapped");
        assert_eq!(update.event_id, "evt.1");
        assert_eq!(update.event_type, "test.event");
    }
}
