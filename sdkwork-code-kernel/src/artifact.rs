use crate::Workspace;
use sdkwork_agent_kernel::{
    KernelEventRedaction, KernelResult, PolicyCategory, PolicyRequest, ProviderHealth,
    SideEffectLevel,
};

pub trait ArtifactProvider {
    fn put_artifact(
        &self,
        workspace: &Workspace,
        artifact: CodeArtifact,
    ) -> KernelResult<ArtifactReceipt>;

    fn get_artifact(&self, workspace: &Workspace, artifact_id: &str) -> KernelResult<CodeArtifact>;

    fn list_artifacts(
        &self,
        workspace: &Workspace,
        filter: ArtifactFilter,
    ) -> KernelResult<Vec<ArtifactDescriptor>>;

    fn health(&self) -> ProviderHealth;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeArtifactKind {
    PatchSet,
    Diff,
    VerificationReport,
    TerminalLog,
    ReviewReport,
    WorkspaceSnapshot,
    DiagnosticReport,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeArtifact {
    pub artifact_id: String,
    pub workspace_id: String,
    pub kind: CodeArtifactKind,
    pub title: String,
    pub content: String,
    pub mime_type: Option<String>,
    pub redaction_classification: KernelEventRedaction,
    pub retention_policy: Option<String>,
}

impl CodeArtifact {
    pub fn new(
        artifact_id: impl Into<String>,
        workspace_id: impl Into<String>,
        kind: CodeArtifactKind,
        title: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            artifact_id: artifact_id.into(),
            workspace_id: workspace_id.into(),
            kind,
            title: title.into(),
            content: content.into(),
            mime_type: None,
            redaction_classification: KernelEventRedaction::Unknown,
            retention_policy: None,
        }
    }

    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
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

    pub fn write_policy_request(&self, policy_request_id: impl Into<String>) -> PolicyRequest {
        let mut request = PolicyRequest::new(
            policy_request_id,
            "code.artifact.write",
            format!(
                "workspace://{}/artifacts/{}",
                self.workspace_id, self.artifact_id
            ),
        )
        .with_category(PolicyCategory::ProductSpecific(
            "code.artifact.write".to_string(),
        ))
        .with_action("artifact.write")
        .with_side_effect_level(SideEffectLevel::SideEffectful)
        .with_redaction(self.redaction_classification)
        .with_context("workspace_id", self.workspace_id.clone())
        .with_context("artifact_id", self.artifact_id.clone())
        .with_context("artifact_kind", self.kind.as_str())
        .with_context("title", self.title.clone());

        if let Some(mime_type) = &self.mime_type {
            request = request.with_context("mime_type", mime_type.clone());
        }

        if let Some(retention_policy) = &self.retention_policy {
            request = request.with_context("retention_policy", retention_policy.clone());
        }

        request
    }
}

impl CodeArtifactKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PatchSet => "patch_set",
            Self::Diff => "diff",
            Self::VerificationReport => "verification_report",
            Self::TerminalLog => "terminal_log",
            Self::ReviewReport => "review_report",
            Self::WorkspaceSnapshot => "workspace_snapshot",
            Self::DiagnosticReport => "diagnostic_report",
            Self::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactReceipt {
    pub artifact_id: String,
    pub storage_ref: Option<String>,
}

impl ArtifactReceipt {
    pub fn new(artifact_id: impl Into<String>, storage_ref: impl Into<String>) -> Self {
        Self {
            artifact_id: artifact_id.into(),
            storage_ref: Some(storage_ref.into()),
        }
    }

    pub fn stored(artifact_id: impl Into<String>) -> Self {
        Self {
            artifact_id: artifact_id.into(),
            storage_ref: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactDescriptor {
    pub artifact_id: String,
    pub workspace_id: String,
    pub kind: CodeArtifactKind,
    pub title: String,
    pub redaction_classification: KernelEventRedaction,
}

impl ArtifactDescriptor {
    pub fn new(
        artifact_id: impl Into<String>,
        workspace_id: impl Into<String>,
        kind: CodeArtifactKind,
        title: impl Into<String>,
    ) -> Self {
        Self {
            artifact_id: artifact_id.into(),
            workspace_id: workspace_id.into(),
            kind,
            title: title.into(),
            redaction_classification: KernelEventRedaction::Unknown,
        }
    }

    pub fn with_redaction(mut self, redaction_classification: KernelEventRedaction) -> Self {
        self.redaction_classification = redaction_classification;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactFilter {
    pub kind: Option<CodeArtifactKind>,
    pub redaction_classification: Option<KernelEventRedaction>,
}

impl ArtifactFilter {
    pub fn new() -> Self {
        Self {
            kind: None,
            redaction_classification: None,
        }
    }

    pub fn with_kind(mut self, kind: CodeArtifactKind) -> Self {
        self.kind = Some(kind);
        self
    }

    pub fn with_redaction(mut self, redaction_classification: KernelEventRedaction) -> Self {
        self.redaction_classification = Some(redaction_classification);
        self
    }

    pub fn matches(&self, descriptor: &ArtifactDescriptor) -> bool {
        if let Some(kind) = self.kind {
            if descriptor.kind != kind {
                return false;
            }
        }

        if let Some(redaction_classification) = self.redaction_classification {
            if descriptor.redaction_classification != redaction_classification {
                return false;
            }
        }

        true
    }
}

impl Default for ArtifactFilter {
    fn default() -> Self {
        Self::new()
    }
}
