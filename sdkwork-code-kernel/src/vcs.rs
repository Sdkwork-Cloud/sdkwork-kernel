use crate::Workspace;
use sdkwork_agent_kernel::{
    KernelResult, PolicyCategory, PolicyRequest, ProviderHealth, SideEffectLevel,
};

pub trait VcsProvider {
    fn snapshot(&self, workspace: &Workspace) -> KernelResult<VcsSnapshot>;

    fn diff(&self, workspace: &Workspace, request: VcsDiffRequest) -> KernelResult<VcsDiff>;

    fn blame(&self, workspace: &Workspace, path: &str) -> KernelResult<Vec<VcsBlameLine>>;

    fn commit_metadata(
        &self,
        workspace: &Workspace,
        revision: &str,
    ) -> KernelResult<VcsCommitMetadata>;

    fn restore(
        &self,
        workspace: &Workspace,
        request: VcsRestoreRequest,
    ) -> KernelResult<VcsRestoreReport>;

    fn health(&self) -> ProviderHealth;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VcsSnapshot {
    pub branch: String,
    pub head_revision: Option<String>,
    pub dirty: bool,
    pub changed_files: Vec<String>,
    pub summary: String,
}

impl VcsSnapshot {
    pub fn new(branch: impl Into<String>) -> Self {
        Self {
            branch: branch.into(),
            head_revision: None,
            dirty: false,
            changed_files: Vec::new(),
            summary: String::new(),
        }
    }

    pub fn with_head_revision(mut self, head_revision: impl Into<String>) -> Self {
        self.head_revision = Some(head_revision.into());
        self
    }

    pub fn with_changed_files(mut self, changed_files: Vec<String>) -> Self {
        self.changed_files = changed_files;
        self
    }

    pub fn mark_dirty(mut self) -> Self {
        self.dirty = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VcsDiffRequest {
    pub base_revision: Option<String>,
    pub head_revision: Option<String>,
    pub paths: Vec<String>,
    pub include_untracked: bool,
}

impl VcsDiffRequest {
    pub fn new() -> Self {
        Self {
            base_revision: None,
            head_revision: None,
            paths: Vec::new(),
            include_untracked: false,
        }
    }

    pub fn with_base_revision(mut self, base_revision: impl Into<String>) -> Self {
        self.base_revision = Some(base_revision.into());
        self
    }

    pub fn with_head_revision(mut self, head_revision: impl Into<String>) -> Self {
        self.head_revision = Some(head_revision.into());
        self
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.paths.push(path.into());
        self
    }

    pub fn include_untracked(mut self) -> Self {
        self.include_untracked = true;
        self
    }
}

impl Default for VcsDiffRequest {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcsFileChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Untracked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VcsDiffFile {
    pub path: String,
    pub old_path: Option<String>,
    pub change_kind: VcsFileChangeKind,
    pub additions: u32,
    pub deletions: u32,
    pub patch: Option<String>,
}

impl VcsDiffFile {
    pub fn new(path: impl Into<String>, change_kind: VcsFileChangeKind) -> Self {
        Self {
            path: path.into(),
            old_path: None,
            change_kind,
            additions: 0,
            deletions: 0,
            patch: None,
        }
    }

    pub fn modified(path: impl Into<String>, additions: u32, deletions: u32) -> Self {
        Self {
            path: path.into(),
            old_path: None,
            change_kind: VcsFileChangeKind::Modified,
            additions,
            deletions,
            patch: None,
        }
    }

    pub fn renamed(
        old_path: impl Into<String>,
        path: impl Into<String>,
        additions: u32,
        deletions: u32,
    ) -> Self {
        Self {
            path: path.into(),
            old_path: Some(old_path.into()),
            change_kind: VcsFileChangeKind::Renamed,
            additions,
            deletions,
            patch: None,
        }
    }

    pub fn with_patch(mut self, patch: impl Into<String>) -> Self {
        self.patch = Some(patch.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VcsDiff {
    pub diff_id: String,
    pub files: Vec<VcsDiffFile>,
    pub summary: String,
}

impl VcsDiff {
    pub fn new(diff_id: impl Into<String>) -> Self {
        Self {
            diff_id: diff_id.into(),
            files: Vec::new(),
            summary: String::new(),
        }
    }

    pub fn add_file(mut self, file: VcsDiffFile) -> Self {
        self.files.push(file);
        self
    }

    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = summary.into();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VcsBlameLine {
    pub path: String,
    pub line: u32,
    pub revision: String,
    pub author: String,
    pub summary: String,
}

impl VcsBlameLine {
    pub fn new(
        path: impl Into<String>,
        line: u32,
        revision: impl Into<String>,
        author: impl Into<String>,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            line,
            revision: revision.into(),
            author: author.into(),
            summary: summary.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VcsCommitMetadata {
    pub revision: String,
    pub author: String,
    pub message: String,
    pub timestamp: Option<String>,
}

impl VcsCommitMetadata {
    pub fn new(
        revision: impl Into<String>,
        author: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            revision: revision.into(),
            author: author.into(),
            message: message.into(),
            timestamp: None,
        }
    }

    pub fn with_timestamp(mut self, timestamp: impl Into<String>) -> Self {
        self.timestamp = Some(timestamp.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VcsRestoreRequest {
    pub paths: Vec<String>,
    pub revision: Option<String>,
    pub policy_categories: Vec<String>,
}

impl VcsRestoreRequest {
    pub fn new(paths: Vec<String>) -> Self {
        Self {
            paths,
            revision: None,
            policy_categories: Vec::new(),
        }
    }

    pub fn with_revision(mut self, revision: impl Into<String>) -> Self {
        self.revision = Some(revision.into());
        self
    }

    pub fn with_policy_categories(mut self, policy_categories: Vec<String>) -> Self {
        self.policy_categories = policy_categories;
        self
    }

    pub fn requires_policy(&self) -> bool {
        true
    }

    pub fn to_policy_request(
        &self,
        policy_request_id: impl Into<String>,
        workspace: &Workspace,
    ) -> PolicyRequest {
        let category = first_policy_category(&self.policy_categories, "code.vcs.restore");
        let mut request = PolicyRequest::new(
            policy_request_id,
            category.clone(),
            format!("workspace://{}/vcs/restore", workspace.workspace_id),
        )
        .with_category(PolicyCategory::ProductSpecific(category))
        .with_action("vcs.restore")
        .with_side_effect_level(SideEffectLevel::Destructive)
        .with_context("workspace_id", workspace.workspace_id.clone())
        .with_context("paths", self.paths.join(","));

        if let Some(revision) = &self.revision {
            request = request.with_context("revision", revision.clone());
        }

        if !self.policy_categories.is_empty() {
            request = request.with_context("policy_categories", self.policy_categories.join(","));
        }

        request
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VcsRestoreReport {
    pub restored_paths: Vec<String>,
    pub skipped_paths: Vec<String>,
    pub revision: Option<String>,
}

impl VcsRestoreReport {
    pub fn restored(paths: Vec<String>) -> Self {
        Self {
            restored_paths: paths,
            skipped_paths: Vec::new(),
            revision: None,
        }
    }

    pub fn with_revision(mut self, revision: impl Into<String>) -> Self {
        self.revision = Some(revision.into());
        self
    }

    pub fn with_skipped_paths(mut self, skipped_paths: Vec<String>) -> Self {
        self.skipped_paths = skipped_paths;
        self
    }
}

fn first_policy_category(policy_categories: &[String], fallback: &str) -> String {
    policy_categories
        .first()
        .cloned()
        .unwrap_or_else(|| fallback.to_string())
}
