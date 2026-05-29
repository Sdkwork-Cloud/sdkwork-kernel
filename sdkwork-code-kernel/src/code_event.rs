use sdkwork_agent_kernel::{
    KernelEvent, KernelEventRedaction, KernelEventSeverity, KernelEventSource,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeEventKind {
    SessionStateChanged,
    TaskStateChanged,
    WorkspaceOpened,
    VcsSnapshotCaptured,
    PatchPreviewed,
    PatchApplied,
    TerminalStarted,
    VerificationCompleted,
    ReviewProduced,
    ArtifactStored,
    KnowledgeSearched,
    SafetyAssessed,
}

impl CodeEventKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SessionStateChanged => "code.session.state_changed",
            Self::TaskStateChanged => "code.task.state_changed",
            Self::WorkspaceOpened => "code.workspace.opened",
            Self::VcsSnapshotCaptured => "code.vcs.snapshot_captured",
            Self::PatchPreviewed => "code.patch.previewed",
            Self::PatchApplied => "code.patch.applied",
            Self::TerminalStarted => "code.terminal.started",
            Self::VerificationCompleted => "code.verification.completed",
            Self::ReviewProduced => "code.review.produced",
            Self::ArtifactStored => "code.artifact.stored",
            Self::KnowledgeSearched => "code.knowledge.searched",
            Self::SafetyAssessed => "code.safety.assessed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeKernelEvent {
    pub event_id: String,
    pub kind: CodeEventKind,
    pub workspace_id: String,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub artifact_id: Option<String>,
    pub payload: Option<String>,
    pub severity: KernelEventSeverity,
    pub redaction_classification: KernelEventRedaction,
}

impl CodeKernelEvent {
    pub fn new(
        event_id: impl Into<String>,
        kind: CodeEventKind,
        workspace_id: impl Into<String>,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            kind,
            workspace_id: workspace_id.into(),
            session_id: None,
            task_id: None,
            artifact_id: None,
            payload: None,
            severity: KernelEventSeverity::Info,
            redaction_classification: KernelEventRedaction::Unknown,
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

    pub fn with_artifact_id(mut self, artifact_id: impl Into<String>) -> Self {
        self.artifact_id = Some(artifact_id.into());
        self
    }

    pub fn with_payload(mut self, payload: impl Into<String>) -> Self {
        self.payload = Some(payload.into());
        self
    }

    pub fn with_severity(mut self, severity: KernelEventSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_redaction(mut self, redaction_classification: KernelEventRedaction) -> Self {
        self.redaction_classification = redaction_classification;
        self
    }

    pub fn to_kernel_event(&self) -> KernelEvent {
        let mut event = KernelEvent::new(
            self.event_id.clone(),
            self.kind.as_str(),
            self.severity,
            self.event_payload(),
        )
        .from_source(KernelEventSource::CodeKernel)
        .with_redaction(self.redaction_classification)
        .with_payload_schema("sdkwork.code.event.v1");

        if let Some(session_id) = &self.session_id {
            event = event.for_session(session_id.clone());
        }

        if let Some(task_id) = &self.task_id {
            event = event.for_task(task_id.clone());
        }

        event
    }

    fn event_payload(&self) -> String {
        let mut payload = format!("workspace_id={}", self.workspace_id);
        if let Some(artifact_id) = &self.artifact_id {
            payload.push_str(&format!(";artifact_id={artifact_id}"));
        }
        if let Some(extra_payload) = &self.payload {
            payload.push(';');
            payload.push_str(extra_payload);
        }
        payload
    }
}
