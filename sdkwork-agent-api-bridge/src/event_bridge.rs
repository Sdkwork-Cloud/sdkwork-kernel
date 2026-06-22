use crate::types::BridgeEvent;
use std::collections::HashMap;

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
                self.events
                    .entry(session_id.clone())
                    .or_default()
                    .push(event.clone());
            }
            self.global_events.push(event.clone());
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
