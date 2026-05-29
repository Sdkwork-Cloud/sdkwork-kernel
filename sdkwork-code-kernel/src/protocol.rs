use crate::{CodeArtifact, CodeKernelEvent, CodeSession, CodeTask, PatchSet};
use sdkwork_agent_kernel::{
    KernelResult, ProtocolFamily, ProtocolObjectEnvelope, ProtocolObjectKind,
};

pub trait CodeProtocolObjectMapper {
    fn map_session(&self, session: &CodeSession) -> KernelResult<ProtocolObjectEnvelope>;

    fn map_task(&self, task: &CodeTask) -> KernelResult<ProtocolObjectEnvelope>;

    fn map_patch(&self, patch: &PatchSet) -> KernelResult<ProtocolObjectEnvelope>;

    fn map_artifact(&self, artifact: &CodeArtifact) -> KernelResult<ProtocolObjectEnvelope>;

    fn map_event(&self, event: &CodeKernelEvent) -> KernelResult<ProtocolObjectEnvelope>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StandardCodeProtocolObjectMapper {
    protocol: ProtocolFamily,
}

impl StandardCodeProtocolObjectMapper {
    pub fn new(protocol: ProtocolFamily) -> Self {
        Self { protocol }
    }
}

impl CodeProtocolObjectMapper for StandardCodeProtocolObjectMapper {
    fn map_session(&self, session: &CodeSession) -> KernelResult<ProtocolObjectEnvelope> {
        let envelope = ProtocolObjectEnvelope::new(
            self.protocol,
            ProtocolObjectKind::ExtensionObject,
            session.session_id.clone(),
            format!(
                "session_id={};workspace_id={};state={};tasks={};provider_bindings={}",
                session.session_id,
                session.workspace.workspace_id,
                session.state.as_str(),
                session.tasks.len(),
                session.provider_bindings.len()
            ),
        )
        .with_schema("sdkwork.code.session.v1")
        .with_metadata("sdkwork.code.object_kind", "code_session")
        .with_metadata(
            "sdkwork.code.workspace_id",
            session.workspace.workspace_id.clone(),
        )
        .with_metadata("sdkwork.code.session_state", session.state.as_str())
        .with_metadata("sdkwork.code.tasks", session.tasks.len().to_string())
        .with_metadata(
            "sdkwork.code.provider_bindings",
            session.provider_bindings.len().to_string(),
        );

        envelope.validate()?;
        Ok(envelope)
    }

    fn map_task(&self, task: &CodeTask) -> KernelResult<ProtocolObjectEnvelope> {
        let envelope = ProtocolObjectEnvelope::new(
            self.protocol,
            ProtocolObjectKind::ExtensionObject,
            task.task_id.clone(),
            format!(
                "task_id={};workspace_id={};state={};plan_steps={};checkpoints={}",
                task.task_id,
                task.workspace.workspace_id,
                task.state.as_str(),
                task.plan.as_ref().map(|plan| plan.steps.len()).unwrap_or(0),
                task.checkpoints.len()
            ),
        )
        .with_schema("sdkwork.code.task.v1")
        .with_metadata("sdkwork.code.object_kind", "code_task")
        .with_metadata(
            "sdkwork.code.workspace_id",
            task.workspace.workspace_id.clone(),
        )
        .with_metadata("sdkwork.code.task_state", task.state.as_str())
        .with_metadata("sdkwork.code.review_status", task.review_status.as_str())
        .with_metadata(
            "sdkwork.code.context_paths",
            task.intent.context_paths.join(","),
        )
        .with_metadata(
            "sdkwork.code.constraints",
            task.intent.constraints.len().to_string(),
        );

        envelope.validate()?;
        Ok(envelope)
    }

    fn map_patch(&self, patch: &PatchSet) -> KernelResult<ProtocolObjectEnvelope> {
        let envelope = ProtocolObjectEnvelope::new(
            self.protocol,
            ProtocolObjectKind::ExtensionObject,
            patch.patch_id.clone(),
            format!(
                "patch_id={};workspace_id={};operations={};affected_files={}",
                patch.patch_id,
                patch.workspace_id,
                patch.operations.len(),
                patch.affected_files().len()
            ),
        )
        .with_schema("sdkwork.code.patch.v1")
        .with_metadata("sdkwork.code.object_kind", "patch_set")
        .with_metadata("sdkwork.code.workspace_id", patch.workspace_id.clone())
        .with_metadata("sdkwork.code.patch_id", patch.patch_id.clone())
        .with_metadata(
            "sdkwork.code.side_effect_level",
            patch.side_effect_level().as_str(),
        )
        .with_metadata(
            "sdkwork.code.affected_files",
            patch.affected_files().join(","),
        );

        envelope.validate()?;
        Ok(envelope)
    }

    fn map_artifact(&self, artifact: &CodeArtifact) -> KernelResult<ProtocolObjectEnvelope> {
        let envelope = ProtocolObjectEnvelope::new(
            self.protocol,
            ProtocolObjectKind::ExtensionObject,
            artifact.artifact_id.clone(),
            format!(
                "artifact_id={};workspace_id={};kind={};title={}",
                artifact.artifact_id,
                artifact.workspace_id,
                artifact.kind.as_str(),
                artifact.title
            ),
        )
        .with_schema("sdkwork.code.artifact.v1")
        .with_metadata("sdkwork.code.object_kind", "code_artifact")
        .with_metadata("sdkwork.code.workspace_id", artifact.workspace_id.clone())
        .with_metadata("sdkwork.code.artifact_id", artifact.artifact_id.clone())
        .with_metadata("sdkwork.code.artifact_kind", artifact.kind.as_str())
        .with_redaction(artifact.redaction_classification);

        envelope.validate()?;
        Ok(envelope)
    }

    fn map_event(&self, event: &CodeKernelEvent) -> KernelResult<ProtocolObjectEnvelope> {
        let kernel_event = event.to_kernel_event();
        let mut envelope = ProtocolObjectEnvelope::new(
            self.protocol,
            ProtocolObjectKind::KernelEvent,
            kernel_event.event_id,
            kernel_event.payload,
        )
        .with_schema("sdkwork.code.event.v1")
        .with_metadata("sdkwork.event.type", kernel_event.event_type)
        .with_metadata("sdkwork.event.version", kernel_event.event_version)
        .with_metadata("sdkwork.code.workspace_id", event.workspace_id.clone())
        .with_redaction(kernel_event.redaction_classification);

        if let Some(session_id) = &event.session_id {
            envelope = envelope.with_metadata("sdkwork.agent.session_id", session_id.clone());
        }

        if let Some(task_id) = &event.task_id {
            envelope = envelope.with_metadata("sdkwork.agent.task_id", task_id.clone());
        }

        if let Some(artifact_id) = &event.artifact_id {
            envelope = envelope.with_metadata("sdkwork.code.artifact_id", artifact_id.clone());
        }

        envelope.validate()?;
        Ok(envelope)
    }
}
