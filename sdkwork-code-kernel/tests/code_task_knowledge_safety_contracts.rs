use sdkwork_agent_kernel::{KernelEventRedaction, KernelResult, ProviderHealth, SideEffectLevel};
use sdkwork_code_kernel::{
    CodeCheckpoint, CodeKernelCapability, CodePlan, CodePlanStep, CodeReviewStatus,
    CodeSafetyAssessment, CodeSafetyProvider, CodeSafetyRiskLevel, CodeSafetyScope, CodeTask,
    CodeTaskIntent, CodeTaskState, CodeTraceRef, KnowledgeDocument, KnowledgeDocumentFilter,
    KnowledgeDocumentKind, KnowledgeProvider, KnowledgeQuery, KnowledgeSearchResult,
    PatchOperation, PatchSet, TerminalCommand, Workspace,
};

#[test]
fn code_task_preserves_intent_plan_checkpoints_review_and_trace() {
    let workspace = Workspace::new("workspace.1", "workspace");
    let task = CodeTask::new(
        "code.task.1",
        workspace.clone(),
        CodeTaskIntent::new("Fix failing Rust test").with_context_path("src/lib.rs"),
    )
    .with_plan(
        CodePlan::new("plan.1")
            .add_step(
                CodePlanStep::new("step.1", CodeKernelCapability::WorkspaceRead.as_str())
                    .with_summary("Inspect failing source"),
            )
            .add_step(
                CodePlanStep::new("step.2", CodeKernelCapability::PatchApply.as_str())
                    .with_summary("Apply minimal patch")
                    .requires_policy(),
            ),
    )
    .add_checkpoint(
        CodeCheckpoint::new("checkpoint.1", "before patch")
            .with_artifact_id("artifact.diff.1")
            .with_vcs_revision("abc123"),
    )
    .with_review_status(CodeReviewStatus::Required)
    .add_trace_ref(CodeTraceRef::new("trace.1", "run.1"));

    assert_eq!(task.task_id, "code.task.1");
    assert_eq!(task.workspace.workspace_id, workspace.workspace_id);
    assert_eq!(task.state, CodeTaskState::Created);
    assert_eq!(task.intent.context_paths, ["src/lib.rs"]);
    assert!(task.plan.as_ref().expect("plan exists").steps[1].policy_required);
    assert_eq!(task.checkpoints[0].artifact_ids, ["artifact.diff.1"]);
    assert_eq!(task.review_status, CodeReviewStatus::Required);
    assert_eq!(task.trace_refs[0].run_id, "run.1");

    let running = task
        .transition(CodeTaskState::Running)
        .expect("task can start running");
    assert_eq!(running.state, CodeTaskState::Running);
}

#[test]
fn knowledge_provider_returns_repository_docs_specs_and_generated_contracts() {
    let provider = FakeKnowledgeProvider;
    let workspace = Workspace::new("workspace.1", "workspace");

    let results = provider
        .search_documents(
            &workspace,
            KnowledgeQuery::new("sdk contract")
                .with_kind(KnowledgeDocumentKind::GeneratedContract)
                .include_external(),
        )
        .expect("knowledge search returns results");
    assert_eq!(results[0].document_id, "doc.openapi.1");
    assert_eq!(results[0].score, 98);

    let document = provider
        .get_document(&workspace, "doc.openapi.1")
        .expect("knowledge document loads");
    assert_eq!(document.kind, KnowledgeDocumentKind::GeneratedContract);
    assert!(document.redaction_classification.is_sensitive());

    let documents = provider
        .list_documents(
            &workspace,
            KnowledgeDocumentFilter::new().with_kind(KnowledgeDocumentKind::Adr),
        )
        .expect("knowledge documents list");
    assert_eq!(documents[0].kind, KnowledgeDocumentKind::Adr);
    assert_eq!(provider.health(), ProviderHealth::available());
}

#[test]
fn safety_provider_assesses_workspace_patch_and_terminal_risk() {
    let provider = FakeCodeSafetyProvider;
    let workspace = Workspace::new("workspace.1", "workspace");

    let workspace_assessment = provider
        .assess_workspace(
            &workspace,
            CodeSafetyScope::new("workspace")
                .allow_path("src")
                .deny_path(".env"),
        )
        .expect("workspace safety assessment loads");
    assert_eq!(workspace_assessment.risk_level, CodeSafetyRiskLevel::Medium);
    assert!(workspace_assessment.requires_approval);

    let patch = PatchSet::new("patch.1", "workspace.1", "Edit generated client")
        .add_operation(PatchOperation::update_file(
            "generated/client.ts",
            "old",
            "new",
        ))
        .with_policy_categories(vec![CodeKernelCapability::PatchApply.as_str().to_string()]);
    let patch_assessment = provider
        .assess_patch(&workspace, &patch)
        .expect("patch safety assessment loads");
    assert_eq!(
        patch_assessment.side_effect_level,
        SideEffectLevel::SideEffectful
    );
    assert_eq!(patch_assessment.policy_categories, ["code.patch.apply"]);

    let command = TerminalCommand::new("cmd.test", "cargo", vec!["test".to_string()], "workspace")
        .with_policy_categories(vec![CodeKernelCapability::TerminalRun.as_str().to_string()]);
    let command_assessment = provider
        .assess_terminal_command(&workspace, &command)
        .expect("terminal safety assessment loads");
    assert_eq!(command_assessment.risk_level, CodeSafetyRiskLevel::High);
    assert_eq!(provider.health(), ProviderHealth::available());
}

struct FakeKnowledgeProvider;

impl KnowledgeProvider for FakeKnowledgeProvider {
    fn search_documents(
        &self,
        _workspace: &Workspace,
        _query: KnowledgeQuery,
    ) -> KernelResult<Vec<KnowledgeSearchResult>> {
        Ok(vec![KnowledgeSearchResult::new(
            "doc.openapi.1",
            KnowledgeDocumentKind::GeneratedContract,
            98,
            "Generated OpenAPI contract",
        )])
    }

    fn get_document(
        &self,
        _workspace: &Workspace,
        document_id: &str,
    ) -> KernelResult<KnowledgeDocument> {
        Ok(KnowledgeDocument::new(
            document_id,
            KnowledgeDocumentKind::GeneratedContract,
            "apps/api/openapi.json",
            "Generated OpenAPI contract",
            "{}",
        )
        .with_redaction(KernelEventRedaction::Internal)
        .with_tag("generated"))
    }

    fn list_documents(
        &self,
        _workspace: &Workspace,
        _filter: KnowledgeDocumentFilter,
    ) -> KernelResult<Vec<KnowledgeDocument>> {
        Ok(vec![KnowledgeDocument::new(
            "doc.adr.1",
            KnowledgeDocumentKind::Adr,
            "docs/adr/0001.md",
            "ADR 0001",
            "Use SDKWork kernel SPI",
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
        scope: CodeSafetyScope,
    ) -> KernelResult<CodeSafetyAssessment> {
        Ok(CodeSafetyAssessment::new(
            "assessment.workspace.1",
            CodeSafetyRiskLevel::Medium,
            SideEffectLevel::ReadOnly,
        )
        .with_reason(format!("denied_paths={}", scope.denied_paths.len()))
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
        .with_policy_categories(patch.policy_categories.clone())
        .with_reason("generated client edit requires review"))
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
