use crate::{
    KernelError, KernelEventRedaction, KernelResult, ProviderHealth, ProviderManifest,
    SideEffectLevel, ToolSchema, TraceContext,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSkillInvocationMode {
    ModelInvocable,
    ToolBacked,
    Workflow,
    HostProvided,
}

impl AgentSkillInvocationMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ModelInvocable => "model_invocable",
            Self::ToolBacked => "tool_backed",
            Self::Workflow => "workflow",
            Self::HostProvided => "host_provided",
        }
    }
}

/// SKILL.md frontmatter contract, aligned with the agent skill ecosystem
/// (Anthropic Agent Skills, ZCode `SKILL.md`, and SDKWORK workspace
/// `.sdkwork/skills/<name>/SKILL.md` conventions).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillMarkdownFrontmatter {
    pub name: String,
    pub description: String,
    pub version: Option<String>,
    pub license: Option<String>,
    pub argument_hint: Option<String>,
    pub allowed_tools: Vec<String>,
    pub disallowed_tools: Vec<String>,
    pub paths: Vec<String>,
}

impl SkillMarkdownFrontmatter {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            version: None,
            license: None,
            argument_hint: None,
            allowed_tools: Vec::new(),
            disallowed_tools: Vec::new(),
            paths: Vec::new(),
        }
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    pub fn with_license(mut self, license: impl Into<String>) -> Self {
        self.license = Some(license.into());
        self
    }

    pub fn with_argument_hint(mut self, argument_hint: impl Into<String>) -> Self {
        self.argument_hint = Some(argument_hint.into());
        self
    }

    pub fn with_allowed_tool(mut self, tool: impl Into<String>) -> Self {
        self.allowed_tools.push(tool.into());
        self
    }

    pub fn with_disallowed_tool(mut self, tool: impl Into<String>) -> Self {
        self.disallowed_tools.push(tool.into());
        self
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.paths.push(path.into());
        self
    }
}

/// Content layer of a skill, following the three-layer progressive
/// disclosure model: SKILL.md body always resident, `references/` and
/// `scripts/` loaded on demand, `assets/` never loaded into context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillContentLayer {
    Body,
    References,
    Scripts,
    Assets,
}

impl SkillContentLayer {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Body => "body",
            Self::References => "references",
            Self::Scripts => "scripts",
            Self::Assets => "assets",
        }
    }
}

/// A skill content file with the one-line description used for progressive
/// disclosure decisions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillContentFile {
    pub path: String,
    pub description: Option<String>,
    pub size_hint: Option<u64>,
}

impl SkillContentFile {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            description: None,
            size_hint: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_size_hint(mut self, size_hint: u64) -> Self {
        self.size_hint = Some(size_hint);
        self
    }
}

/// Three-layer skill content layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillContentLayout {
    /// SKILL.md body (resident in context).
    pub body: String,
    /// `references/` — loaded on demand.
    pub references: Vec<SkillContentFile>,
    /// `scripts/` — executed or read on demand.
    pub scripts: Vec<SkillContentFile>,
    /// `assets/` — output resources, never loaded into context.
    pub assets: Vec<SkillContentFile>,
}

impl SkillContentLayout {
    pub fn with_body(body: impl Into<String>) -> Self {
        Self {
            body: body.into(),
            references: Vec::new(),
            scripts: Vec::new(),
            assets: Vec::new(),
        }
    }

    pub fn with_reference(mut self, reference: SkillContentFile) -> Self {
        self.references.push(reference);
        self
    }

    pub fn with_script(mut self, script: SkillContentFile) -> Self {
        self.scripts.push(script);
        self
    }

    pub fn with_asset(mut self, asset: SkillContentFile) -> Self {
        self.assets.push(asset);
        self
    }
}

/// Skill visibility control, aligned with the `skillOverrides` settings
/// (`off` / `user-invocable-only` / `name-only`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillVisibility {
    /// Default: description-driven model invocation is allowed.
    ModelInvocable,
    /// Only explicit user invocation (or the Skill tool) can trigger it.
    UserInvocableOnly,
    /// Only exact-name invocation can trigger it.
    NameOnly,
    /// Disabled entirely.
    Off,
}

impl SkillVisibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ModelInvocable => "model_invocable",
            Self::UserInvocableOnly => "user_invocable_only",
            Self::NameOnly => "name_only",
            Self::Off => "off",
        }
    }

    /// Whether the model may autonomously invoke the skill from its
    /// description alone.
    pub fn allows_model_invocation(&self) -> bool {
        matches!(self, Self::ModelInvocable)
    }
}

/// Parse the YAML-style frontmatter block of a SKILL.md document
/// (`---` delimited). Unknown keys are ignored; known list keys accept
/// comma-separated values. Returns `None` when no frontmatter block exists.
pub fn parse_skill_markdown_frontmatter(input: &str) -> Option<SkillMarkdownFrontmatter> {
    let trimmed = input.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }
    let rest = &trimmed[3..];
    let end = rest.find("\n---")?;
    let block = &rest[..end];

    let mut frontmatter = SkillMarkdownFrontmatter::new("", "");
    let mut seen_name = false;
    let mut seen_description = false;
    for line in block.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim().trim_matches('"').trim();
        match key {
            "name" => {
                frontmatter.name = value.to_string();
                seen_name = true;
            }
            "description" => {
                frontmatter.description = value.to_string();
                seen_description = true;
            }
            "version" => frontmatter.version = Some(value.to_string()),
            "license" => frontmatter.license = Some(value.to_string()),
            "argument-hint" => frontmatter.argument_hint = Some(value.to_string()),
            "allowed-tools" => {
                frontmatter.allowed_tools = comma_list(value);
            }
            "disallowed-tools" => {
                frontmatter.disallowed_tools = comma_list(value);
            }
            "paths" => {
                frontmatter.paths = comma_list(value);
            }
            _ => {}
        }
    }
    if !seen_name && !seen_description {
        return None;
    }
    Some(frontmatter)
}

fn comma_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSkillDescriptor {
    pub skill_id: String,
    pub provider_id: String,
    pub display_name: String,
    pub description: String,
    pub invocation_mode: AgentSkillInvocationMode,
    pub version: Option<String>,
    pub model_hint: Option<String>,
    pub allowed_tools: Vec<String>,
    pub input_schema: Option<ToolSchema>,
    pub output_schema: Option<ToolSchema>,
    pub side_effect_level: SideEffectLevel,
    pub policy_categories: Vec<String>,
    pub timeout_ms: Option<u64>,
    pub cancellation_supported: bool,
    pub audit_required: bool,
    pub metadata: Vec<(String, String)>,
    /// SKILL.md frontmatter when the skill ships as a markdown skill.
    pub frontmatter: Option<SkillMarkdownFrontmatter>,
    /// Progressive-disclosure content layout when the skill ships files.
    pub content_layout: Option<SkillContentLayout>,
    /// Context budget hint (characters) for the skill body.
    pub context_budget: Option<u64>,
    /// Skill visibility override.
    pub visibility: SkillVisibility,
    pub disallowed_tools: Vec<String>,
    pub paths: Vec<String>,
    pub argument_hint: Option<String>,
}

impl AgentSkillDescriptor {
    pub fn new(
        skill_id: impl Into<String>,
        provider_id: impl Into<String>,
        display_name: impl Into<String>,
        description: impl Into<String>,
        invocation_mode: AgentSkillInvocationMode,
    ) -> Self {
        Self {
            skill_id: skill_id.into(),
            provider_id: provider_id.into(),
            display_name: display_name.into(),
            description: description.into(),
            invocation_mode,
            version: None,
            model_hint: None,
            allowed_tools: Vec::new(),
            input_schema: None,
            output_schema: None,
            side_effect_level: SideEffectLevel::SideEffectful,
            policy_categories: vec!["skill.invoke".to_string()],
            timeout_ms: None,
            cancellation_supported: false,
            audit_required: false,
            metadata: Vec::new(),
            frontmatter: None,
            content_layout: None,
            context_budget: None,
            visibility: SkillVisibility::ModelInvocable,
            disallowed_tools: Vec::new(),
            paths: Vec::new(),
            argument_hint: None,
        }
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    pub fn with_model_hint(mut self, model_hint: impl Into<String>) -> Self {
        self.model_hint = Some(model_hint.into());
        self
    }

    pub fn with_allowed_tool(mut self, tool: impl Into<String>) -> Self {
        self.allowed_tools.push(tool.into());
        self
    }

    pub fn with_input_schema(mut self, input_schema: ToolSchema) -> Self {
        self.input_schema = Some(input_schema);
        self
    }

    pub fn with_output_schema(mut self, output_schema: ToolSchema) -> Self {
        self.output_schema = Some(output_schema);
        self
    }

    pub fn with_side_effect_level(mut self, side_effect_level: SideEffectLevel) -> Self {
        self.side_effect_level = side_effect_level;
        self
    }

    pub fn with_policy_category(mut self, policy_category: impl Into<String>) -> Self {
        self.policy_categories.push(policy_category.into());
        self
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    pub fn supports_cancellation(mut self, cancellation_supported: bool) -> Self {
        self.cancellation_supported = cancellation_supported;
        self
    }

    pub fn require_audit(mut self) -> Self {
        self.audit_required = true;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }

    pub fn with_frontmatter(mut self, frontmatter: SkillMarkdownFrontmatter) -> Self {
        self.frontmatter = Some(frontmatter);
        self
    }

    pub fn with_content_layout(mut self, content_layout: SkillContentLayout) -> Self {
        self.content_layout = Some(content_layout);
        self
    }

    pub fn with_context_budget(mut self, context_budget: u64) -> Self {
        self.context_budget = Some(context_budget);
        self
    }

    pub fn with_visibility(mut self, visibility: SkillVisibility) -> Self {
        self.visibility = visibility;
        self
    }

    pub fn with_disallowed_tool(mut self, tool: impl Into<String>) -> Self {
        self.disallowed_tools.push(tool.into());
        self
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.paths.push(path.into());
        self
    }

    pub fn with_argument_hint(mut self, argument_hint: impl Into<String>) -> Self {
        self.argument_hint = Some(argument_hint.into());
        self
    }

    pub fn with_license(mut self, license: impl Into<String>) -> Self {
        self.frontmatter = Some(
            self.frontmatter
                .unwrap_or_else(|| SkillMarkdownFrontmatter::new("", ""))
                .with_license(license),
        );
        self
    }

    /// Whether the model may autonomously invoke this skill from its
    /// description alone, honoring the visibility override.
    pub fn allows_model_invocation(&self) -> bool {
        self.visibility.allows_model_invocation()
    }

    /// Whether a tool call with this name may invoke the skill.
    pub fn allows_tool_invocation(&self, tool_name: &str) -> bool {
        !matches!(self.visibility, SkillVisibility::Off)
            && !self.disallowed_tools.iter().any(|tool| tool == tool_name)
    }

    pub fn requires_policy(&self) -> bool {
        self.side_effect_level != SideEffectLevel::ReadOnly || !self.policy_categories.is_empty()
    }

    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        self.metadata
            .iter()
            .find(|(metadata_key, _)| metadata_key == key)
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSkillRequest {
    pub skill_request_id: String,
    pub skill_id: String,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub step_id: Option<String>,
    pub arguments: Vec<(String, String)>,
    pub policy_decision_id: Option<String>,
    pub trace_context: Option<TraceContext>,
    pub timeout_ms: Option<u64>,
    pub metadata: Vec<(String, String)>,
}

impl AgentSkillRequest {
    pub fn new(skill_request_id: impl Into<String>, skill_id: impl Into<String>) -> Self {
        Self {
            skill_request_id: skill_request_id.into(),
            skill_id: skill_id.into(),
            session_id: None,
            task_id: None,
            run_id: None,
            step_id: None,
            arguments: Vec::new(),
            policy_decision_id: None,
            trace_context: None,
            timeout_ms: None,
            metadata: Vec::new(),
        }
    }

    pub fn for_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn for_task(mut self, task_id: impl Into<String>) -> Self {
        self.task_id = Some(task_id.into());
        self
    }

    pub fn for_run(mut self, run_id: impl Into<String>) -> Self {
        self.run_id = Some(run_id.into());
        self
    }

    pub fn for_step(mut self, step_id: impl Into<String>) -> Self {
        self.step_id = Some(step_id.into());
        self
    }

    pub fn with_argument(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.arguments.push((key.into(), value.into()));
        self
    }

    pub fn with_policy_context(mut self, policy_decision_id: impl Into<String>) -> Self {
        self.policy_decision_id = Some(policy_decision_id.into());
        self
    }

    pub fn with_trace_context(mut self, trace_context: TraceContext) -> Self {
        self.trace_context = Some(trace_context);
        self
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }

    pub fn argument_value(&self, key: &str) -> Option<&str> {
        self.arguments
            .iter()
            .find(|(argument_key, _)| argument_key == key)
            .map(|(_, value)| value.as_str())
    }

    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        self.metadata
            .iter()
            .find(|(metadata_key, _)| metadata_key == key)
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSkillStatus {
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
    PolicyDenied,
}

impl AgentSkillStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::TimedOut => "timed_out",
            Self::PolicyDenied => "policy_denied",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSkillResult {
    pub skill_request_id: String,
    pub skill_id: String,
    pub status: AgentSkillStatus,
    pub output: String,
    pub error: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub trace_context: Option<TraceContext>,
    pub redaction_classification: KernelEventRedaction,
    pub audit_refs: Vec<String>,
    pub diagnostics: Vec<String>,
}

impl AgentSkillResult {
    pub fn succeeded(
        skill_request_id: impl Into<String>,
        skill_id: impl Into<String>,
        output: impl Into<String>,
    ) -> Self {
        Self {
            skill_request_id: skill_request_id.into(),
            skill_id: skill_id.into(),
            status: AgentSkillStatus::Succeeded,
            output: output.into(),
            error: None,
            started_at: None,
            completed_at: None,
            duration_ms: None,
            trace_context: None,
            redaction_classification: KernelEventRedaction::Unknown,
            audit_refs: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn failed(
        skill_request_id: impl Into<String>,
        skill_id: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        Self {
            skill_request_id: skill_request_id.into(),
            skill_id: skill_id.into(),
            status: AgentSkillStatus::Failed,
            output: String::new(),
            error: Some(error.into()),
            started_at: None,
            completed_at: None,
            duration_ms: None,
            trace_context: None,
            redaction_classification: KernelEventRedaction::Unknown,
            audit_refs: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn with_status(mut self, status: AgentSkillStatus) -> Self {
        self.status = status;
        self
    }

    pub fn started_at(mut self, started_at: impl Into<String>) -> Self {
        self.started_at = Some(started_at.into());
        self
    }

    pub fn completed_at(mut self, completed_at: impl Into<String>) -> Self {
        self.completed_at = Some(completed_at.into());
        self
    }

    pub fn with_duration_ms(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    pub fn with_trace_context(mut self, trace_context: TraceContext) -> Self {
        self.trace_context = Some(trace_context);
        self
    }

    pub fn with_redaction(mut self, redaction_classification: KernelEventRedaction) -> Self {
        self.redaction_classification = redaction_classification;
        self
    }

    pub fn with_audit_ref(mut self, audit_ref: impl Into<String>) -> Self {
        self.audit_refs.push(audit_ref.into());
        self
    }

    pub fn with_diagnostic(mut self, diagnostic: impl Into<String>) -> Self {
        self.diagnostics.push(diagnostic.into());
        self
    }
}

pub trait AgentSkillProvider {
    fn provider_manifest(&self) -> ProviderManifest;

    fn health(&self) -> ProviderHealth;

    fn list_skills(&self) -> Vec<AgentSkillDescriptor>;

    fn describe_skill(&self, skill_id: &str) -> KernelResult<AgentSkillDescriptor> {
        self.list_skills()
            .into_iter()
            .find(|skill| skill.skill_id == skill_id)
            .ok_or_else(|| KernelError::CapabilityMissing {
                capability_id: skill_id.to_string(),
            })
    }

    /// Prepare a skill for invocation: load frontmatter, resolve content
    /// layers, warm caches. Providers without a prepare phase may leave the
    /// default no-op.
    fn prepare_skill(&self, _skill_id: &str) -> KernelResult<()> {
        Ok(())
    }

    /// Load a content file from a skill's progressive-disclosure layer.
    /// Providers without file-backed skills report capability missing.
    fn load_skill_content(
        &self,
        _skill_id: &str,
        _layer: SkillContentLayer,
        _path: &str,
    ) -> KernelResult<String> {
        Err(KernelError::CapabilityMissing {
            capability_id: format!("skill.content.{_layer:?}"),
        })
    }

    fn invoke_skill(&self, request: AgentSkillRequest) -> KernelResult<AgentSkillResult>;

    fn cancel_skill(&self, skill_request_id: &str) -> KernelResult<AgentSkillResult> {
        Err(KernelError::CapabilityMissing {
            capability_id: format!("skill.cancel.{skill_request_id}"),
        })
    }
}
