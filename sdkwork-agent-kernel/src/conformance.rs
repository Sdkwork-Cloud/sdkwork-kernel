#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelConformanceCaseStatus {
    Passed,
    Failed,
    Skipped,
}

impl KernelConformanceCaseStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRuntimeConformanceProfile {
    Manifest,
    LocalRuntime,
}

impl AgentRuntimeConformanceProfile {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Manifest => "runtime-manifest",
            Self::LocalRuntime => "runtime-local",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelConformanceCase {
    pub case_id: String,
    pub status: KernelConformanceCaseStatus,
    pub message: String,
    pub required: bool,
    pub capability_id: Option<String>,
    pub skip_reason: Option<String>,
}

impl KernelConformanceCase {
    pub fn passed(case_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(case_id, KernelConformanceCaseStatus::Passed, message)
    }

    pub fn failed(case_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(case_id, KernelConformanceCaseStatus::Failed, message)
    }

    pub fn skipped(case_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(case_id, KernelConformanceCaseStatus::Skipped, message)
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn for_capability(mut self, capability_id: impl Into<String>) -> Self {
        self.capability_id = Some(capability_id.into());
        self
    }

    pub fn with_skip_reason(mut self, skip_reason: impl Into<String>) -> Self {
        self.skip_reason = Some(skip_reason.into());
        self
    }

    fn new(
        case_id: impl Into<String>,
        status: KernelConformanceCaseStatus,
        message: impl Into<String>,
    ) -> Self {
        Self {
            case_id: case_id.into(),
            status,
            message: message.into(),
            required: false,
            capability_id: None,
            skip_reason: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelConformanceReport {
    pub report_id: String,
    pub profile_id: String,
    pub implementation_id: String,
    pub implementation_version: String,
    pub spec_version: Option<String>,
    pub test_suite_version: Option<String>,
    pub security_profile: Option<String>,
    pub required_capabilities: Vec<String>,
    pub cases: Vec<KernelConformanceCase>,
}

impl KernelConformanceReport {
    pub fn new(
        report_id: impl Into<String>,
        profile_id: impl Into<String>,
        implementation_id: impl Into<String>,
        implementation_version: impl Into<String>,
    ) -> Self {
        Self {
            report_id: report_id.into(),
            profile_id: profile_id.into(),
            implementation_id: implementation_id.into(),
            implementation_version: implementation_version.into(),
            spec_version: None,
            test_suite_version: None,
            security_profile: None,
            required_capabilities: Vec::new(),
            cases: Vec::new(),
        }
    }

    pub fn with_spec_version(mut self, spec_version: impl Into<String>) -> Self {
        self.spec_version = Some(spec_version.into());
        self
    }

    pub fn with_test_suite_version(mut self, test_suite_version: impl Into<String>) -> Self {
        self.test_suite_version = Some(test_suite_version.into());
        self
    }

    pub fn with_security_profile(mut self, security_profile: impl Into<String>) -> Self {
        self.security_profile = Some(security_profile.into());
        self
    }

    pub fn with_required_capability(mut self, capability_id: impl Into<String>) -> Self {
        let capability_id = capability_id.into();
        if !self.required_capabilities.contains(&capability_id) {
            self.required_capabilities.push(capability_id);
        }
        self
    }

    pub fn add_case(mut self, case: KernelConformanceCase) -> Self {
        self.cases.push(case);
        self
    }

    pub fn case(&self, case_id: &str) -> Option<&KernelConformanceCase> {
        self.cases.iter().find(|case| case.case_id == case_id)
    }

    pub fn passed_count(&self) -> usize {
        self.count_status(KernelConformanceCaseStatus::Passed)
    }

    pub fn failed_count(&self) -> usize {
        self.count_status(KernelConformanceCaseStatus::Failed)
    }

    pub fn skipped_count(&self) -> usize {
        self.count_status(KernelConformanceCaseStatus::Skipped)
    }

    pub fn failed_case_ids(&self) -> Vec<String> {
        self.cases
            .iter()
            .filter(|case| case.status == KernelConformanceCaseStatus::Failed)
            .map(|case| case.case_id.clone())
            .collect()
    }

    pub fn required_skipped_case_ids(&self) -> Vec<String> {
        self.cases
            .iter()
            .filter(|case| case.required && case.status == KernelConformanceCaseStatus::Skipped)
            .map(|case| case.case_id.clone())
            .collect()
    }

    pub fn is_passed(&self) -> bool {
        self.failed_count() == 0 && self.required_skipped_case_ids().is_empty()
    }

    fn count_status(&self, status: KernelConformanceCaseStatus) -> usize {
        self.cases
            .iter()
            .filter(|case| case.status == status)
            .count()
    }
}
