use crate::{AgentManifest, KernelError, KernelResult, MemoryScope};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentProviderFamily {
    Model,
    Tool,
    Context,
    Memory,
    Planning,
    Policy,
    Telemetry,
    Host,
    ProtocolAdapter,
    Mcp,
    Skill,
    Collaboration,
    AgentInstaller,
    AgentConfiguration,
}

impl AgentProviderFamily {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Model => "model",
            Self::Tool => "tool",
            Self::Context => "context",
            Self::Memory => "memory",
            Self::Planning => "planning",
            Self::Policy => "policy",
            Self::Telemetry => "telemetry",
            Self::Host => "host",
            Self::ProtocolAdapter => "protocol_adapter",
            Self::Mcp => "mcp",
            Self::Skill => "skill",
            Self::Collaboration => "collaboration",
            Self::AgentInstaller => "agent_installer",
            Self::AgentConfiguration => "agent_configuration",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentProviderBindingMode {
    ManifestOnly,
    TypedLocal,
    Remote,
    ManifestOrTyped,
}

impl AgentProviderBindingMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ManifestOnly => "manifest_only",
            Self::TypedLocal => "typed_local",
            Self::Remote => "remote",
            Self::ManifestOrTyped => "manifest_or_typed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentProviderBinding {
    pub binding_id: String,
    pub family: AgentProviderFamily,
    pub provider_id: String,
    pub required: bool,
    pub default: bool,
    pub mode: AgentProviderBindingMode,
    pub capabilities: Vec<String>,
    pub min_version: Option<String>,
}

impl AgentProviderBinding {
    pub fn new(
        binding_id: impl Into<String>,
        family: AgentProviderFamily,
        provider_id: impl Into<String>,
        required: bool,
    ) -> Self {
        Self {
            binding_id: binding_id.into(),
            family,
            provider_id: provider_id.into(),
            required,
            default: false,
            mode: AgentProviderBindingMode::ManifestOrTyped,
            capabilities: Vec::new(),
            min_version: None,
        }
    }

    pub fn as_default(mut self) -> Self {
        self.default = true;
        self
    }

    pub fn with_mode(mut self, mode: AgentProviderBindingMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.capabilities.push(capability.into());
        self
    }

    pub fn with_capabilities(mut self, capabilities: Vec<String>) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn with_min_version(mut self, min_version: impl Into<String>) -> Self {
        self.min_version = Some(min_version.into());
        self
    }

    pub fn supports_capability(&self, capability: &str) -> bool {
        self.capabilities
            .iter()
            .any(|registered| registered == capability)
    }

    pub fn satisfies_version(&self, version: &str) -> bool {
        self.min_version
            .as_deref()
            .is_none_or(|min_version| parse_semver_like(version) >= parse_semver_like(min_version))
    }

    pub fn validate(&self) -> KernelResult<()> {
        validate_standard_id(&self.binding_id, "binding_id", Some("binding."))?;
        validate_provider_id_for_family(&self.provider_id, self.family)?;
        validate_unique_capabilities(&self.capabilities)?;
        if let Some(min_version) = &self.min_version {
            validate_semver(min_version, "min_version")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSelectionPolicy {
    pub default_provider_id: Option<String>,
    pub default_model_id: Option<String>,
    pub required_capabilities: Vec<String>,
    pub allow_provider_fallback: bool,
}

impl ModelSelectionPolicy {
    pub fn default_provider(provider_id: impl Into<String>) -> Self {
        Self {
            default_provider_id: Some(provider_id.into()),
            default_model_id: None,
            required_capabilities: Vec::new(),
            allow_provider_fallback: false,
        }
    }

    pub fn with_default_model_id(mut self, model_id: impl Into<String>) -> Self {
        self.default_model_id = Some(model_id.into());
        self
    }

    pub fn with_required_capability(mut self, capability: impl Into<String>) -> Self {
        self.required_capabilities.push(capability.into());
        self
    }

    pub fn allow_provider_fallback(mut self, allow_provider_fallback: bool) -> Self {
        self.allow_provider_fallback = allow_provider_fallback;
        self
    }

    pub fn requires_capability(&self, capability: &str) -> bool {
        self.required_capabilities
            .iter()
            .any(|registered| registered == capability)
    }
}

impl Default for ModelSelectionPolicy {
    fn default() -> Self {
        Self {
            default_provider_id: None,
            default_model_id: None,
            required_capabilities: Vec::new(),
            allow_provider_fallback: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallPolicy {
    pub default_provider_id: Option<String>,
    pub policy_required: bool,
    pub allowed_tool_ids: Vec<String>,
    pub denied_tool_ids: Vec<String>,
    pub max_parallel_calls: Option<u32>,
}

impl ToolCallPolicy {
    pub fn default_provider(provider_id: impl Into<String>) -> Self {
        Self {
            default_provider_id: Some(provider_id.into()),
            policy_required: true,
            allowed_tool_ids: Vec::new(),
            denied_tool_ids: Vec::new(),
            max_parallel_calls: None,
        }
    }

    pub fn with_policy_required(mut self, policy_required: bool) -> Self {
        self.policy_required = policy_required;
        self
    }

    pub fn with_allowed_tool_id(mut self, tool_id: impl Into<String>) -> Self {
        self.allowed_tool_ids.push(tool_id.into());
        self
    }

    pub fn with_denied_tool_id(mut self, tool_id: impl Into<String>) -> Self {
        self.denied_tool_ids.push(tool_id.into());
        self
    }

    pub fn with_max_parallel_calls(mut self, max_parallel_calls: u32) -> Self {
        self.max_parallel_calls = Some(max_parallel_calls);
        self
    }

    pub fn allows_tool(&self, tool_id: &str) -> bool {
        if self
            .denied_tool_ids
            .iter()
            .any(|registered| registered == tool_id)
        {
            return false;
        }

        self.allowed_tool_ids.is_empty()
            || self
                .allowed_tool_ids
                .iter()
                .any(|registered| registered == tool_id)
    }
}

impl Default for ToolCallPolicy {
    fn default() -> Self {
        Self {
            default_provider_id: None,
            policy_required: true,
            allowed_tool_ids: Vec::new(),
            denied_tool_ids: Vec::new(),
            max_parallel_calls: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryStrategy {
    pub default_provider_id: Option<String>,
    pub enabled_scopes: Vec<MemoryScope>,
    pub write_policy_required: bool,
    pub read_policy_required_for_sensitive: bool,
    pub retention_required: bool,
}

impl MemoryStrategy {
    pub fn disabled() -> Self {
        Self {
            default_provider_id: None,
            enabled_scopes: Vec::new(),
            write_policy_required: true,
            read_policy_required_for_sensitive: true,
            retention_required: false,
        }
    }

    pub fn default_provider(provider_id: impl Into<String>) -> Self {
        Self {
            default_provider_id: Some(provider_id.into()),
            enabled_scopes: Vec::new(),
            write_policy_required: true,
            read_policy_required_for_sensitive: true,
            retention_required: false,
        }
    }

    pub fn with_scope(mut self, scope: MemoryScope) -> Self {
        if !self.enabled_scopes.contains(&scope) {
            self.enabled_scopes.push(scope);
        }
        self
    }

    pub fn with_write_policy_required(mut self, write_policy_required: bool) -> Self {
        self.write_policy_required = write_policy_required;
        self
    }

    pub fn with_read_policy_required_for_sensitive(
        mut self,
        read_policy_required_for_sensitive: bool,
    ) -> Self {
        self.read_policy_required_for_sensitive = read_policy_required_for_sensitive;
        self
    }

    pub fn with_retention_required(mut self, retention_required: bool) -> Self {
        self.retention_required = retention_required;
        self
    }

    pub fn scope_enabled(&self, scope: MemoryScope) -> bool {
        self.enabled_scopes.contains(&scope)
    }
}

impl Default for MemoryStrategy {
    fn default() -> Self {
        Self::disabled()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentDefinition {
    pub schema_version: String,
    pub manifest_type: String,
    pub definition_id: String,
    pub manifest: AgentManifest,
    pub provider_bindings: Vec<AgentProviderBinding>,
    pub model_selection: ModelSelectionPolicy,
    pub tool_call_policy: ToolCallPolicy,
    pub memory_strategy: MemoryStrategy,
    pub extensions: Vec<(String, String)>,
}

impl AgentDefinition {
    pub fn new(definition_id: impl Into<String>, manifest: AgentManifest) -> Self {
        Self {
            schema_version: manifest.schema_version.clone(),
            manifest_type: "agent_definition".to_string(),
            definition_id: definition_id.into(),
            manifest,
            provider_bindings: Vec::new(),
            model_selection: ModelSelectionPolicy::default(),
            tool_call_policy: ToolCallPolicy::default(),
            memory_strategy: MemoryStrategy::default(),
            extensions: Vec::new(),
        }
    }

    pub fn from_json(input: &str) -> KernelResult<Self> {
        let manifest_type = extract_string(input, "manifest_type")?;
        if manifest_type != "agent_definition" {
            return Err(KernelError::validation(
                "manifest_type must be agent_definition",
            ));
        }

        let agent_body = extract_object_body(input, "agent")?;
        let mut definition = Self::new(
            extract_string(input, "definition_id")?,
            AgentManifest::from_json(&agent_body)?,
        );
        definition.schema_version = extract_string(input, "schema_version")?;
        definition.provider_bindings = parse_provider_bindings(input)?;
        if let Some(model_selection_body) = extract_optional_object_body(input, "model_selection")?
        {
            definition.model_selection = parse_model_selection(&model_selection_body)?;
        }
        if let Some(tool_call_policy_body) =
            extract_optional_object_body(input, "tool_call_policy")?
        {
            definition.tool_call_policy = parse_tool_call_policy(&tool_call_policy_body)?;
        }
        if let Some(memory_strategy_body) = extract_optional_object_body(input, "memory_strategy")?
        {
            definition.memory_strategy = parse_memory_strategy(&memory_strategy_body)?;
        }
        definition.validate()
    }

    pub fn with_provider_binding(mut self, binding: AgentProviderBinding) -> Self {
        self.provider_bindings.push(binding);
        self
    }

    pub fn with_model_selection(mut self, model_selection: ModelSelectionPolicy) -> Self {
        self.model_selection = model_selection;
        self
    }

    pub fn with_tool_call_policy(mut self, tool_call_policy: ToolCallPolicy) -> Self {
        self.tool_call_policy = tool_call_policy;
        self
    }

    pub fn with_memory_strategy(mut self, memory_strategy: MemoryStrategy) -> Self {
        self.memory_strategy = memory_strategy;
        self
    }

    pub fn requires_provider_family(&self, family: AgentProviderFamily) -> bool {
        self.provider_bindings
            .iter()
            .any(|binding| binding.family == family && binding.required)
    }

    pub fn default_binding(&self, family: AgentProviderFamily) -> Option<&AgentProviderBinding> {
        self.provider_bindings
            .iter()
            .find(|binding| binding.family == family && binding.default)
    }

    pub fn binding_for_provider(&self, provider_id: &str) -> Option<&AgentProviderBinding> {
        self.provider_bindings
            .iter()
            .find(|binding| binding.provider_id == provider_id)
    }

    pub fn validate(self) -> KernelResult<Self> {
        validate_standard_id(&self.definition_id, "definition_id", Some("definition."))?;

        for binding in &self.provider_bindings {
            binding.validate()?;
        }

        for family in [
            AgentProviderFamily::Model,
            AgentProviderFamily::Tool,
            AgentProviderFamily::Context,
            AgentProviderFamily::Memory,
            AgentProviderFamily::Planning,
            AgentProviderFamily::Policy,
            AgentProviderFamily::Telemetry,
            AgentProviderFamily::Host,
            AgentProviderFamily::ProtocolAdapter,
            AgentProviderFamily::Mcp,
            AgentProviderFamily::Skill,
            AgentProviderFamily::Collaboration,
            AgentProviderFamily::AgentInstaller,
            AgentProviderFamily::AgentConfiguration,
        ] {
            let default_count = self
                .provider_bindings
                .iter()
                .filter(|binding| binding.family == family && binding.default)
                .count();
            if default_count > 1 {
                return Err(KernelError::validation(format!(
                    "multiple default provider bindings for family: {}",
                    family.as_str()
                )));
            }
        }

        if let Some(provider_id) = &self.model_selection.default_provider_id {
            match self.binding_for_provider(provider_id) {
                Some(binding) if binding.family == AgentProviderFamily::Model => {}
                _ => {
                    return Err(KernelError::validation(format!(
                        "model selection references unknown provider: {provider_id}"
                    )));
                }
            }
        }

        if let Some(provider_id) = &self.tool_call_policy.default_provider_id {
            match self.binding_for_provider(provider_id) {
                Some(binding) if binding.family == AgentProviderFamily::Tool => {}
                _ => {
                    return Err(KernelError::validation(format!(
                        "tool call policy references unknown provider: {provider_id}"
                    )));
                }
            }
        }

        if let Some(provider_id) = &self.memory_strategy.default_provider_id {
            match self.binding_for_provider(provider_id) {
                Some(binding) if binding.family == AgentProviderFamily::Memory => {}
                _ => {
                    return Err(KernelError::validation(format!(
                        "memory strategy references unknown provider: {provider_id}"
                    )));
                }
            }
        }

        Ok(self)
    }
}

fn parse_provider_bindings(input: &str) -> KernelResult<Vec<AgentProviderBinding>> {
    let mut bindings = Vec::new();
    for body in extract_object_array(input, "provider_bindings")? {
        let mut binding = AgentProviderBinding::new(
            extract_string(&body, "binding_id")?,
            parse_provider_family(&extract_string(&body, "family")?)?,
            extract_string(&body, "provider_id")?,
            extract_bool(&body, "required")?,
        );
        if extract_optional_bool(&body, "default")?.unwrap_or(false) {
            binding = binding.as_default();
        }
        if let Some(mode) = extract_optional_string(&body, "mode")? {
            binding = binding.with_mode(parse_binding_mode(&mode)?);
        }
        binding = binding.with_capabilities(extract_string_array(&body, "capabilities")?);
        if let Some(min_version) = extract_optional_string(&body, "min_version")? {
            binding = binding.with_min_version(min_version);
        }
        bindings.push(binding);
    }
    Ok(bindings)
}

fn parse_model_selection(input: &str) -> KernelResult<ModelSelectionPolicy> {
    let mut policy = ModelSelectionPolicy::default();
    policy.default_provider_id = extract_optional_string(input, "default_provider_id")?;
    policy.default_model_id = extract_optional_string(input, "default_model_id")?;
    policy.required_capabilities =
        extract_optional_string_array(input, "required_capabilities")?.unwrap_or_default();
    policy.allow_provider_fallback =
        extract_optional_bool(input, "allow_provider_fallback")?.unwrap_or(false);
    validate_unique_capabilities(&policy.required_capabilities)?;
    Ok(policy)
}

fn parse_tool_call_policy(input: &str) -> KernelResult<ToolCallPolicy> {
    let mut policy = ToolCallPolicy::default();
    policy.default_provider_id = extract_optional_string(input, "default_provider_id")?;
    policy.policy_required = extract_optional_bool(input, "policy_required")?.unwrap_or(true);
    policy.allowed_tool_ids =
        extract_optional_string_array(input, "allowed_tool_ids")?.unwrap_or_default();
    policy.denied_tool_ids =
        extract_optional_string_array(input, "denied_tool_ids")?.unwrap_or_default();
    policy.max_parallel_calls = extract_optional_u32(input, "max_parallel_calls")?;
    validate_unique_ids(&policy.allowed_tool_ids, "allowed_tool_ids")?;
    validate_unique_ids(&policy.denied_tool_ids, "denied_tool_ids")?;
    Ok(policy)
}

fn parse_memory_strategy(input: &str) -> KernelResult<MemoryStrategy> {
    let mut strategy = MemoryStrategy::default();
    strategy.default_provider_id = extract_optional_string(input, "default_provider_id")?;
    strategy.enabled_scopes = extract_optional_string_array(input, "enabled_scopes")?
        .unwrap_or_default()
        .into_iter()
        .map(|scope| parse_memory_scope(&scope))
        .collect::<KernelResult<Vec<_>>>()?;
    strategy.write_policy_required =
        extract_optional_bool(input, "write_policy_required")?.unwrap_or(true);
    strategy.read_policy_required_for_sensitive =
        extract_optional_bool(input, "read_policy_required_for_sensitive")?.unwrap_or(true);
    strategy.retention_required =
        extract_optional_bool(input, "retention_required")?.unwrap_or(false);
    Ok(strategy)
}

fn parse_provider_family(input: &str) -> KernelResult<AgentProviderFamily> {
    match input {
        "model" => Ok(AgentProviderFamily::Model),
        "tool" => Ok(AgentProviderFamily::Tool),
        "context" => Ok(AgentProviderFamily::Context),
        "memory" => Ok(AgentProviderFamily::Memory),
        "planning" => Ok(AgentProviderFamily::Planning),
        "policy" => Ok(AgentProviderFamily::Policy),
        "telemetry" => Ok(AgentProviderFamily::Telemetry),
        "host" => Ok(AgentProviderFamily::Host),
        "protocol_adapter" => Ok(AgentProviderFamily::ProtocolAdapter),
        "mcp" => Ok(AgentProviderFamily::Mcp),
        "skill" => Ok(AgentProviderFamily::Skill),
        "collaboration" => Ok(AgentProviderFamily::Collaboration),
        "agent_installer" => Ok(AgentProviderFamily::AgentInstaller),
        "agent_configuration" => Ok(AgentProviderFamily::AgentConfiguration),
        _ => Err(KernelError::validation(format!(
            "unknown provider family: {input}"
        ))),
    }
}

fn parse_binding_mode(input: &str) -> KernelResult<AgentProviderBindingMode> {
    match input {
        "manifest_only" => Ok(AgentProviderBindingMode::ManifestOnly),
        "typed_local" => Ok(AgentProviderBindingMode::TypedLocal),
        "remote" => Ok(AgentProviderBindingMode::Remote),
        "manifest_or_typed" => Ok(AgentProviderBindingMode::ManifestOrTyped),
        _ => Err(KernelError::validation(format!(
            "unknown provider binding mode: {input}"
        ))),
    }
}

fn parse_memory_scope(input: &str) -> KernelResult<MemoryScope> {
    match input {
        "session" => Ok(MemoryScope::Session),
        "user" => Ok(MemoryScope::User),
        "tenant" => Ok(MemoryScope::Tenant),
        "organization" => Ok(MemoryScope::Organization),
        "agent" => Ok(MemoryScope::Agent),
        "application" => Ok(MemoryScope::Application),
        _ => Err(KernelError::validation(format!(
            "unknown memory scope: {input}"
        ))),
    }
}

fn validate_provider_id_for_family(
    provider_id: &str,
    family: AgentProviderFamily,
) -> KernelResult<()> {
    let prefix = match family {
        AgentProviderFamily::ProtocolAdapter => Some("adapter."),
        AgentProviderFamily::AgentInstaller => Some("provider.agent.installer."),
        AgentProviderFamily::AgentConfiguration => Some("provider.agent.configuration."),
        _ => Some("provider."),
    };
    validate_standard_id(provider_id, "provider_id", prefix)
}

fn validate_standard_id(
    value: &str,
    field_name: &str,
    required_prefix: Option<&str>,
) -> KernelResult<()> {
    if value.trim().is_empty() {
        return Err(KernelError::validation(format!("{field_name} is required")));
    }
    if value.trim() != value {
        return Err(KernelError::validation(format!(
            "{field_name} must not contain leading or trailing whitespace"
        )));
    }
    if value.chars().count() > 128 {
        return Err(KernelError::validation(format!(
            "{field_name} must be at most 128 characters"
        )));
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '.' | '_' | '-'))
    {
        return Err(KernelError::validation(format!(
            "{field_name} must use lowercase standard id characters"
        )));
    }
    if !has_non_empty_dot_segments(value) {
        return Err(KernelError::validation(format!(
            "{field_name} must use non-empty dot-delimited segments"
        )));
    }
    if let Some(prefix) = required_prefix {
        if !value.starts_with(prefix) {
            return Err(KernelError::validation(format!(
                "{field_name} must start with {prefix}"
            )));
        }
    }
    Ok(())
}

fn validate_semver(value: &str, field_name: &str) -> KernelResult<()> {
    let parts: Vec<&str> = value.split('.').collect();
    if parts.len() != 3
        || parts
            .iter()
            .any(|part| part.is_empty() || !part.chars().all(|ch| ch.is_ascii_digit()))
    {
        return Err(KernelError::validation(format!(
            "{field_name} must be a semantic version"
        )));
    }
    Ok(())
}

fn validate_unique_capabilities(capabilities: &[String]) -> KernelResult<()> {
    validate_unique_ids(capabilities, "capabilities")?;
    for capability in capabilities {
        validate_standard_id(capability, "capability_id", None)?;
    }
    Ok(())
}

fn validate_unique_ids(values: &[String], field_name: &str) -> KernelResult<()> {
    for (index, value) in values.iter().enumerate() {
        if values.iter().skip(index + 1).any(|other| other == value) {
            return Err(KernelError::validation(format!(
                "{field_name} must not contain duplicate id: {value}"
            )));
        }
    }
    Ok(())
}

fn has_non_empty_dot_segments(value: &str) -> bool {
    let mut segment_count = 0;
    for segment in value.split('.') {
        segment_count += 1;
        if segment.is_empty() {
            return false;
        }
    }
    segment_count >= 2
}

fn parse_semver_like(version: &str) -> (u64, u64, u64) {
    let core = version
        .split_once('-')
        .map(|(core, _)| core)
        .unwrap_or(version)
        .split_once('+')
        .map(|(core, _)| core)
        .unwrap_or(version);
    let mut parts = core.split('.').map(|part| part.parse::<u64>().unwrap_or(0));

    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

fn extract_string(input: &str, key: &str) -> KernelResult<String> {
    let pattern = format!("\"{key}\"");
    let key_start = input
        .find(&pattern)
        .ok_or_else(|| KernelError::validation(format!("missing field: {key}")))?;
    extract_string_after_key(&input[key_start + pattern.len()..], key)
}

fn extract_optional_string(input: &str, key: &str) -> KernelResult<Option<String>> {
    let pattern = format!("\"{key}\"");
    let Some(key_start) = input.find(&pattern) else {
        return Ok(None);
    };
    Ok(Some(extract_string_after_key(
        &input[key_start + pattern.len()..],
        key,
    )?))
}

fn extract_string_after_key(input: &str, key: &str) -> KernelResult<String> {
    let colon = input
        .find(':')
        .ok_or_else(|| KernelError::validation(format!("missing ':' after field: {key}")))?;
    let after_colon = input[colon + 1..].trim_start();
    let value_start = after_colon
        .find('"')
        .ok_or_else(|| KernelError::validation(format!("field is not a string: {key}")))?;
    let rest = &after_colon[value_start + 1..];
    let value_end = rest
        .find('"')
        .ok_or_else(|| KernelError::validation(format!("unterminated string field: {key}")))?;
    Ok(rest[..value_end].to_string())
}

fn extract_bool(input: &str, key: &str) -> KernelResult<bool> {
    let value = extract_raw_json_value(input, key)?;
    match value.as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(KernelError::validation(format!(
            "field is not a boolean: {key}"
        ))),
    }
}

fn extract_optional_bool(input: &str, key: &str) -> KernelResult<Option<bool>> {
    let pattern = format!("\"{key}\"");
    if !input.contains(&pattern) {
        return Ok(None);
    }
    extract_bool(input, key).map(Some)
}

fn extract_optional_u32(input: &str, key: &str) -> KernelResult<Option<u32>> {
    let pattern = format!("\"{key}\"");
    if !input.contains(&pattern) {
        return Ok(None);
    }
    extract_raw_json_value(input, key)?
        .parse::<u32>()
        .map(Some)
        .map_err(|_| KernelError::validation(format!("field is not an integer: {key}")))
}

fn extract_optional_string_array(input: &str, key: &str) -> KernelResult<Option<Vec<String>>> {
    let pattern = format!("\"{key}\"");
    if !input.contains(&pattern) {
        return Ok(None);
    }
    extract_string_array(input, key).map(Some)
}

fn extract_string_array(input: &str, key: &str) -> KernelResult<Vec<String>> {
    let array = extract_array_body(input, key)?;
    let mut values = Vec::new();
    let mut rest = array.as_str();

    while let Some(start) = rest.find('"') {
        let after_start = &rest[start + 1..];
        let Some(end) = after_start.find('"') else {
            return Err(KernelError::validation(format!(
                "unterminated string array field: {key}"
            )));
        };
        values.push(after_start[..end].to_string());
        rest = &after_start[end + 1..];
    }

    Ok(values)
}

fn extract_object_array(input: &str, key: &str) -> KernelResult<Vec<String>> {
    let array = extract_array_body(input, key)?;
    let mut objects = Vec::new();
    let mut depth = 0usize;
    let mut start = None;

    for (index, ch) in array.char_indices() {
        match ch {
            '{' => {
                if depth == 0 {
                    start = Some(index);
                }
                depth += 1;
            }
            '}' => {
                if depth == 0 {
                    return Err(KernelError::validation(format!(
                        "unbalanced object array field: {key}"
                    )));
                }
                depth -= 1;
                if depth == 0 {
                    if let Some(start_index) = start {
                        objects.push(array[start_index..=index].to_string());
                    }
                }
            }
            _ => {}
        }
    }

    if depth != 0 {
        return Err(KernelError::validation(format!(
            "unterminated object array field: {key}"
        )));
    }

    Ok(objects)
}

fn extract_array_body(input: &str, key: &str) -> KernelResult<String> {
    extract_delimited_body(input, key, '[', ']')
}

fn extract_object_body(input: &str, key: &str) -> KernelResult<String> {
    extract_delimited_body(input, key, '{', '}')
}

fn extract_optional_object_body(input: &str, key: &str) -> KernelResult<Option<String>> {
    let pattern = format!("\"{key}\"");
    if !input.contains(&pattern) {
        return Ok(None);
    }
    extract_object_body(input, key).map(Some)
}

fn extract_delimited_body(input: &str, key: &str, open: char, close: char) -> KernelResult<String> {
    let pattern = format!("\"{key}\"");
    let key_start = input
        .find(&pattern)
        .ok_or_else(|| KernelError::validation(format!("missing field: {key}")))?;
    let after_key = &input[key_start + pattern.len()..];
    let open_start = after_key
        .find(open)
        .ok_or_else(|| KernelError::validation(format!("field has wrong shape: {key}")))?;
    let mut depth = 0usize;
    let mut end = None;

    for (index, ch) in after_key[open_start..].char_indices() {
        match ch {
            candidate if candidate == open => depth += 1,
            candidate if candidate == close => {
                depth -= 1;
                if depth == 0 {
                    end = Some(open_start + index);
                    break;
                }
            }
            _ => {}
        }
    }

    let end = end.ok_or_else(|| KernelError::validation(format!("unterminated field: {key}")))?;
    Ok(after_key[open_start..=end].to_string())
}

fn extract_raw_json_value(input: &str, key: &str) -> KernelResult<String> {
    let pattern = format!("\"{key}\"");
    let key_start = input
        .find(&pattern)
        .ok_or_else(|| KernelError::validation(format!("missing field: {key}")))?;
    let after_key = &input[key_start + pattern.len()..];
    let colon = after_key
        .find(':')
        .ok_or_else(|| KernelError::validation(format!("missing ':' after field: {key}")))?;
    let after_colon = after_key[colon + 1..].trim_start();
    if after_colon.starts_with('"') {
        return extract_string_after_key(after_key, key).map(|value| format!("\"{value}\""));
    }

    let end = after_colon
        .find([',', '}', ']'])
        .unwrap_or(after_colon.len());
    Ok(after_colon[..end].trim().to_string())
}
