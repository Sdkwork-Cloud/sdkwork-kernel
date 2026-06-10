use crate::{
    AgentArtifact, AgentMessage, AgentTask, EventStreamItem, KernelError, KernelErrorKind,
    KernelEvent, KernelEventRedaction, KernelResult, ProviderHealth, TraceContext,
};

pub trait ProtocolAdapter {
    fn manifest(&self) -> ProtocolAdapterManifest;

    fn health(&self) -> ProviderHealth;

    fn handle_request(
        &self,
        _runtime: &crate::AgentRuntime,
        _request: ProtocolAdapterRequest,
    ) -> KernelResult<ProtocolObjectEnvelope> {
        Err(KernelError::provider_error(
            self.manifest().adapter_id,
            "protocol adapter does not implement direct request handling",
        ))
    }

    fn map_request_to_task(&self, request: ProtocolAdapterRequest) -> KernelResult<AgentTask>;

    fn map_event_to_stream_update(&self, event: KernelEvent) -> KernelResult<ProtocolStreamUpdate>;

    fn map_stream_item_to_stream_update(
        &self,
        item: EventStreamItem,
    ) -> KernelResult<ProtocolStreamUpdate> {
        Ok(ProtocolStreamUpdate::from_event(item.event, item.sequence))
    }

    fn map_response(&self, task: AgentTask) -> KernelResult<ProtocolAdapterResponse>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolFamily {
    Mcp,
    A2a,
    Http,
    Rpc,
    Ipc,
    Tauri,
    WebSocket,
    KernelUiClient,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolTransport {
    Http,
    Rpc,
    Ipc,
    Tauri,
    WebSocket,
    InProcess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolAdapterAuthMode {
    None,
    LocalTrusted,
    Bearer,
    MutualTls,
    SignedRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolAdapterStreamingSupport {
    None,
    Unordered,
    Ordered,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolAdapterManifest {
    pub adapter_id: String,
    pub provider_family: String,
    pub protocol: ProtocolFamily,
    pub protocol_version: String,
    pub transport: ProtocolTransport,
    pub auth_mode: ProtocolAdapterAuthMode,
    pub exposed_capabilities: Vec<String>,
    pub kernel_object_mappings: Vec<String>,
    pub streaming_support: ProtocolAdapterStreamingSupport,
    pub trace_support: bool,
    pub security_requirements: Vec<String>,
    pub status: String,
}

impl ProtocolAdapterManifest {
    pub fn new(
        adapter_id: impl Into<String>,
        protocol: ProtocolFamily,
        protocol_version: impl Into<String>,
        transport: ProtocolTransport,
        auth_mode: ProtocolAdapterAuthMode,
    ) -> Self {
        Self {
            adapter_id: adapter_id.into(),
            provider_family: "protocol_adapter".to_string(),
            protocol,
            protocol_version: protocol_version.into(),
            transport,
            auth_mode,
            exposed_capabilities: Vec::new(),
            kernel_object_mappings: Vec::new(),
            streaming_support: ProtocolAdapterStreamingSupport::None,
            trace_support: false,
            security_requirements: Vec::new(),
            status: "candidate".to_string(),
        }
    }

    pub fn with_exposed_capabilities(mut self, exposed_capabilities: Vec<String>) -> Self {
        self.exposed_capabilities = exposed_capabilities;
        self
    }

    pub fn with_kernel_object_mappings(mut self, kernel_object_mappings: Vec<String>) -> Self {
        self.kernel_object_mappings = kernel_object_mappings;
        self
    }

    pub fn with_streaming_support(
        mut self,
        streaming_support: ProtocolAdapterStreamingSupport,
    ) -> Self {
        self.streaming_support = streaming_support;
        self
    }

    pub fn with_trace_support(mut self, trace_support: bool) -> Self {
        self.trace_support = trace_support;
        self
    }

    pub fn with_security_requirements(mut self, security_requirements: Vec<String>) -> Self {
        self.security_requirements = security_requirements;
        self
    }

    pub fn validate(&self) -> KernelResult<()> {
        if !self.adapter_id.starts_with("adapter.") {
            return Err(KernelError::validation(
                "protocol adapter id must start with adapter.",
            ));
        }

        if self.protocol_version.trim().is_empty() {
            return Err(KernelError::validation(
                "protocol adapter manifest requires protocol_version",
            ));
        }

        Ok(())
    }

    pub fn exposes_capability(&self, capability_id: &str) -> bool {
        self.exposed_capabilities
            .iter()
            .any(|capability| capability == capability_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolAdapterRequest {
    pub protocol_request_id: String,
    pub protocol: ProtocolFamily,
    pub operation: String,
    pub payload: String,
    pub external_id: Option<String>,
    pub metadata: Vec<(String, String)>,
    pub trace_context: Option<TraceContext>,
}

impl ProtocolAdapterRequest {
    pub fn new(
        protocol_request_id: impl Into<String>,
        protocol: ProtocolFamily,
        operation: impl Into<String>,
        payload: impl Into<String>,
    ) -> Self {
        Self {
            protocol_request_id: protocol_request_id.into(),
            protocol,
            operation: operation.into(),
            payload: payload.into(),
            external_id: None,
            metadata: Vec::new(),
            trace_context: None,
        }
    }

    pub fn with_external_id(mut self, external_id: impl Into<String>) -> Self {
        self.external_id = Some(external_id.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }

    pub fn with_trace_context(mut self, trace_context: TraceContext) -> Self {
        self.trace_context = Some(trace_context);
        self
    }

    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        self.metadata
            .iter()
            .find(|(metadata_key, _)| metadata_key == key)
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolAdapterResponse {
    pub protocol_response_id: String,
    pub task_id: String,
    pub status: String,
}

impl ProtocolAdapterResponse {
    pub fn accepted(protocol_response_id: impl Into<String>, task_id: impl Into<String>) -> Self {
        Self {
            protocol_response_id: protocol_response_id.into(),
            task_id: task_id.into(),
            status: "accepted".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolStreamUpdate {
    pub event_id: String,
    pub event_type: String,
    pub event_version: String,
    pub sequence: u64,
    pub payload: String,
    pub trace_context: Option<TraceContext>,
}

impl ProtocolStreamUpdate {
    pub fn from_event(event: KernelEvent, sequence: u64) -> Self {
        Self {
            event_id: event.event_id,
            event_type: event.event_type,
            event_version: event.event_version,
            sequence,
            payload: event.payload,
            trace_context: event.trace_context,
        }
    }

    pub fn to_sse_event(&self) -> ProtocolSseEvent {
        ProtocolSseEvent::from_stream_update(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolSseEvent {
    pub event_id: String,
    pub event_type: String,
    pub data: Vec<String>,
}

impl ProtocolSseEvent {
    pub fn from_stream_update(update: &ProtocolStreamUpdate) -> Self {
        let mut data = vec![
            format!("event_version={}", update.event_version),
            format!("sequence={}", update.sequence),
        ];
        data.extend(
            format!("payload={}", update.payload)
                .lines()
                .map(std::string::ToString::to_string),
        );

        Self {
            event_id: update.event_id.clone(),
            event_type: update.event_type.clone(),
            data,
        }
    }

    pub fn to_frame(&self) -> String {
        let mut frame = String::new();
        frame.push_str("id: ");
        frame.push_str(&self.event_id);
        frame.push('\n');
        frame.push_str("event: ");
        frame.push_str(&self.event_type);
        frame.push('\n');
        for data_line in &self.data {
            frame.push_str("data: ");
            frame.push_str(data_line);
            frame.push('\n');
        }
        frame.push('\n');
        frame
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolError {
    pub code: String,
    pub safe_message: String,
}

impl ProtocolError {
    pub fn from_kernel_error(error: KernelError) -> Self {
        let code = match error.kind() {
            KernelErrorKind::PolicyDenied => "permission_denied",
            _ => error.code(),
        };

        Self {
            code: code.to_string(),
            safe_message: error.safe_message().to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolObjectKind {
    AgentCard,
    AgentTask,
    AgentMessage,
    AgentPart,
    AgentArtifact,
    ToolDescriptor,
    ToolCall,
    KernelEvent,
    KernelError,
    ExtensionObject,
}

impl ProtocolObjectKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AgentCard => "agent_card",
            Self::AgentTask => "agent_task",
            Self::AgentMessage => "agent_message",
            Self::AgentPart => "agent_part",
            Self::AgentArtifact => "agent_artifact",
            Self::ToolDescriptor => "tool_descriptor",
            Self::ToolCall => "tool_call",
            Self::KernelEvent => "kernel_event",
            Self::KernelError => "kernel_error",
            Self::ExtensionObject => "extension_object",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolObjectEnvelope {
    pub protocol: ProtocolFamily,
    pub object_kind: ProtocolObjectKind,
    pub object_id: String,
    pub external_id: Option<String>,
    pub payload_schema: Option<String>,
    pub payload: String,
    pub metadata: Vec<(String, String)>,
    pub trace_context: Option<TraceContext>,
    pub redaction_classification: KernelEventRedaction,
    pub loss_notes: Vec<String>,
}

impl ProtocolObjectEnvelope {
    pub fn new(
        protocol: ProtocolFamily,
        object_kind: ProtocolObjectKind,
        object_id: impl Into<String>,
        payload: impl Into<String>,
    ) -> Self {
        Self {
            protocol,
            object_kind,
            object_id: object_id.into(),
            external_id: None,
            payload_schema: None,
            payload: payload.into(),
            metadata: Vec::new(),
            trace_context: None,
            redaction_classification: KernelEventRedaction::Unknown,
            loss_notes: Vec::new(),
        }
    }

    pub fn with_external_id(mut self, external_id: impl Into<String>) -> Self {
        self.external_id = Some(external_id.into());
        self
    }

    pub fn with_schema(mut self, payload_schema: impl Into<String>) -> Self {
        self.payload_schema = Some(payload_schema.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }

    pub fn with_trace_context(mut self, trace_context: TraceContext) -> Self {
        self.trace_context = Some(trace_context);
        self
    }

    pub fn with_redaction(mut self, redaction_classification: KernelEventRedaction) -> Self {
        self.redaction_classification = redaction_classification;
        self
    }

    pub fn with_loss_note(mut self, loss_note: impl Into<String>) -> Self {
        self.loss_notes.push(loss_note.into());
        self
    }

    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        self.metadata
            .iter()
            .find(|(metadata_key, _)| metadata_key == key)
            .map(|(_, value)| value.as_str())
    }

    pub fn validate(&self) -> KernelResult<()> {
        for (key, _) in &self.metadata {
            if !key.contains('.') {
                return Err(KernelError::validation(format!(
                    "metadata key must be namespaced: {key}"
                )));
            }
        }

        Ok(())
    }
}

pub trait ProtocolObjectMapper {
    fn map_message(&self, message: &AgentMessage) -> KernelResult<ProtocolObjectEnvelope>;

    fn map_artifact(&self, artifact: &AgentArtifact) -> KernelResult<ProtocolObjectEnvelope>;

    fn map_event(&self, event: &KernelEvent) -> KernelResult<ProtocolObjectEnvelope>;

    fn map_error(&self, error: &KernelError) -> KernelResult<ProtocolObjectEnvelope>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StandardProtocolObjectMapper {
    protocol: ProtocolFamily,
}

impl StandardProtocolObjectMapper {
    pub fn new(protocol: ProtocolFamily) -> Self {
        Self { protocol }
    }
}

impl ProtocolObjectMapper for StandardProtocolObjectMapper {
    fn map_message(&self, message: &AgentMessage) -> KernelResult<ProtocolObjectEnvelope> {
        message.validate()?;

        let mut envelope = ProtocolObjectEnvelope::new(
            self.protocol,
            ProtocolObjectKind::AgentMessage,
            message.message_id.clone(),
            format!(
                "message_id={};role={};parts={};untrusted={}",
                message.message_id,
                message.role.as_str(),
                message.parts.len(),
                message.untrusted
            ),
        )
        .with_schema("sdkwork.agent.message.v1")
        .with_metadata("sdkwork.message.role", message.role.as_str())
        .with_metadata("sdkwork.message.parts", message.parts.len().to_string())
        .with_redaction(message.highest_redaction());

        if let Some(session_id) = &message.session_id {
            envelope = envelope.with_metadata("sdkwork.agent.session_id", session_id.clone());
        }

        if let Some(task_id) = &message.task_id {
            envelope = envelope.with_metadata("sdkwork.agent.task_id", task_id.clone());
        }

        if let Some(run_id) = &message.run_id {
            envelope = envelope.with_metadata("sdkwork.agent.run_id", run_id.clone());
        }

        if let Some(step_id) = &message.step_id {
            envelope = envelope.with_metadata("sdkwork.agent.step_id", step_id.clone());
        }

        if let Some(trace_context) = &message.trace_context {
            envelope = envelope.with_trace_context(trace_context.clone());
        }

        envelope.validate()?;
        Ok(envelope)
    }

    fn map_artifact(&self, artifact: &AgentArtifact) -> KernelResult<ProtocolObjectEnvelope> {
        let mut envelope = ProtocolObjectEnvelope::new(
            self.protocol,
            ProtocolObjectKind::AgentArtifact,
            artifact.artifact_id.clone(),
            format!(
                "artifact_id={};task_id={};kind={};content_ref={}",
                artifact.artifact_id,
                artifact.task_id,
                artifact.kind.as_str(),
                artifact.content_ref
            ),
        )
        .with_schema("sdkwork.agent.artifact.v1")
        .with_metadata("sdkwork.agent.task_id", artifact.task_id.clone())
        .with_metadata("sdkwork.artifact.kind", artifact.kind.as_str())
        .with_redaction(artifact.redaction_classification);

        if let Some(step_id) = &artifact.producer_step_id {
            envelope = envelope.with_metadata("sdkwork.agent.step_id", step_id.clone());
        }

        if let Some(mime_type) = &artifact.mime_type {
            envelope = envelope.with_metadata("sdkwork.artifact.mime_type", mime_type.clone());
        }

        if let Some(name) = &artifact.name {
            envelope = envelope.with_metadata("sdkwork.artifact.name", name.clone());
        }

        envelope.validate()?;
        Ok(envelope)
    }

    fn map_event(&self, event: &KernelEvent) -> KernelResult<ProtocolObjectEnvelope> {
        let mut envelope = ProtocolObjectEnvelope::new(
            self.protocol,
            ProtocolObjectKind::KernelEvent,
            event.event_id.clone(),
            event.payload.clone(),
        )
        .with_schema(
            event
                .payload_schema
                .clone()
                .unwrap_or_else(|| "sdkwork.agent.kernel_event.v1".to_string()),
        )
        .with_metadata("sdkwork.event.type", event.event_type.clone())
        .with_metadata("sdkwork.event.version", event.event_version.clone())
        .with_redaction(event.redaction_classification);

        if let Some(session_id) = &event.session_id {
            envelope = envelope.with_metadata("sdkwork.agent.session_id", session_id.clone());
        }

        if let Some(task_id) = &event.task_id {
            envelope = envelope.with_metadata("sdkwork.agent.task_id", task_id.clone());
        }

        if let Some(run_id) = &event.run_id {
            envelope = envelope.with_metadata("sdkwork.agent.run_id", run_id.clone());
        }

        if let Some(step_id) = &event.step_id {
            envelope = envelope.with_metadata("sdkwork.agent.step_id", step_id.clone());
        }

        if let Some(trace_context) = &event.trace_context {
            envelope = envelope.with_trace_context(trace_context.clone());
        }

        envelope.validate()?;
        Ok(envelope)
    }

    fn map_error(&self, error: &KernelError) -> KernelResult<ProtocolObjectEnvelope> {
        let mut envelope = ProtocolObjectEnvelope::new(
            self.protocol,
            ProtocolObjectKind::KernelError,
            format!("error.{}", error.code()),
            format!(
                "kind={};code={};safe_message={};retryable={};safe_for_user={}",
                error.kind().as_str(),
                error.code(),
                error.safe_message(),
                error.retryable(),
                error.safe_for_user()
            ),
        )
        .with_schema("sdkwork.agent.error.v1")
        .with_metadata("sdkwork.error.kind", error.kind().as_str())
        .with_metadata("sdkwork.error.code", error.code())
        .with_metadata("sdkwork.error.source", error.source().as_str())
        .with_redaction(error.redaction_classification());

        if let Some(provider_id) = error.provider_id() {
            envelope = envelope.with_metadata("sdkwork.provider.id", provider_id);
        }

        if let Some(trace_context) = error.trace_context() {
            envelope = envelope.with_trace_context(trace_context.clone());
        }

        envelope.validate()?;
        Ok(envelope)
    }
}
