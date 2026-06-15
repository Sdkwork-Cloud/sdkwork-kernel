use crate::first_policy_category;
use crate::Workspace;
use sdkwork_agent_kernel::{
    KernelResult, PolicyCategory, PolicyRequest, ProviderHealth, SideEffectLevel,
};

pub trait PatchProvider {
    fn validate_patch(&self, workspace: &Workspace, patch: &PatchSet) -> KernelResult<()>;

    fn preview_patch(&self, workspace: &Workspace, patch: &PatchSet) -> KernelResult<PatchPreview>;

    fn apply_patch(&self, workspace: &Workspace, patch: PatchSet)
        -> KernelResult<PatchApplyResult>;

    fn reject_patch(
        &self,
        workspace: &Workspace,
        patch_id: &str,
        reason: &str,
    ) -> KernelResult<PatchRejection>;

    fn rollback_patch(
        &self,
        workspace: &Workspace,
        rollback_token: &str,
    ) -> KernelResult<PatchRollbackResult>;

    fn explain_patch(
        &self,
        workspace: &Workspace,
        patch: &PatchSet,
    ) -> KernelResult<PatchExplanation>;

    fn health(&self) -> ProviderHealth;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchSet {
    pub patch_id: String,
    pub workspace_id: String,
    pub summary: String,
    pub operations: Vec<PatchOperation>,
    pub policy_categories: Vec<String>,
}

impl PatchSet {
    pub fn new(
        patch_id: impl Into<String>,
        workspace_id: impl Into<String>,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            patch_id: patch_id.into(),
            workspace_id: workspace_id.into(),
            summary: summary.into(),
            operations: Vec::new(),
            policy_categories: Vec::new(),
        }
    }

    pub fn add_operation(mut self, operation: PatchOperation) -> Self {
        self.operations.push(operation);
        self
    }

    pub fn with_policy_categories(mut self, policy_categories: Vec<String>) -> Self {
        self.policy_categories = policy_categories;
        self
    }

    pub fn side_effect_level(&self) -> SideEffectLevel {
        if self.operations.is_empty() {
            SideEffectLevel::ReadOnly
        } else if self
            .operations
            .iter()
            .any(|operation| matches!(operation, PatchOperation::DeleteFile { .. }))
        {
            SideEffectLevel::Destructive
        } else {
            SideEffectLevel::SideEffectful
        }
    }

    pub fn requires_policy(&self) -> bool {
        self.side_effect_level() != SideEffectLevel::ReadOnly || !self.policy_categories.is_empty()
    }

    pub fn apply_policy_request(&self, policy_request_id: impl Into<String>) -> PolicyRequest {
        let category = first_policy_category(&self.policy_categories, "code.patch.apply");
        let mut request = PolicyRequest::new(
            policy_request_id,
            category.clone(),
            format!(
                "workspace://{}/patches/{}",
                self.workspace_id, self.patch_id
            ),
        )
        .with_category(PolicyCategory::ProductSpecific(category))
        .with_action("patch.apply")
        .with_side_effect_level(self.side_effect_level())
        .with_context("workspace_id", self.workspace_id.clone())
        .with_context("patch_id", self.patch_id.clone())
        .with_context("summary", self.summary.clone())
        .with_context("operation_count", self.operations.len().to_string())
        .with_context("affected_files", self.affected_files().join(","));

        if !self.policy_categories.is_empty() {
            request = request.with_context("policy_categories", self.policy_categories.join(","));
        }

        request
    }

    pub fn rollback_policy_request(
        policy_request_id: impl Into<String>,
        workspace_id: impl Into<String>,
        rollback_token: impl Into<String>,
    ) -> PolicyRequest {
        let workspace_id = workspace_id.into();
        let rollback_token = rollback_token.into();

        PolicyRequest::new(
            policy_request_id,
            "code.patch.rollback",
            format!("workspace://{workspace_id}/patch-rollbacks/{rollback_token}"),
        )
        .with_category(PolicyCategory::ProductSpecific(
            "code.patch.rollback".to_string(),
        ))
        .with_action("patch.rollback")
        .with_side_effect_level(SideEffectLevel::Destructive)
        .with_context("workspace_id", workspace_id)
        .with_context("rollback_token", rollback_token)
    }

    pub fn affected_files(&self) -> Vec<String> {
        let mut affected_files = Vec::new();
        for operation in &self.operations {
            let path = operation.path();
            if !affected_files.iter().any(|affected| affected == path) {
                affected_files.push(path.to_string());
            }
        }

        affected_files
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchOperation {
    CreateFile {
        path: String,
        content: String,
    },
    UpdateFile {
        path: String,
        before: String,
        after: String,
    },
    DeleteFile {
        path: String,
    },
}

impl PatchOperation {
    pub fn create_file(path: impl Into<String>, content: impl Into<String>) -> Self {
        Self::CreateFile {
            path: path.into(),
            content: content.into(),
        }
    }

    pub fn update_file(
        path: impl Into<String>,
        before: impl Into<String>,
        after: impl Into<String>,
    ) -> Self {
        Self::UpdateFile {
            path: path.into(),
            before: before.into(),
            after: after.into(),
        }
    }

    pub fn delete_file(path: impl Into<String>) -> Self {
        Self::DeleteFile { path: path.into() }
    }

    pub fn path(&self) -> &str {
        match self {
            Self::CreateFile { path, .. }
            | Self::UpdateFile { path, .. }
            | Self::DeleteFile { path } => path,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchPreview {
    pub patch_id: String,
    pub affected_files: Vec<String>,
    pub summary: String,
    pub requires_policy: bool,
}

impl PatchPreview {
    pub fn from_patch(patch: &PatchSet) -> Self {
        Self {
            patch_id: patch.patch_id.clone(),
            affected_files: patch.affected_files(),
            summary: patch.summary.clone(),
            requires_policy: patch.requires_policy(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchApplyResult {
    pub patch_id: String,
    pub status: String,
    pub rollback_token: Option<String>,
}

impl PatchApplyResult {
    pub fn applied(patch_id: impl Into<String>, rollback_token: impl Into<String>) -> Self {
        Self {
            patch_id: patch_id.into(),
            status: "applied".to_string(),
            rollback_token: Some(rollback_token.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchRejection {
    pub patch_id: String,
    pub reason: String,
    pub status: String,
}

impl PatchRejection {
    pub fn rejected(patch_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            patch_id: patch_id.into(),
            reason: reason.into(),
            status: "rejected".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchRollbackResult {
    pub rollback_token: String,
    pub status: String,
    pub restored_files: Vec<String>,
}

impl PatchRollbackResult {
    pub fn rolled_back(rollback_token: impl Into<String>, restored_files: Vec<String>) -> Self {
        Self {
            rollback_token: rollback_token.into(),
            status: "rolled_back".to_string(),
            restored_files,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchExplanation {
    pub patch_id: String,
    pub summary: String,
    pub risk_notes: Vec<String>,
    pub policy_categories: Vec<String>,
}

impl PatchExplanation {
    pub fn new(patch_id: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            patch_id: patch_id.into(),
            summary: summary.into(),
            risk_notes: Vec::new(),
            policy_categories: Vec::new(),
        }
    }

    pub fn add_risk_note(mut self, risk_note: impl Into<String>) -> Self {
        self.risk_notes.push(risk_note.into());
        self
    }

    pub fn with_policy_categories(mut self, policy_categories: Vec<String>) -> Self {
        self.policy_categories = policy_categories;
        self
    }
}
