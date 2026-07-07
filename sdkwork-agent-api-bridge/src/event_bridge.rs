use crate::types::BridgeEvent;
use std::collections::HashMap;

/// Maximum in-bridge events retained per session.
const MAX_SESSION_BRIDGE_EVENTS: usize = 512;
/// Maximum in-bridge events retained globally for transient snapshots.
const MAX_GLOBAL_BRIDGE_EVENTS: usize = 4096;

/// Manages event recording and retrieval
pub struct EventBridge {
    events: HashMap<String, Vec<BridgeEvent>>,
    global_events: Vec<BridgeEvent>,
}

impl EventBridge {
    pub fn new() -> Self {
        Self {
            events: HashMap::new(),
            global_events: Vec::new(),
        }
    }

    /// Record events
    pub fn record_events(&mut self, events: &[BridgeEvent]) {
        for event in events {
            if let Some(session_id) = &event.session_id {
                let session_events = self.events.entry(session_id.clone()).or_default();
                session_events.push(event.clone());
                Self::trim_events(session_events, MAX_SESSION_BRIDGE_EVENTS);
            }
            self.global_events.push(event.clone());
            Self::trim_events(&mut self.global_events, MAX_GLOBAL_BRIDGE_EVENTS);
        }
    }

    /// Get events for a session
    pub fn get_events(&self, session_id: &str) -> Vec<BridgeEvent> {
        self.events.get(session_id).cloned().unwrap_or_default()
    }

    /// Get all global events
    pub fn get_global_events(&self) -> Vec<BridgeEvent> {
        self.global_events.clone()
    }

    /// Clear events for a session
    pub fn clear_events(&mut self, session_id: &str) {
        self.events.remove(session_id);
    }

    /// Clear all events
    pub fn clear_all(&mut self) {
        self.events.clear();
        self.global_events.clear();
    }

    fn trim_events(events: &mut Vec<BridgeEvent>, max_events: usize) {
        if events.len() > max_events {
            let overflow = events.len() - max_events;
            events.drain(0..overflow);
        }
    }
}

impl Default for EventBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::BridgeEventSeverity;

    fn test_event(session_id: Option<&str>, index: usize) -> BridgeEvent {
        BridgeEvent {
            event_type: "test.event".to_string(),
            session_id: session_id.map(str::to_string),
            task_id: None,
            payload: format!("event.{index}"),
            severity: BridgeEventSeverity::Info,
        }
    }

    #[test]
    fn record_and_get_events() {
        let mut bridge = EventBridge::new();
        let events = vec![BridgeEvent {
            event_type: "test.event".to_string(),
            session_id: Some("session.1".to_string()),
            task_id: None,
            payload: "test".to_string(),
            severity: BridgeEventSeverity::Info,
        }];

        bridge.record_events(&events);
        let retrieved = bridge.get_events("session.1");
        assert_eq!(retrieved.len(), 1);
    }

    #[test]
    fn record_events_bounds_session_event_history() {
        let mut bridge = EventBridge::new();

        for index in 0..600 {
            bridge.record_events(&[test_event(Some("session.1"), index)]);
        }

        let retrieved = bridge.get_events("session.1");
        assert_eq!(
            retrieved.len(),
            512,
            "session bridge event history must be bounded"
        );
        assert_eq!(
            retrieved.first().map(|event| event.payload.as_str()),
            Some("event.88"),
            "oldest session bridge events should be evicted first"
        );
    }

    #[test]
    fn record_events_bounds_global_event_history() {
        let mut bridge = EventBridge::new();

        for index in 0..4100 {
            let session_id = format!("session.{index}");
            bridge.record_events(&[test_event(Some(&session_id), index)]);
        }

        let retrieved = bridge.get_global_events();
        assert_eq!(
            retrieved.len(),
            4096,
            "global bridge event history must be bounded"
        );
        assert_eq!(
            retrieved.first().map(|event| event.payload.as_str()),
            Some("event.4"),
            "oldest global bridge events should be evicted first"
        );
    }

    #[test]
    fn clear_events() {
        let mut bridge = EventBridge::new();
        let events = vec![BridgeEvent {
            event_type: "test.event".to_string(),
            session_id: Some("session.1".to_string()),
            task_id: None,
            payload: "test".to_string(),
            severity: BridgeEventSeverity::Info,
        }];

        bridge.record_events(&events);
        bridge.clear_events("session.1");
        let retrieved = bridge.get_events("session.1");
        assert!(retrieved.is_empty());
    }
}
