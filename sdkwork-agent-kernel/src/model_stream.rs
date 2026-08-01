//! Model Stream Provider for streaming model responses.
//!
//! This module provides streaming response support:
//! - Incremental response processing
//! - Cancellation propagation
//! - Backpressure control
//! - Multiple streaming protocols (SSE, WebSocket, gRPC)

use std::collections::HashMap;

/// Stream chunk for incremental response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamChunk {
    /// Chunk sequence number.
    pub sequence: u64,
    /// Chunk content (text, code, etc.).
    pub content: String,
    /// Chunk type.
    pub chunk_type: StreamChunkType,
    /// Chunk metadata.
    pub metadata: HashMap<String, String>,
    /// Is this the final chunk?
    pub is_final: bool,
}

impl StreamChunk {
    pub fn new(sequence: u64, content: impl Into<String>) -> Self {
        Self {
            sequence,
            content: content.into(),
            chunk_type: StreamChunkType::Text,
            metadata: HashMap::new(),
            is_final: false,
        }
    }

    pub fn with_type(mut self, chunk_type: StreamChunkType) -> Self {
        self.chunk_type = chunk_type;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn final_chunk(sequence: u64, content: impl Into<String>) -> Self {
        Self {
            sequence,
            content: content.into(),
            chunk_type: StreamChunkType::Final,
            metadata: HashMap::new(),
            is_final: true,
        }
    }
}

/// Stream chunk type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamChunkType {
    /// Text content.
    Text,
    /// Code block.
    Code,
    /// Thinking process.
    Thinking,
    /// Error message.
    Error,
    /// Control signal (pause, cancel).
    Control,
    /// Final chunk.
    Final,
}

impl StreamChunkType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Code => "code",
            Self::Thinking => "thinking",
            Self::Error => "error",
            Self::Control => "control",
            Self::Final => "final",
        }
    }
}

/// Stream control for managing stream flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamControl {
    /// Continue streaming.
    Continue,
    /// Pause streaming (backpressure).
    Pause,
    /// Cancel streaming.
    Cancel {
        /// Reason for cancellation.
        reason: String,
    },
    /// Resume streaming.
    Resume,
}

impl StreamControl {
    pub fn cancel(reason: impl Into<String>) -> Self {
        Self::Cancel {
            reason: reason.into(),
        }
    }
}

/// Stream status for monitoring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamStatus {
    /// Stream ID.
    pub stream_id: String,
    /// Current state.
    pub state: StreamState,
    /// Total chunks sent.
    pub chunks_sent: u64,
    /// Total bytes sent.
    pub bytes_sent: u64,
    /// Stream duration (ms).
    pub duration_ms: u64,
    /// Current throughput (bytes/sec).
    pub throughput_bps: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamState {
    /// Stream initializing.
    Initializing,
    /// Stream active.
    Active,
    /// Stream paused.
    Paused,
    /// Stream cancelled.
    Cancelled,
    /// Stream completed.
    Completed,
    /// Stream failed.
    Failed,
}

impl StreamState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Initializing => "initializing",
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Cancelled => "cancelled",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

/// Stream request for initiating streaming.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamRequest {
    /// Stream ID (unique).
    pub stream_id: String,
    /// Model provider ID.
    pub provider_id: String,
    /// Model ID.
    pub model_id: String,
    /// Prompt/input.
    pub prompt: String,
    /// Stream configuration.
    pub config: StreamConfig,
    /// Request metadata.
    pub metadata: HashMap<String, String>,
}

impl StreamRequest {
    pub fn new(
        stream_id: impl Into<String>,
        provider_id: impl Into<String>,
        model_id: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Self {
        Self {
            stream_id: stream_id.into(),
            provider_id: provider_id.into(),
            model_id: model_id.into(),
            prompt: prompt.into(),
            config: StreamConfig::default(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_config(mut self, config: StreamConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Stream configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamConfig {
    /// Chunk size (bytes).
    pub chunk_size: usize,
    /// Max chunks per second (rate limit).
    pub max_chunks_per_second: usize,
    /// Enable backpressure.
    pub enable_backpressure: bool,
    /// Buffer size for chunks.
    pub buffer_size: usize,
    /// Timeout (ms).
    pub timeout_ms: u64,
    /// Protocol to use.
    pub protocol: StreamProtocol,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            chunk_size: 1024, // 1KB chunks
            max_chunks_per_second: 100,
            enable_backpressure: true,
            buffer_size: 10,
            timeout_ms: 60000, // 60s
            protocol: StreamProtocol::Sse,
        }
    }
}

impl StreamConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }

    pub fn with_protocol(mut self, protocol: StreamProtocol) -> Self {
        self.protocol = protocol;
        self
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn with_backpressure(mut self, enable: bool) -> Self {
        self.enable_backpressure = enable;
        self
    }
}

/// Streaming protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamProtocol {
    /// Server-Sent Events (HTTP SSE).
    Sse,
    /// WebSocket.
    WebSocket,
    /// gRPC streaming.
    GrpcStream,
    /// Custom protocol.
    Custom,
}

impl StreamProtocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sse => "sse",
            Self::WebSocket => "websocket",
            Self::GrpcStream => "grpc_stream",
            Self::Custom => "custom",
        }
    }
}

/// Stream result for final response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamResult {
    /// Stream ID.
    pub stream_id: String,
    /// Final status.
    pub status: StreamStatus,
    /// Complete response (aggregated).
    pub complete_response: String,
    /// Total chunks received.
    pub total_chunks: u64,
    /// Total duration (ms).
    pub total_duration_ms: u64,
    /// Errors encountered.
    pub errors: Vec<String>,
}

impl StreamResult {
    pub fn success(
        stream_id: impl Into<String>,
        complete_response: impl Into<String>,
        total_chunks: u64,
        total_duration_ms: u64,
    ) -> Self {
        let stream_id = stream_id.into();
        let complete_response = complete_response.into();
        let bytes_sent = complete_response.len() as u64;
        let throughput_bps = bytes_sent
            .saturating_mul(1000)
            .checked_div(total_duration_ms)
            .unwrap_or(0);

        Self {
            stream_id: stream_id.clone(),
            status: StreamStatus {
                stream_id,
                state: StreamState::Completed,
                chunks_sent: total_chunks,
                bytes_sent,
                duration_ms: total_duration_ms,
                throughput_bps,
            },
            complete_response,
            total_chunks,
            total_duration_ms,
            errors: Vec::new(),
        }
    }

    pub fn failure(stream_id: impl Into<String>, errors: Vec<String>) -> Self {
        let stream_id = stream_id.into();
        Self {
            stream_id: stream_id.clone(),
            status: StreamStatus {
                stream_id,
                state: StreamState::Failed,
                chunks_sent: 0,
                bytes_sent: 0,
                duration_ms: 0,
                throughput_bps: 0,
            },
            complete_response: String::new(),
            total_chunks: 0,
            total_duration_ms: 0,
            errors,
        }
    }

    pub fn cancelled(stream_id: impl Into<String>, reason: String, chunks_received: u64) -> Self {
        let stream_id = stream_id.into();
        Self {
            stream_id: stream_id.clone(),
            status: StreamStatus {
                stream_id,
                state: StreamState::Cancelled,
                chunks_sent: chunks_received,
                bytes_sent: 0,
                duration_ms: 0,
                throughput_bps: 0,
            },
            complete_response: String::new(),
            total_chunks: chunks_received,
            total_duration_ms: 0,
            errors: vec![reason],
        }
    }
}

/// Model Stream Provider trait for streaming responses.
pub trait ModelStreamProvider: Send + Sync {
    /// Initiate a stream request.
    fn initiate_stream(&mut self, request: StreamRequest) -> Result<StreamStatus, StreamError>;

    /// Receive next chunk from stream.
    fn receive_chunk(&mut self, stream_id: &str) -> Result<StreamChunk, StreamError>;

    /// Send control signal to stream.
    fn control_stream(
        &mut self,
        stream_id: &str,
        control: StreamControl,
    ) -> Result<(), StreamError>;

    /// Get stream status.
    fn get_status(&self, stream_id: &str) -> Result<StreamStatus, StreamError>;

    /// Finalize stream and get complete result.
    fn finalize_stream(&mut self, stream_id: &str) -> Result<StreamResult, StreamError>;

    /// List active streams.
    fn list_active_streams(&self) -> Result<Vec<StreamStatus>, StreamError>;

    /// Cancel all streams for a provider.
    fn cancel_provider_streams(&mut self, provider_id: &str) -> Result<usize, StreamError>;

    /// Provider health check.
    fn health_check(&self) -> Result<StreamProviderHealth, StreamError>;

    /// Provider manifest.
    fn provider_manifest(&self) -> StreamProviderManifest;
}

/// Stream provider health status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamProviderHealth {
    /// Health status.
    pub status: StreamProviderStatus,
    /// Active streams count.
    pub active_streams: usize,
    /// Total streams processed.
    pub total_streams_processed: u64,
    /// Average throughput (bytes/sec).
    pub avg_throughput_bps: u64,
    /// Last health check time (ms).
    pub last_check_time: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamProviderStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

impl StreamProviderStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Unhealthy => "unhealthy",
        }
    }
}

/// Stream provider manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamProviderManifest {
    /// Provider ID.
    pub provider_id: String,
    /// Provider name.
    pub name: String,
    /// Provider version.
    pub version: String,
    /// Supported protocols.
    pub supported_protocols: Vec<StreamProtocol>,
    /// Max concurrent streams.
    pub max_concurrent_streams: usize,
    /// Max stream duration (ms).
    pub max_stream_duration_ms: u64,
    /// Supports backpressure.
    pub supports_backpressure: bool,
    /// Supports cancellation.
    pub supports_cancellation: bool,
}

/// Stream error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamError {
    /// Stream not found.
    StreamNotFound(String),
    /// Stream already exists.
    StreamAlreadyExists(String),
    /// Stream cancelled.
    StreamCancelled(String),
    /// Stream timeout.
    StreamTimeout(String),
    /// Provider unavailable.
    ProviderUnavailable(String),
    /// Backpressure triggered.
    BackpressureTriggered(String),
    /// Invalid request.
    InvalidRequest(String),
    /// Protocol error.
    ProtocolError(String),
}

impl std::fmt::Display for StreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StreamNotFound(id) => write!(f, "Stream not found: {}", id),
            Self::StreamAlreadyExists(id) => write!(f, "Stream already exists: {}", id),
            Self::StreamCancelled(id) => write!(f, "Stream cancelled: {}", id),
            Self::StreamTimeout(id) => write!(f, "Stream timeout: {}", id),
            Self::ProviderUnavailable(msg) => write!(f, "Provider unavailable: {}", msg),
            Self::BackpressureTriggered(id) => write!(f, "Backpressure triggered: {}", id),
            Self::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            Self::ProtocolError(msg) => write!(f, "Protocol error: {}", msg),
        }
    }
}

impl std::error::Error for StreamError {}

/// In-memory stream provider (for testing).
#[derive(Debug, Clone)]
pub struct InMemoryStreamProvider {
    streams: HashMap<String, (StreamStatus, Vec<StreamChunk>)>,
    max_concurrent: usize,
}

impl InMemoryStreamProvider {
    pub fn new() -> Self {
        Self {
            streams: HashMap::new(),
            max_concurrent: 10,
        }
    }

    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = max;
        self
    }
}

impl Default for InMemoryStreamProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelStreamProvider for InMemoryStreamProvider {
    fn initiate_stream(&mut self, request: StreamRequest) -> Result<StreamStatus, StreamError> {
        if self.streams.len() >= self.max_concurrent {
            return Err(StreamError::ProviderUnavailable(
                "Max concurrent streams reached".to_string(),
            ));
        }

        if self.streams.contains_key(&request.stream_id) {
            return Err(StreamError::StreamAlreadyExists(request.stream_id));
        }

        let status = StreamStatus {
            stream_id: request.stream_id.clone(),
            state: StreamState::Initializing,
            chunks_sent: 0,
            bytes_sent: 0,
            duration_ms: 0,
            throughput_bps: 0,
        };

        self.streams
            .insert(request.stream_id.clone(), (status.clone(), Vec::new()));
        Ok(status)
    }

    fn receive_chunk(&mut self, stream_id: &str) -> Result<StreamChunk, StreamError> {
        let (status, chunks) = self
            .streams
            .get(stream_id)
            .ok_or_else(|| StreamError::StreamNotFound(stream_id.to_string()))?;

        if status.state == StreamState::Cancelled {
            return Err(StreamError::StreamCancelled(stream_id.to_string()));
        }

        // Return next chunk (simulated)
        if chunks.is_empty() {
            Ok(StreamChunk::final_chunk(0, "Complete"))
        } else {
            Ok(chunks[0].clone())
        }
    }

    fn control_stream(
        &mut self,
        stream_id: &str,
        control: StreamControl,
    ) -> Result<(), StreamError> {
        let (status, _) = self
            .streams
            .get_mut(stream_id)
            .ok_or_else(|| StreamError::StreamNotFound(stream_id.to_string()))?;

        match control {
            StreamControl::Cancel { reason: _ } => {
                status.state = StreamState::Cancelled;
                Ok(())
            }
            StreamControl::Pause => {
                status.state = StreamState::Paused;
                Ok(())
            }
            StreamControl::Resume => {
                status.state = StreamState::Active;
                Ok(())
            }
            StreamControl::Continue => Ok(()),
        }
    }

    fn get_status(&self, stream_id: &str) -> Result<StreamStatus, StreamError> {
        self.streams
            .get(stream_id)
            .map(|(status, _)| status.clone())
            .ok_or_else(|| StreamError::StreamNotFound(stream_id.to_string()))
    }

    fn finalize_stream(&mut self, stream_id: &str) -> Result<StreamResult, StreamError> {
        let (status, chunks) = self
            .streams
            .remove(stream_id)
            .ok_or_else(|| StreamError::StreamNotFound(stream_id.to_string()))?;

        let complete_response = chunks
            .iter()
            .map(|c| c.content.clone())
            .collect::<Vec<_>>()
            .join("");

        Ok(StreamResult::success(
            stream_id,
            complete_response,
            chunks.len() as u64,
            status.duration_ms,
        ))
    }

    fn list_active_streams(&self) -> Result<Vec<StreamStatus>, StreamError> {
        Ok(self
            .streams
            .values()
            .map(|(status, _)| status.clone())
            .collect())
    }

    fn cancel_provider_streams(&mut self, _provider_id: &str) -> Result<usize, StreamError> {
        let mut count = 0;
        for (status, _) in self.streams.values_mut() {
            status.state = StreamState::Cancelled;
            count += 1;
        }
        Ok(count)
    }

    fn health_check(&self) -> Result<StreamProviderHealth, StreamError> {
        Ok(StreamProviderHealth {
            status: if self.streams.len() >= self.max_concurrent {
                StreamProviderStatus::Degraded
            } else {
                StreamProviderStatus::Healthy
            },
            active_streams: self.streams.len(),
            total_streams_processed: 0,
            avg_throughput_bps: 0,
            last_check_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        })
    }

    fn provider_manifest(&self) -> StreamProviderManifest {
        StreamProviderManifest {
            provider_id: "in-memory-stream-provider".to_string(),
            name: "In-Memory Stream Provider".to_string(),
            version: "1.0.0".to_string(),
            supported_protocols: vec![StreamProtocol::Sse, StreamProtocol::WebSocket],
            max_concurrent_streams: self.max_concurrent,
            max_stream_duration_ms: 60000,
            supports_backpressure: true,
            supports_cancellation: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_chunk_new() {
        let chunk = StreamChunk::new(1, "Hello");
        assert_eq!(chunk.sequence, 1);
        assert_eq!(chunk.content, "Hello");
        assert_eq!(chunk.chunk_type, StreamChunkType::Text);
        assert!(!chunk.is_final);
    }

    #[test]
    fn test_stream_chunk_final() {
        let chunk = StreamChunk::final_chunk(100, "Complete");
        assert_eq!(chunk.sequence, 100);
        assert!(chunk.is_final);
        assert_eq!(chunk.chunk_type, StreamChunkType::Final);
    }

    #[test]
    fn test_stream_chunk_type_as_str() {
        assert_eq!(StreamChunkType::Text.as_str(), "text");
        assert_eq!(StreamChunkType::Code.as_str(), "code");
    }

    #[test]
    fn test_stream_control_cancel() {
        let control = StreamControl::cancel("User request");
        assert_eq!(
            control,
            StreamControl::Cancel {
                reason: "User request".to_string()
            }
        );
    }

    #[test]
    fn test_stream_state_as_str() {
        assert_eq!(StreamState::Active.as_str(), "active");
        assert_eq!(StreamState::Paused.as_str(), "paused");
    }

    #[test]
    fn test_stream_request_new() {
        let request = StreamRequest::new("stream-1", "provider-1", "gpt-4", "Hello");
        assert_eq!(request.stream_id, "stream-1");
        assert_eq!(request.provider_id, "provider-1");
    }

    #[test]
    fn test_stream_config_default() {
        let config = StreamConfig::default();
        assert_eq!(config.chunk_size, 1024);
        assert_eq!(config.protocol, StreamProtocol::Sse);
    }

    #[test]
    fn test_stream_config_with_protocol() {
        let config = StreamConfig::new().with_protocol(StreamProtocol::WebSocket);
        assert_eq!(config.protocol, StreamProtocol::WebSocket);
    }

    #[test]
    fn test_stream_protocol_as_str() {
        assert_eq!(StreamProtocol::Sse.as_str(), "sse");
        assert_eq!(StreamProtocol::WebSocket.as_str(), "websocket");
    }

    #[test]
    fn test_stream_result_success() {
        let result = StreamResult::success("stream-1", "Response", 10, 1000);
        assert_eq!(result.stream_id, "stream-1");
        assert_eq!(result.complete_response, "Response");
        assert_eq!(result.total_chunks, 10);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_stream_result_failure() {
        let result = StreamResult::failure("stream-1", vec!["Error 1".to_string()]);
        assert_eq!(result.stream_id, "stream-1");
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_stream_result_cancelled() {
        let result = StreamResult::cancelled("stream-1", "User cancelled".to_string(), 5);
        assert_eq!(result.total_chunks, 5);
        assert_eq!(result.status.state, StreamState::Cancelled);
    }

    #[test]
    fn test_stream_error_display() {
        assert_eq!(
            StreamError::StreamNotFound("stream-1".to_string()).to_string(),
            "Stream not found: stream-1"
        );
        assert_eq!(
            StreamError::StreamTimeout("stream-1".to_string()).to_string(),
            "Stream timeout: stream-1"
        );
    }

    #[test]
    fn test_in_memory_stream_provider_initiate() {
        let mut provider = InMemoryStreamProvider::new();
        let request = StreamRequest::new("stream-1", "provider-1", "model-1", "Test");

        let status = provider.initiate_stream(request).unwrap();
        assert_eq!(status.stream_id, "stream-1");
        assert_eq!(status.state, StreamState::Initializing);
    }

    #[test]
    fn test_in_memory_stream_provider_receive_chunk() {
        let mut provider = InMemoryStreamProvider::new();
        let request = StreamRequest::new("stream-1", "provider-1", "model-1", "Test");
        provider.initiate_stream(request).unwrap();

        let chunk = provider.receive_chunk("stream-1").unwrap();
        assert!(chunk.is_final);
    }

    #[test]
    fn test_in_memory_stream_provider_control() {
        let mut provider = InMemoryStreamProvider::new();
        let request = StreamRequest::new("stream-1", "provider-1", "model-1", "Test");
        provider.initiate_stream(request).unwrap();

        provider
            .control_stream("stream-1", StreamControl::cancel("Test"))
            .unwrap();
        let status = provider.get_status("stream-1").unwrap();
        assert_eq!(status.state, StreamState::Cancelled);
    }

    #[test]
    fn test_in_memory_stream_provider_finalize_releases_concurrency_slot() {
        let mut provider = InMemoryStreamProvider::new().with_max_concurrent(1);
        provider
            .initiate_stream(StreamRequest::new(
                "stream-1",
                "provider-1",
                "model-1",
                "Test",
            ))
            .unwrap();

        provider.finalize_stream("stream-1").unwrap();

        provider
            .initiate_stream(StreamRequest::new(
                "stream-2",
                "provider-1",
                "model-1",
                "Test",
            ))
            .expect("finalized streams must release the concurrency slot");
        assert!(
            provider.get_status("stream-1").is_err(),
            "finalized stream state should not remain active in the in-memory provider"
        );
    }

    #[test]
    fn test_in_memory_stream_provider_health() {
        let provider = InMemoryStreamProvider::new();
        let health = provider.health_check().unwrap();
        assert_eq!(health.status, StreamProviderStatus::Healthy);
        assert_eq!(health.active_streams, 0);
    }

    #[test]
    fn test_in_memory_stream_provider_manifest() {
        let provider = InMemoryStreamProvider::new();
        let manifest = provider.provider_manifest();
        assert_eq!(manifest.provider_id, "in-memory-stream-provider");
        assert!(manifest.supports_cancellation);
    }
}
