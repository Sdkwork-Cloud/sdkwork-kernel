use sdkwork_agent_kernel::{
    KernelEvent, KernelEventSeverity, ProtocolAdapterStreamingSupport, ProtocolFamily,
    ProtocolSseEvent, ProtocolStreamUpdate, ProtocolTransport,
};

/// Maps kernel events to protocol-specific stream formats
pub struct EventMapper {
    protocol: ProtocolFamily,
    transport: ProtocolTransport,
}

impl EventMapper {
    pub fn new(protocol: ProtocolFamily, transport: ProtocolTransport) -> Self {
        Self {
            protocol,
            transport,
        }
    }

    /// Map a kernel event to a stream update
    pub fn map_event(&self, event: &KernelEvent, sequence: u64) -> ProtocolStreamUpdate {
        ProtocolStreamUpdate::from_event(event.clone(), sequence)
    }

    /// Convert a stream update to SSE frame
    pub fn to_sse_frame(&self, update: &ProtocolStreamUpdate) -> String {
        let sse_event = ProtocolSseEvent::from_stream_update(update);
        sse_event.to_frame()
    }

    /// Convert a stream update to WebSocket text message
    pub fn to_ws_message(&self, update: &ProtocolStreamUpdate) -> String {
        serde_json::json!({
            "event_id": update.event_id,
            "event_type": update.event_type,
            "event_version": update.event_version,
            "sequence": update.sequence,
            "payload": update.payload,
            "trace_id": update.trace_context.as_ref().map(|t| &t.trace_id),
            "span_id": update.trace_context.as_ref().map(|t| &t.span_id),
        })
        .to_string()
    }

    /// Get the streaming support level for this mapper
    pub fn streaming_support(&self) -> ProtocolAdapterStreamingSupport {
        match self.transport {
            ProtocolTransport::Http => ProtocolAdapterStreamingSupport::Ordered,
            ProtocolTransport::WebSocket => ProtocolAdapterStreamingSupport::Ordered,
            ProtocolTransport::InProcess => ProtocolAdapterStreamingSupport::Ordered,
            _ => ProtocolAdapterStreamingSupport::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_kernel::KernelEventSource;

    #[test]
    fn map_event_to_stream_update() {
        let mapper = EventMapper::new(ProtocolFamily::Http, ProtocolTransport::Http);
        let event = KernelEvent::new(
            "evt.1",
            "test.event",
            KernelEventSeverity::Info,
            "test payload",
        );

        let update = mapper.map_event(&event, 0);
        assert_eq!(update.event_id, "evt.1");
        assert_eq!(update.event_type, "test.event");
        assert_eq!(update.sequence, 0);
    }

    #[test]
    fn to_sse_frame_format() {
        let mapper = EventMapper::new(ProtocolFamily::Http, ProtocolTransport::Http);
        let event = KernelEvent::new(
            "evt.1",
            "test.event",
            KernelEventSeverity::Info,
            "test payload",
        );
        let update = mapper.map_event(&event, 0);

        let frame = mapper.to_sse_frame(&update);
        assert!(frame.contains("id: evt.1"));
        assert!(frame.contains("event: test.event"));
        assert!(frame.contains("data:"));
    }

    #[test]
    fn to_ws_message_is_json() {
        let mapper = EventMapper::new(ProtocolFamily::WebSocket, ProtocolTransport::WebSocket);
        let event = KernelEvent::new(
            "evt.1",
            "test.event",
            KernelEventSeverity::Info,
            "test payload",
        );
        let update = mapper.map_event(&event, 0);

        let msg = mapper.to_ws_message(&update);
        let parsed: serde_json::Value = serde_json::from_str(&msg).expect("valid json");
        assert_eq!(parsed["event_id"], "evt.1");
        assert_eq!(parsed["event_type"], "test.event");
    }

    #[test]
    fn streaming_support_by_transport() {
        let http = EventMapper::new(ProtocolFamily::Http, ProtocolTransport::Http);
        assert_eq!(
            http.streaming_support(),
            ProtocolAdapterStreamingSupport::Ordered
        );

        let ws = EventMapper::new(ProtocolFamily::WebSocket, ProtocolTransport::WebSocket);
        assert_eq!(
            ws.streaming_support(),
            ProtocolAdapterStreamingSupport::Ordered
        );

        let ipc = EventMapper::new(ProtocolFamily::Ipc, ProtocolTransport::Ipc);
        assert_eq!(
            ipc.streaming_support(),
            ProtocolAdapterStreamingSupport::None
        );
    }
}
