use sdkwork_agent_kernel::{HostEnvPolicy, KernelEventRedaction, PolicyCategory, SideEffectLevel};
use sdkwork_code_kernel::{
    CodeArtifact, CodeArtifactKind, PatchOperation, PatchSet, TerminalCommand, VcsRestoreRequest,
    VerificationPlan, Workspace, WorkspaceWriteRequest,
};

#[test]
fn code_side_effects_build_standard_policy_requests() {
    let workspace = Workspace::new("workspace.1", "workspace");

    let write = WorkspaceWriteRequest::new("src/lib.rs", "pub fn main() {}\n")
        .with_expected_version("abc123")
        .with_policy_categories(vec!["code.workspace.write".to_string()]);
    let write_policy = write.to_policy_request("policy.workspace.write.1", &workspace);
    assert_eq!(write_policy.category, "code.workspace.write");
    assert_eq!(
        write_policy.typed_category,
        Some(PolicyCategory::ProductSpecific(
            "code.workspace.write".to_string()
        ))
    );
    assert_eq!(write_policy.resource, "workspace://workspace.1/src/lib.rs");
    assert_eq!(write_policy.action.as_deref(), Some("workspace.write"));
    assert_eq!(
        write_policy.side_effect_level,
        Some(SideEffectLevel::SideEffectful)
    );
    assert_eq!(
        write_policy.context_value("workspace_id"),
        Some("workspace.1")
    );
    assert_eq!(write_policy.context_value("path"), Some("src/lib.rs"));
    assert_eq!(
        write_policy.context_value("expected_version"),
        Some("abc123")
    );
    assert_eq!(
        write_policy.context_value("policy_categories"),
        Some("code.workspace.write")
    );

    let patch = PatchSet::new("patch.1", "workspace.1", "Remove generated file")
        .add_operation(PatchOperation::delete_file("generated/client.ts"))
        .with_policy_categories(vec!["code.patch.apply".to_string()]);
    let patch_policy = patch.apply_policy_request("policy.patch.apply.1");
    assert_eq!(patch_policy.category, "code.patch.apply");
    assert_eq!(
        patch_policy.resource,
        "workspace://workspace.1/patches/patch.1"
    );
    assert_eq!(patch_policy.action.as_deref(), Some("patch.apply"));
    assert_eq!(
        patch_policy.side_effect_level,
        Some(SideEffectLevel::Destructive)
    );
    assert_eq!(patch_policy.context_value("patch_id"), Some("patch.1"));
    assert_eq!(
        patch_policy.context_value("affected_files"),
        Some("generated/client.ts")
    );

    let restore = VcsRestoreRequest::new(vec!["src/lib.rs".to_string()])
        .with_revision("HEAD")
        .with_policy_categories(vec!["code.vcs.restore".to_string()]);
    let restore_policy = restore.to_policy_request("policy.vcs.restore.1", &workspace);
    assert_eq!(restore_policy.category, "code.vcs.restore");
    assert_eq!(
        restore_policy.side_effect_level,
        Some(SideEffectLevel::Destructive)
    );
    assert_eq!(restore_policy.action.as_deref(), Some("vcs.restore"));
    assert_eq!(restore_policy.context_value("paths"), Some("src/lib.rs"));
    assert_eq!(restore_policy.context_value("revision"), Some("HEAD"));

    let command = TerminalCommand::new("cmd.test", "cargo", vec!["test".to_string()], "workspace")
        .with_timeout_ms(60_000)
        .with_env_policy(HostEnvPolicy::AllowList(vec!["PATH".to_string()]))
        .with_policy_categories(vec!["code.terminal.run".to_string()]);
    let command_policy = command.to_policy_request("policy.terminal.run.1", &workspace);
    assert_eq!(command_policy.category, "code.terminal.run");
    assert_eq!(
        command_policy.side_effect_level,
        Some(SideEffectLevel::SideEffectful)
    );
    assert_eq!(command_policy.action.as_deref(), Some("terminal.run"));
    assert_eq!(command_policy.context_value("command_id"), Some("cmd.test"));
    assert_eq!(command_policy.context_value("command"), Some("cargo"));
    assert_eq!(command_policy.context_value("args"), Some("test"));
    assert_eq!(
        command_policy.context_value("working_directory"),
        Some("workspace")
    );
    assert_eq!(command_policy.context_value("timeout_ms"), Some("60000"));

    let verification = VerificationPlan::new("verify.rust", "workspace.1").add_command(command);
    let verification_policy = verification.to_policy_request("policy.verification.run.1");
    assert_eq!(verification_policy.category, "code.verification.run");
    assert_eq!(
        verification_policy.side_effect_level,
        Some(SideEffectLevel::SideEffectful)
    );
    assert_eq!(
        verification_policy.action.as_deref(),
        Some("verification.run")
    );
    assert_eq!(
        verification_policy.resource,
        "workspace://workspace.1/verifications/verify.rust"
    );
    assert_eq!(
        verification_policy.context_value("command_count"),
        Some("1")
    );

    let artifact = CodeArtifact::new(
        "artifact.review.1",
        "workspace.1",
        CodeArtifactKind::ReviewReport,
        "Review report",
        "redacted",
    )
    .with_redaction(KernelEventRedaction::Internal);
    let artifact_policy = artifact.write_policy_request("policy.artifact.write.1");
    assert_eq!(artifact_policy.category, "code.artifact.write");
    assert_eq!(
        artifact_policy.side_effect_level,
        Some(SideEffectLevel::SideEffectful)
    );
    assert_eq!(artifact_policy.action.as_deref(), Some("artifact.write"));
    assert_eq!(
        artifact_policy.resource,
        "workspace://workspace.1/artifacts/artifact.review.1"
    );
    assert_eq!(
        artifact_policy.redaction_classification,
        KernelEventRedaction::Internal
    );
}
