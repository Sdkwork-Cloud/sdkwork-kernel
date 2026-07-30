use crate::{
    AgentConfiguration, AgentConfigurationSpec, KernelEvent, KernelEventRedaction,
    KernelEventSeverity, KernelEventSource, KernelResult, PolicyCategory, PolicyRequest,
    ProviderHealth, ProviderManifest, SideEffectLevel,
};

pub trait AgentInstaller {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.agent.installer.unspecified",
            "agent_installer",
            "agent-installer",
            "0.0.0",
            vec![
                "agent.install".to_string(),
                "agent.uninstall".to_string(),
                "agent.upgrade".to_string(),
            ],
        )
    }

    fn detect_installation(&self, agent_id: &str) -> KernelResult<AgentInstallation>;

    fn configuration_spec(&self, agent_id: &str) -> KernelResult<AgentConfigurationSpec>;

    fn plan_install(&self, request: &AgentInstallRequest) -> KernelResult<AgentInstallPlan>;

    fn install(&self, request: AgentInstallRequest) -> KernelResult<AgentInstallReport>;

    fn plan_upgrade(&self, request: &AgentUpgradeRequest) -> KernelResult<AgentUpgradePlan>;

    fn upgrade(&self, request: AgentUpgradeRequest) -> KernelResult<AgentUpgradeReport>;

    fn plan_uninstall(&self, request: &AgentUninstallRequest) -> KernelResult<AgentUninstallPlan>;

    fn uninstall(&self, request: AgentUninstallRequest) -> KernelResult<AgentUninstallReport>;

    fn health(&self) -> ProviderHealth;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentInstallationState {
    Installed,
    NotInstalled,
    Degraded,
}

impl AgentInstallationState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Installed => "installed",
            Self::NotInstalled => "not_installed",
            Self::Degraded => "degraded",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentInstallationDependency {
    pub registry_id: String,
    pub package_id: String,
    pub expected_version: Option<String>,
    pub installed_version: Option<String>,
}

impl AgentInstallationDependency {
    pub fn installed(
        registry_id: impl Into<String>,
        package_id: impl Into<String>,
        expected_version: impl Into<String>,
        installed_version: impl Into<String>,
    ) -> Self {
        Self {
            registry_id: registry_id.into(),
            package_id: package_id.into(),
            expected_version: Some(expected_version.into()),
            installed_version: Some(installed_version.into()),
        }
    }

    pub fn missing(
        registry_id: impl Into<String>,
        package_id: impl Into<String>,
        expected_version: impl Into<String>,
    ) -> Self {
        Self {
            registry_id: registry_id.into(),
            package_id: package_id.into(),
            expected_version: Some(expected_version.into()),
            installed_version: None,
        }
    }

    pub fn version_matches(&self) -> bool {
        match (&self.expected_version, &self.installed_version) {
            (Some(expected), Some(installed)) => expected == installed,
            (None, Some(_)) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentInstallation {
    pub agent_id: String,
    pub state: AgentInstallationState,
    pub installed_version: Option<String>,
    pub dependencies: Vec<AgentInstallationDependency>,
    pub safe_summary: String,
}

impl AgentInstallation {
    pub fn installed(agent_id: impl Into<String>, installed_version: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            state: AgentInstallationState::Installed,
            installed_version: Some(installed_version.into()),
            dependencies: Vec::new(),
            safe_summary: "agent provider is installed".to_string(),
        }
    }

    pub fn not_installed(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            state: AgentInstallationState::NotInstalled,
            installed_version: None,
            dependencies: Vec::new(),
            safe_summary: "agent provider is not installed".to_string(),
        }
    }

    pub fn degraded(agent_id: impl Into<String>, installed_version: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            state: AgentInstallationState::Degraded,
            installed_version: Some(installed_version.into()),
            dependencies: Vec::new(),
            safe_summary: "agent provider installation is degraded".to_string(),
        }
    }

    pub fn partially_installed(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            state: AgentInstallationState::Degraded,
            installed_version: None,
            dependencies: Vec::new(),
            safe_summary: "agent provider installation is partially present".to_string(),
        }
    }

    pub fn with_dependency(mut self, dependency: AgentInstallationDependency) -> Self {
        self.dependencies.push(dependency);
        self
    }

    pub fn is_installed(&self) -> bool {
        self.state == AgentInstallationState::Installed
    }

    pub fn is_degraded(&self) -> bool {
        self.state == AgentInstallationState::Degraded
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentPackageSource {
    LocalPath {
        path: String,
    },
    Registry {
        registry_id: String,
        package_id: String,
        version: String,
    },
    RemoteArchive {
        url: String,
        checksum: String,
    },
}

impl AgentPackageSource {
    pub fn local_path(path: impl Into<String>) -> Self {
        Self::LocalPath { path: path.into() }
    }

    pub fn registry(
        registry_id: impl Into<String>,
        package_id: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self::Registry {
            registry_id: registry_id.into(),
            package_id: package_id.into(),
            version: version.into(),
        }
    }

    pub fn remote_archive(url: impl Into<String>, checksum: impl Into<String>) -> Self {
        Self::RemoteArchive {
            url: url.into(),
            checksum: checksum.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentInstallRequest {
    pub request_id: String,
    pub agent_id: String,
    pub target_version: String,
    pub source: AgentPackageSource,
    pub profile_id: Option<String>,
    pub configuration: Option<AgentConfiguration>,
    pub requested_by: Option<String>,
    pub dry_run: bool,
}

impl AgentInstallRequest {
    pub fn new(
        request_id: impl Into<String>,
        agent_id: impl Into<String>,
        target_version: impl Into<String>,
        source: AgentPackageSource,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            agent_id: agent_id.into(),
            target_version: target_version.into(),
            source,
            profile_id: None,
            configuration: None,
            requested_by: None,
            dry_run: false,
        }
    }

    pub fn with_profile(mut self, profile_id: impl Into<String>) -> Self {
        self.profile_id = Some(profile_id.into());
        self
    }

    pub fn with_configuration(mut self, configuration: AgentConfiguration) -> Self {
        self.profile_id = Some(configuration.profile_id.clone());
        self.configuration = Some(configuration);
        self
    }

    pub fn requested_by(mut self, requested_by: impl Into<String>) -> Self {
        self.requested_by = Some(requested_by.into());
        self
    }

    pub fn dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentUpgradeRequest {
    pub request_id: String,
    pub agent_id: String,
    pub from_version: String,
    pub to_version: String,
    pub configuration: Option<AgentConfiguration>,
    pub rollback_required: bool,
    pub requested_by: Option<String>,
    pub dry_run: bool,
}

impl AgentUpgradeRequest {
    pub fn new(
        request_id: impl Into<String>,
        agent_id: impl Into<String>,
        from_version: impl Into<String>,
        to_version: impl Into<String>,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            agent_id: agent_id.into(),
            from_version: from_version.into(),
            to_version: to_version.into(),
            configuration: None,
            rollback_required: false,
            requested_by: None,
            dry_run: false,
        }
    }

    pub fn with_configuration(mut self, configuration: AgentConfiguration) -> Self {
        self.configuration = Some(configuration);
        self
    }

    pub fn with_rollback_required(mut self) -> Self {
        self.rollback_required = true;
        self
    }

    pub fn requested_by(mut self, requested_by: impl Into<String>) -> Self {
        self.requested_by = Some(requested_by.into());
        self
    }

    pub fn dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentUninstallRequest {
    pub request_id: String,
    pub agent_id: String,
    pub remove_configuration: bool,
    pub preserve_data: bool,
    pub requested_by: Option<String>,
    pub dry_run: bool,
}

impl AgentUninstallRequest {
    pub fn new(request_id: impl Into<String>, agent_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            agent_id: agent_id.into(),
            remove_configuration: false,
            preserve_data: true,
            requested_by: None,
            dry_run: false,
        }
    }

    pub fn remove_configuration(mut self) -> Self {
        self.remove_configuration = true;
        self
    }

    pub fn remove_data(mut self) -> Self {
        self.preserve_data = false;
        self
    }

    pub fn requested_by(mut self, requested_by: impl Into<String>) -> Self {
        self.requested_by = Some(requested_by.into());
        self
    }

    pub fn dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentInstallPlan {
    pub plan_id: String,
    pub agent_id: String,
    pub target_version: String,
    pub steps: Vec<AgentInstallStep>,
    pub required_policy_categories: Vec<String>,
    pub configuration_spec: Option<AgentConfigurationSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentUninstallPlan {
    pub plan_id: String,
    pub agent_id: String,
    pub steps: Vec<AgentInstallStep>,
    pub required_policy_categories: Vec<String>,
}

impl AgentUninstallPlan {
    pub fn new(plan_id: impl Into<String>, agent_id: impl Into<String>) -> Self {
        Self {
            plan_id: plan_id.into(),
            agent_id: agent_id.into(),
            steps: Vec::new(),
            required_policy_categories: Vec::new(),
        }
    }

    pub fn add_step(mut self, step: AgentInstallStep) -> Self {
        self.steps.push(step);
        self
    }

    pub fn require_policy(mut self, category: PolicyCategory) -> Self {
        let category = category.as_str().to_string();
        if !self.required_policy_categories.contains(&category) {
            self.required_policy_categories.push(category);
        }
        self
    }

    pub fn requires_policy(&self) -> bool {
        !self.required_policy_categories.is_empty()
            || self
                .steps
                .iter()
                .any(|step| step.side_effect_level != SideEffectLevel::ReadOnly)
    }

    pub fn to_policy_request(&self, policy_request_id: impl Into<String>) -> PolicyRequest {
        PolicyRequest::new(
            policy_request_id,
            PolicyCategory::AgentUninstall.as_str(),
            self.agent_id.clone(),
        )
        .with_category(PolicyCategory::AgentUninstall)
        .with_action("agent.uninstall")
        .with_side_effect_level(SideEffectLevel::Destructive)
        .with_redaction(KernelEventRedaction::Internal)
    }
}

impl AgentInstallPlan {
    pub fn new(
        plan_id: impl Into<String>,
        agent_id: impl Into<String>,
        target_version: impl Into<String>,
    ) -> Self {
        Self {
            plan_id: plan_id.into(),
            agent_id: agent_id.into(),
            target_version: target_version.into(),
            steps: Vec::new(),
            required_policy_categories: Vec::new(),
            configuration_spec: None,
        }
    }

    pub fn add_step(mut self, step: AgentInstallStep) -> Self {
        self.steps.push(step);
        self
    }

    pub fn require_policy(mut self, category: PolicyCategory) -> Self {
        let category = category.as_str().to_string();
        if !self.required_policy_categories.contains(&category) {
            self.required_policy_categories.push(category);
        }
        self
    }

    pub fn with_configuration_spec(mut self, configuration_spec: AgentConfigurationSpec) -> Self {
        self.configuration_spec = Some(configuration_spec);
        self
    }

    pub fn requires_policy(&self) -> bool {
        !self.required_policy_categories.is_empty()
            || self
                .steps
                .iter()
                .any(|step| step.side_effect_level != SideEffectLevel::ReadOnly)
    }

    pub fn to_policy_request(&self, policy_request_id: impl Into<String>) -> PolicyRequest {
        let category = self
            .required_policy_categories
            .first()
            .cloned()
            .unwrap_or_else(|| PolicyCategory::AgentInstall.as_str().to_string());

        PolicyRequest::new(
            policy_request_id,
            category,
            format!("{}@{}", self.agent_id, self.target_version),
        )
        .with_category(PolicyCategory::AgentInstall)
        .with_action("agent.install")
        .with_side_effect_level(SideEffectLevel::SideEffectful)
        .with_redaction(KernelEventRedaction::Internal)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentUpgradePlan {
    pub plan_id: String,
    pub agent_id: String,
    pub from_version: String,
    pub to_version: String,
    pub steps: Vec<AgentInstallStep>,
    pub required_policy_categories: Vec<String>,
    pub rollback_required: bool,
}

impl AgentUpgradePlan {
    pub fn new(
        plan_id: impl Into<String>,
        agent_id: impl Into<String>,
        from_version: impl Into<String>,
        to_version: impl Into<String>,
    ) -> Self {
        Self {
            plan_id: plan_id.into(),
            agent_id: agent_id.into(),
            from_version: from_version.into(),
            to_version: to_version.into(),
            steps: Vec::new(),
            required_policy_categories: Vec::new(),
            rollback_required: false,
        }
    }

    pub fn add_step(mut self, step: AgentInstallStep) -> Self {
        self.steps.push(step);
        self
    }

    pub fn require_policy(mut self, category: PolicyCategory) -> Self {
        let category = category.as_str().to_string();
        if !self.required_policy_categories.contains(&category) {
            self.required_policy_categories.push(category);
        }
        self
    }

    pub fn with_rollback_required(mut self, rollback_required: bool) -> Self {
        self.rollback_required = rollback_required;
        self
    }

    pub fn requires_policy(&self) -> bool {
        !self.required_policy_categories.is_empty()
            || self
                .steps
                .iter()
                .any(|step| step.side_effect_level != SideEffectLevel::ReadOnly)
    }

    pub fn to_policy_request(&self, policy_request_id: impl Into<String>) -> PolicyRequest {
        PolicyRequest::new(
            policy_request_id,
            PolicyCategory::AgentUpgrade.as_str(),
            format!(
                "{}@{}->{}",
                self.agent_id, self.from_version, self.to_version
            ),
        )
        .with_category(PolicyCategory::AgentUpgrade)
        .with_action("agent.upgrade")
        .with_side_effect_level(SideEffectLevel::SideEffectful)
        .with_redaction(KernelEventRedaction::Internal)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentInstallStep {
    pub step_id: String,
    pub kind: AgentInstallStepKind,
    pub description: String,
    pub side_effect_level: SideEffectLevel,
}

impl AgentInstallStep {
    pub fn new(
        step_id: impl Into<String>,
        kind: AgentInstallStepKind,
        description: impl Into<String>,
    ) -> Self {
        let side_effect_level = kind.side_effect_level();
        Self {
            step_id: step_id.into(),
            kind,
            description: description.into(),
            side_effect_level,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentInstallStepKind {
    DownloadPackage,
    VerifyPackage,
    WriteFiles,
    RegisterAgent,
    ConfigureAgent,
    StartAgent,
    StopAgent,
    BackupCurrentVersion,
    ReplaceVersion,
    RemoveFiles,
    RemoveConfiguration,
}

impl AgentInstallStepKind {
    pub fn side_effect_level(&self) -> SideEffectLevel {
        match self {
            Self::VerifyPackage | Self::BackupCurrentVersion => SideEffectLevel::ReadOnly,
            Self::RemoveFiles | Self::RemoveConfiguration => SideEffectLevel::Destructive,
            _ => SideEffectLevel::SideEffectful,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentInstallStatus {
    Planned,
    Installed,
    Upgraded,
    Uninstalled,
    Failed,
    RolledBack,
}

impl AgentInstallStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Installed => "installed",
            Self::Upgraded => "upgraded",
            Self::Uninstalled => "uninstalled",
            Self::Failed => "failed",
            Self::RolledBack => "rolled_back",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentInstallReport {
    pub request_id: String,
    pub agent_id: String,
    pub status: AgentInstallStatus,
    pub target_version: String,
    pub installed_version: Option<String>,
    pub safe_summary: String,
}

impl AgentInstallReport {
    pub fn planned(
        request_id: impl Into<String>,
        agent_id: impl Into<String>,
        target_version: impl Into<String>,
    ) -> Self {
        let target_version = target_version.into();
        Self {
            request_id: request_id.into(),
            agent_id: agent_id.into(),
            status: AgentInstallStatus::Planned,
            target_version,
            installed_version: None,
            safe_summary: "agent installation planned".to_string(),
        }
    }

    pub fn installed(
        request_id: impl Into<String>,
        agent_id: impl Into<String>,
        installed_version: impl Into<String>,
    ) -> Self {
        let installed_version = installed_version.into();
        Self {
            request_id: request_id.into(),
            agent_id: agent_id.into(),
            status: AgentInstallStatus::Installed,
            target_version: installed_version.clone(),
            installed_version: Some(installed_version),
            safe_summary: "agent installed".to_string(),
        }
    }

    pub fn to_event(&self, event_id: impl Into<String>) -> KernelEvent {
        KernelEvent::new(
            event_id,
            format!("agent.install.{}", self.status.as_str()),
            KernelEventSeverity::Info,
            format!(
                "request_id={};agent_id={};status={};target_version={};installed_version={};summary={}",
                encode_event_field(&self.request_id),
                encode_event_field(&self.agent_id),
                self.status.as_str(),
                encode_event_field(&self.target_version),
                encode_event_field(self.installed_version.as_deref().unwrap_or("")),
                encode_event_field(&self.safe_summary)
            ),
        )
        .from_source(KernelEventSource::Runtime)
        .with_redaction(KernelEventRedaction::Internal)
        .with_payload_schema("sdkwork.agent.installation.report.v1")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentUpgradeReport {
    pub request_id: String,
    pub agent_id: String,
    pub status: AgentInstallStatus,
    pub from_version: String,
    pub to_version: String,
    pub rollback_token: Option<String>,
    pub safe_summary: String,
}

impl AgentUpgradeReport {
    pub fn planned(
        request_id: impl Into<String>,
        agent_id: impl Into<String>,
        from_version: impl Into<String>,
        to_version: impl Into<String>,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            agent_id: agent_id.into(),
            status: AgentInstallStatus::Planned,
            from_version: from_version.into(),
            to_version: to_version.into(),
            rollback_token: None,
            safe_summary: "agent upgrade planned".to_string(),
        }
    }

    pub fn upgraded(
        request_id: impl Into<String>,
        agent_id: impl Into<String>,
        from_version: impl Into<String>,
        to_version: impl Into<String>,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            agent_id: agent_id.into(),
            status: AgentInstallStatus::Upgraded,
            from_version: from_version.into(),
            to_version: to_version.into(),
            rollback_token: None,
            safe_summary: "agent upgraded".to_string(),
        }
    }

    pub fn with_rollback_token(mut self, rollback_token: impl Into<String>) -> Self {
        self.rollback_token = Some(rollback_token.into());
        self
    }

    pub fn to_event(&self, event_id: impl Into<String>) -> KernelEvent {
        KernelEvent::new(
            event_id,
            format!("agent.install.{}", self.status.as_str()),
            KernelEventSeverity::Info,
            format!(
                "request_id={};agent_id={};status={};from_version={};to_version={};rollback_available={};summary={}",
                encode_event_field(&self.request_id),
                encode_event_field(&self.agent_id),
                self.status.as_str(),
                encode_event_field(&self.from_version),
                encode_event_field(&self.to_version),
                self.rollback_token.is_some(),
                encode_event_field(&self.safe_summary)
            ),
        )
        .from_source(KernelEventSource::Runtime)
        .with_redaction(KernelEventRedaction::Internal)
        .with_payload_schema("sdkwork.agent.installation.report.v1")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentUninstallReport {
    pub request_id: String,
    pub agent_id: String,
    pub status: AgentInstallStatus,
    pub configuration_removed: bool,
    pub safe_summary: String,
}

impl AgentUninstallReport {
    pub fn planned(request_id: impl Into<String>, agent_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            agent_id: agent_id.into(),
            status: AgentInstallStatus::Planned,
            configuration_removed: false,
            safe_summary: "agent uninstall planned".to_string(),
        }
    }

    pub fn uninstalled(request_id: impl Into<String>, agent_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            agent_id: agent_id.into(),
            status: AgentInstallStatus::Uninstalled,
            configuration_removed: false,
            safe_summary: "agent uninstalled".to_string(),
        }
    }

    pub fn with_configuration_removed(mut self, configuration_removed: bool) -> Self {
        self.configuration_removed = configuration_removed;
        self
    }

    pub fn to_event(&self, event_id: impl Into<String>) -> KernelEvent {
        KernelEvent::new(
            event_id,
            format!("agent.install.{}", self.status.as_str()),
            KernelEventSeverity::Info,
            format!(
                "request_id={};agent_id={};status={};configuration_removed={};summary={}",
                encode_event_field(&self.request_id),
                encode_event_field(&self.agent_id),
                self.status.as_str(),
                self.configuration_removed,
                encode_event_field(&self.safe_summary)
            ),
        )
        .from_source(KernelEventSource::Runtime)
        .with_redaction(KernelEventRedaction::Internal)
        .with_payload_schema("sdkwork.agent.installation.report.v1")
    }
}

fn encode_event_field(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '%' => encoded.push_str("%25"),
            ';' => encoded.push_str("%3B"),
            '=' => encoded.push_str("%3D"),
            '\r' => encoded.push_str("%0D"),
            '\n' => encoded.push_str("%0A"),
            _ => encoded.push(character),
        }
    }
    encoded
}
