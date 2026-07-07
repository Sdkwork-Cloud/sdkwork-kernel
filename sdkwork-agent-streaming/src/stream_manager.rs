use sdkwork_agent_kernel::{KernelError, KernelResult, ProtocolStreamUpdate};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

/// Maximum buffered stream chunks per connection before backpressure applies.
const MAX_STREAM_BUFFER_CHUNKS: usize = 1024;

/// Maximum concurrent stream connections per process before new connects are rejected.
const MAX_STREAM_CONNECTIONS: usize = 4096;

/// Manages streaming connections and event distribution
pub struct StreamManager {
    connections: Arc<Mutex<HashMap<String, StreamConnection>>>,
}

#[derive(Debug, Clone)]
pub struct StreamConnection {
    pub connection_id: String,
    pub session_id: Option<String>,
    pub connection_type: StreamType,
    pub state: StreamState,
    pub sequence: u64,
    pub buffer: VecDeque<ProtocolStreamUpdate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamType {
    Sse,
    WebSocket,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamState {
    Connected,
    Streaming,
    Paused,
    Disconnected,
}

impl StreamManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a new stream connection
    pub fn connect(
        &self,
        connection_id: impl Into<String>,
        connection_type: StreamType,
        session_id: Option<String>,
    ) -> KernelResult<()> {
        let mut connections = self
            .connections
            .lock()
            .map_err(|e| KernelError::validation(format!("failed to acquire lock: {}", e)))?;

        if connections.len() >= MAX_STREAM_CONNECTIONS {
            return Err(KernelError::validation(format!(
                "stream connection capacity exceeded ({MAX_STREAM_CONNECTIONS})"
            )));
        }

        let connection = StreamConnection {
            connection_id: connection_id.into(),
            session_id,
            connection_type,
            state: StreamState::Connected,
            sequence: 0,
            buffer: VecDeque::new(),
        };

        connections.insert(connection.connection_id.clone(), connection);
        Ok(())
    }

    /// Disconnect a stream connection
    pub fn disconnect(&self, connection_id: &str) -> KernelResult<()> {
        let mut connections = self
            .connections
            .lock()
            .map_err(|e| KernelError::validation(format!("failed to acquire lock: {}", e)))?;

        connections.remove(connection_id);
        Ok(())
    }

    /// Get connection state
    pub fn get_state(&self, connection_id: &str) -> Option<StreamState> {
        self.connections
            .lock()
            .ok()
            .and_then(|connections| connections.get(connection_id).map(|c| c.state))
    }

    /// Start streaming for a connection
    pub fn start_stream(&self, connection_id: &str) -> KernelResult<()> {
        let mut connections = self
            .connections
            .lock()
            .map_err(|e| KernelError::validation(format!("failed to acquire lock: {}", e)))?;

        if let Some(connection) = connections.get_mut(connection_id) {
            connection.state = StreamState::Streaming;
            Ok(())
        } else {
            Err(KernelError::validation(format!(
                "connection not found: {}",
                connection_id
            )))
        }
    }

    /// Pause streaming for a connection
    pub fn pause_stream(&self, connection_id: &str) -> KernelResult<()> {
        let mut connections = self
            .connections
            .lock()
            .map_err(|e| KernelError::validation(format!("failed to acquire lock: {}", e)))?;

        if let Some(connection) = connections.get_mut(connection_id) {
            connection.state = StreamState::Paused;
            Ok(())
        } else {
            Err(KernelError::validation(format!(
                "connection not found: {}",
                connection_id
            )))
        }
    }

    /// Push an update to a connection's buffer
    pub fn push_update(
        &self,
        connection_id: &str,
        update: ProtocolStreamUpdate,
    ) -> KernelResult<()> {
        let mut connections = self
            .connections
            .lock()
            .map_err(|e| KernelError::validation(format!("failed to acquire lock: {}", e)))?;

        if let Some(connection) = connections.get_mut(connection_id) {
            if connection.buffer.len() >= MAX_STREAM_BUFFER_CHUNKS {
                return Err(KernelError::validation(format!(
                    "stream buffer exceeded max chunks ({MAX_STREAM_BUFFER_CHUNKS}) for connection {connection_id}"
                )));
            }
            connection.sequence += 1;
            connection.buffer.push_back(update);
            Ok(())
        } else {
            Err(KernelError::validation(format!(
                "connection not found: {}",
                connection_id
            )))
        }
    }

    /// Pop the next buffered update for a connection without discarding later items.
    pub fn pop_update(&self, connection_id: &str) -> KernelResult<Option<ProtocolStreamUpdate>> {
        let mut connections = self
            .connections
            .lock()
            .map_err(|e| KernelError::validation(format!("failed to acquire lock: {}", e)))?;

        if let Some(connection) = connections.get_mut(connection_id) {
            if connection.buffer.is_empty() {
                return Ok(None);
            }
            Ok(Some(
                connection.buffer.pop_front().expect("buffer non-empty"),
            ))
        } else {
            Err(KernelError::validation(format!(
                "connection not found: {}",
                connection_id
            )))
        }
    }

    /// Drain buffered updates for a connection
    pub fn drain_updates(&self, connection_id: &str) -> KernelResult<Vec<ProtocolStreamUpdate>> {
        let mut connections = self
            .connections
            .lock()
            .map_err(|e| KernelError::validation(format!("failed to acquire lock: {}", e)))?;

        if let Some(connection) = connections.get_mut(connection_id) {
            Ok(connection.buffer.drain(..).collect())
        } else {
            Err(KernelError::validation(format!(
                "connection not found: {}",
                connection_id
            )))
        }
    }

    /// Get the number of active connections
    pub fn connection_count(&self) -> usize {
        self.connections.lock().map(|c| c.len()).unwrap_or(0)
    }

    /// Get the number of active connections for a session
    pub fn session_connection_count(&self, session_id: &str) -> usize {
        self.connections
            .lock()
            .map(|c| {
                c.values()
                    .filter(|conn| conn.session_id.as_deref() == Some(session_id))
                    .count()
            })
            .unwrap_or(0)
    }
}

impl Default for StreamManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_and_disconnect() {
        let manager = StreamManager::new();
        manager
            .connect("conn.1", StreamType::Sse, None)
            .expect("connected");
        assert_eq!(manager.connection_count(), 1);

        manager.disconnect("conn.1").expect("disconnected");
        assert_eq!(manager.connection_count(), 0);
    }

    #[test]
    fn start_and_pause_stream() {
        let manager = StreamManager::new();
        manager
            .connect("conn.1", StreamType::Sse, None)
            .expect("connected");

        manager.start_stream("conn.1").expect("started");
        assert_eq!(manager.get_state("conn.1"), Some(StreamState::Streaming));

        manager.pause_stream("conn.1").expect("paused");
        assert_eq!(manager.get_state("conn.1"), Some(StreamState::Paused));
    }

    #[test]
    fn pop_update_preserves_remaining_items() {
        let manager = StreamManager::new();
        manager
            .connect("conn.1", StreamType::WebSocket, None)
            .expect("connected");

        for event_id in ["evt.1", "evt.2"] {
            manager
                .push_update(
                    "conn.1",
                    ProtocolStreamUpdate {
                        event_id: event_id.to_string(),
                        event_type: "test.event".to_string(),
                        event_version: "0.1.0".to_string(),
                        sequence: 0,
                        payload: event_id.to_string(),
                        trace_context: None,
                    },
                )
                .expect("pushed");
        }

        let first = manager.pop_update("conn.1").expect("first").expect("item");
        assert_eq!(first.event_id, "evt.1");
        let second = manager.pop_update("conn.1").expect("second").expect("item");
        assert_eq!(second.event_id, "evt.2");
        assert!(manager.pop_update("conn.1").expect("empty").is_none());
    }

    #[test]
    fn push_and_drain_updates() {
        let manager = StreamManager::new();
        manager
            .connect("conn.1", StreamType::Sse, None)
            .expect("connected");

        let update = ProtocolStreamUpdate {
            event_id: "evt.1".to_string(),
            event_type: "test.event".to_string(),
            event_version: "0.1.0".to_string(),
            sequence: 0,
            payload: "test".to_string(),
            trace_context: None,
        };

        manager.push_update("conn.1", update).expect("pushed");
        let updates = manager.drain_updates("conn.1").expect("drained");
        assert_eq!(updates.len(), 1);
    }

    #[test]
    fn session_connection_count() {
        let manager = StreamManager::new();
        manager
            .connect("conn.1", StreamType::Sse, Some("session.1".to_string()))
            .expect("connected");
        manager
            .connect("conn.2", StreamType::Sse, Some("session.1".to_string()))
            .expect("connected");
        manager
            .connect("conn.3", StreamType::Sse, Some("session.2".to_string()))
            .expect("connected");

        assert_eq!(manager.session_connection_count("session.1"), 2);
        assert_eq!(manager.session_connection_count("session.2"), 1);
    }

    #[test]
    fn connect_rejects_when_capacity_exceeded() {
        let manager = StreamManager::new();
        for index in 0..MAX_STREAM_CONNECTIONS {
            manager
                .connect(format!("conn.{index}"), StreamType::Sse, None)
                .expect("connected");
        }
        let error = manager
            .connect("conn.overflow", StreamType::Sse, None)
            .expect_err("capacity should be enforced");
        assert!(error.to_string().contains("capacity exceeded"));
    }
}
