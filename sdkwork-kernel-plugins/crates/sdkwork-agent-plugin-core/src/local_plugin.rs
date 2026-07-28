use std::path::PathBuf;

/// A provider-neutral, manifest-only view of a plugin discovered on disk.
/// Discovery never loads executable plugin code; execution remains behind the
/// typed skill/MCP provider SPIs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalPluginDescriptor {
    pub plugin_id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub root_path: PathBuf,
    pub manifest_path: PathBuf,
    pub source: LocalPluginSource,
    pub status: LocalPluginStatus,
    pub skills: Vec<LocalPluginSkillDescriptor>,
    pub mcp_servers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalPluginSkillDescriptor {
    pub skill_id: String,
    pub name: String,
    pub description: Option<String>,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalPluginSource {
    User,
    Repository,
    Workspace,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalPluginStatus {
    ManifestOnly,
    TypedLocalProvider,
    ProcessAdapter,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalPluginLoadErrorKind {
    SourceUnavailable,
    InvalidManifest,
    InvalidSkill,
    PermissionDenied,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalPluginLoadError {
    pub provider_id: String,
    pub path: Option<PathBuf>,
    pub kind: LocalPluginLoadErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LocalPluginCatalog {
    pub provider_id: String,
    pub plugins: Vec<LocalPluginDescriptor>,
    pub errors: Vec<LocalPluginLoadError>,
}

impl LocalPluginCatalog {
    pub fn new(provider_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            plugins: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn is_partial(&self) -> bool {
        !self.plugins.is_empty() && !self.errors.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LocalPluginDiscoveryRequest {
    pub roots: Vec<PathBuf>,
    pub include_disabled: bool,
}

/// Provider-specific local plugin discovery SPI.
pub trait LocalPluginProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    fn discover(&self, request: &LocalPluginDiscoveryRequest) -> LocalPluginCatalog;
}
