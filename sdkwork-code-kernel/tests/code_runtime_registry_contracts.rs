use sdkwork_agent_kernel::{
    HostEnvPolicy, KernelErrorKind, KernelEventRedaction, KernelResult, ProviderHealth,
    SideEffectLevel,
};
use sdkwork_code_kernel::{
    ArtifactDescriptor, ArtifactFilter, ArtifactReceipt, CodeArtifact, CodeArtifactKind,
    CodeKernelCapability, CodeKernelRuntimeBuilder, CodeSafetyAssessment, CodeSafetyProvider,
    CodeSafetyRiskLevel, CodeSafetyScope, CommandResult, KnowledgeDocument,
    KnowledgeDocumentFilter, KnowledgeDocumentKind, KnowledgeProvider, KnowledgeQuery,
    KnowledgeSearchResult, LanguageDiagnostic, LanguageDiagnosticSeverity,
    LanguageDiagnosticsRequest, LanguageFormatRequest, LanguageFormatResult, LanguageProvider,
    LanguageSymbol, LanguageSymbolKind, LanguageSymbolsRequest, PatchApplyResult, PatchOperation,
    PatchProvider, PatchSet, ReviewFinding, ReviewProvider, ReviewReport, ReviewSeverity,
    TerminalCommand, TerminalOutputChunk, TerminalProvider, VcsProvider, VcsSnapshot,
    VerificationPlan, VerificationProvider, VerificationReport, Workspace, WorkspaceFile,
    WorkspaceProvider,
};

#[test]
fn code_runtime_registry_invokes_typed_code_spi_providers() {
    let runtime = CodeKernelRuntimeBuilder::new("code.runtime.local")
        .register_workspace_provider(
            "provider.code.workspace.typed",
            "0.1.0",
            FakeWorkspaceProvider,
        )
        .register_vcs_provider("provider.code.vcs.typed", "0.1.0", FakeVcsProvider)
        .register_patch_provider("provider.code.patch.typed", "0.1.0", FakePatchProvider)
        .register_terminal_provider(
            "provider.code.terminal.typed",
            "0.1.0",
            FakeTerminalProvider,
        )
        .register_verification_provider(
            "provider.code.verification.typed",
            "0.1.0",
            FakeVerificationProvider,
        )
        .register_language_provider(
            "provider.code.language.typed",
            "0.1.0",
            FakeLanguageProvider,
        )
        .register_review_provider("provider.code.review.typed", "0.1.0", FakeReviewProvider)
        .register_artifact_provider(
            "provider.code.artifact.typed",
            "0.1.0",
            FakeArtifactProvider,
        )
        .register_knowledge_provider(
            "provider.code.knowledge.typed",
            "0.1.0",
            FakeKnowledgeProvider,
        )
        .register_safety_provider(
            "provider.code.safety.typed",
            "0.1.0",
            FakeCodeSafetyProvider,
        )
        .bootstrap()
        .expect("code runtime bootstraps");

    let workspace = Workspace::new("workspace.1", "workspace");

    let file = runtime
        .workspace_provider()
        .expect("workspace provider is registered")
        .read_file(&workspace, "src/lib.rs")
        .expect("workspace file reads");
    assert_eq!(file.content.as_deref(), Some("pub fn main() {}"));

    let vcs = runtime
        .vcs_provider()
        .expect("vcs provider is registered")
        .snapshot(&workspace)
        .expect("vcs snapshot loads");
    assert_eq!(vcs.branch, "main");

    let patch = PatchSet::new("patch.1", "workspace.1", "Edit lib.rs")
        .add_operation(PatchOperation::update_file("src/lib.rs", "old", "new"));
    let applied = runtime
        .patch_provider()
        .expect("patch provider is registered")
        .apply_patch(&workspace, patch)
        .expect("patch applies");
    assert_eq!(applied.rollback_token.as_deref(), Some("rollback.patch.1"));

    let command = TerminalCommand::new(
        "cmd.cargo-test",
        "cargo",
        vec!["test".to_string()],
        "workspace",
    )
    .with_env_policy(HostEnvPolicy::AllowList(vec!["PATH".to_string()]));
    let command_result = runtime
        .terminal_provider()
        .expect("terminal provider is registered")
        .run_command(&workspace, command)
        .expect("terminal command runs");
    assert!(command_result.is_success());

    let verification_report = runtime
        .verification_provider()
        .expect("verification provider is registered")
        .run_verification(
            &workspace,
            VerificationPlan::new("verify.rust", "workspace.1").add_command(TerminalCommand::new(
                "cmd.cargo-test",
                "cargo",
                vec!["test".to_string()],
                "workspace",
            )),
        )
        .expect("verification runs");
    assert!(verification_report.is_success());

    let diagnostics = runtime
        .language_provider()
        .expect("language provider is registered")
        .diagnostics(
            &workspace,
            LanguageDiagnosticsRequest::new("src/lib.rs").with_language_id("rust"),
        )
        .expect("diagnostics load");
    assert_eq!(diagnostics[0].severity, LanguageDiagnosticSeverity::Error);

    let review = runtime
        .review_provider()
        .expect("review provider is registered")
        .review_verification(&workspace, &verification_report)
        .expect("verification review loads");
    assert_eq!(review.report_id, "review.verification.1");

    let artifact = runtime
        .artifact_provider()
        .expect("artifact provider is registered")
        .get_artifact(&workspace, "artifact.review.1")
        .expect("artifact loads");
    assert_eq!(artifact.kind, CodeArtifactKind::ReviewReport);

    let knowledge = runtime
        .knowledge_provider()
        .expect("knowledge provider is registered")
        .search_documents(&workspace, KnowledgeQuery::new("kernel"))
        .expect("knowledge search runs");
    assert_eq!(knowledge[0].document_id, "doc.kernel.1");

    let safety = runtime
        .safety_provider()
        .expect("safety provider is registered")
        .assess_workspace(&workspace, CodeSafetyScope::new("workspace"))
        .expect("safety assessment runs");
    assert_eq!(safety.risk_level, CodeSafetyRiskLevel::Medium);

    let manifest = runtime.capability_manifest();
    assert!(manifest.providers.iter().any(|provider| {
        provider.provider_id == "provider.code.workspace.typed"
            && provider.provider_family == "code_workspace"
    }));
    assert!(manifest.capabilities.iter().any(|capability| {
        capability.capability_id == CodeKernelCapability::PatchApply.as_str()
            && capability.provider_id == "provider.code.patch.typed"
    }));
    assert!(manifest.capabilities.iter().any(|capability| {
        capability.capability_id == CodeKernelCapability::WorkspaceWrite.as_str()
            && capability.provider_id == "provider.code.workspace.typed"
            && capability.operations == vec!["write_file".to_string()]
            && capability.side_effect_level.as_deref() == Some("side_effectful")
            && capability.policy_categories == vec!["code.workspace.write".to_string()]
    }));
    assert!(manifest.capabilities.iter().any(|capability| {
        capability.capability_id == CodeKernelCapability::VcsRestore.as_str()
            && capability.provider_id == "provider.code.vcs.typed"
            && capability.operations == vec!["restore".to_string()]
            && capability.side_effect_level.as_deref() == Some("destructive")
            && capability.policy_categories == vec!["code.vcs.restore".to_string()]
    }));
    assert!(manifest.capabilities.iter().any(|capability| {
        capability.capability_id == CodeKernelCapability::PatchRollback.as_str()
            && capability.provider_id == "provider.code.patch.typed"
            && capability.operations == vec!["rollback_patch".to_string()]
            && capability.side_effect_level.as_deref() == Some("destructive")
            && capability.policy_categories == vec!["code.patch.rollback".to_string()]
    }));
    assert!(manifest.capabilities.iter().any(|capability| {
        capability.capability_id == CodeKernelCapability::KnowledgeSearch.as_str()
            && capability.provider_id == "provider.code.knowledge.typed"
    }));
    assert!(manifest.capabilities.iter().any(|capability| {
        capability.capability_id == CodeKernelCapability::KnowledgeList.as_str()
            && capability.provider_id == "provider.code.knowledge.typed"
            && capability.operations == ["list_documents"]
            && capability.policy_categories == ["code.knowledge.list"]
    }));
    assert!(manifest.capabilities.iter().any(|capability| {
        capability.capability_id == CodeKernelCapability::SafetyAssess.as_str()
            && capability.provider_id == "provider.code.safety.typed"
    }));
}

#[test]
fn code_runtime_registry_reports_provider_unavailable_for_manifest_only_code_provider() {
    let runtime = CodeKernelRuntimeBuilder::new("code.runtime.local")
        .register_workspace_provider_manifest("provider.code.workspace.manifest", "0.1.0")
        .register_vcs_provider_manifest("provider.code.vcs.manifest", "0.1.0")
        .register_patch_provider_manifest("provider.code.patch.manifest", "0.1.0")
        .register_terminal_provider_manifest("provider.code.terminal.manifest", "0.1.0")
        .register_verification_provider_manifest("provider.code.verification.manifest", "0.1.0")
        .register_language_provider_manifest("provider.code.language.manifest", "0.1.0")
        .register_review_provider_manifest("provider.code.review.manifest", "0.1.0")
        .register_artifact_provider_manifest("provider.code.artifact.manifest", "0.1.0")
        .register_knowledge_provider_manifest("provider.code.knowledge.manifest", "0.1.0")
        .register_safety_provider_manifest("provider.code.safety.manifest", "0.1.0")
        .bootstrap()
        .expect("manifest-only code runtime bootstraps");

    let error = match runtime.workspace_provider() {
        Ok(_) => panic!("typed workspace provider instance is not registered"),
        Err(error) => error,
    };
    assert_eq!(error.kind(), KernelErrorKind::ProviderUnavailable);
    assert_eq!(
        error.provider_id(),
        Some("provider.code.workspace.manifest")
    );

    let error = match runtime.patch_provider() {
        Ok(_) => panic!("typed patch provider instance is not registered"),
        Err(error) => error,
    };
    assert_eq!(error.kind(), KernelErrorKind::ProviderUnavailable);
    assert_eq!(error.provider_id(), Some("provider.code.patch.manifest"));

    let error = match runtime.safety_provider() {
        Ok(_) => panic!("typed safety provider instance is not registered"),
        Err(error) => error,
    };
    assert_eq!(error.kind(), KernelErrorKind::ProviderUnavailable);
    assert_eq!(error.provider_id(), Some("provider.code.safety.manifest"));
}

struct FakeWorkspaceProvider;

impl WorkspaceProvider for FakeWorkspaceProvider {
    fn list_files(
        &self,
        _workspace: &Workspace,
        root: &str,
    ) -> KernelResult<Vec<sdkwork_code_kernel::WorkspaceFileEntry>> {
        Ok(vec![
            sdkwork_code_kernel::WorkspaceFileEntry::directory(root),
            sdkwork_code_kernel::WorkspaceFileEntry::file("src/lib.rs"),
        ])
    }

    fn read_file(&self, _workspace: &Workspace, path: &str) -> KernelResult<WorkspaceFile> {
        Ok(WorkspaceFile::with_content(path, "pub fn main() {}"))
    }

    fn write_file(
        &self,
        _workspace: &Workspace,
        request: sdkwork_code_kernel::WorkspaceWriteRequest,
    ) -> KernelResult<sdkwork_code_kernel::WorkspaceWriteResult> {
        Ok(
            sdkwork_code_kernel::WorkspaceWriteResult::written(request.path, request.content.len())
                .with_version("abc124"),
        )
    }

    fn stat_file(
        &self,
        _workspace: &Workspace,
        path: &str,
    ) -> KernelResult<sdkwork_code_kernel::WorkspaceFileStat> {
        Ok(sdkwork_code_kernel::WorkspaceFileStat::file(path, 16).with_version("abc123"))
    }

    fn watch_events(
        &self,
        _workspace: &Workspace,
        _since_cursor: Option<&str>,
    ) -> KernelResult<Vec<sdkwork_code_kernel::WorkspaceWatchEvent>> {
        Ok(vec![sdkwork_code_kernel::WorkspaceWatchEvent::new(
            "event.workspace.1",
            "src/lib.rs",
            sdkwork_code_kernel::WorkspaceWatchEventKind::Modified,
            "cursor.1",
        )])
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

struct FakeVcsProvider;

impl VcsProvider for FakeVcsProvider {
    fn snapshot(&self, _workspace: &Workspace) -> KernelResult<VcsSnapshot> {
        Ok(VcsSnapshot::new("main").with_head_revision("abc123"))
    }

    fn diff(
        &self,
        _workspace: &Workspace,
        _request: sdkwork_code_kernel::VcsDiffRequest,
    ) -> KernelResult<sdkwork_code_kernel::VcsDiff> {
        Ok(sdkwork_code_kernel::VcsDiff::new("diff.1").add_file(
            sdkwork_code_kernel::VcsDiffFile::modified("src/lib.rs", 1, 1),
        ))
    }

    fn blame(
        &self,
        _workspace: &Workspace,
        path: &str,
    ) -> KernelResult<Vec<sdkwork_code_kernel::VcsBlameLine>> {
        Ok(vec![sdkwork_code_kernel::VcsBlameLine::new(
            path,
            1,
            "abc123",
            "SDKWork",
            "initial commit",
        )])
    }

    fn commit_metadata(
        &self,
        _workspace: &Workspace,
        revision: &str,
    ) -> KernelResult<sdkwork_code_kernel::VcsCommitMetadata> {
        Ok(sdkwork_code_kernel::VcsCommitMetadata::new(
            revision,
            "SDKWork",
            "initial commit",
        ))
    }

    fn restore(
        &self,
        _workspace: &Workspace,
        request: sdkwork_code_kernel::VcsRestoreRequest,
    ) -> KernelResult<sdkwork_code_kernel::VcsRestoreReport> {
        Ok(sdkwork_code_kernel::VcsRestoreReport::restored(
            request.paths,
        ))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

struct FakePatchProvider;

impl PatchProvider for FakePatchProvider {
    fn validate_patch(&self, _workspace: &Workspace, patch: &PatchSet) -> KernelResult<()> {
        assert!(!patch.operations.is_empty());
        Ok(())
    }

    fn preview_patch(
        &self,
        _workspace: &Workspace,
        patch: &PatchSet,
    ) -> KernelResult<sdkwork_code_kernel::PatchPreview> {
        Ok(sdkwork_code_kernel::PatchPreview::from_patch(patch))
    }

    fn apply_patch(
        &self,
        _workspace: &Workspace,
        patch: PatchSet,
    ) -> KernelResult<PatchApplyResult> {
        Ok(PatchApplyResult::applied(
            patch.patch_id,
            "rollback.patch.1",
        ))
    }

    fn reject_patch(
        &self,
        _workspace: &Workspace,
        patch_id: &str,
        reason: &str,
    ) -> KernelResult<sdkwork_code_kernel::PatchRejection> {
        Ok(sdkwork_code_kernel::PatchRejection::rejected(
            patch_id, reason,
        ))
    }

    fn rollback_patch(
        &self,
        _workspace: &Workspace,
        rollback_token: &str,
    ) -> KernelResult<sdkwork_code_kernel::PatchRollbackResult> {
        Ok(sdkwork_code_kernel::PatchRollbackResult::rolled_back(
            rollback_token,
            vec!["src/lib.rs".to_string()],
        ))
    }

    fn explain_patch(
        &self,
        _workspace: &Workspace,
        patch: &PatchSet,
    ) -> KernelResult<sdkwork_code_kernel::PatchExplanation> {
        Ok(sdkwork_code_kernel::PatchExplanation::new(
            patch.patch_id.clone(),
            patch.summary.clone(),
        )
        .with_policy_categories(vec!["code.patch.apply".to_string()]))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

struct FakeTerminalProvider;

impl TerminalProvider for FakeTerminalProvider {
    fn run_command(
        &self,
        _workspace: &Workspace,
        command: TerminalCommand,
    ) -> KernelResult<CommandResult> {
        Ok(CommandResult::exited(command.command_id, 0, "ok", ""))
    }

    fn stream_output(
        &self,
        _workspace: &Workspace,
        command_id: &str,
    ) -> KernelResult<Vec<TerminalOutputChunk>> {
        Ok(vec![TerminalOutputChunk::stdout(command_id, 1, "ok")])
    }

    fn cancel_command(
        &self,
        _workspace: &Workspace,
        command_id: &str,
    ) -> KernelResult<CommandResult> {
        Ok(CommandResult {
            command_id: command_id.to_string(),
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            cancelled: true,
            timed_out: false,
        })
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

struct FakeVerificationProvider;

impl VerificationProvider for FakeVerificationProvider {
    fn discover_plans(&self, _workspace: &Workspace) -> KernelResult<Vec<VerificationPlan>> {
        Ok(vec![VerificationPlan::new("verify.rust", "workspace.1")])
    }

    fn run_verification(
        &self,
        _workspace: &Workspace,
        plan: VerificationPlan,
    ) -> KernelResult<VerificationReport> {
        Ok(
            VerificationReport::new("report.verify", plan.verification_id)
                .add_command_result(CommandResult::exited("cmd.cargo-test", 0, "ok", "")),
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

struct FakeLanguageProvider;

impl LanguageProvider for FakeLanguageProvider {
    fn diagnostics(
        &self,
        _workspace: &Workspace,
        _request: LanguageDiagnosticsRequest,
    ) -> KernelResult<Vec<LanguageDiagnostic>> {
        Ok(vec![LanguageDiagnostic::new(
            LanguageDiagnosticSeverity::Error,
            "src/lib.rs",
            7,
            "type mismatch",
        )])
    }

    fn symbols(
        &self,
        _workspace: &Workspace,
        _request: LanguageSymbolsRequest,
    ) -> KernelResult<Vec<LanguageSymbol>> {
        Ok(vec![LanguageSymbol::new(
            "run",
            LanguageSymbolKind::Function,
            "src/lib.rs",
            3,
        )])
    }

    fn format(
        &self,
        _workspace: &Workspace,
        request: LanguageFormatRequest,
    ) -> KernelResult<LanguageFormatResult> {
        Ok(LanguageFormatResult::unchanged(
            request.path,
            request.content,
        ))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

struct FakeReviewProvider;

impl ReviewProvider for FakeReviewProvider {
    fn review_patch(&self, _workspace: &Workspace, patch: &PatchSet) -> KernelResult<ReviewReport> {
        Ok(ReviewReport::new(
            format!("review.{}", patch.patch_id),
            patch.workspace_id.clone(),
        )
        .add_finding(ReviewFinding::new(
            "finding.1",
            ReviewSeverity::Medium,
            "src/lib.rs",
            12,
            "patch requires policy review",
        )))
    }

    fn review_verification(
        &self,
        _workspace: &Workspace,
        _report: &VerificationReport,
    ) -> KernelResult<ReviewReport> {
        Ok(ReviewReport::new("review.verification.1", "workspace.1"))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

struct FakeArtifactProvider;

impl sdkwork_code_kernel::ArtifactProvider for FakeArtifactProvider {
    fn put_artifact(
        &self,
        _workspace: &Workspace,
        artifact: CodeArtifact,
    ) -> KernelResult<ArtifactReceipt> {
        Ok(ArtifactReceipt::new(
            artifact.artifact_id.clone(),
            format!("memory://{}", artifact.artifact_id),
        ))
    }

    fn get_artifact(
        &self,
        _workspace: &Workspace,
        artifact_id: &str,
    ) -> KernelResult<CodeArtifact> {
        Ok(CodeArtifact::new(
            artifact_id,
            "workspace.1",
            CodeArtifactKind::ReviewReport,
            "Review report",
            "redacted",
        )
        .with_redaction(KernelEventRedaction::Internal))
    }

    fn list_artifacts(
        &self,
        _workspace: &Workspace,
        _filter: ArtifactFilter,
    ) -> KernelResult<Vec<ArtifactDescriptor>> {
        Ok(vec![ArtifactDescriptor::new(
            "artifact.review.1",
            "workspace.1",
            CodeArtifactKind::ReviewReport,
            "Review report",
        )])
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

struct FakeKnowledgeProvider;

impl KnowledgeProvider for FakeKnowledgeProvider {
    fn search_documents(
        &self,
        _workspace: &Workspace,
        _query: KnowledgeQuery,
    ) -> KernelResult<Vec<KnowledgeSearchResult>> {
        Ok(vec![KnowledgeSearchResult::new(
            "doc.kernel.1",
            KnowledgeDocumentKind::Spec,
            100,
            "Kernel spec",
        )])
    }

    fn get_document(
        &self,
        _workspace: &Workspace,
        document_id: &str,
    ) -> KernelResult<KnowledgeDocument> {
        Ok(KnowledgeDocument::new(
            document_id,
            KnowledgeDocumentKind::Spec,
            "kernel/specs/CODE_KERNEL_SPEC.md",
            "Code kernel spec",
            "provider-neutral code kernel",
        ))
    }

    fn list_documents(
        &self,
        _workspace: &Workspace,
        _filter: KnowledgeDocumentFilter,
    ) -> KernelResult<Vec<KnowledgeDocument>> {
        Ok(vec![KnowledgeDocument::new(
            "doc.kernel.1",
            KnowledgeDocumentKind::Spec,
            "kernel/specs/CODE_KERNEL_SPEC.md",
            "Code kernel spec",
            "provider-neutral code kernel",
        )])
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

struct FakeCodeSafetyProvider;

impl CodeSafetyProvider for FakeCodeSafetyProvider {
    fn assess_workspace(
        &self,
        _workspace: &Workspace,
        _scope: CodeSafetyScope,
    ) -> KernelResult<CodeSafetyAssessment> {
        Ok(CodeSafetyAssessment::new(
            "assessment.workspace.1",
            CodeSafetyRiskLevel::Medium,
            SideEffectLevel::ReadOnly,
        )
        .require_approval())
    }

    fn assess_patch(
        &self,
        _workspace: &Workspace,
        patch: &PatchSet,
    ) -> KernelResult<CodeSafetyAssessment> {
        Ok(CodeSafetyAssessment::new(
            "assessment.patch.1",
            CodeSafetyRiskLevel::High,
            patch.side_effect_level(),
        )
        .with_policy_categories(patch.policy_categories.clone()))
    }

    fn assess_terminal_command(
        &self,
        _workspace: &Workspace,
        command: &TerminalCommand,
    ) -> KernelResult<CodeSafetyAssessment> {
        Ok(CodeSafetyAssessment::new(
            "assessment.command.1",
            CodeSafetyRiskLevel::High,
            SideEffectLevel::SideEffectful,
        )
        .with_policy_categories(command.policy_categories.clone())
        .require_approval())
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}
