mod artifact;
mod code_event;
mod conformance;
mod knowledge;
mod language;
mod patch;
mod protocol;
mod review;
mod runtime;
mod safety;
mod session;
mod task;
mod terminal;
mod vcs;
mod verification;
mod workspace;

pub use artifact::{
    ArtifactDescriptor, ArtifactFilter, ArtifactProvider, ArtifactReceipt, CodeArtifact,
    CodeArtifactKind,
};
pub use code_event::{CodeEventKind, CodeKernelEvent};
pub use conformance::{CodeConformanceCase, CodeConformanceProfile, CodeConformanceReport};
pub use knowledge::{
    KnowledgeDocument, KnowledgeDocumentFilter, KnowledgeDocumentKind, KnowledgeProvider,
    KnowledgeQuery, KnowledgeSearchResult,
};
pub use language::{
    LanguageDiagnostic, LanguageDiagnosticSeverity, LanguageDiagnosticsRequest,
    LanguageFormatRequest, LanguageFormatResult, LanguageProvider, LanguageSymbol,
    LanguageSymbolKind, LanguageSymbolsRequest,
};
pub use patch::{
    PatchApplyResult, PatchExplanation, PatchOperation, PatchPreview, PatchProvider,
    PatchRejection, PatchRollbackResult, PatchSet,
};
pub use protocol::{CodeProtocolObjectMapper, StandardCodeProtocolObjectMapper};
pub use review::{ReviewFinding, ReviewProvider, ReviewReport, ReviewSeverity};
pub use runtime::{
    CodeKernelCapabilityManifest, CodeKernelRuntime, CodeKernelRuntimeBuilder,
    CodeKernelRuntimeDiagnostics, CodeProviderDiagnostic,
};
pub use safety::{CodeSafetyAssessment, CodeSafetyProvider, CodeSafetyRiskLevel, CodeSafetyScope};
pub use session::{CodeProviderBinding, CodeSession, CodeSessionState};
pub use task::{
    CodeCheckpoint, CodePlan, CodePlanStep, CodeReviewStatus, CodeTask, CodeTaskIntent,
    CodeTaskState, CodeTraceRef,
};
pub use terminal::{TerminalCommand, TerminalOutputChannel, TerminalOutputChunk, TerminalProvider};
pub use vcs::{
    VcsBlameLine, VcsCommitMetadata, VcsDiff, VcsDiffFile, VcsDiffRequest, VcsFileChangeKind,
    VcsProvider, VcsRestoreReport, VcsRestoreRequest, VcsSnapshot,
};
pub use verification::{CommandResult, VerificationPlan, VerificationProvider, VerificationReport};
pub use workspace::{
    GeneratedFilePolicy, Workspace, WorkspaceFile, WorkspaceFileEntry, WorkspaceFileKind,
    WorkspaceFileStat, WorkspaceProvider, WorkspaceWatchEvent, WorkspaceWatchEventKind,
    WorkspaceWriteRequest, WorkspaceWriteResult,
};

pub const CODE_KERNEL_SPEC_VERSION: &str = "0.1.0";
pub const CODE_CAPABILITY_MANIFEST_SCHEMA: &str =
    include_str!("../../specs/schemas/code-capability-manifest.schema.json");
pub const CODE_RUNTIME_DIAGNOSTICS_SCHEMA: &str =
    include_str!("../../specs/schemas/code-runtime-diagnostics.schema.json");
pub const CODE_CONFORMANCE_REPORT_SCHEMA: &str =
    include_str!("../../specs/schemas/code-conformance-report.schema.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeKernelCapability {
    WorkspaceList,
    WorkspaceRead,
    WorkspaceWrite,
    WorkspaceStat,
    WorkspaceWatch,
    VcsStatus,
    VcsDiff,
    VcsBlame,
    VcsCommitMetadata,
    VcsRestore,
    PatchValidate,
    PatchPreview,
    PatchApply,
    PatchReject,
    PatchRollback,
    PatchExplain,
    TerminalRun,
    VerificationRun,
    LanguageDiagnostics,
    LanguageSymbols,
    LanguageFormat,
    ReviewProduce,
    ArtifactRead,
    ArtifactWrite,
    KnowledgeSearch,
    KnowledgeRead,
    KnowledgeList,
    SafetyAssess,
}

impl CodeKernelCapability {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::WorkspaceList => "code.workspace.list",
            Self::WorkspaceRead => "code.workspace.read",
            Self::WorkspaceWrite => "code.workspace.write",
            Self::WorkspaceStat => "code.workspace.stat",
            Self::WorkspaceWatch => "code.workspace.watch",
            Self::VcsStatus => "code.vcs.status",
            Self::VcsDiff => "code.vcs.diff",
            Self::VcsBlame => "code.vcs.blame",
            Self::VcsCommitMetadata => "code.vcs.commit_metadata",
            Self::VcsRestore => "code.vcs.restore",
            Self::PatchValidate => "code.patch.validate",
            Self::PatchPreview => "code.patch.preview",
            Self::PatchApply => "code.patch.apply",
            Self::PatchReject => "code.patch.reject",
            Self::PatchRollback => "code.patch.rollback",
            Self::PatchExplain => "code.patch.explain",
            Self::TerminalRun => "code.terminal.run",
            Self::VerificationRun => "code.verification.run",
            Self::LanguageDiagnostics => "code.language.diagnostics",
            Self::LanguageSymbols => "code.language.symbols",
            Self::LanguageFormat => "code.language.format",
            Self::ReviewProduce => "code.review.produce",
            Self::ArtifactRead => "code.artifact.read",
            Self::ArtifactWrite => "code.artifact.write",
            Self::KnowledgeSearch => "code.knowledge.search",
            Self::KnowledgeRead => "code.knowledge.read",
            Self::KnowledgeList => "code.knowledge.list",
            Self::SafetyAssess => "code.safety.assess",
        }
    }
}
