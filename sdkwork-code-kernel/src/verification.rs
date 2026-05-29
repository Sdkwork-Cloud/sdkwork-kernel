use crate::{TerminalCommand, Workspace};
use sdkwork_agent_kernel::{
    KernelResult, PolicyCategory, PolicyRequest, ProviderHealth, SideEffectLevel,
};

pub trait VerificationProvider {
    fn discover_plans(&self, workspace: &Workspace) -> KernelResult<Vec<VerificationPlan>>;

    fn run_verification(
        &self,
        workspace: &Workspace,
        plan: VerificationPlan,
    ) -> KernelResult<VerificationReport>;

    fn health(&self) -> ProviderHealth;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationPlan {
    pub verification_id: String,
    pub workspace_id: String,
    pub commands: Vec<TerminalCommand>,
}

impl VerificationPlan {
    pub fn new(verification_id: impl Into<String>, workspace_id: impl Into<String>) -> Self {
        Self {
            verification_id: verification_id.into(),
            workspace_id: workspace_id.into(),
            commands: Vec::new(),
        }
    }

    pub fn add_command(mut self, command: TerminalCommand) -> Self {
        self.commands.push(command);
        self
    }

    pub fn to_policy_request(&self, policy_request_id: impl Into<String>) -> PolicyRequest {
        PolicyRequest::new(
            policy_request_id,
            "code.verification.run",
            format!(
                "workspace://{}/verifications/{}",
                self.workspace_id, self.verification_id
            ),
        )
        .with_category(PolicyCategory::ProductSpecific(
            "code.verification.run".to_string(),
        ))
        .with_action("verification.run")
        .with_side_effect_level(SideEffectLevel::SideEffectful)
        .with_context("workspace_id", self.workspace_id.clone())
        .with_context("verification_id", self.verification_id.clone())
        .with_context("command_count", self.commands.len().to_string())
        .with_context("command_ids", self.command_ids().join(","))
    }

    pub fn command_ids(&self) -> Vec<String> {
        self.commands
            .iter()
            .map(|command| command.command_id.clone())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandResult {
    pub command_id: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub cancelled: bool,
    pub timed_out: bool,
}

impl CommandResult {
    pub fn exited(
        command_id: impl Into<String>,
        exit_code: i32,
        stdout: impl Into<String>,
        stderr: impl Into<String>,
    ) -> Self {
        Self {
            command_id: command_id.into(),
            exit_code: Some(exit_code),
            stdout: stdout.into(),
            stderr: stderr.into(),
            cancelled: false,
            timed_out: false,
        }
    }

    pub fn is_success(&self) -> bool {
        self.exit_code == Some(0) && !self.cancelled && !self.timed_out
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationReport {
    pub report_id: String,
    pub verification_id: String,
    pub command_results: Vec<CommandResult>,
    pub failures: Vec<String>,
}

impl VerificationReport {
    pub fn new(report_id: impl Into<String>, verification_id: impl Into<String>) -> Self {
        Self {
            report_id: report_id.into(),
            verification_id: verification_id.into(),
            command_results: Vec::new(),
            failures: Vec::new(),
        }
    }

    pub fn add_command_result(mut self, command_result: CommandResult) -> Self {
        self.command_results.push(command_result);
        self
    }

    pub fn add_failure(mut self, failure: impl Into<String>) -> Self {
        self.failures.push(failure.into());
        self
    }

    pub fn is_success(&self) -> bool {
        self.failures.is_empty() && self.command_results.iter().all(CommandResult::is_success)
    }
}
