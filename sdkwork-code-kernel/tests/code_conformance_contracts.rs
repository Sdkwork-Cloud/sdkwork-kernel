use sdkwork_agent_kernel::{KernelResult, ProviderHealth, ProviderManifest};
use sdkwork_code_kernel::{
    CodeConformanceProfile, CodeKernelRuntimeBuilder, CodeSafetyAssessment, CodeSafetyProvider,
    CodeSafetyRiskLevel, CodeSafetyScope, Workspace, WorkspaceFile, WorkspaceProvider,
};

#[test]
fn manifest_conformance_passes_when_all_standard_provider_families_are_declared() {
    let runtime = CodeKernelRuntimeBuilder::new("code.runtime.manifest")
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
        .expect("runtime bootstraps");

    let report = runtime.conformance_report(CodeConformanceProfile::Manifest);

    assert_eq!(report.profile, CodeConformanceProfile::Manifest);
    assert_eq!(report.runtime_id, "code.runtime.manifest");
    assert!(report.passed);
    assert_eq!(report.failed_case_ids(), Vec::<String>::new());
    assert!(
        report
            .case("code.conformance.standard_provider_families.complete")
            .expect("provider family case exists")
            .passed
    );
    assert!(
        report
            .case("code.conformance.standard_capabilities.namespaced")
            .expect("capability namespace case exists")
            .passed
    );
    assert!(report
        .case("code.conformance.local_providers.typed")
        .is_none());
}

#[test]
fn local_runtime_conformance_reports_manifest_only_missing_and_degraded_providers() {
    let runtime = CodeKernelRuntimeBuilder::new("code.runtime.local.partial")
        .register_workspace_provider(
            "provider.code.workspace.typed",
            "0.1.0",
            HealthyWorkspaceProvider,
        )
        .register_knowledge_provider_manifest("provider.code.knowledge.manifest", "0.1.0")
        .register_safety_provider(
            "provider.code.safety.typed",
            "0.1.0",
            DegradedSafetyProvider,
        )
        .bootstrap()
        .expect("runtime bootstraps");

    let report = runtime.conformance_report(CodeConformanceProfile::LocalRuntime);

    assert_eq!(report.profile, CodeConformanceProfile::LocalRuntime);
    assert_eq!(report.runtime_id, "code.runtime.local.partial");
    assert!(!report.passed);
    assert_eq!(
        report.failed_case_ids(),
        [
            "code.conformance.standard_provider_families.complete",
            "code.conformance.local_providers.typed",
            "code.conformance.local_providers.health_available"
        ]
    );
    assert_eq!(
        report
            .case("code.conformance.standard_provider_families.complete")
            .expect("provider family case exists")
            .message,
        "missing standard provider families: code_vcs, code_patch, code_terminal, code_verification, code_language, code_review, code_artifact"
    );
    assert_eq!(
        report
            .case("code.conformance.local_providers.typed")
            .expect("typed provider case exists")
            .message,
        "manifest-only providers cannot satisfy local runtime conformance: provider.code.knowledge.manifest"
    );
    assert_eq!(
        report
            .case("code.conformance.local_providers.health_available")
            .expect("health case exists")
            .message,
        "typed providers with non-available health: provider.code.safety.typed"
    );
}

#[test]
fn manifest_conformance_reports_incomplete_standard_capability_coverage() {
    let runtime = CodeKernelRuntimeBuilder::new("code.runtime.partial-capabilities")
        .register_provider(ProviderManifest::new(
            "provider.code.workspace.partial",
            "code_workspace",
            "provider.code.workspace.partial",
            "0.1.0",
            vec!["code.workspace.read".to_string()],
        ))
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
        .expect("runtime bootstraps");

    let report = runtime.conformance_report(CodeConformanceProfile::Manifest);

    assert!(!report.passed);
    assert_eq!(
        report.failed_case_ids(),
        ["code.conformance.standard_capabilities.complete"]
    );
    assert_eq!(
        report
            .case("code.conformance.standard_capabilities.complete")
            .expect("capability coverage case exists")
            .message,
        "missing standard capabilities: code_workspace=code.workspace.list, code.workspace.write, code.workspace.stat, code.workspace.watch"
    );
}

#[test]
fn manifest_conformance_rejects_code_capabilities_that_are_not_lowercase_namespaces() {
    let runtime = CodeKernelRuntimeBuilder::new("code.runtime.invalid-capability")
        .register_provider(ProviderManifest::new(
            "provider.code.workspace.invalid",
            "code_workspace",
            "provider.code.workspace.invalid",
            "0.1.0",
            vec!["code.Workspace.Read".to_string()],
        ))
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
        .expect("runtime bootstraps");

    let report = runtime.conformance_report(CodeConformanceProfile::Manifest);

    assert!(!report.passed);
    let namespace_case = report
        .case("code.conformance.standard_capabilities.namespaced")
        .expect("capability namespace case exists");
    assert!(!namespace_case.passed);
    assert!(namespace_case.message.contains("code.Workspace.Read"));
}

struct HealthyWorkspaceProvider;

impl WorkspaceProvider for HealthyWorkspaceProvider {
    fn list_files(
        &self,
        _workspace: &Workspace,
        _root: &str,
    ) -> KernelResult<Vec<sdkwork_code_kernel::WorkspaceFileEntry>> {
        Ok(vec![sdkwork_code_kernel::WorkspaceFileEntry::file(
            "src/lib.rs",
        )])
    }

    fn read_file(&self, _workspace: &Workspace, path: &str) -> KernelResult<WorkspaceFile> {
        Ok(WorkspaceFile::with_content(path, "pub fn main() {}"))
    }

    fn write_file(
        &self,
        _workspace: &Workspace,
        request: sdkwork_code_kernel::WorkspaceWriteRequest,
    ) -> KernelResult<sdkwork_code_kernel::WorkspaceWriteResult> {
        Ok(sdkwork_code_kernel::WorkspaceWriteResult::written(
            request.path,
            request.content.len(),
        ))
    }

    fn stat_file(
        &self,
        _workspace: &Workspace,
        path: &str,
    ) -> KernelResult<sdkwork_code_kernel::WorkspaceFileStat> {
        Ok(sdkwork_code_kernel::WorkspaceFileStat::file(path, 16))
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

struct DegradedSafetyProvider;

impl CodeSafetyProvider for DegradedSafetyProvider {
    fn assess_workspace(
        &self,
        _workspace: &Workspace,
        _scope: CodeSafetyScope,
    ) -> KernelResult<CodeSafetyAssessment> {
        Ok(CodeSafetyAssessment::new(
            "assessment.workspace.1",
            CodeSafetyRiskLevel::Low,
            sdkwork_agent_kernel::SideEffectLevel::ReadOnly,
        ))
    }

    fn assess_patch(
        &self,
        _workspace: &Workspace,
        patch: &sdkwork_code_kernel::PatchSet,
    ) -> KernelResult<CodeSafetyAssessment> {
        Ok(CodeSafetyAssessment::new(
            "assessment.patch.1",
            CodeSafetyRiskLevel::Medium,
            patch.side_effect_level(),
        ))
    }

    fn assess_terminal_command(
        &self,
        _workspace: &Workspace,
        _command: &sdkwork_code_kernel::TerminalCommand,
    ) -> KernelResult<CodeSafetyAssessment> {
        Ok(CodeSafetyAssessment::new(
            "assessment.command.1",
            CodeSafetyRiskLevel::High,
            sdkwork_agent_kernel::SideEffectLevel::SideEffectful,
        ))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth {
            status: "degraded".to_string(),
        }
    }
}
