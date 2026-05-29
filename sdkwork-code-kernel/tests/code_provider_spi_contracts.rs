use sdkwork_agent_kernel::{HostEnvPolicy, KernelEventRedaction, KernelResult, ProviderHealth};
use sdkwork_code_kernel::{
    ArtifactDescriptor, ArtifactFilter, ArtifactProvider, ArtifactReceipt, CodeArtifact,
    CodeArtifactKind, LanguageDiagnostic, LanguageDiagnosticSeverity, LanguageDiagnosticsRequest,
    LanguageFormatRequest, LanguageFormatResult, LanguageProvider, LanguageSymbol,
    LanguageSymbolKind, LanguageSymbolsRequest, PatchOperation, PatchSet, ReviewFinding,
    ReviewProvider, ReviewReport, ReviewSeverity, TerminalCommand, TerminalOutputChannel,
    TerminalOutputChunk, TerminalProvider, VerificationPlan, VerificationProvider,
    VerificationReport, Workspace,
};

#[test]
fn verification_provider_discovers_plans_runs_evidence_and_reports_health() {
    let provider = FakeVerificationProvider;
    let workspace = Workspace::new("workspace.1", "workspace");

    let plans = provider
        .discover_plans(&workspace)
        .expect("verification plans discovered");
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].verification_id, "verify.rust");
    assert_eq!(plans[0].commands[0].command, "cargo");

    let report = provider
        .run_verification(&workspace, plans[0].clone())
        .expect("verification runs");
    assert!(report.is_success());
    assert_eq!(report.command_results[0].command_id, "cmd.cargo-test");
    assert_eq!(provider.health(), ProviderHealth::available());
}

#[test]
fn terminal_provider_runs_streams_and_cancels_policy_bound_commands() {
    let provider = FakeTerminalProvider;
    let workspace = Workspace::new("workspace.1", "workspace");
    let command = TerminalCommand::new(
        "cmd.cargo-test",
        "cargo",
        vec!["test".to_string()],
        "workspace",
    )
    .with_timeout_ms(30_000)
    .with_env_policy(HostEnvPolicy::AllowList(vec!["PATH".to_string()]))
    .with_policy_categories(vec!["code.terminal.run".to_string()]);

    assert!(command.requires_policy());

    let result = provider
        .run_command(&workspace, command.clone())
        .expect("terminal command runs");
    assert!(result.is_success());

    let chunks = provider
        .stream_output(&workspace, "cmd.cargo-test")
        .expect("terminal output streams");
    assert_eq!(chunks[0].sequence, 1);
    assert_eq!(chunks[0].channel, TerminalOutputChannel::Stdout);
    assert_eq!(chunks[0].content, "running 1 test");

    let cancelled = provider
        .cancel_command(&workspace, "cmd.long-running")
        .expect("terminal command cancels");
    assert!(cancelled.cancelled);
    assert_eq!(provider.health(), ProviderHealth::available());
}

#[test]
fn language_provider_normalizes_diagnostics_symbols_and_formatting() {
    let provider = FakeLanguageProvider;
    let workspace = Workspace::new("workspace.1", "workspace");

    let diagnostics = provider
        .diagnostics(
            &workspace,
            LanguageDiagnosticsRequest::new("src/lib.rs").with_language_id("rust"),
        )
        .expect("diagnostics returned");
    assert_eq!(diagnostics[0].severity, LanguageDiagnosticSeverity::Error);
    assert_eq!(diagnostics[0].line, 7);
    assert_eq!(diagnostics[0].source.as_deref(), Some("rustc"));

    let symbols = provider
        .symbols(&workspace, LanguageSymbolsRequest::new("src/lib.rs"))
        .expect("symbols returned");
    assert_eq!(symbols[0].kind, LanguageSymbolKind::Function);
    assert_eq!(symbols[0].name, "run");

    let formatted = provider
        .format(
            &workspace,
            LanguageFormatRequest::new("src/lib.rs", "fn main(){println!(\"hi\");}"),
        )
        .expect("formatting returned");
    assert!(formatted.changed);
    assert!(formatted.formatted_content.contains("fn main()"));
}

#[test]
fn review_provider_returns_structured_patch_and_verification_reports() {
    let provider = FakeReviewProvider;
    let workspace = Workspace::new("workspace.1", "workspace");
    let patch = PatchSet::new("patch.1", "workspace.1", "Edit lib.rs")
        .add_operation(PatchOperation::update_file("src/lib.rs", "old", "new"));
    let verification_report =
        VerificationReport::new("report.verify", "verify.rust").add_failure("missing lint run");

    let patch_review = provider
        .review_patch(&workspace, &patch)
        .expect("patch review returned");
    assert_eq!(patch_review.report_id, "review.patch.1");
    assert_eq!(patch_review.findings[0].severity, ReviewSeverity::Medium);
    assert_eq!(
        patch_review.risk_summary.as_deref(),
        Some("policy review required")
    );

    let verification_review = provider
        .review_verification(&workspace, &verification_report)
        .expect("verification review returned");
    assert_eq!(
        verification_review.missing_tests,
        ["lint evidence must be attached"]
    );
}

#[test]
fn artifact_provider_stores_retrieves_and_lists_redacted_kernel_artifacts() {
    let provider = FakeArtifactProvider;
    let workspace = Workspace::new("workspace.1", "workspace");
    let artifact = CodeArtifact::new(
        "artifact.review.1",
        "workspace.1",
        CodeArtifactKind::ReviewReport,
        "Review report",
        "high risk command output redacted",
    )
    .with_mime_type("text/markdown")
    .with_redaction(KernelEventRedaction::Internal)
    .with_retention_policy("session");

    let receipt = provider
        .put_artifact(&workspace, artifact.clone())
        .expect("artifact stored");
    assert_eq!(receipt.artifact_id, "artifact.review.1");
    assert_eq!(
        receipt.storage_ref.as_deref(),
        Some("memory://artifact.review.1")
    );

    let loaded = provider
        .get_artifact(&workspace, "artifact.review.1")
        .expect("artifact loaded");
    assert_eq!(loaded.kind, CodeArtifactKind::ReviewReport);
    assert!(loaded.redaction_classification.is_sensitive());

    let descriptors = provider
        .list_artifacts(
            &workspace,
            ArtifactFilter::new().with_kind(CodeArtifactKind::ReviewReport),
        )
        .expect("artifacts listed");
    assert_eq!(descriptors[0].artifact_id, "artifact.review.1");
    assert_eq!(descriptors[0].title, "Review report");
}

struct FakeVerificationProvider;

impl VerificationProvider for FakeVerificationProvider {
    fn discover_plans(&self, _workspace: &Workspace) -> KernelResult<Vec<VerificationPlan>> {
        Ok(vec![VerificationPlan::new("verify.rust", "workspace.1")
            .add_command(sdkwork_code_kernel::TerminalCommand::new(
                "cmd.cargo-test",
                "cargo",
                vec!["test".to_string()],
                "workspace",
            ))])
    }

    fn run_verification(
        &self,
        _workspace: &Workspace,
        plan: VerificationPlan,
    ) -> KernelResult<VerificationReport> {
        Ok(
            VerificationReport::new("report.verify", plan.verification_id).add_command_result(
                sdkwork_code_kernel::CommandResult::exited("cmd.cargo-test", 0, "ok", ""),
            ),
        )
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
    ) -> KernelResult<sdkwork_code_kernel::CommandResult> {
        Ok(sdkwork_code_kernel::CommandResult::exited(
            command.command_id,
            0,
            "ok",
            "",
        ))
    }

    fn stream_output(
        &self,
        _workspace: &Workspace,
        command_id: &str,
    ) -> KernelResult<Vec<TerminalOutputChunk>> {
        Ok(vec![TerminalOutputChunk::stdout(
            command_id,
            1,
            "running 1 test",
        )])
    }

    fn cancel_command(
        &self,
        _workspace: &Workspace,
        command_id: &str,
    ) -> KernelResult<sdkwork_code_kernel::CommandResult> {
        Ok(sdkwork_code_kernel::CommandResult {
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
        )
        .with_source("rustc")
        .with_code("E0308")])
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
        Ok(LanguageFormatResult::changed(
            request.path,
            "fn main() {\n  println!(\"hi\");\n}\n",
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
        .add_finding(
            ReviewFinding::new(
                "finding.1",
                ReviewSeverity::Medium,
                "src/lib.rs",
                12,
                "patch requires policy review",
            )
            .with_remediation("Attach policy decision before apply"),
        )
        .with_risk_summary("policy review required"))
    }

    fn review_verification(
        &self,
        _workspace: &Workspace,
        _report: &VerificationReport,
    ) -> KernelResult<ReviewReport> {
        Ok(ReviewReport::new("review.verification.1", "workspace.1")
            .add_missing_test("lint evidence must be attached"))
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
            "high risk command output redacted",
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
