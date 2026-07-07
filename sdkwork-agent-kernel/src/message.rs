use crate::modality::ContentReference;
use crate::{
    KernelError, KernelEvent, KernelEventRedaction, KernelEventSeverity, KernelEventSource,
    KernelResult, PolicyCategory, PolicyRequest,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentMessageRole {
    User,
    Agent,
    System,
    Model,
    Tool,
    Policy,
    Adapter,
}

impl AgentMessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Agent => "agent",
            Self::System => "system",
            Self::Model => "model",
            Self::Tool => "tool",
            Self::Policy => "policy",
            Self::Adapter => "adapter",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentPartKind {
    Text,
    Json,
    BinaryRef,
    FileRef,
    ArtifactRef,
    ImageRef,
    AudioRef,
    VideoRef,
    ToolCallRef,
    PolicyDecisionRef,
    Error,
}

impl AgentPartKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Json => "json",
            Self::BinaryRef => "binary_ref",
            Self::FileRef => "file_ref",
            Self::ArtifactRef => "artifact_ref",
            Self::ImageRef => "image_ref",
            Self::AudioRef => "audio_ref",
            Self::VideoRef => "video_ref",
            Self::ToolCallRef => "tool_call_ref",
            Self::PolicyDecisionRef => "policy_decision_ref",
            Self::Error => "error",
        }
    }

    pub fn parse(value: &str) -> KernelResult<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            "binary_ref" | "binary" => Ok(Self::BinaryRef),
            "file_ref" | "file" => Ok(Self::FileRef),
            "artifact_ref" | "artifact" => Ok(Self::ArtifactRef),
            "image_ref" | "image" => Ok(Self::ImageRef),
            "audio_ref" | "audio" => Ok(Self::AudioRef),
            "video_ref" | "video" => Ok(Self::VideoRef),
            "tool_call_ref" | "tool_call" => Ok(Self::ToolCallRef),
            "policy_decision_ref" | "policy_decision" => Ok(Self::PolicyDecisionRef),
            "error" => Ok(Self::Error),
            other => Err(KernelError::validation(format!(
                "unsupported agent part kind: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentPart {
    pub part_id: String,
    pub kind: AgentPartKind,
    pub text: Option<String>,
    pub json: Option<String>,
    pub content_ref: Option<String>,
    pub artifact_id: Option<String>,
    pub tool_call_id: Option<String>,
    pub policy_decision_id: Option<String>,
    pub error_code: Option<String>,
    pub mime_type: Option<String>,
    pub name: Option<String>,
    pub schema: Option<String>,
    pub provenance: Option<String>,
    pub redaction_classification: KernelEventRedaction,
    pub metadata: Vec<(String, String)>,
}

impl AgentPart {
    pub fn text(part_id: impl Into<String>, text: impl Into<String>) -> Self {
        Self::base(part_id, AgentPartKind::Text).with_text(text)
    }

    pub fn json(part_id: impl Into<String>, json: impl Into<String>) -> Self {
        Self::base(part_id, AgentPartKind::Json).with_json(json)
    }

    pub fn binary_ref(
        part_id: impl Into<String>,
        content_ref: impl Into<String>,
        mime_type: impl Into<String>,
    ) -> Self {
        Self::base(part_id, AgentPartKind::BinaryRef)
            .with_content_ref(content_ref)
            .with_mime_type(mime_type)
    }

    pub fn file_ref(
        part_id: impl Into<String>,
        content_ref: impl Into<String>,
        mime_type: impl Into<String>,
    ) -> Self {
        Self::base(part_id, AgentPartKind::FileRef)
            .with_content_ref(content_ref)
            .with_mime_type(mime_type)
    }

    pub fn artifact_ref(part_id: impl Into<String>, artifact_id: impl Into<String>) -> Self {
        let mut part = Self::base(part_id, AgentPartKind::ArtifactRef);
        part.artifact_id = Some(artifact_id.into());
        part
    }

    pub fn image_ref(
        part_id: impl Into<String>,
        content_ref: impl Into<String>,
        mime_type: impl Into<String>,
    ) -> Self {
        Self::base(part_id, AgentPartKind::ImageRef)
            .with_content_ref(content_ref)
            .with_mime_type(mime_type)
    }

    pub fn audio_ref(
        part_id: impl Into<String>,
        content_ref: impl Into<String>,
        mime_type: impl Into<String>,
    ) -> Self {
        Self::base(part_id, AgentPartKind::AudioRef)
            .with_content_ref(content_ref)
            .with_mime_type(mime_type)
    }

    pub fn video_ref(
        part_id: impl Into<String>,
        content_ref: impl Into<String>,
        mime_type: impl Into<String>,
    ) -> Self {
        Self::base(part_id, AgentPartKind::VideoRef)
            .with_content_ref(content_ref)
            .with_mime_type(mime_type)
    }

    pub fn tool_call_ref(part_id: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        let mut part = Self::base(part_id, AgentPartKind::ToolCallRef);
        part.tool_call_id = Some(tool_call_id.into());
        part
    }

    pub fn policy_decision_ref(
        part_id: impl Into<String>,
        policy_decision_id: impl Into<String>,
    ) -> Self {
        let mut part = Self::base(part_id, AgentPartKind::PolicyDecisionRef);
        part.policy_decision_id = Some(policy_decision_id.into());
        part
    }

    pub fn error(
        part_id: impl Into<String>,
        error_code: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        let mut part = Self::base(part_id, AgentPartKind::Error).with_text(text);
        part.error_code = Some(error_code.into());
        part
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_schema(mut self, schema: impl Into<String>) -> Self {
        self.schema = Some(schema.into());
        self
    }

    pub fn from_provider(mut self, provider_id: impl Into<String>) -> Self {
        self.provenance = Some(provider_id.into());
        self
    }

    pub fn with_redaction(mut self, redaction_classification: KernelEventRedaction) -> Self {
        self.redaction_classification = redaction_classification;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }

    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        self.metadata
            .iter()
            .find(|(metadata_key, _)| metadata_key == key)
            .map(|(_, value)| value.as_str())
    }

    /// Parses `content_ref` or `artifact_id` into a typed kernel-neutral reference.
    pub fn content_reference(&self) -> KernelResult<Option<ContentReference>> {
        if let Some(uri) = &self.content_ref {
            return ContentReference::parse(uri).map(Some);
        }
        if let Some(artifact_id) = &self.artifact_id {
            return Ok(Some(ContentReference::artifact(artifact_id.clone())));
        }
        Ok(None)
    }

    fn base(part_id: impl Into<String>, kind: AgentPartKind) -> Self {
        Self {
            part_id: part_id.into(),
            kind,
            text: None,
            json: None,
            content_ref: None,
            artifact_id: None,
            tool_call_id: None,
            policy_decision_id: None,
            error_code: None,
            mime_type: None,
            name: None,
            schema: None,
            provenance: None,
            redaction_classification: KernelEventRedaction::Unknown,
            metadata: Vec::new(),
        }
    }

    fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    fn with_json(mut self, json: impl Into<String>) -> Self {
        self.json = Some(json.into());
        self
    }

    fn with_content_ref(mut self, content_ref: impl Into<String>) -> Self {
        self.content_ref = Some(content_ref.into());
        self
    }

    fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMessage {
    pub message_id: String,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub step_id: Option<String>,
    pub role: AgentMessageRole,
    pub parts: Vec<AgentPart>,
    pub created_at: Option<String>,
    pub trace_context: Option<crate::TraceContext>,
    pub metadata: Vec<(String, String)>,
    pub untrusted: bool,
}

impl AgentMessage {
    pub fn new(
        message_id: impl Into<String>,
        role: AgentMessageRole,
        parts: Vec<AgentPart>,
    ) -> Self {
        Self {
            message_id: message_id.into(),
            session_id: None,
            task_id: None,
            run_id: None,
            step_id: None,
            role,
            parts,
            created_at: None,
            trace_context: None,
            metadata: Vec::new(),
            untrusted: false,
        }
    }

    pub fn for_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn for_task(mut self, task_id: impl Into<String>) -> Self {
        self.task_id = Some(task_id.into());
        self
    }

    pub fn for_run(mut self, run_id: impl Into<String>) -> Self {
        self.run_id = Some(run_id.into());
        self
    }

    pub fn for_step(mut self, step_id: impl Into<String>) -> Self {
        self.step_id = Some(step_id.into());
        self
    }

    pub fn created_at(mut self, created_at: impl Into<String>) -> Self {
        self.created_at = Some(created_at.into());
        self
    }

    pub fn with_trace_context(mut self, trace_context: crate::TraceContext) -> Self {
        self.trace_context = Some(trace_context);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }

    pub fn mark_untrusted(mut self) -> Self {
        self.untrusted = true;
        self
    }

    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        self.metadata
            .iter()
            .find(|(metadata_key, _)| metadata_key == key)
            .map(|(_, value)| value.as_str())
    }

    pub fn highest_redaction(&self) -> KernelEventRedaction {
        self.parts
            .iter()
            .fold(KernelEventRedaction::Public, |highest, part| {
                redaction_max(highest, part.redaction_classification)
            })
    }

    pub fn validate(&self) -> KernelResult<()> {
        if self.parts.is_empty() {
            return Err(KernelError::validation(
                "agent messages must contain at least one part",
            ));
        }

        Ok(())
    }

    pub fn to_event(&self, event_id: impl Into<String>) -> KernelEvent {
        let mut event = KernelEvent::new(
            event_id,
            "agent.message.created",
            KernelEventSeverity::Info,
            format!(
                "message_id={};role={};parts={};untrusted={}",
                self.message_id,
                self.role.as_str(),
                self.parts.len(),
                self.untrusted
            ),
        )
        .from_source(KernelEventSource::Runtime)
        .with_redaction(self.highest_redaction())
        .with_payload_schema("sdkwork.agent.message.event.v1");

        if let Some(session_id) = &self.session_id {
            event = event.for_session(session_id.clone());
        }

        if let Some(task_id) = &self.task_id {
            event = event.for_task(task_id.clone());
        }

        if let Some(run_id) = &self.run_id {
            event = event.for_run(run_id.clone());
        }

        if let Some(step_id) = &self.step_id {
            event = event.for_step(step_id.clone());
        }

        if let Some(trace_context) = &self.trace_context {
            event = event.with_trace_context(trace_context.clone());
        }

        event
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactKind {
    File,
    Patch,
    Log,
    Report,
    Data,
    Media,
    Other,
}

impl ArtifactKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Patch => "patch",
            Self::Log => "log",
            Self::Report => "report",
            Self::Data => "data",
            Self::Media => "media",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentArtifact {
    pub artifact_id: String,
    pub task_id: String,
    pub producer_step_id: Option<String>,
    pub kind: ArtifactKind,
    pub content_ref: String,
    pub mime_type: Option<String>,
    pub created_at: Option<String>,
    pub name: Option<String>,
    pub provenance: Option<String>,
    pub redaction_classification: KernelEventRedaction,
    pub retention_policy: Option<String>,
    pub metadata: Vec<(String, String)>,
}

impl AgentArtifact {
    pub fn new(
        artifact_id: impl Into<String>,
        task_id: impl Into<String>,
        kind: ArtifactKind,
        content_ref: impl Into<String>,
    ) -> Self {
        Self {
            artifact_id: artifact_id.into(),
            task_id: task_id.into(),
            producer_step_id: None,
            kind,
            content_ref: content_ref.into(),
            mime_type: None,
            created_at: None,
            name: None,
            provenance: None,
            redaction_classification: KernelEventRedaction::Unknown,
            retention_policy: None,
            metadata: Vec::new(),
        }
    }

    pub fn produced_by_step(mut self, producer_step_id: impl Into<String>) -> Self {
        self.producer_step_id = Some(producer_step_id.into());
        self
    }

    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    pub fn created_at(mut self, created_at: impl Into<String>) -> Self {
        self.created_at = Some(created_at.into());
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_provenance(mut self, provenance: impl Into<String>) -> Self {
        self.provenance = Some(provenance.into());
        self
    }

    pub fn with_redaction(mut self, redaction_classification: KernelEventRedaction) -> Self {
        self.redaction_classification = redaction_classification;
        self
    }

    pub fn with_retention_policy(mut self, retention_policy: impl Into<String>) -> Self {
        self.retention_policy = Some(retention_policy.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }

    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        self.metadata
            .iter()
            .find(|(metadata_key, _)| metadata_key == key)
            .map(|(_, value)| value.as_str())
    }

    pub fn read_policy_request(&self, policy_request_id: impl Into<String>) -> PolicyRequest {
        self.policy_request(policy_request_id, PolicyCategory::ArtifactRead, "read")
    }

    pub fn write_policy_request(&self, policy_request_id: impl Into<String>) -> PolicyRequest {
        self.policy_request(policy_request_id, PolicyCategory::ArtifactWrite, "write")
    }

    pub fn to_event(&self, event_id: impl Into<String>) -> KernelEvent {
        let mut event = KernelEvent::new(
            event_id,
            "agent.artifact.created",
            KernelEventSeverity::Info,
            format!(
                "artifact_id={};task_id={};kind={};content_ref={}",
                self.artifact_id,
                self.task_id,
                self.kind.as_str(),
                self.content_ref
            ),
        )
        .from_source(KernelEventSource::Runtime)
        .for_task(self.task_id.clone())
        .with_redaction(self.redaction_classification)
        .with_payload_schema("sdkwork.agent.artifact.event.v1");

        if let Some(step_id) = &self.producer_step_id {
            event = event.for_step(step_id.clone());
        }

        event
    }

    fn policy_request(
        &self,
        policy_request_id: impl Into<String>,
        category: PolicyCategory,
        action: &'static str,
    ) -> PolicyRequest {
        PolicyRequest::new(
            policy_request_id,
            category.as_str(),
            self.artifact_id.clone(),
        )
        .with_category(category)
        .with_action(action)
        .with_task(self.task_id.clone())
        .with_redaction(self.redaction_classification)
    }
}

fn redaction_max(left: KernelEventRedaction, right: KernelEventRedaction) -> KernelEventRedaction {
    if redaction_rank(right) > redaction_rank(left) {
        right
    } else {
        left
    }
}

fn redaction_rank(redaction: KernelEventRedaction) -> u8 {
    match redaction {
        KernelEventRedaction::Public => 0,
        KernelEventRedaction::Unknown => 1,
        KernelEventRedaction::Internal => 2,
        KernelEventRedaction::TenantSensitive => 3,
        KernelEventRedaction::PersonalData => 4,
        KernelEventRedaction::Secret => 5,
        KernelEventRedaction::Regulated => 6,
    }
}
