use crate::{
    ArtifactProvider, CodeConformanceProfile, CodeConformanceReport, CodeKernelCapability,
    CodeSafetyProvider, KnowledgeProvider, LanguageProvider, PatchProvider, ReviewProvider,
    TerminalProvider, VcsProvider, VerificationProvider, WorkspaceProvider,
    CODE_KERNEL_SPEC_VERSION,
};
use sdkwork_agent_kernel::{
    Capability, KernelError, KernelResult, ProviderHealth, ProviderManifest, SideEffectLevel,
};
use std::sync::Arc;

const CODE_WORKSPACE_PROVIDER_FAMILY: &str = "code_workspace";
const CODE_VCS_PROVIDER_FAMILY: &str = "code_vcs";
const CODE_PATCH_PROVIDER_FAMILY: &str = "code_patch";
const CODE_TERMINAL_PROVIDER_FAMILY: &str = "code_terminal";
const CODE_VERIFICATION_PROVIDER_FAMILY: &str = "code_verification";
const CODE_LANGUAGE_PROVIDER_FAMILY: &str = "code_language";
const CODE_REVIEW_PROVIDER_FAMILY: &str = "code_review";
const CODE_ARTIFACT_PROVIDER_FAMILY: &str = "code_artifact";
const CODE_KNOWLEDGE_PROVIDER_FAMILY: &str = "code_knowledge";
const CODE_SAFETY_PROVIDER_FAMILY: &str = "code_safety";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeKernelCapabilityManifest {
    pub schema_version: String,
    pub manifest_type: String,
    pub runtime_id: String,
    pub kernel_version: String,
    pub providers: Vec<ProviderManifest>,
    pub capabilities: Vec<Capability>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeKernelRuntime {
    capability_manifest: CodeKernelCapabilityManifest,
    provider_registry: CodeKernelProviderRegistry,
}

impl CodeKernelRuntime {
    pub fn capability_manifest(&self) -> &CodeKernelCapabilityManifest {
        &self.capability_manifest
    }

    pub fn diagnostics(&self) -> CodeKernelRuntimeDiagnostics {
        let provider_diagnostics: Vec<CodeProviderDiagnostic> = self
            .capability_manifest
            .providers
            .iter()
            .map(|provider| {
                let typed_registered = self.provider_registry.has_typed_provider(provider);

                CodeProviderDiagnostic {
                    provider_id: provider.provider_id.clone(),
                    provider_family: provider.provider_family.clone(),
                    provider_version: provider.version.clone(),
                    typed_registered,
                    health: self.provider_registry.health_for_provider(provider),
                    capabilities: provider.capabilities.clone(),
                }
            })
            .collect();

        let typed_provider_count = provider_diagnostics
            .iter()
            .filter(|provider| provider.typed_registered)
            .count();

        CodeKernelRuntimeDiagnostics {
            runtime_id: self.capability_manifest.runtime_id.clone(),
            provider_count: self.capability_manifest.providers.len(),
            capability_count: self.capability_manifest.capabilities.len(),
            typed_provider_count,
            manifest_only_provider_count: self
                .capability_manifest
                .providers
                .len()
                .saturating_sub(typed_provider_count),
            provider_diagnostics,
        }
    }

    pub fn conformance_report(&self, profile: CodeConformanceProfile) -> CodeConformanceReport {
        CodeConformanceReport::from_manifest_and_diagnostics(
            profile,
            &self.capability_manifest,
            &self.diagnostics(),
        )
    }

    pub fn workspace_provider(&self) -> KernelResult<&(dyn WorkspaceProvider + Send + Sync)> {
        self.provider_registry
            .workspace_provider
            .as_deref()
            .ok_or_else(|| {
                self.provider_error_for_family(
                    CODE_WORKSPACE_PROVIDER_FAMILY,
                    CodeKernelCapability::WorkspaceRead.as_str(),
                )
            })
    }

    pub fn vcs_provider(&self) -> KernelResult<&(dyn VcsProvider + Send + Sync)> {
        self.provider_registry
            .vcs_provider
            .as_deref()
            .ok_or_else(|| {
                self.provider_error_for_family(
                    CODE_VCS_PROVIDER_FAMILY,
                    CodeKernelCapability::VcsStatus.as_str(),
                )
            })
    }

    pub fn patch_provider(&self) -> KernelResult<&(dyn PatchProvider + Send + Sync)> {
        self.provider_registry
            .patch_provider
            .as_deref()
            .ok_or_else(|| {
                self.provider_error_for_family(
                    CODE_PATCH_PROVIDER_FAMILY,
                    CodeKernelCapability::PatchValidate.as_str(),
                )
            })
    }

    pub fn terminal_provider(&self) -> KernelResult<&(dyn TerminalProvider + Send + Sync)> {
        self.provider_registry
            .terminal_provider
            .as_deref()
            .ok_or_else(|| {
                self.provider_error_for_family(
                    CODE_TERMINAL_PROVIDER_FAMILY,
                    CodeKernelCapability::TerminalRun.as_str(),
                )
            })
    }

    pub fn verification_provider(&self) -> KernelResult<&(dyn VerificationProvider + Send + Sync)> {
        self.provider_registry
            .verification_provider
            .as_deref()
            .ok_or_else(|| {
                self.provider_error_for_family(
                    CODE_VERIFICATION_PROVIDER_FAMILY,
                    CodeKernelCapability::VerificationRun.as_str(),
                )
            })
    }

    pub fn language_provider(&self) -> KernelResult<&(dyn LanguageProvider + Send + Sync)> {
        self.provider_registry
            .language_provider
            .as_deref()
            .ok_or_else(|| {
                self.provider_error_for_family(
                    CODE_LANGUAGE_PROVIDER_FAMILY,
                    CodeKernelCapability::LanguageDiagnostics.as_str(),
                )
            })
    }

    pub fn review_provider(&self) -> KernelResult<&(dyn ReviewProvider + Send + Sync)> {
        self.provider_registry
            .review_provider
            .as_deref()
            .ok_or_else(|| {
                self.provider_error_for_family(
                    CODE_REVIEW_PROVIDER_FAMILY,
                    CodeKernelCapability::ReviewProduce.as_str(),
                )
            })
    }

    pub fn artifact_provider(&self) -> KernelResult<&(dyn ArtifactProvider + Send + Sync)> {
        self.provider_registry
            .artifact_provider
            .as_deref()
            .ok_or_else(|| {
                self.provider_error_for_family(
                    CODE_ARTIFACT_PROVIDER_FAMILY,
                    CodeKernelCapability::ArtifactRead.as_str(),
                )
            })
    }

    pub fn knowledge_provider(&self) -> KernelResult<&(dyn KnowledgeProvider + Send + Sync)> {
        self.provider_registry
            .knowledge_provider
            .as_deref()
            .ok_or_else(|| {
                self.provider_error_for_family(
                    CODE_KNOWLEDGE_PROVIDER_FAMILY,
                    CodeKernelCapability::KnowledgeSearch.as_str(),
                )
            })
    }

    pub fn safety_provider(&self) -> KernelResult<&(dyn CodeSafetyProvider + Send + Sync)> {
        self.provider_registry
            .safety_provider
            .as_deref()
            .ok_or_else(|| {
                self.provider_error_for_family(
                    CODE_SAFETY_PROVIDER_FAMILY,
                    CodeKernelCapability::SafetyAssess.as_str(),
                )
            })
    }

    fn provider_id_for_family(&self, provider_family: &str) -> Option<&str> {
        self.capability_manifest
            .providers
            .iter()
            .find(|provider| provider.provider_family == provider_family)
            .map(|provider| provider.provider_id.as_str())
    }

    fn provider_error_for_family(&self, provider_family: &str, capability_id: &str) -> KernelError {
        match self.provider_id_for_family(provider_family) {
            Some(provider_id) => KernelError::ProviderUnavailable {
                provider_id: provider_id.to_string(),
            },
            None => KernelError::CapabilityMissing {
                capability_id: capability_id.to_string(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeKernelRuntimeDiagnostics {
    pub runtime_id: String,
    pub provider_count: usize,
    pub capability_count: usize,
    pub typed_provider_count: usize,
    pub manifest_only_provider_count: usize,
    pub provider_diagnostics: Vec<CodeProviderDiagnostic>,
}

impl CodeKernelRuntimeDiagnostics {
    pub fn provider(&self, provider_id: &str) -> Option<&CodeProviderDiagnostic> {
        self.provider_diagnostics
            .iter()
            .find(|provider| provider.provider_id == provider_id)
    }

    pub fn manifest_only_provider_ids(&self) -> Vec<String> {
        self.provider_diagnostics
            .iter()
            .filter(|provider| !provider.typed_registered)
            .map(|provider| provider.provider_id.clone())
            .collect()
    }

    pub fn missing_standard_provider_families(&self) -> Vec<String> {
        standard_code_provider_families()
            .iter()
            .filter(|provider_family| {
                !self
                    .provider_diagnostics
                    .iter()
                    .any(|provider| provider.provider_family == **provider_family)
            })
            .map(|provider_family| (*provider_family).to_string())
            .collect()
    }

    pub fn is_degraded(&self) -> bool {
        self.manifest_only_provider_count > 0
            || !self.missing_standard_provider_families().is_empty()
            || self
                .provider_diagnostics
                .iter()
                .any(|provider| provider.health_is_degraded())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeProviderDiagnostic {
    pub provider_id: String,
    pub provider_family: String,
    pub provider_version: String,
    pub typed_registered: bool,
    pub health: Option<ProviderHealth>,
    pub capabilities: Vec<String>,
}

impl CodeProviderDiagnostic {
    pub fn health_is_degraded(&self) -> bool {
        self.health
            .as_ref()
            .is_some_and(|health| health.status != "available")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeKernelRuntimeBuilder {
    runtime_id: String,
    providers: Vec<ProviderManifest>,
    provider_registry: CodeKernelProviderRegistry,
}

impl CodeKernelRuntimeBuilder {
    pub fn new(runtime_id: impl Into<String>) -> Self {
        Self {
            runtime_id: runtime_id.into(),
            providers: Vec::new(),
            provider_registry: CodeKernelProviderRegistry::default(),
        }
    }

    pub fn register_provider(mut self, provider: ProviderManifest) -> Self {
        self.providers.push(provider);
        self
    }

    pub fn register_workspace_provider_manifest(
        self,
        provider_id: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        self.register_provider(code_provider_manifest(
            provider_id,
            CODE_WORKSPACE_PROVIDER_FAMILY,
            version,
            &[
                CodeKernelCapability::WorkspaceList,
                CodeKernelCapability::WorkspaceRead,
                CodeKernelCapability::WorkspaceWrite,
                CodeKernelCapability::WorkspaceStat,
                CodeKernelCapability::WorkspaceWatch,
            ],
        ))
    }

    pub fn register_workspace_provider<T>(
        mut self,
        provider_id: impl Into<String>,
        version: impl Into<String>,
        provider: T,
    ) -> Self
    where
        T: WorkspaceProvider + Send + Sync + 'static,
    {
        let provider_id = provider_id.into();
        let version = version.into();
        self.providers.push(code_provider_manifest(
            provider_id.clone(),
            CODE_WORKSPACE_PROVIDER_FAMILY,
            version,
            &[
                CodeKernelCapability::WorkspaceList,
                CodeKernelCapability::WorkspaceRead,
                CodeKernelCapability::WorkspaceWrite,
                CodeKernelCapability::WorkspaceStat,
                CodeKernelCapability::WorkspaceWatch,
            ],
        ));
        self.provider_registry.workspace_provider_id = Some(provider_id);
        self.provider_registry.workspace_provider = Some(Arc::new(provider));
        self
    }

    pub fn register_vcs_provider_manifest(
        self,
        provider_id: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        self.register_provider(code_provider_manifest(
            provider_id,
            CODE_VCS_PROVIDER_FAMILY,
            version,
            &[
                CodeKernelCapability::VcsStatus,
                CodeKernelCapability::VcsDiff,
                CodeKernelCapability::VcsBlame,
                CodeKernelCapability::VcsCommitMetadata,
                CodeKernelCapability::VcsRestore,
            ],
        ))
    }

    pub fn register_vcs_provider<T>(
        mut self,
        provider_id: impl Into<String>,
        version: impl Into<String>,
        provider: T,
    ) -> Self
    where
        T: VcsProvider + Send + Sync + 'static,
    {
        let provider_id = provider_id.into();
        let version = version.into();
        self.providers.push(code_provider_manifest(
            provider_id.clone(),
            CODE_VCS_PROVIDER_FAMILY,
            version,
            &[
                CodeKernelCapability::VcsStatus,
                CodeKernelCapability::VcsDiff,
                CodeKernelCapability::VcsBlame,
                CodeKernelCapability::VcsCommitMetadata,
                CodeKernelCapability::VcsRestore,
            ],
        ));
        self.provider_registry.vcs_provider_id = Some(provider_id);
        self.provider_registry.vcs_provider = Some(Arc::new(provider));
        self
    }

    pub fn register_patch_provider_manifest(
        self,
        provider_id: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        self.register_provider(code_provider_manifest(
            provider_id,
            CODE_PATCH_PROVIDER_FAMILY,
            version,
            &[
                CodeKernelCapability::PatchValidate,
                CodeKernelCapability::PatchPreview,
                CodeKernelCapability::PatchApply,
                CodeKernelCapability::PatchReject,
                CodeKernelCapability::PatchRollback,
                CodeKernelCapability::PatchExplain,
            ],
        ))
    }

    pub fn register_patch_provider<T>(
        mut self,
        provider_id: impl Into<String>,
        version: impl Into<String>,
        provider: T,
    ) -> Self
    where
        T: PatchProvider + Send + Sync + 'static,
    {
        let provider_id = provider_id.into();
        let version = version.into();
        self.providers.push(code_provider_manifest(
            provider_id.clone(),
            CODE_PATCH_PROVIDER_FAMILY,
            version,
            &[
                CodeKernelCapability::PatchValidate,
                CodeKernelCapability::PatchPreview,
                CodeKernelCapability::PatchApply,
                CodeKernelCapability::PatchReject,
                CodeKernelCapability::PatchRollback,
                CodeKernelCapability::PatchExplain,
            ],
        ));
        self.provider_registry.patch_provider_id = Some(provider_id);
        self.provider_registry.patch_provider = Some(Arc::new(provider));
        self
    }

    pub fn register_terminal_provider_manifest(
        self,
        provider_id: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        self.register_provider(code_provider_manifest(
            provider_id,
            CODE_TERMINAL_PROVIDER_FAMILY,
            version,
            &[CodeKernelCapability::TerminalRun],
        ))
    }

    pub fn register_terminal_provider<T>(
        mut self,
        provider_id: impl Into<String>,
        version: impl Into<String>,
        provider: T,
    ) -> Self
    where
        T: TerminalProvider + Send + Sync + 'static,
    {
        let provider_id = provider_id.into();
        let version = version.into();
        self.providers.push(code_provider_manifest(
            provider_id.clone(),
            CODE_TERMINAL_PROVIDER_FAMILY,
            version,
            &[CodeKernelCapability::TerminalRun],
        ));
        self.provider_registry.terminal_provider_id = Some(provider_id);
        self.provider_registry.terminal_provider = Some(Arc::new(provider));
        self
    }

    pub fn register_verification_provider_manifest(
        self,
        provider_id: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        self.register_provider(code_provider_manifest(
            provider_id,
            CODE_VERIFICATION_PROVIDER_FAMILY,
            version,
            &[CodeKernelCapability::VerificationRun],
        ))
    }

    pub fn register_verification_provider<T>(
        mut self,
        provider_id: impl Into<String>,
        version: impl Into<String>,
        provider: T,
    ) -> Self
    where
        T: VerificationProvider + Send + Sync + 'static,
    {
        let provider_id = provider_id.into();
        let version = version.into();
        self.providers.push(code_provider_manifest(
            provider_id.clone(),
            CODE_VERIFICATION_PROVIDER_FAMILY,
            version,
            &[CodeKernelCapability::VerificationRun],
        ));
        self.provider_registry.verification_provider_id = Some(provider_id);
        self.provider_registry.verification_provider = Some(Arc::new(provider));
        self
    }

    pub fn register_language_provider_manifest(
        self,
        provider_id: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        self.register_provider(code_provider_manifest(
            provider_id,
            CODE_LANGUAGE_PROVIDER_FAMILY,
            version,
            &[
                CodeKernelCapability::LanguageDiagnostics,
                CodeKernelCapability::LanguageSymbols,
                CodeKernelCapability::LanguageFormat,
            ],
        ))
    }

    pub fn register_language_provider<T>(
        mut self,
        provider_id: impl Into<String>,
        version: impl Into<String>,
        provider: T,
    ) -> Self
    where
        T: LanguageProvider + Send + Sync + 'static,
    {
        let provider_id = provider_id.into();
        let version = version.into();
        self.providers.push(code_provider_manifest(
            provider_id.clone(),
            CODE_LANGUAGE_PROVIDER_FAMILY,
            version,
            &[
                CodeKernelCapability::LanguageDiagnostics,
                CodeKernelCapability::LanguageSymbols,
                CodeKernelCapability::LanguageFormat,
            ],
        ));
        self.provider_registry.language_provider_id = Some(provider_id);
        self.provider_registry.language_provider = Some(Arc::new(provider));
        self
    }

    pub fn register_review_provider_manifest(
        self,
        provider_id: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        self.register_provider(code_provider_manifest(
            provider_id,
            CODE_REVIEW_PROVIDER_FAMILY,
            version,
            &[CodeKernelCapability::ReviewProduce],
        ))
    }

    pub fn register_review_provider<T>(
        mut self,
        provider_id: impl Into<String>,
        version: impl Into<String>,
        provider: T,
    ) -> Self
    where
        T: ReviewProvider + Send + Sync + 'static,
    {
        let provider_id = provider_id.into();
        let version = version.into();
        self.providers.push(code_provider_manifest(
            provider_id.clone(),
            CODE_REVIEW_PROVIDER_FAMILY,
            version,
            &[CodeKernelCapability::ReviewProduce],
        ));
        self.provider_registry.review_provider_id = Some(provider_id);
        self.provider_registry.review_provider = Some(Arc::new(provider));
        self
    }

    pub fn register_artifact_provider_manifest(
        self,
        provider_id: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        self.register_provider(code_provider_manifest(
            provider_id,
            CODE_ARTIFACT_PROVIDER_FAMILY,
            version,
            &[
                CodeKernelCapability::ArtifactRead,
                CodeKernelCapability::ArtifactWrite,
            ],
        ))
    }

    pub fn register_artifact_provider<T>(
        mut self,
        provider_id: impl Into<String>,
        version: impl Into<String>,
        provider: T,
    ) -> Self
    where
        T: ArtifactProvider + Send + Sync + 'static,
    {
        let provider_id = provider_id.into();
        let version = version.into();
        self.providers.push(code_provider_manifest(
            provider_id.clone(),
            CODE_ARTIFACT_PROVIDER_FAMILY,
            version,
            &[
                CodeKernelCapability::ArtifactRead,
                CodeKernelCapability::ArtifactWrite,
            ],
        ));
        self.provider_registry.artifact_provider_id = Some(provider_id);
        self.provider_registry.artifact_provider = Some(Arc::new(provider));
        self
    }

    pub fn register_knowledge_provider_manifest(
        self,
        provider_id: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        self.register_provider(code_provider_manifest(
            provider_id,
            CODE_KNOWLEDGE_PROVIDER_FAMILY,
            version,
            &[
                CodeKernelCapability::KnowledgeSearch,
                CodeKernelCapability::KnowledgeRead,
            ],
        ))
    }

    pub fn register_knowledge_provider<T>(
        mut self,
        provider_id: impl Into<String>,
        version: impl Into<String>,
        provider: T,
    ) -> Self
    where
        T: KnowledgeProvider + Send + Sync + 'static,
    {
        let provider_id = provider_id.into();
        let version = version.into();
        self.providers.push(code_provider_manifest(
            provider_id.clone(),
            CODE_KNOWLEDGE_PROVIDER_FAMILY,
            version,
            &[
                CodeKernelCapability::KnowledgeSearch,
                CodeKernelCapability::KnowledgeRead,
            ],
        ));
        self.provider_registry.knowledge_provider_id = Some(provider_id);
        self.provider_registry.knowledge_provider = Some(Arc::new(provider));
        self
    }

    pub fn register_safety_provider_manifest(
        self,
        provider_id: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        self.register_provider(code_provider_manifest(
            provider_id,
            CODE_SAFETY_PROVIDER_FAMILY,
            version,
            &[CodeKernelCapability::SafetyAssess],
        ))
    }

    pub fn register_safety_provider<T>(
        mut self,
        provider_id: impl Into<String>,
        version: impl Into<String>,
        provider: T,
    ) -> Self
    where
        T: CodeSafetyProvider + Send + Sync + 'static,
    {
        let provider_id = provider_id.into();
        let version = version.into();
        self.providers.push(code_provider_manifest(
            provider_id.clone(),
            CODE_SAFETY_PROVIDER_FAMILY,
            version,
            &[CodeKernelCapability::SafetyAssess],
        ));
        self.provider_registry.safety_provider_id = Some(provider_id);
        self.provider_registry.safety_provider = Some(Arc::new(provider));
        self
    }

    pub fn bootstrap(self) -> KernelResult<CodeKernelRuntime> {
        validate_runtime_id(&self.runtime_id)?;
        validate_providers(&self.providers)?;

        Ok(CodeKernelRuntime {
            capability_manifest: self.build_capability_manifest(),
            provider_registry: self.provider_registry,
        })
    }

    fn build_capability_manifest(&self) -> CodeKernelCapabilityManifest {
        CodeKernelCapabilityManifest {
            schema_version: CODE_KERNEL_SPEC_VERSION.to_string(),
            manifest_type: "capability".to_string(),
            runtime_id: self.runtime_id.clone(),
            kernel_version: CODE_KERNEL_SPEC_VERSION.to_string(),
            providers: self.providers.clone(),
            capabilities: self
                .providers
                .iter()
                .flat_map(|provider| {
                    provider
                        .capabilities
                        .iter()
                        .map(|capability_id| capability_from_provider(capability_id, provider))
                })
                .collect(),
        }
    }
}

#[derive(Clone, Default)]
pub struct CodeKernelProviderRegistry {
    workspace_provider_id: Option<String>,
    workspace_provider: Option<Arc<dyn WorkspaceProvider + Send + Sync>>,
    vcs_provider_id: Option<String>,
    vcs_provider: Option<Arc<dyn VcsProvider + Send + Sync>>,
    patch_provider_id: Option<String>,
    patch_provider: Option<Arc<dyn PatchProvider + Send + Sync>>,
    terminal_provider_id: Option<String>,
    terminal_provider: Option<Arc<dyn TerminalProvider + Send + Sync>>,
    verification_provider_id: Option<String>,
    verification_provider: Option<Arc<dyn VerificationProvider + Send + Sync>>,
    language_provider_id: Option<String>,
    language_provider: Option<Arc<dyn LanguageProvider + Send + Sync>>,
    review_provider_id: Option<String>,
    review_provider: Option<Arc<dyn ReviewProvider + Send + Sync>>,
    artifact_provider_id: Option<String>,
    artifact_provider: Option<Arc<dyn ArtifactProvider + Send + Sync>>,
    knowledge_provider_id: Option<String>,
    knowledge_provider: Option<Arc<dyn KnowledgeProvider + Send + Sync>>,
    safety_provider_id: Option<String>,
    safety_provider: Option<Arc<dyn CodeSafetyProvider + Send + Sync>>,
}

impl CodeKernelProviderRegistry {
    pub fn has_workspace_provider(&self) -> bool {
        self.workspace_provider.is_some()
    }

    pub fn has_vcs_provider(&self) -> bool {
        self.vcs_provider.is_some()
    }

    pub fn has_patch_provider(&self) -> bool {
        self.patch_provider.is_some()
    }

    pub fn has_terminal_provider(&self) -> bool {
        self.terminal_provider.is_some()
    }

    pub fn has_verification_provider(&self) -> bool {
        self.verification_provider.is_some()
    }

    pub fn has_language_provider(&self) -> bool {
        self.language_provider.is_some()
    }

    pub fn has_review_provider(&self) -> bool {
        self.review_provider.is_some()
    }

    pub fn has_artifact_provider(&self) -> bool {
        self.artifact_provider.is_some()
    }

    pub fn has_knowledge_provider(&self) -> bool {
        self.knowledge_provider.is_some()
    }

    pub fn has_safety_provider(&self) -> bool {
        self.safety_provider.is_some()
    }

    fn has_typed_provider(&self, provider: &ProviderManifest) -> bool {
        match provider.provider_family.as_str() {
            CODE_WORKSPACE_PROVIDER_FAMILY => {
                self.workspace_provider_id.as_ref() == Some(&provider.provider_id)
                    && self.workspace_provider.is_some()
            }
            CODE_VCS_PROVIDER_FAMILY => {
                self.vcs_provider_id.as_ref() == Some(&provider.provider_id)
                    && self.vcs_provider.is_some()
            }
            CODE_PATCH_PROVIDER_FAMILY => {
                self.patch_provider_id.as_ref() == Some(&provider.provider_id)
                    && self.patch_provider.is_some()
            }
            CODE_TERMINAL_PROVIDER_FAMILY => {
                self.terminal_provider_id.as_ref() == Some(&provider.provider_id)
                    && self.terminal_provider.is_some()
            }
            CODE_VERIFICATION_PROVIDER_FAMILY => {
                self.verification_provider_id.as_ref() == Some(&provider.provider_id)
                    && self.verification_provider.is_some()
            }
            CODE_LANGUAGE_PROVIDER_FAMILY => {
                self.language_provider_id.as_ref() == Some(&provider.provider_id)
                    && self.language_provider.is_some()
            }
            CODE_REVIEW_PROVIDER_FAMILY => {
                self.review_provider_id.as_ref() == Some(&provider.provider_id)
                    && self.review_provider.is_some()
            }
            CODE_ARTIFACT_PROVIDER_FAMILY => {
                self.artifact_provider_id.as_ref() == Some(&provider.provider_id)
                    && self.artifact_provider.is_some()
            }
            CODE_KNOWLEDGE_PROVIDER_FAMILY => {
                self.knowledge_provider_id.as_ref() == Some(&provider.provider_id)
                    && self.knowledge_provider.is_some()
            }
            CODE_SAFETY_PROVIDER_FAMILY => {
                self.safety_provider_id.as_ref() == Some(&provider.provider_id)
                    && self.safety_provider.is_some()
            }
            _ => false,
        }
    }

    fn health_for_provider(&self, provider: &ProviderManifest) -> Option<ProviderHealth> {
        if !self.has_typed_provider(provider) {
            return None;
        }

        match provider.provider_family.as_str() {
            CODE_WORKSPACE_PROVIDER_FAMILY => self
                .workspace_provider
                .as_ref()
                .map(|provider| provider.health()),
            CODE_VCS_PROVIDER_FAMILY => {
                self.vcs_provider.as_ref().map(|provider| provider.health())
            }
            CODE_PATCH_PROVIDER_FAMILY => self
                .patch_provider
                .as_ref()
                .map(|provider| provider.health()),
            CODE_TERMINAL_PROVIDER_FAMILY => self
                .terminal_provider
                .as_ref()
                .map(|provider| provider.health()),
            CODE_VERIFICATION_PROVIDER_FAMILY => self
                .verification_provider
                .as_ref()
                .map(|provider| provider.health()),
            CODE_LANGUAGE_PROVIDER_FAMILY => self
                .language_provider
                .as_ref()
                .map(|provider| provider.health()),
            CODE_REVIEW_PROVIDER_FAMILY => self
                .review_provider
                .as_ref()
                .map(|provider| provider.health()),
            CODE_ARTIFACT_PROVIDER_FAMILY => self
                .artifact_provider
                .as_ref()
                .map(|provider| provider.health()),
            CODE_KNOWLEDGE_PROVIDER_FAMILY => self
                .knowledge_provider
                .as_ref()
                .map(|provider| provider.health()),
            CODE_SAFETY_PROVIDER_FAMILY => self
                .safety_provider
                .as_ref()
                .map(|provider| provider.health()),
            _ => None,
        }
    }
}

impl std::fmt::Debug for CodeKernelProviderRegistry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CodeKernelProviderRegistry")
            .field("workspace_provider_id", &self.workspace_provider_id)
            .field("has_workspace_provider", &self.has_workspace_provider())
            .field("vcs_provider_id", &self.vcs_provider_id)
            .field("has_vcs_provider", &self.has_vcs_provider())
            .field("patch_provider_id", &self.patch_provider_id)
            .field("has_patch_provider", &self.has_patch_provider())
            .field("terminal_provider_id", &self.terminal_provider_id)
            .field("has_terminal_provider", &self.has_terminal_provider())
            .field("verification_provider_id", &self.verification_provider_id)
            .field(
                "has_verification_provider",
                &self.has_verification_provider(),
            )
            .field("language_provider_id", &self.language_provider_id)
            .field("has_language_provider", &self.has_language_provider())
            .field("review_provider_id", &self.review_provider_id)
            .field("has_review_provider", &self.has_review_provider())
            .field("artifact_provider_id", &self.artifact_provider_id)
            .field("has_artifact_provider", &self.has_artifact_provider())
            .field("knowledge_provider_id", &self.knowledge_provider_id)
            .field("has_knowledge_provider", &self.has_knowledge_provider())
            .field("safety_provider_id", &self.safety_provider_id)
            .field("has_safety_provider", &self.has_safety_provider())
            .finish()
    }
}

impl PartialEq for CodeKernelProviderRegistry {
    fn eq(&self, other: &Self) -> bool {
        self.workspace_provider_id == other.workspace_provider_id
            && self.has_workspace_provider() == other.has_workspace_provider()
            && self.vcs_provider_id == other.vcs_provider_id
            && self.has_vcs_provider() == other.has_vcs_provider()
            && self.patch_provider_id == other.patch_provider_id
            && self.has_patch_provider() == other.has_patch_provider()
            && self.terminal_provider_id == other.terminal_provider_id
            && self.has_terminal_provider() == other.has_terminal_provider()
            && self.verification_provider_id == other.verification_provider_id
            && self.has_verification_provider() == other.has_verification_provider()
            && self.language_provider_id == other.language_provider_id
            && self.has_language_provider() == other.has_language_provider()
            && self.review_provider_id == other.review_provider_id
            && self.has_review_provider() == other.has_review_provider()
            && self.artifact_provider_id == other.artifact_provider_id
            && self.has_artifact_provider() == other.has_artifact_provider()
            && self.knowledge_provider_id == other.knowledge_provider_id
            && self.has_knowledge_provider() == other.has_knowledge_provider()
            && self.safety_provider_id == other.safety_provider_id
            && self.has_safety_provider() == other.has_safety_provider()
    }
}

impl Eq for CodeKernelProviderRegistry {}

fn code_provider_manifest(
    provider_id: impl Into<String>,
    provider_family: impl Into<String>,
    version: impl Into<String>,
    capabilities: &[CodeKernelCapability],
) -> ProviderManifest {
    let provider_id = provider_id.into();
    ProviderManifest::new(
        provider_id.clone(),
        provider_family,
        provider_id,
        version,
        capabilities
            .iter()
            .map(|capability| capability.as_str().to_string())
            .collect(),
    )
}

fn standard_code_provider_families() -> &'static [&'static str] {
    &[
        CODE_WORKSPACE_PROVIDER_FAMILY,
        CODE_VCS_PROVIDER_FAMILY,
        CODE_PATCH_PROVIDER_FAMILY,
        CODE_TERMINAL_PROVIDER_FAMILY,
        CODE_VERIFICATION_PROVIDER_FAMILY,
        CODE_LANGUAGE_PROVIDER_FAMILY,
        CODE_REVIEW_PROVIDER_FAMILY,
        CODE_ARTIFACT_PROVIDER_FAMILY,
        CODE_KNOWLEDGE_PROVIDER_FAMILY,
        CODE_SAFETY_PROVIDER_FAMILY,
    ]
}

fn capability_from_provider(capability_id: &str, provider: &ProviderManifest) -> Capability {
    let metadata = capability_metadata(capability_id);

    Capability {
        capability_id: capability_id.to_string(),
        version: provider.version.clone(),
        provider_id: provider.provider_id.clone(),
        status: "available".to_string(),
        required: false,
        operations: metadata.operations,
        side_effect_level: metadata.side_effect_level,
        policy_categories: metadata.policy_categories,
    }
}

struct CapabilityMetadata {
    operations: Vec<String>,
    side_effect_level: Option<String>,
    policy_categories: Vec<String>,
}

fn capability_metadata(capability_id: &str) -> CapabilityMetadata {
    match capability_id {
        "code.workspace.list" => code_capability_metadata(
            vec!["list_files"],
            SideEffectLevel::ReadOnly,
            vec!["code.workspace.list"],
        ),
        "code.workspace.read" => code_capability_metadata(
            vec!["read_file"],
            SideEffectLevel::ReadOnly,
            vec!["code.workspace.read"],
        ),
        "code.workspace.write" => code_capability_metadata(
            vec!["write_file"],
            SideEffectLevel::SideEffectful,
            vec!["code.workspace.write"],
        ),
        "code.workspace.stat" => code_capability_metadata(
            vec!["stat_file"],
            SideEffectLevel::ReadOnly,
            vec!["code.workspace.stat"],
        ),
        "code.workspace.watch" => code_capability_metadata(
            vec!["watch_events"],
            SideEffectLevel::ReadOnly,
            vec!["code.workspace.watch"],
        ),
        "code.vcs.status" => code_capability_metadata(
            vec!["snapshot"],
            SideEffectLevel::ReadOnly,
            vec!["code.vcs.status"],
        ),
        "code.vcs.diff" => code_capability_metadata(
            vec!["diff"],
            SideEffectLevel::ReadOnly,
            vec!["code.vcs.diff"],
        ),
        "code.vcs.blame" => code_capability_metadata(
            vec!["blame"],
            SideEffectLevel::ReadOnly,
            vec!["code.vcs.blame"],
        ),
        "code.vcs.commit_metadata" => code_capability_metadata(
            vec!["commit_metadata"],
            SideEffectLevel::ReadOnly,
            vec!["code.vcs.commit_metadata"],
        ),
        "code.vcs.restore" => code_capability_metadata(
            vec!["restore"],
            SideEffectLevel::Destructive,
            vec!["code.vcs.restore"],
        ),
        "code.patch.validate" => code_capability_metadata(
            vec!["validate_patch"],
            SideEffectLevel::ReadOnly,
            vec!["code.patch.validate"],
        ),
        "code.patch.preview" => code_capability_metadata(
            vec!["preview_patch"],
            SideEffectLevel::ReadOnly,
            vec!["code.patch.preview"],
        ),
        "code.patch.apply" => code_capability_metadata(
            vec!["apply_patch"],
            SideEffectLevel::SideEffectful,
            vec!["code.patch.apply"],
        ),
        "code.patch.reject" => code_capability_metadata(
            vec!["reject_patch"],
            SideEffectLevel::ReadOnly,
            vec!["code.patch.reject"],
        ),
        "code.patch.rollback" => code_capability_metadata(
            vec!["rollback_patch"],
            SideEffectLevel::Destructive,
            vec!["code.patch.rollback"],
        ),
        "code.patch.explain" => code_capability_metadata(
            vec!["explain_patch"],
            SideEffectLevel::ReadOnly,
            vec!["code.patch.explain"],
        ),
        "code.terminal.run" => code_capability_metadata(
            vec!["run_command", "stream_output", "cancel_command"],
            SideEffectLevel::SideEffectful,
            vec!["code.terminal.run"],
        ),
        "code.verification.run" => code_capability_metadata(
            vec!["discover_plans", "run_verification"],
            SideEffectLevel::SideEffectful,
            vec!["code.verification.run"],
        ),
        "code.language.diagnostics" => code_capability_metadata(
            vec!["diagnostics"],
            SideEffectLevel::ReadOnly,
            vec!["code.language.diagnostics"],
        ),
        "code.language.symbols" => code_capability_metadata(
            vec!["symbols"],
            SideEffectLevel::ReadOnly,
            vec!["code.language.symbols"],
        ),
        "code.language.format" => code_capability_metadata(
            vec!["format"],
            SideEffectLevel::ReadOnly,
            vec!["code.language.format"],
        ),
        "code.review.produce" => code_capability_metadata(
            vec!["review_patch", "review_verification"],
            SideEffectLevel::ReadOnly,
            vec!["code.review.produce"],
        ),
        "code.artifact.read" => code_capability_metadata(
            vec!["get_artifact", "list_artifacts"],
            SideEffectLevel::ReadOnly,
            vec!["code.artifact.read"],
        ),
        "code.artifact.write" => code_capability_metadata(
            vec!["put_artifact"],
            SideEffectLevel::SideEffectful,
            vec!["code.artifact.write"],
        ),
        "code.knowledge.search" => code_capability_metadata(
            vec!["search_documents"],
            SideEffectLevel::ReadOnly,
            vec!["code.knowledge.search"],
        ),
        "code.knowledge.read" => code_capability_metadata(
            vec!["get_document", "list_documents"],
            SideEffectLevel::ReadOnly,
            vec!["code.knowledge.read"],
        ),
        "code.safety.assess" => code_capability_metadata(
            vec![
                "assess_workspace",
                "assess_patch",
                "assess_terminal_command",
            ],
            SideEffectLevel::ReadOnly,
            vec!["code.safety.assess"],
        ),
        _ => CapabilityMetadata {
            operations: Vec::new(),
            side_effect_level: None,
            policy_categories: Vec::new(),
        },
    }
}

fn code_capability_metadata(
    operations: Vec<&str>,
    side_effect_level: SideEffectLevel,
    policy_categories: Vec<&str>,
) -> CapabilityMetadata {
    CapabilityMetadata {
        operations: operations
            .into_iter()
            .map(std::string::ToString::to_string)
            .collect(),
        side_effect_level: Some(side_effect_level.as_str().to_string()),
        policy_categories: policy_categories
            .into_iter()
            .map(std::string::ToString::to_string)
            .collect(),
    }
}

fn validate_runtime_id(runtime_id: &str) -> KernelResult<()> {
    if runtime_id.trim().is_empty() {
        return Err(KernelError::validation("code runtime id must not be empty"));
    }

    Ok(())
}

fn validate_providers(providers: &[ProviderManifest]) -> KernelResult<()> {
    for provider in providers {
        if provider.provider_id.trim().is_empty() {
            return Err(KernelError::validation("provider id must not be empty"));
        }

        if provider.provider_family.trim().is_empty() {
            return Err(KernelError::validation("provider family must not be empty"));
        }

        if provider.version.trim().is_empty() {
            return Err(KernelError::validation(
                "provider version must not be empty",
            ));
        }

        if provider.capabilities.is_empty() {
            return Err(KernelError::validation(
                "provider must declare at least one capability",
            ));
        }
    }

    Ok(())
}
