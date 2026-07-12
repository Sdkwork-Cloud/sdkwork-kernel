use sdkwork_agent_kernel::{KernelError, KernelResult, ProtocolStreamUpdate};
use std::collections::{HashMap, VecDeque};
use std::mem::size_of;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

/// Maximum buffered stream chunks per connection before backpressure applies.
const MAX_STREAM_BUFFER_CHUNKS: usize = 1024;

/// Maximum concurrent stream connections sharing the process stream budget.
const MAX_STREAM_CONNECTIONS: usize = 4096;

/// Maximum retained bytes for connection and optional session identifiers.
const MAX_STREAM_IDENTIFIER_BYTES: usize = 256;

/// Maximum bytes retained by one stream update, including its owned metadata.
const MAX_STREAM_UPDATE_BYTES: usize = 256 * 1024;

/// Maximum bytes retained by one connection's queued updates.
const MAX_STREAM_BUFFER_BYTES_PER_CONNECTION: usize = 4 * 1024 * 1024;

/// Maximum estimated bytes retained by all queues sharing the process stream budget.
const MAX_STREAM_BUFFER_BYTES_TOTAL: usize = 64 * 1024 * 1024;

#[derive(Debug)]
struct StreamResourceBudget {
    max_connections: usize,
    max_buffered_bytes: usize,
    connections: AtomicUsize,
    buffered_bytes: AtomicUsize,
}

impl StreamResourceBudget {
    fn new(max_connections: usize, max_buffered_bytes: usize) -> Self {
        Self {
            max_connections,
            max_buffered_bytes,
            connections: AtomicUsize::new(0),
            buffered_bytes: AtomicUsize::new(0),
        }
    }

    fn try_reserve_connection(&self) -> bool {
        self.connections
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                (current < self.max_connections).then_some(current + 1)
            })
            .is_ok()
    }

    fn release_connections(&self, amount: usize) {
        release_atomic(&self.connections, amount);
    }

    fn try_reserve_bytes(&self, amount: usize) -> bool {
        self.buffered_bytes
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                current
                    .checked_add(amount)
                    .filter(|next| *next <= self.max_buffered_bytes)
            })
            .is_ok()
    }

    fn release_bytes(&self, amount: usize) {
        release_atomic(&self.buffered_bytes, amount);
    }
}

fn process_stream_budget() -> Arc<StreamResourceBudget> {
    static PROCESS_STREAM_BUDGET: OnceLock<Arc<StreamResourceBudget>> = OnceLock::new();
    PROCESS_STREAM_BUDGET
        .get_or_init(|| {
            Arc::new(StreamResourceBudget::new(
                MAX_STREAM_CONNECTIONS,
                MAX_STREAM_BUFFER_BYTES_TOTAL,
            ))
        })
        .clone()
}

fn release_atomic(counter: &AtomicUsize, amount: usize) {
    let _ = counter.fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
        Some(current.saturating_sub(amount))
    });
}

#[derive(Default)]
struct StreamManagerState {
    connections: HashMap<String, BufferedStreamConnection>,
    total_buffered_bytes: usize,
}

struct BufferedStreamConnection {
    connection: StreamConnection,
    buffered_bytes: usize,
}

/// Manages streaming connections and event distribution
pub struct StreamManager {
    state: Arc<Mutex<StreamManagerState>>,
    resource_budget: Arc<StreamResourceBudget>,
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
        Self::with_resource_budget(process_stream_budget())
    }

    fn with_resource_budget(resource_budget: Arc<StreamResourceBudget>) -> Self {
        Self {
            state: Arc::new(Mutex::new(StreamManagerState::default())),
            resource_budget,
        }
    }

    /// Register a new stream connection
    pub fn connect(
        &self,
        connection_id: impl Into<String>,
        connection_type: StreamType,
        session_id: Option<String>,
    ) -> KernelResult<()> {
        let connection_id = connection_id.into();
        if connection_id.is_empty() || connection_id.len() > MAX_STREAM_IDENTIFIER_BYTES {
            return Err(KernelError::validation(format!(
                "stream connection id must contain between 1 and {MAX_STREAM_IDENTIFIER_BYTES} bytes"
            )));
        }
        if session_id
            .as_deref()
            .is_some_and(|value| value.is_empty() || value.len() > MAX_STREAM_IDENTIFIER_BYTES)
        {
            return Err(KernelError::validation(format!(
                "stream session id must contain between 1 and {MAX_STREAM_IDENTIFIER_BYTES} bytes"
            )));
        }

        let mut state = self
            .state
            .lock()
            .map_err(|e| KernelError::validation(format!("failed to acquire lock: {e}")))?;
        if state.connections.contains_key(&connection_id) {
            return Err(KernelError::validation(
                "stream connection id is already active",
            ));
        }
        if !self.resource_budget.try_reserve_connection() {
            return Err(KernelError::resource_exhausted(format!(
                "stream connection capacity exceeded ({MAX_STREAM_CONNECTIONS})"
            )));
        }

        let connection = StreamConnection {
            connection_id: connection_id.clone(),
            session_id,
            connection_type,
            state: StreamState::Connected,
            sequence: 0,
            buffer: VecDeque::new(),
        };

        state.connections.insert(
            connection.connection_id.clone(),
            BufferedStreamConnection {
                connection,
                buffered_bytes: 0,
            },
        );
        Ok(())
    }

    /// Disconnect a stream connection
    pub fn disconnect(&self, connection_id: &str) -> KernelResult<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|e| KernelError::validation(format!("failed to acquire lock: {}", e)))?;

        if let Some(removed) = state.connections.remove(connection_id) {
            state.total_buffered_bytes = state
                .total_buffered_bytes
                .saturating_sub(removed.buffered_bytes);
            self.resource_budget.release_bytes(removed.buffered_bytes);
            self.resource_budget.release_connections(1);
        }
        Ok(())
    }

    /// Get connection state
    pub fn get_state(&self, connection_id: &str) -> Option<StreamState> {
        self.state.lock().ok().and_then(|state| {
            state
                .connections
                .get(connection_id)
                .map(|entry| entry.connection.state)
        })
    }

    /// Start streaming for a connection
    pub fn start_stream(&self, connection_id: &str) -> KernelResult<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|e| KernelError::validation(format!("failed to acquire lock: {}", e)))?;

        if let Some(entry) = state.connections.get_mut(connection_id) {
            entry.connection.state = StreamState::Streaming;
            Ok(())
        } else {
            Err(KernelError::validation(format!(
                "stream connection not found"
            )))
        }
    }

    /// Pause streaming for a connection
    pub fn pause_stream(&self, connection_id: &str) -> KernelResult<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|e| KernelError::validation(format!("failed to acquire lock: {}", e)))?;

        if let Some(entry) = state.connections.get_mut(connection_id) {
            entry.connection.state = StreamState::Paused;
            Ok(())
        } else {
            Err(KernelError::validation(format!(
                "stream connection not found"
            )))
        }
    }

    /// Push an update to a connection's buffer
    pub fn push_update(
        &self,
        connection_id: &str,
        mut update: ProtocolStreamUpdate,
    ) -> KernelResult<()> {
        let update_bytes = stream_update_bytes(&update);
        if update_bytes > MAX_STREAM_UPDATE_BYTES {
            return Err(KernelError::resource_exhausted(format!(
                "stream update exceeds max bytes ({MAX_STREAM_UPDATE_BYTES})"
            )));
        }

        let mut state = self
            .state
            .lock()
            .map_err(|e| KernelError::validation(format!("failed to acquire lock: {}", e)))?;

        let current_bytes = match state.connections.get(connection_id) {
            Some(entry) => {
                if entry.connection.buffer.len() >= MAX_STREAM_BUFFER_CHUNKS {
                    return Err(KernelError::resource_exhausted(format!(
                        "stream buffer exceeded max chunks ({MAX_STREAM_BUFFER_CHUNKS}) for connection {connection_id}"
                    )));
                }
                entry.buffered_bytes
            }
            None => {
                return Err(KernelError::validation(format!(
                    "stream connection not found"
                )));
            }
        };

        if current_bytes.saturating_add(update_bytes) > MAX_STREAM_BUFFER_BYTES_PER_CONNECTION {
            return Err(KernelError::resource_exhausted(format!(
                "stream buffer exceeded max bytes ({MAX_STREAM_BUFFER_BYTES_PER_CONNECTION})"
            )));
        }
        if !self.resource_budget.try_reserve_bytes(update_bytes) {
            return Err(KernelError::resource_exhausted(format!(
                "stream buffers exceeded process max bytes ({MAX_STREAM_BUFFER_BYTES_TOTAL})"
            )));
        }

        {
            let entry = state
                .connections
                .get_mut(connection_id)
                .expect("connection existence checked above");
            let next_sequence = match entry.connection.sequence.checked_add(1) {
                Some(sequence) => sequence,
                None => {
                    self.resource_budget.release_bytes(update_bytes);
                    return Err(KernelError::resource_exhausted(
                        "stream sequence number exhausted",
                    ));
                }
            };
            entry.connection.sequence = next_sequence;
            update.sequence = next_sequence;
            entry.connection.buffer.push_back(update);
            entry.buffered_bytes = entry.buffered_bytes.saturating_add(update_bytes);
        }
        state.total_buffered_bytes = state.total_buffered_bytes.saturating_add(update_bytes);
        Ok(())
    }

    /// Pop the next buffered update for a connection without discarding later items.
    pub fn pop_update(&self, connection_id: &str) -> KernelResult<Option<ProtocolStreamUpdate>> {
        let mut state = self
            .state
            .lock()
            .map_err(|e| KernelError::validation(format!("failed to acquire lock: {}", e)))?;

        let update = match state.connections.get_mut(connection_id) {
            Some(entry) => {
                let update = entry.connection.buffer.pop_front();
                if entry.connection.buffer.is_empty() {
                    // VecDeque retains its allocation after the last pop;
                    // replace it so disconnected/idle streams do not keep a
                    // peak-sized queue allocation alive indefinitely.
                    entry.connection.buffer = VecDeque::new();
                }
                if let Some(update) = &update {
                    entry.buffered_bytes = entry
                        .buffered_bytes
                        .saturating_sub(stream_update_bytes(update));
                }
                update
            }
            None => {
                return Err(KernelError::validation(format!(
                    "stream connection not found"
                )));
            }
        };

        if let Some(update) = update {
            let update_bytes = stream_update_bytes(&update);
            state.total_buffered_bytes = state.total_buffered_bytes.saturating_sub(update_bytes);
            self.resource_budget.release_bytes(update_bytes);
            Ok(Some(update))
        } else {
            Ok(None)
        }
    }

    /// Drain buffered updates for a connection
    pub fn drain_updates(&self, connection_id: &str) -> KernelResult<Vec<ProtocolStreamUpdate>> {
        let mut state = self
            .state
            .lock()
            .map_err(|e| KernelError::validation(format!("failed to acquire lock: {}", e)))?;

        let (updates, buffered_bytes) = match state.connections.get_mut(connection_id) {
            Some(entry) => {
                // Taking the queue, rather than draining in place, releases
                // its backing allocation as soon as ownership moves to the
                // caller.
                let updates = std::mem::take(&mut entry.connection.buffer)
                    .into_iter()
                    .collect::<Vec<_>>();
                let buffered_bytes = entry.buffered_bytes;
                entry.buffered_bytes = 0;
                (updates, buffered_bytes)
            }
            None => {
                return Err(KernelError::validation(format!(
                    "stream connection not found"
                )));
            }
        };

        state.total_buffered_bytes = state.total_buffered_bytes.saturating_sub(buffered_bytes);
        self.resource_budget.release_bytes(buffered_bytes);
        Ok(updates)
    }

    /// Return the bytes currently buffered for one connection.
    pub fn connection_buffered_bytes(&self, connection_id: &str) -> Option<usize> {
        self.state.lock().ok().and_then(|state| {
            state
                .connections
                .get(connection_id)
                .map(|entry| entry.buffered_bytes)
        })
    }

    /// Return the bytes currently buffered across all connections.
    pub fn total_buffered_bytes(&self) -> usize {
        self.state
            .lock()
            .map(|state| state.total_buffered_bytes)
            .unwrap_or(0)
    }

    /// Get the number of active connections
    pub fn connection_count(&self) -> usize {
        self.state
            .lock()
            .map(|state| state.connections.len())
            .unwrap_or(0)
    }

    /// Get the number of active connections for a session
    pub fn session_connection_count(&self, session_id: &str) -> usize {
        self.state
            .lock()
            .map(|c| {
                c.connections
                    .values()
                    .filter(|entry| entry.connection.session_id.as_deref() == Some(session_id))
                    .count()
            })
            .unwrap_or(0)
    }
}

impl Drop for StreamManager {
    fn drop(&mut self) {
        let state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.resource_budget
            .release_connections(state.connections.len());
        self.resource_budget
            .release_bytes(state.total_buffered_bytes);
    }
}

/// Estimate the owned allocation represented by one queued update.
///
/// Counting capacities (rather than only string lengths) prevents callers
/// from reserving an oversized `String` and bypassing the byte budgets with a
/// short visible value. The fixed struct size covers the queue element and
/// inline metadata; nested trace strings are accounted for separately.
fn stream_update_bytes(update: &ProtocolStreamUpdate) -> usize {
    let mut bytes = size_of::<ProtocolStreamUpdate>()
        .saturating_add(update.event_id.capacity())
        .saturating_add(update.event_type.capacity())
        .saturating_add(update.event_version.capacity())
        .saturating_add(update.payload.capacity());

    if let Some(trace_context) = &update.trace_context {
        bytes = bytes
            .saturating_add(trace_context.trace_id.capacity())
            .saturating_add(trace_context.span_id.capacity());
        if let Some(parent_span_id) = &trace_context.parent_span_id {
            bytes = bytes.saturating_add(parent_span_id.capacity());
        }
    }

    bytes
}

impl Default for StreamManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_manager() -> StreamManager {
        StreamManager::with_resource_budget(Arc::new(StreamResourceBudget::new(
            MAX_STREAM_CONNECTIONS,
            MAX_STREAM_BUFFER_BYTES_TOTAL,
        )))
    }

    fn update_with_payload(payload_bytes: usize) -> ProtocolStreamUpdate {
        ProtocolStreamUpdate {
            event_id: "evt.test".to_string(),
            event_type: "test.event".to_string(),
            event_version: "0.1.0".to_string(),
            sequence: 0,
            payload: "x".repeat(payload_bytes),
            trace_context: None,
        }
    }

    #[test]
    fn connect_and_disconnect() {
        let manager = test_manager();
        manager
            .connect("conn.1", StreamType::Sse, None)
            .expect("connected");
        assert_eq!(manager.connection_count(), 1);

        manager.disconnect("conn.1").expect("disconnected");
        assert_eq!(manager.connection_count(), 0);
    }

    #[test]
    fn connect_rejects_unbounded_identifiers_without_retaining_state() {
        let manager = test_manager();
        assert!(manager
            .connect("", StreamType::Sse, None)
            .expect_err("empty connection id must fail")
            .to_string()
            .contains("connection id"));
        assert!(manager
            .connect(
                "x".repeat(MAX_STREAM_IDENTIFIER_BYTES + 1),
                StreamType::Sse,
                None,
            )
            .expect_err("oversized connection id must fail")
            .to_string()
            .contains("connection id"));
        assert!(manager
            .connect(
                "conn.1",
                StreamType::Sse,
                Some("x".repeat(MAX_STREAM_IDENTIFIER_BYTES + 1)),
            )
            .expect_err("oversized session id must fail")
            .to_string()
            .contains("session id"));
        assert_eq!(manager.connection_count(), 0);
    }

    #[test]
    fn start_and_pause_stream() {
        let manager = test_manager();
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
        let manager = test_manager();
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
        assert_eq!(first.sequence, 1);
        let second = manager.pop_update("conn.1").expect("second").expect("item");
        assert_eq!(second.event_id, "evt.2");
        assert_eq!(second.sequence, 2);
        assert!(manager.pop_update("conn.1").expect("empty").is_none());
    }

    #[test]
    fn push_and_drain_updates() {
        let manager = test_manager();
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
        assert_eq!(manager.connection_buffered_bytes("conn.1"), Some(0));
        assert_eq!(manager.total_buffered_bytes(), 0);
    }

    #[test]
    fn rejects_an_oversized_update_before_buffering() {
        let manager = test_manager();
        manager
            .connect("conn.1", StreamType::Sse, None)
            .expect("connected");

        let error = manager
            .push_update("conn.1", update_with_payload(MAX_STREAM_UPDATE_BYTES))
            .expect_err("oversized update should be rejected");

        assert_eq!(
            error.kind(),
            sdkwork_agent_kernel::KernelErrorKind::ResourceExhausted
        );
        assert_eq!(manager.connection_buffered_bytes("conn.1"), Some(0));
        assert_eq!(manager.total_buffered_bytes(), 0);
    }

    #[test]
    fn enforces_per_connection_bytes_and_releases_on_pop() {
        let manager = test_manager();
        manager
            .connect("conn.1", StreamType::WebSocket, None)
            .expect("connected");

        let template = update_with_payload(64 * 1024);
        let update_bytes = stream_update_bytes(&template);
        let accepted = MAX_STREAM_BUFFER_BYTES_PER_CONNECTION / update_bytes;
        assert!(accepted < MAX_STREAM_BUFFER_CHUNKS);

        for _ in 0..accepted {
            manager
                .push_update("conn.1", template.clone())
                .expect("update fits the connection budget");
        }
        assert!(
            manager
                .connection_buffered_bytes("conn.1")
                .expect("connection exists")
                <= MAX_STREAM_BUFFER_BYTES_PER_CONNECTION
        );

        let error = manager
            .push_update("conn.1", template)
            .expect_err("connection byte budget should be enforced");
        assert_eq!(
            error.kind(),
            sdkwork_agent_kernel::KernelErrorKind::ResourceExhausted
        );

        let before = manager
            .connection_buffered_bytes("conn.1")
            .expect("connection exists");
        manager
            .pop_update("conn.1")
            .expect("pop succeeds")
            .expect("buffer is non-empty");
        assert_eq!(
            manager.connection_buffered_bytes("conn.1"),
            Some(before - update_bytes)
        );
        assert_eq!(manager.total_buffered_bytes(), before - update_bytes);
    }

    #[test]
    fn global_byte_capacity_rejects_and_drain_releases_budget() {
        let manager = test_manager();
        let template = update_with_payload(MAX_STREAM_UPDATE_BYTES - 512);
        let update_bytes = stream_update_bytes(&template);
        let updates_per_connection = MAX_STREAM_BUFFER_BYTES_PER_CONNECTION / update_bytes;
        assert!(updates_per_connection > 0);

        let mut filled_connection = None;
        let mut rejected = false;
        for connection_index in 0..MAX_STREAM_CONNECTIONS {
            let connection_id = format!("conn.global.{connection_index}");
            manager
                .connect(&connection_id, StreamType::Sse, None)
                .expect("connection fits the connection limit");

            for _ in 0..updates_per_connection {
                match manager.push_update(&connection_id, template.clone()) {
                    Ok(()) => {
                        filled_connection = Some(connection_id.clone());
                    }
                    Err(error) => {
                        assert_eq!(
                            error.kind(),
                            sdkwork_agent_kernel::KernelErrorKind::ResourceExhausted
                        );
                        rejected = true;
                        break;
                    }
                }
            }
            if rejected {
                break;
            }
        }

        assert!(rejected, "process-wide byte budget should be enforced");
        let before = manager.total_buffered_bytes();
        assert!(before <= MAX_STREAM_BUFFER_BYTES_TOTAL);

        let connection_id = filled_connection.expect("at least one connection was filled");
        let drained = manager
            .drain_updates(&connection_id)
            .expect("drain succeeds");
        assert!(!drained.is_empty());
        assert!(manager.total_buffered_bytes() < before);
        manager
            .push_update(&connection_id, template)
            .expect("drain should release process-wide budget");
    }

    #[test]
    fn disconnect_and_reconnect_release_buffered_bytes() {
        let manager = test_manager();
        manager
            .connect("conn.1", StreamType::Sse, None)
            .expect("connected");
        manager
            .push_update("conn.1", update_with_payload(1024))
            .expect("pushed");
        assert!(manager.total_buffered_bytes() > 0);

        manager.disconnect("conn.1").expect("disconnected");
        assert_eq!(manager.total_buffered_bytes(), 0);

        manager
            .connect("conn.1", StreamType::Sse, None)
            .expect("reconnected");
        assert_eq!(manager.connection_buffered_bytes("conn.1"), Some(0));
        manager
            .push_update("conn.1", update_with_payload(1024))
            .expect("reconnected stream can buffer updates");

        let duplicate_error = manager
            .connect("conn.1", StreamType::WebSocket, None)
            .expect_err("an active connection id must not be replaced");
        assert_eq!(
            duplicate_error.kind(),
            sdkwork_agent_kernel::KernelErrorKind::ValidationError
        );
        assert!(manager.total_buffered_bytes() > 0);
        manager.disconnect("conn.1").expect("disconnect again");
        assert_eq!(manager.total_buffered_bytes(), 0);
    }

    #[test]
    fn session_connection_count() {
        let manager = test_manager();
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
        let manager = test_manager();
        for index in 0..MAX_STREAM_CONNECTIONS {
            manager
                .connect(format!("conn.{index}"), StreamType::Sse, None)
                .expect("connected");
        }
        let error = manager
            .connect("conn.overflow", StreamType::Sse, None)
            .expect_err("capacity should be enforced");
        assert!(error.to_string().contains("capacity exceeded"));
        assert_eq!(
            error.kind(),
            sdkwork_agent_kernel::KernelErrorKind::ResourceExhausted
        );
    }

    #[test]
    fn shared_budget_is_enforced_across_manager_instances() {
        let template = update_with_payload(1024);
        let update_bytes = stream_update_bytes(&template);
        let budget = Arc::new(StreamResourceBudget::new(2, update_bytes));
        let first = StreamManager::with_resource_budget(budget.clone());
        let second = StreamManager::with_resource_budget(budget);
        first
            .connect("conn.first", StreamType::Sse, None)
            .expect("first connected");
        second
            .connect("conn.second", StreamType::Sse, None)
            .expect("second connected");
        first
            .push_update("conn.first", template.clone())
            .expect("first update consumes the shared byte budget");
        let error = second
            .push_update("conn.second", template.clone())
            .expect_err("the second manager must share the byte budget");
        assert_eq!(
            error.kind(),
            sdkwork_agent_kernel::KernelErrorKind::ResourceExhausted
        );
        first
            .pop_update("conn.first")
            .expect("pop succeeds")
            .expect("update exists");
        second
            .push_update("conn.second", template)
            .expect("released bytes are available to another manager");
    }
}
