use crate::{PatchSet, TerminalCommand, Workspace};
use sdkwork_agent_kernel::{KernelResult, ProviderHealth, SideEffectLevel};

pub trait CodeSafetyProvider {
    fn assess_workspace(
        &self,
        workspace: &Workspace,
        scope: CodeSafetyScope,
    ) -> KernelResult<CodeSafetyAssessment>;

    fn assess_patch(
        &self,
        workspace: &Workspace,
        patch: &PatchSet,
    ) -> KernelResult<CodeSafetyAssessment>;

    fn assess_terminal_command(
        &self,
        workspace: &Workspace,
        command: &TerminalCommand,
    ) -> KernelResult<CodeSafetyAssessment>;

    fn health(&self) -> ProviderHealth;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeSafetyScope {
    pub root: String,
    pub allowed_paths: Vec<String>,
    pub denied_paths: Vec<String>,
}

impl CodeSafetyScope {
    pub fn new(root: impl Into<String>) -> Self {
        Self {
            root: root.into(),
            allowed_paths: Vec::new(),
            denied_paths: Vec::new(),
        }
    }

    pub fn allow_path(mut self, path: impl Into<String>) -> Self {
        self.allowed_paths.push(path.into());
        self
    }

    pub fn deny_path(mut self, path: impl Into<String>) -> Self {
        self.denied_paths.push(path.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeSafetyRiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeSafetyAssessment {
    pub assessment_id: String,
    pub risk_level: CodeSafetyRiskLevel,
    pub side_effect_level: SideEffectLevel,
    pub policy_categories: Vec<String>,
    pub reasons: Vec<String>,
    pub requires_approval: bool,
}

impl CodeSafetyAssessment {
    pub fn new(
        assessment_id: impl Into<String>,
        risk_level: CodeSafetyRiskLevel,
        side_effect_level: SideEffectLevel,
    ) -> Self {
        Self {
            assessment_id: assessment_id.into(),
            risk_level,
            side_effect_level,
            policy_categories: Vec::new(),
            reasons: Vec::new(),
            requires_approval: false,
        }
    }

    pub fn with_policy_categories(mut self, policy_categories: Vec<String>) -> Self {
        self.policy_categories = policy_categories;
        self
    }

    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reasons.push(reason.into());
        self
    }

    pub fn require_approval(mut self) -> Self {
        self.requires_approval = true;
        self
    }
}
