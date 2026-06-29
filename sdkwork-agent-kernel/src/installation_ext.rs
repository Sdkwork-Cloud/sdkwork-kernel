//! Additional structures for AgentInstaller SPI
//!
//! Rollback, verify, and list_installed support

/// Rollback request for reverting to a previous version
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRollbackRequest {
    pub request_id: String,
    pub agent_id: String,
    pub target_version: Option<String>,
    pub rollback_token: Option<String>,
    pub preserve_data: bool,
    pub requested_by: Option<String>,
}

impl AgentRollbackRequest {
    pub fn new(request_id: impl Into<String>, agent_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            agent_id: agent_id.into(),
            target_version: None,
            rollback_token: None,
            preserve_data: true,
            requested_by: None,
        }
    }

    pub fn to_version(mut self, version: impl Into<String>) -> Self {
        self.target_version = Some(version.into());
        self
    }

    pub fn with_rollback_token(mut self, token: impl Into<String>) -> Self {
        self.rollback_token = Some(token.into());
        self
    }

    pub fn preserve_data(mut self, preserve: bool) -> Self {
        self.preserve_data = preserve;
        self
    }

    pub fn requested_by(mut self, requested_by: impl Into<String>) -> Self {
        self.requested_by = Some(requested_by.into());
        self
    }
}

/// Rollback report
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRollbackReport {
    pub request_id: String,
    pub agent_id: String,
    pub status: AgentRollbackStatus,
    pub from_version: String,
    pub to_version: String,
    pub message: Option<String>,
}

impl AgentRollbackReport {
    pub fn success(
        request_id: impl Into<String>,
        agent_id: impl Into<String>,
        from_version: impl Into<String>,
        to_version: impl Into<String>,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            agent_id: agent_id.into(),
            status: AgentRollbackStatus::Success,
            from_version: from_version.into(),
            to_version: to_version.into(),
            message: None,
        }
    }

    pub fn failed(
        request_id: impl Into<String>,
        agent_id: impl Into<String>,
        from_version: impl Into<String>,
        to_version: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            agent_id: agent_id.into(),
            status: AgentRollbackStatus::Failed,
            from_version: from_version.into(),
            to_version: to_version.into(),
            message: Some(message.into()),
        }
    }
}

/// Rollback status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRollbackStatus {
    InProgress,
    Success,
    Failed,
    Partial,
}

/// Verify request for checking installed agent integrity
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentVerifyRequest {
    pub request_id: String,
    pub agent_id: String,
    pub version: Option<String>,
    pub verify_checksum: bool,
    pub verify_configuration: bool,
    pub verify_capabilities: bool,
}

impl AgentVerifyRequest {
    pub fn new(request_id: impl Into<String>, agent_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            agent_id: agent_id.into(),
            version: None,
            verify_checksum: true,
            verify_configuration: true,
            verify_capabilities: true,
        }
    }

    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    pub fn verify_checksum(mut self, verify: bool) -> Self {
        self.verify_checksum = verify;
        self
    }

    pub fn verify_configuration(mut self, verify: bool) -> Self {
        self.verify_configuration = verify;
        self
    }

    pub fn verify_capabilities(mut self, verify: bool) -> Self {
        self.verify_capabilities = verify;
        self
    }
}

/// Verify report
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentVerifyReport {
    pub request_id: String,
    pub agent_id: String,
    pub status: AgentVerifyStatus,
    pub checksum_valid: Option<bool>,
    pub configuration_valid: Option<bool>,
    pub capabilities_valid: Option<bool>,
    pub issues: Vec<AgentVerifyIssue>,
}

impl AgentVerifyReport {
    pub fn valid(request_id: impl Into<String>, agent_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            agent_id: agent_id.into(),
            status: AgentVerifyStatus::Valid,
            checksum_valid: Some(true),
            configuration_valid: Some(true),
            capabilities_valid: Some(true),
            issues: Vec::new(),
        }
    }

    pub fn with_issues(
        request_id: impl Into<String>,
        agent_id: impl Into<String>,
        issues: Vec<AgentVerifyIssue>,
    ) -> Self {
        let status = if issues.iter().any(|i| i.severity == AgentVerifyIssueSeverity::Critical) {
            AgentVerifyStatus::Invalid
        } else {
            AgentVerifyStatus::Warnings
        };

        Self {
            request_id: request_id.into(),
            agent_id: agent_id.into(),
            status,
            checksum_valid: None,
            configuration_valid: None,
            capabilities_valid: None,
            issues,
        }
    }
}

/// Verify status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentVerifyStatus {
    Valid,
    Warnings,
    Invalid,
    NotFound,
}

/// Verify issue
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentVerifyIssue {
    pub severity: AgentVerifyIssueSeverity,
    pub category: AgentVerifyIssueCategory,
    pub message: String,
    pub details: Option<String>,
}

impl AgentVerifyIssue {
    pub fn critical(category: AgentVerifyIssueCategory, message: impl Into<String>) -> Self {
        Self {
            severity: AgentVerifyIssueSeverity::Critical,
            category,
            message: message.into(),
            details: None,
        }
    }

    pub fn warning(category: AgentVerifyIssueCategory, message: impl Into<String>) -> Self {
        Self {
            severity: AgentVerifyIssueSeverity::Warning,
            category,
            message: message.into(),
            details: None,
        }
    }

    pub fn info(category: AgentVerifyIssueCategory, message: impl Into<String>) -> Self {
        Self {
            severity: AgentVerifyIssueSeverity::Info,
            category,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

/// Verify issue severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentVerifyIssueSeverity {
    Info,
    Warning,
    Critical,
}

/// Verify issue category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentVerifyIssueCategory {
    Checksum,
    Configuration,
    Capability,
    Dependency,
    Security,
    Performance,
}

/// Installation record for list_installed
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentInstallRecord {
    pub agent_id: String,
    pub version: String,
    pub installed_at: String,
    pub source: AgentPackageSourceInfo,
    pub status: AgentInstallRecordStatus,
    pub configuration_profile: Option<String>,
    pub capabilities: Vec<String>,
}

impl AgentInstallRecord {
    pub fn new(
        agent_id: impl Into<String>,
        version: impl Into<String>,
        installed_at: impl Into<String>,
    ) -> Self {
        Self {
            agent_id: agent_id.into(),
            version: version.into(),
            installed_at: installed_at.into(),
            source: AgentPackageSourceInfo::Unknown,
            status: AgentInstallRecordStatus::Active,
            configuration_profile: None,
            capabilities: Vec::new(),
        }
    }

    pub fn with_source(mut self, source: AgentPackageSourceInfo) -> Self {
        self.source = source;
        self
    }

    pub fn with_status(mut self, status: AgentInstallRecordStatus) -> Self {
        self.status = status;
        self
    }

    pub fn with_configuration_profile(mut self, profile: impl Into<String>) -> Self {
        self.configuration_profile = Some(profile.into());
        self
    }

    pub fn with_capabilities(mut self, capabilities: Vec<String>) -> Self {
        self.capabilities = capabilities;
        self
    }
}

/// Package source info for installed record
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentPackageSourceInfo {
    Registry { registry_id: String, package_id: String },
    Local { path: String },
    Remote { url: String },
    Unknown,
}

/// Installation record status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentInstallRecordStatus {
    Active,
    Inactive,
    Deprecated,
    Broken,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollback_request_builder() {
        let req = AgentRollbackRequest::new("req-1", "agent-1")
            .to_version("1.0.0")
            .with_rollback_token("token-123")
            .preserve_data(false)
            .requested_by("user-1");

        assert_eq!(req.request_id, "req-1");
        assert_eq!(req.agent_id, "agent-1");
        assert_eq!(req.target_version, Some("1.0.0".to_string()));
        assert_eq!(req.rollback_token, Some("token-123".to_string()));
        assert!(!req.preserve_data);
        assert_eq!(req.requested_by, Some("user-1".to_string()));
    }

    #[test]
    fn verify_request_builder() {
        let req = AgentVerifyRequest::new("req-1", "agent-1")
            .version("2.0.0")
            .verify_checksum(true)
            .verify_configuration(false);

        assert_eq!(req.request_id, "req-1");
        assert_eq!(req.agent_id, "agent-1");
        assert_eq!(req.version, Some("2.0.0".to_string()));
        assert!(req.verify_checksum);
        assert!(!req.verify_configuration);
        assert!(req.verify_capabilities);
    }

    #[test]
    fn verify_report_valid() {
        let report = AgentVerifyReport::valid("req-1", "agent-1");

        assert_eq!(report.status, AgentVerifyStatus::Valid);
        assert_eq!(report.checksum_valid, Some(true));
        assert_eq!(report.configuration_valid, Some(true));
        assert_eq!(report.capabilities_valid, Some(true));
        assert!(report.issues.is_empty());
    }

    #[test]
    fn verify_report_with_issues() {
        let issues = vec![
            AgentVerifyIssue::warning(AgentVerifyIssueCategory::Configuration, "Deprecated config"),
            AgentVerifyIssue::critical(AgentVerifyIssueCategory::Checksum, "Checksum mismatch"),
        ];

        let report = AgentVerifyReport::with_issues("req-1", "agent-1", issues);

        assert_eq!(report.status, AgentVerifyStatus::Invalid);
        assert_eq!(report.issues.len(), 2);
    }

    #[test]
    fn install_record_builder() {
        let record = AgentInstallRecord::new("agent-1", "1.0.0", "2024-01-01T00:00:00Z")
            .with_source(AgentPackageSourceInfo::Registry {
                registry_id: "registry-1".to_string(),
                package_id: "pkg-1".to_string(),
            })
            .with_status(AgentInstallRecordStatus::Active)
            .with_configuration_profile("prod")
            .with_capabilities(vec!["cap1".to_string(), "cap2".to_string()]);

        assert_eq!(record.agent_id, "agent-1");
        assert_eq!(record.version, "1.0.0");
        assert_eq!(record.status, AgentInstallRecordStatus::Active);
        assert_eq!(record.configuration_profile, Some("prod".to_string()));
        assert_eq!(record.capabilities.len(), 2);
    }

    #[test]
    fn rollback_report_success() {
        let report = AgentRollbackReport::success("req-1", "agent-1", "2.0.0", "1.0.0");

        assert_eq!(report.status, AgentRollbackStatus::Success);
        assert_eq!(report.from_version, "2.0.0");
        assert_eq!(report.to_version, "1.0.0");
        assert!(report.message.is_none());
    }

    #[test]
    fn rollback_report_failed() {
        let report = AgentRollbackReport::failed(
            "req-1",
            "agent-1",
            "2.0.0",
            "1.0.0",
            "Checksum verification failed",
        );

        assert_eq!(report.status, AgentRollbackStatus::Failed);
        assert_eq!(report.message, Some("Checksum verification failed".to_string()));
    }
}