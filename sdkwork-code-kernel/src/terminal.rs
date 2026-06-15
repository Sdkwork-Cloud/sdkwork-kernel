use crate::first_policy_category;
use crate::{CommandResult, Workspace};
use sdkwork_agent_kernel::{
    HostEnvPolicy, KernelEventRedaction, KernelResult, PolicyCategory, PolicyRequest,
    ProviderHealth, SideEffectLevel,
};

pub trait TerminalProvider {
    fn run_command(
        &self,
        workspace: &Workspace,
        command: TerminalCommand,
    ) -> KernelResult<CommandResult>;

    fn stream_output(
        &self,
        workspace: &Workspace,
        command_id: &str,
    ) -> KernelResult<Vec<TerminalOutputChunk>>;

    fn cancel_command(
        &self,
        workspace: &Workspace,
        command_id: &str,
    ) -> KernelResult<CommandResult>;

    fn health(&self) -> ProviderHealth;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalCommand {
    pub command_id: String,
    pub command: String,
    pub args: Vec<String>,
    pub working_directory: String,
    pub timeout_ms: Option<u64>,
    pub env_policy: HostEnvPolicy,
    pub policy_categories: Vec<String>,
}

impl TerminalCommand {
    pub fn new(
        command_id: impl Into<String>,
        command: impl Into<String>,
        args: Vec<String>,
        working_directory: impl Into<String>,
    ) -> Self {
        Self {
            command_id: command_id.into(),
            command: command.into(),
            args,
            working_directory: working_directory.into(),
            timeout_ms: None,
            env_policy: HostEnvPolicy::None,
            policy_categories: Vec::new(),
        }
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    pub fn with_env_policy(mut self, env_policy: HostEnvPolicy) -> Self {
        self.env_policy = env_policy;
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
        let category = first_policy_category(&self.policy_categories, "code.terminal.run");
        let mut request = PolicyRequest::new(
            policy_request_id,
            category.clone(),
            format!(
                "workspace://{}/commands/{}",
                workspace.workspace_id, self.command_id
            ),
        )
        .with_category(PolicyCategory::ProductSpecific(category))
        .with_action("terminal.run")
        .with_side_effect_level(SideEffectLevel::SideEffectful)
        .with_context("workspace_id", workspace.workspace_id.clone())
        .with_context("command_id", self.command_id.clone())
        .with_context("command", self.command.clone())
        .with_context("args", self.args.join(" "))
        .with_context("working_directory", self.working_directory.clone())
        .with_context("env_policy", env_policy_name(&self.env_policy));

        if let Some(timeout_ms) = self.timeout_ms {
            request = request.with_context("timeout_ms", timeout_ms.to_string());
        }

        if !self.policy_categories.is_empty() {
            request = request.with_context("policy_categories", self.policy_categories.join(","));
        }

        request
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalOutputChannel {
    Stdout,
    Stderr,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalOutputChunk {
    pub command_id: String,
    pub sequence: u64,
    pub channel: TerminalOutputChannel,
    pub content: String,
    pub redaction_classification: KernelEventRedaction,
}

impl TerminalOutputChunk {
    pub fn new(
        command_id: impl Into<String>,
        sequence: u64,
        channel: TerminalOutputChannel,
        content: impl Into<String>,
    ) -> Self {
        Self {
            command_id: command_id.into(),
            sequence,
            channel,
            content: content.into(),
            redaction_classification: KernelEventRedaction::Unknown,
        }
    }

    pub fn stdout(
        command_id: impl Into<String>,
        sequence: u64,
        content: impl Into<String>,
    ) -> Self {
        Self::new(command_id, sequence, TerminalOutputChannel::Stdout, content)
    }

    pub fn stderr(
        command_id: impl Into<String>,
        sequence: u64,
        content: impl Into<String>,
    ) -> Self {
        Self::new(command_id, sequence, TerminalOutputChannel::Stderr, content)
    }

    pub fn system(
        command_id: impl Into<String>,
        sequence: u64,
        content: impl Into<String>,
    ) -> Self {
        Self::new(command_id, sequence, TerminalOutputChannel::System, content)
    }

    pub fn with_redaction(mut self, redaction_classification: KernelEventRedaction) -> Self {
        self.redaction_classification = redaction_classification;
        self
    }
}

fn env_policy_name(env_policy: &HostEnvPolicy) -> &'static str {
    match env_policy {
        HostEnvPolicy::None => "none",
        HostEnvPolicy::Inherit => "inherit",
        HostEnvPolicy::AllowList(_) => "allow_list",
        HostEnvPolicy::Explicit(_) => "explicit",
    }
}
