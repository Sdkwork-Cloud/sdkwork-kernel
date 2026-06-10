use crate::{CodeKernelCapabilityManifest, CodeKernelRuntimeDiagnostics};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeConformanceProfile {
    Manifest,
    LocalRuntime,
}

impl CodeConformanceProfile {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Manifest => "manifest",
            Self::LocalRuntime => "local_runtime",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeConformanceReport {
    pub runtime_id: String,
    pub profile: CodeConformanceProfile,
    pub passed: bool,
    pub cases: Vec<CodeConformanceCase>,
}

impl CodeConformanceReport {
    pub fn from_manifest_and_diagnostics(
        profile: CodeConformanceProfile,
        manifest: &CodeKernelCapabilityManifest,
        diagnostics: &CodeKernelRuntimeDiagnostics,
    ) -> Self {
        let mut cases = vec![
            standard_provider_families_case(diagnostics),
            standard_capabilities_complete_case(manifest),
            standard_capabilities_namespaced_case(manifest),
        ];

        if profile == CodeConformanceProfile::LocalRuntime {
            cases.push(local_providers_typed_case(diagnostics));
            cases.push(local_provider_health_case(diagnostics));
        }

        Self {
            runtime_id: manifest.runtime_id.clone(),
            profile,
            passed: cases.iter().all(|case| case.passed),
            cases,
        }
    }

    pub fn case(&self, case_id: &str) -> Option<&CodeConformanceCase> {
        self.cases.iter().find(|case| case.case_id == case_id)
    }

    pub fn failed_case_ids(&self) -> Vec<String> {
        self.cases
            .iter()
            .filter(|case| !case.passed)
            .map(|case| case.case_id.clone())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeConformanceCase {
    pub case_id: String,
    pub passed: bool,
    pub message: String,
}

impl CodeConformanceCase {
    pub fn passed(case_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            case_id: case_id.into(),
            passed: true,
            message: message.into(),
        }
    }

    pub fn failed(case_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            case_id: case_id.into(),
            passed: false,
            message: message.into(),
        }
    }
}

fn standard_provider_families_case(
    diagnostics: &CodeKernelRuntimeDiagnostics,
) -> CodeConformanceCase {
    let missing_families = diagnostics.missing_standard_provider_families();

    if missing_families.is_empty() {
        return CodeConformanceCase::passed(
            "code.conformance.standard_provider_families.complete",
            "all standard provider families are declared",
        );
    }

    CodeConformanceCase::failed(
        "code.conformance.standard_provider_families.complete",
        format!(
            "missing standard provider families: {}",
            missing_families.join(", ")
        ),
    )
}

fn standard_capabilities_namespaced_case(
    manifest: &CodeKernelCapabilityManifest,
) -> CodeConformanceCase {
    let non_code_capabilities: Vec<String> = manifest
        .providers
        .iter()
        .filter(|provider| provider.provider_family.starts_with("code_"))
        .flat_map(|provider| {
            provider
                .capabilities
                .iter()
                .filter(|capability| !is_valid_code_capability_id(capability))
                .cloned()
        })
        .collect();

    if non_code_capabilities.is_empty() {
        return CodeConformanceCase::passed(
            "code.conformance.standard_capabilities.namespaced",
            "all standard code capabilities use the code.* namespace",
        );
    }

    CodeConformanceCase::failed(
        "code.conformance.standard_capabilities.namespaced",
        format!(
            "standard code capabilities outside code.* namespace: {}",
            non_code_capabilities.join(", ")
        ),
    )
}

fn standard_capabilities_complete_case(
    manifest: &CodeKernelCapabilityManifest,
) -> CodeConformanceCase {
    let mut missing_by_family = Vec::new();

    for (provider_family, required_capabilities) in standard_capabilities_by_family() {
        let declared_capabilities: Vec<&str> = manifest
            .providers
            .iter()
            .filter(|provider| provider.provider_family == *provider_family)
            .flat_map(|provider| provider.capabilities.iter().map(String::as_str))
            .collect();

        if declared_capabilities.is_empty() {
            continue;
        }

        let missing_capabilities: Vec<&str> = required_capabilities
            .iter()
            .copied()
            .filter(|capability| !declared_capabilities.contains(capability))
            .collect();

        if !missing_capabilities.is_empty() {
            missing_by_family.push(format!(
                "{provider_family}={}",
                missing_capabilities.join(", ")
            ));
        }
    }

    if missing_by_family.is_empty() {
        return CodeConformanceCase::passed(
            "code.conformance.standard_capabilities.complete",
            "all declared standard provider families expose their standard capabilities",
        );
    }

    CodeConformanceCase::failed(
        "code.conformance.standard_capabilities.complete",
        format!(
            "missing standard capabilities: {}",
            missing_by_family.join("; ")
        ),
    )
}

fn local_providers_typed_case(diagnostics: &CodeKernelRuntimeDiagnostics) -> CodeConformanceCase {
    let manifest_only_provider_ids = diagnostics.manifest_only_provider_ids();

    if manifest_only_provider_ids.is_empty() {
        return CodeConformanceCase::passed(
            "code.conformance.local_providers.typed",
            "all declared providers have typed local SPI instances",
        );
    }

    CodeConformanceCase::failed(
        "code.conformance.local_providers.typed",
        format!(
            "manifest-only providers cannot satisfy local runtime conformance: {}",
            manifest_only_provider_ids.join(", ")
        ),
    )
}

fn local_provider_health_case(diagnostics: &CodeKernelRuntimeDiagnostics) -> CodeConformanceCase {
    let degraded_provider_ids: Vec<String> = diagnostics
        .provider_diagnostics
        .iter()
        .filter(|provider| {
            provider.typed_registered
                && match provider.health.as_ref() {
                    Some(health) => health.status != "available",
                    None => true,
                }
        })
        .map(|provider| provider.provider_id.clone())
        .collect();

    if degraded_provider_ids.is_empty() {
        return CodeConformanceCase::passed(
            "code.conformance.local_providers.health_available",
            "all typed local providers report available health",
        );
    }

    CodeConformanceCase::failed(
        "code.conformance.local_providers.health_available",
        format!(
            "typed providers with non-available health: {}",
            degraded_provider_ids.join(", ")
        ),
    )
}

fn standard_capabilities_by_family() -> &'static [(&'static str, &'static [&'static str])] {
    &[
        (
            "code_workspace",
            &[
                "code.workspace.list",
                "code.workspace.read",
                "code.workspace.write",
                "code.workspace.stat",
                "code.workspace.watch",
            ],
        ),
        (
            "code_vcs",
            &[
                "code.vcs.status",
                "code.vcs.diff",
                "code.vcs.blame",
                "code.vcs.commit_metadata",
                "code.vcs.restore",
            ],
        ),
        (
            "code_patch",
            &[
                "code.patch.validate",
                "code.patch.preview",
                "code.patch.apply",
                "code.patch.reject",
                "code.patch.rollback",
                "code.patch.explain",
            ],
        ),
        ("code_terminal", &["code.terminal.run"]),
        ("code_verification", &["code.verification.run"]),
        (
            "code_language",
            &[
                "code.language.diagnostics",
                "code.language.symbols",
                "code.language.format",
            ],
        ),
        ("code_review", &["code.review.produce"]),
        (
            "code_artifact",
            &["code.artifact.read", "code.artifact.write"],
        ),
        (
            "code_knowledge",
            &[
                "code.knowledge.search",
                "code.knowledge.read",
                "code.knowledge.list",
            ],
        ),
        ("code_safety", &["code.safety.assess"]),
    ]
}

fn is_valid_code_capability_id(capability_id: &str) -> bool {
    capability_id.starts_with("code.")
        && capability_id.chars().all(|ch| {
            ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '.' | '_' | '-')
        })
}
