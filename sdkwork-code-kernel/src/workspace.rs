use crate::first_policy_category;
use sdkwork_agent_kernel::{
    KernelResult, PolicyCategory, PolicyRequest, ProviderHealth, SideEffectLevel,
};

pub trait WorkspaceProvider {
    fn list_files(
        &self,
        workspace: &Workspace,
        root: &str,
    ) -> KernelResult<Vec<WorkspaceFileEntry>>;

    fn read_file(&self, workspace: &Workspace, path: &str) -> KernelResult<WorkspaceFile>;

    fn write_file(
        &self,
        workspace: &Workspace,
        request: WorkspaceWriteRequest,
    ) -> KernelResult<WorkspaceWriteResult>;

    fn stat_file(&self, workspace: &Workspace, path: &str) -> KernelResult<WorkspaceFileStat>;

    fn watch_events(
        &self,
        workspace: &Workspace,
        since_cursor: Option<&str>,
    ) -> KernelResult<Vec<WorkspaceWatchEvent>>;

    fn health(&self) -> ProviderHealth;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneratedFilePolicy {
    Allow,
    Protected,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub workspace_id: String,
    pub root: String,
    pub trust_level: String,
    pub generated_file_policy: GeneratedFilePolicy,
    pub language_hints: Vec<String>,
}

impl Workspace {
    pub fn new(workspace_id: impl Into<String>, root: impl Into<String>) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            root: root.into(),
            trust_level: "untrusted".to_string(),
            generated_file_policy: GeneratedFilePolicy::Protected,
            language_hints: Vec::new(),
        }
    }

    pub fn with_trust_level(mut self, trust_level: impl Into<String>) -> Self {
        self.trust_level = trust_level.into();
        self
    }

    pub fn with_generated_file_policy(
        mut self,
        generated_file_policy: GeneratedFilePolicy,
    ) -> Self {
        self.generated_file_policy = generated_file_policy;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceFileKind {
    File,
    Directory,
    Symlink,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceFileEntry {
    pub path: String,
    pub kind: WorkspaceFileKind,
    pub generated: bool,
    pub readonly: bool,
}

impl WorkspaceFileEntry {
    pub fn new(path: impl Into<String>, kind: WorkspaceFileKind) -> Self {
        Self {
            path: path.into(),
            kind,
            generated: false,
            readonly: false,
        }
    }

    pub fn file(path: impl Into<String>) -> Self {
        Self::new(path, WorkspaceFileKind::File)
    }

    pub fn directory(path: impl Into<String>) -> Self {
        Self::new(path, WorkspaceFileKind::Directory)
    }

    pub fn with_generated(mut self, generated: bool) -> Self {
        self.generated = generated;
        self
    }

    pub fn with_readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }

    pub fn is_file(&self) -> bool {
        self.kind == WorkspaceFileKind::File
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceFile {
    pub path: String,
    pub content: Option<String>,
    pub generated: bool,
    pub readonly: bool,
}

impl WorkspaceFile {
    pub fn with_content(path: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            content: Some(content.into()),
            generated: false,
            readonly: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceWriteRequest {
    pub path: String,
    pub content: String,
    pub expected_version: Option<String>,
    pub create_parent_dirs: bool,
    pub overwrite: bool,
    pub policy_categories: Vec<String>,
}

impl WorkspaceWriteRequest {
    pub fn new(path: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            content: content.into(),
            expected_version: None,
            create_parent_dirs: false,
            overwrite: true,
            policy_categories: Vec::new(),
        }
    }

    pub fn with_expected_version(mut self, expected_version: impl Into<String>) -> Self {
        self.expected_version = Some(expected_version.into());
        self
    }

    pub fn create_parent_dirs(mut self) -> Self {
        self.create_parent_dirs = true;
        self
    }

    pub fn no_overwrite(mut self) -> Self {
        self.overwrite = false;
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
        let category = first_policy_category(&self.policy_categories, "code.workspace.write");
        let mut request = PolicyRequest::new(
            policy_request_id,
            category.clone(),
            format!("workspace://{}/{}", workspace.workspace_id, self.path),
        )
        .with_category(PolicyCategory::ProductSpecific(category))
        .with_action("workspace.write")
        .with_side_effect_level(SideEffectLevel::SideEffectful)
        .with_context("workspace_id", workspace.workspace_id.clone())
        .with_context("path", self.path.clone())
        .with_context("create_parent_dirs", self.create_parent_dirs.to_string())
        .with_context("overwrite", self.overwrite.to_string())
        .with_context("content_bytes", self.content.len().to_string());

        if let Some(expected_version) = &self.expected_version {
            request = request.with_context("expected_version", expected_version.clone());
        }

        if !self.policy_categories.is_empty() {
            request = request.with_context("policy_categories", self.policy_categories.join(","));
        }

        request
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceWriteResult {
    pub path: String,
    pub version: Option<String>,
    pub bytes_written: usize,
    pub created: bool,
}

impl WorkspaceWriteResult {
    pub fn written(path: impl Into<String>, bytes_written: usize) -> Self {
        Self {
            path: path.into(),
            version: None,
            bytes_written,
            created: false,
        }
    }

    pub fn created(path: impl Into<String>, bytes_written: usize) -> Self {
        Self {
            path: path.into(),
            version: None,
            bytes_written,
            created: true,
        }
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceFileStat {
    pub path: String,
    pub kind: WorkspaceFileKind,
    pub size_bytes: Option<u64>,
    pub version: Option<String>,
    pub generated: bool,
    pub readonly: bool,
}

impl WorkspaceFileStat {
    pub fn file(path: impl Into<String>, size_bytes: u64) -> Self {
        Self {
            path: path.into(),
            kind: WorkspaceFileKind::File,
            size_bytes: Some(size_bytes),
            version: None,
            generated: false,
            readonly: false,
        }
    }

    pub fn directory(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            kind: WorkspaceFileKind::Directory,
            size_bytes: None,
            version: None,
            generated: false,
            readonly: false,
        }
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    pub fn with_generated(mut self, generated: bool) -> Self {
        self.generated = generated;
        self
    }

    pub fn with_readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceWatchEventKind {
    Created,
    Modified,
    Deleted,
    Renamed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceWatchEvent {
    pub event_id: String,
    pub path: String,
    pub kind: WorkspaceWatchEventKind,
    pub cursor: String,
    pub previous_path: Option<String>,
}

impl WorkspaceWatchEvent {
    pub fn new(
        event_id: impl Into<String>,
        path: impl Into<String>,
        kind: WorkspaceWatchEventKind,
        cursor: impl Into<String>,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            path: path.into(),
            kind,
            cursor: cursor.into(),
            previous_path: None,
        }
    }

    pub fn renamed(
        event_id: impl Into<String>,
        previous_path: impl Into<String>,
        path: impl Into<String>,
        cursor: impl Into<String>,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            path: path.into(),
            kind: WorkspaceWatchEventKind::Renamed,
            cursor: cursor.into(),
            previous_path: Some(previous_path.into()),
        }
    }
}
