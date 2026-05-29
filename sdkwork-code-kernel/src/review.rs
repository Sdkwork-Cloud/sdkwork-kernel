use crate::{PatchSet, VerificationReport, Workspace};
use sdkwork_agent_kernel::{KernelResult, ProviderHealth};

pub trait ReviewProvider {
    fn review_patch(&self, workspace: &Workspace, patch: &PatchSet) -> KernelResult<ReviewReport>;

    fn review_verification(
        &self,
        workspace: &Workspace,
        report: &VerificationReport,
    ) -> KernelResult<ReviewReport>;

    fn health(&self) -> ProviderHealth;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewFinding {
    pub finding_id: String,
    pub severity: ReviewSeverity,
    pub file_path: String,
    pub line: Option<u32>,
    pub message: String,
    pub remediation: Option<String>,
    pub missing_test: Option<String>,
}

impl ReviewFinding {
    pub fn new(
        finding_id: impl Into<String>,
        severity: ReviewSeverity,
        file_path: impl Into<String>,
        line: u32,
        message: impl Into<String>,
    ) -> Self {
        Self {
            finding_id: finding_id.into(),
            severity,
            file_path: file_path.into(),
            line: Some(line),
            message: message.into(),
            remediation: None,
            missing_test: None,
        }
    }

    pub fn with_remediation(mut self, remediation: impl Into<String>) -> Self {
        self.remediation = Some(remediation.into());
        self
    }

    pub fn with_missing_test(mut self, missing_test: impl Into<String>) -> Self {
        self.missing_test = Some(missing_test.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewReport {
    pub report_id: String,
    pub workspace_id: String,
    pub findings: Vec<ReviewFinding>,
    pub risk_summary: Option<String>,
    pub missing_tests: Vec<String>,
    pub artifact_ids: Vec<String>,
}

impl ReviewReport {
    pub fn new(report_id: impl Into<String>, workspace_id: impl Into<String>) -> Self {
        Self {
            report_id: report_id.into(),
            workspace_id: workspace_id.into(),
            findings: Vec::new(),
            risk_summary: None,
            missing_tests: Vec::new(),
            artifact_ids: Vec::new(),
        }
    }

    pub fn add_finding(mut self, finding: ReviewFinding) -> Self {
        self.findings.push(finding);
        self
    }

    pub fn with_risk_summary(mut self, risk_summary: impl Into<String>) -> Self {
        self.risk_summary = Some(risk_summary.into());
        self
    }

    pub fn add_missing_test(mut self, missing_test: impl Into<String>) -> Self {
        self.missing_tests.push(missing_test.into());
        self
    }

    pub fn add_artifact_id(mut self, artifact_id: impl Into<String>) -> Self {
        self.artifact_ids.push(artifact_id.into());
        self
    }
}
