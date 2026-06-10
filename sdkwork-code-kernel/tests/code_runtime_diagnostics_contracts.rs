use sdkwork_agent_kernel::{KernelResult, ProviderHealth};
use sdkwork_code_kernel::{
    CodeKernelRuntimeBuilder, CodeSafetyAssessment, CodeSafetyProvider, CodeSafetyRiskLevel,
    CodeSafetyScope, Workspace, WorkspaceFile, WorkspaceProvider,
};

#[test]
fn runtime_diagnostics_report_typed_manifest_only_and_missing_provider_state() {
    let runtime = CodeKernelRuntimeBuilder::new("code.runtime.diagnostics")
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

    let diagnostics = runtime.diagnostics();

    assert_eq!(diagnostics.runtime_id, "code.runtime.diagnostics");
    assert_eq!(diagnostics.provider_count, 3);
    assert_eq!(diagnostics.capability_count, 9);
    assert_eq!(diagnostics.typed_provider_count, 2);
    assert_eq!(diagnostics.manifest_only_provider_count, 1);
    assert!(diagnostics.is_degraded());

    let workspace = diagnostics
        .provider("provider.code.workspace.typed")
        .expect("workspace provider diagnostic exists");
    assert_eq!(workspace.provider_family, "code_workspace");
    assert!(workspace.typed_registered);
    assert_eq!(workspace.health, Some(ProviderHealth::available()));
    assert_eq!(
        workspace.capabilities,
        [
            "code.workspace.list",
            "code.workspace.read",
            "code.workspace.write",
            "code.workspace.stat",
            "code.workspace.watch"
        ]
    );

    let knowledge = diagnostics
        .provider("provider.code.knowledge.manifest")
        .expect("knowledge provider diagnostic exists");
    assert_eq!(knowledge.provider_family, "code_knowledge");
    assert!(!knowledge.typed_registered);
    assert_eq!(knowledge.health, None);
    assert_eq!(
        knowledge.capabilities,
        [
            "code.knowledge.search",
            "code.knowledge.read",
            "code.knowledge.list"
        ]
    );
    assert_eq!(
        diagnostics.manifest_only_provider_ids(),
        ["provider.code.knowledge.manifest"]
    );

    let safety = diagnostics
        .provider("provider.code.safety.typed")
        .expect("safety provider diagnostic exists");
    assert_eq!(safety.provider_family, "code_safety");
    assert!(safety.typed_registered);
    assert_eq!(
        safety.health,
        Some(ProviderHealth {
            status: "degraded".to_string()
        })
    );

    assert_eq!(
        diagnostics.missing_standard_provider_families(),
        [
            "code_vcs",
            "code_patch",
            "code_terminal",
            "code_verification",
            "code_language",
            "code_review",
            "code_artifact"
        ]
    );
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
