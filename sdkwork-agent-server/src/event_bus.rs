use sdkwork_agent_database::EventRow;
use tokio::sync::broadcast;

const EVENT_BUS_CAPACITY: usize = 1024;

/// In-process broadcast bus for persisted session events.
#[derive(Clone, Debug)]
pub struct SessionEventBus {
    sender: broadcast::Sender<EventRow>,
}

impl SessionEventBus {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(EVENT_BUS_CAPACITY);
        Self { sender }
    }

    pub fn publish(&self, event: EventRow) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<EventRow> {
        self.sender.subscribe()
    }
}

impl Default for SessionEventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_delivers_to_subscriber() {
        let bus = SessionEventBus::new();
        let mut receiver = bus.subscribe();
        let event = EventRow {
            event_id: "evt.1".to_string(),
            session_id: Some("session.1".to_string()),
            event_type: "session.created".to_string(),
            severity: "info".to_string(),
            payload: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        };
        bus.publish(event.clone());
        let received = receiver.try_recv().expect("event should be delivered");
        assert_eq!(received.event_id, event.event_id);
    }
}
