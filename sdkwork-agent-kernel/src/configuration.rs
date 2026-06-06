use crate::{
    KernelError, KernelEvent, KernelEventRedaction, KernelEventSeverity, KernelEventSource,
    KernelResult, PolicyCategory, PolicyRequest, ProviderHealth, ProviderManifest, SideEffectLevel,
};

pub trait AgentConfigurationProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.agent.configuration.unspecified",
            "agent_configuration",
            "agent-configuration-provider",
            "0.0.0",
            vec!["agent.configure".to_string()],
        )
    }

    fn configuration_spec(&self, agent_id: &str) -> KernelResult<AgentConfigurationSpec>;

    fn validate_configuration(
        &self,
        configuration: &AgentConfiguration,
    ) -> KernelResult<AgentConfigurationValidation>;

    fn plan_configuration_upgrade(
        &self,
        _request: &AgentConfigurationUpgradeRequest,
    ) -> KernelResult<AgentConfigurationUpgradePlan> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.configure.migrate".to_string(),
        })
    }

    fn health(&self) -> ProviderHealth;
}

pub trait AgentConfigurationStore {
    fn save_profile(
        &mut self,
        profile: AgentConfigurationProfile,
    ) -> KernelResult<AgentConfigurationStoreRecord>;

    fn load_profile(
        &self,
        agent_id: &str,
        profile_id: &str,
    ) -> KernelResult<AgentConfigurationProfile>;

    fn list_profiles(&self, agent_id: &str) -> KernelResult<Vec<AgentConfigurationProfile>>;

    fn migrate_profile(
        &mut self,
        plan: &AgentConfigurationUpgradePlan,
        current_profile: AgentConfigurationProfile,
    ) -> KernelResult<AgentConfigurationStoreRecord>;

    fn archive_profile(
        &mut self,
        request: &AgentProfileArchiveRequest,
    ) -> KernelResult<AgentConfigurationStoreRecord>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConfigurationSpec {
    pub schema_version: String,
    pub agent_id: String,
    pub sections: Vec<AgentConfigSection>,
}

impl AgentConfigurationSpec {
    pub fn new(agent_id: impl Into<String>) -> Self {
        Self {
            schema_version: "0.1.0".to_string(),
            agent_id: agent_id.into(),
            sections: Vec::new(),
        }
    }

    pub fn add_section(mut self, section: AgentConfigSection) -> Self {
        self.sections.push(section);
        self
    }

    pub fn from_json(input: &str) -> KernelResult<Self> {
        let manifest_type = extract_string(input, "manifest_type")?;
        if manifest_type != "agent_configuration_spec" {
            return Err(KernelError::validation(
                "manifest_type must be agent_configuration_spec",
            ));
        }

        let mut spec = Self::new(extract_string(input, "agent_id")?);
        spec.schema_version = extract_string(input, "schema_version")?;

        for section_body in extract_object_array(input, "sections")? {
            spec = spec.add_section(parse_config_section(&section_body)?);
        }

        if spec.sections.is_empty() {
            return Err(KernelError::validation(
                "configuration spec must declare at least one section",
            ));
        }

        Ok(spec)
    }

    pub fn field(&self, key: &str) -> Option<&AgentConfigField> {
        self.sections
            .iter()
            .flat_map(|section| section.fields.iter())
            .find(|field| field.key == key)
    }

    pub fn required_keys(&self) -> Vec<&str> {
        self.sections
            .iter()
            .flat_map(|section| section.fields.iter())
            .filter(|field| field.required)
            .map(|field| field.key.as_str())
            .collect()
    }

    pub fn validate(&self, configuration: &AgentConfiguration) -> AgentConfigurationValidation {
        let mut validation = AgentConfigurationValidation::new(
            configuration.agent_id.clone(),
            configuration.profile_id.clone(),
        );

        if configuration.agent_id != self.agent_id {
            validation
                .invalid_fields
                .push(AgentConfigurationInvalidField::new(
                    "agent_id",
                    "agent_id_mismatch",
                    "configuration belongs to a different agent",
                ));
        }

        for key in self.required_keys() {
            if configuration.value(key).is_none() {
                validation.missing_required_fields.push(key.to_string());
            }
        }

        for entry in &configuration.entries {
            let Some(field) = self.field(&entry.key) else {
                validation
                    .invalid_fields
                    .push(AgentConfigurationInvalidField::new(
                        entry.key.clone(),
                        "unknown_field",
                        "configuration field is not declared by the agent",
                    ));
                continue;
            };

            if field.secret_ref_required && !entry.value.is_secret_ref() {
                validation
                    .invalid_fields
                    .push(AgentConfigurationInvalidField::new(
                        entry.key.clone(),
                        "secret_ref_required",
                        "secret values must be supplied through a secret reference",
                    ));
                continue;
            }

            if !field.value_kind.matches(&entry.value) {
                validation
                    .invalid_fields
                    .push(AgentConfigurationInvalidField::new(
                        entry.key.clone(),
                        "invalid_value_kind",
                        "configuration value kind does not match the field schema",
                    ));
            }
        }

        validation.valid =
            validation.missing_required_fields.is_empty() && validation.invalid_fields.is_empty();
        validation
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConfigSection {
    pub section_id: String,
    pub title: String,
    pub kind: AgentConfigSectionKind,
    pub fields: Vec<AgentConfigField>,
}

impl AgentConfigSection {
    pub fn new(
        section_id: impl Into<String>,
        title: impl Into<String>,
        kind: AgentConfigSectionKind,
    ) -> Self {
        Self {
            section_id: section_id.into(),
            title: title.into(),
            kind,
            fields: Vec::new(),
        }
    }

    pub fn base(section_id: impl Into<String>, title: impl Into<String>) -> Self {
        Self::new(section_id, title, AgentConfigSectionKind::Base)
    }

    pub fn login_auth(section_id: impl Into<String>, title: impl Into<String>) -> Self {
        Self::new(section_id, title, AgentConfigSectionKind::LoginAuth)
    }

    pub fn llm_api_key(section_id: impl Into<String>, title: impl Into<String>) -> Self {
        Self::new(section_id, title, AgentConfigSectionKind::LlmApiKey)
    }

    pub fn add_field(mut self, mut field: AgentConfigField) -> Self {
        field.section_kind = self.kind.clone();
        self.fields.push(field);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentConfigSectionKind {
    Base,
    LoginAuth,
    LlmApiKey,
    Runtime,
    Security,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConfigField {
    pub key: String,
    pub label: String,
    pub value_kind: AgentConfigValueKind,
    pub section_kind: AgentConfigSectionKind,
    pub required: bool,
    pub secret_ref_required: bool,
    pub redaction_classification: KernelEventRedaction,
    pub description: Option<String>,
    pub default_value: Option<AgentConfigValue>,
}

impl AgentConfigField {
    pub fn new(
        key: impl Into<String>,
        label: impl Into<String>,
        value_kind: AgentConfigValueKind,
    ) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            value_kind,
            section_kind: AgentConfigSectionKind::Custom("unassigned".to_string()),
            required: false,
            secret_ref_required: false,
            redaction_classification: KernelEventRedaction::Public,
            description: None,
            default_value: None,
        }
    }

    pub fn text(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(key, label, AgentConfigValueKind::String)
    }

    pub fn secret(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(key, label, AgentConfigValueKind::SecretRef)
            .secret_ref_required()
            .with_redaction(KernelEventRedaction::Secret)
    }

    pub fn llm_api_key(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self::secret(key, label)
            .required()
            .with_description("LLM API keys must be provided as host secret references")
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn secret_ref_required(mut self) -> Self {
        self.secret_ref_required = true;
        self
    }

    pub fn with_redaction(mut self, redaction: KernelEventRedaction) -> Self {
        self.redaction_classification = redaction;
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_default(mut self, default_value: AgentConfigValue) -> Self {
        self.default_value = Some(default_value);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentConfigValueKind {
    String,
    Boolean,
    Integer,
    SecretRef,
    StringList,
    Json,
}

impl AgentConfigValueKind {
    pub fn matches(&self, value: &AgentConfigValue) -> bool {
        matches!(
            (self, value),
            (Self::String, AgentConfigValue::String(_))
                | (Self::Boolean, AgentConfigValue::Boolean(_))
                | (Self::Integer, AgentConfigValue::Integer(_))
                | (Self::SecretRef, AgentConfigValue::SecretRef(_))
                | (Self::StringList, AgentConfigValue::StringList(_))
                | (Self::Json, AgentConfigValue::Json(_))
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConfiguration {
    pub agent_id: String,
    pub profile_id: String,
    pub entries: Vec<AgentConfigEntry>,
}

impl AgentConfiguration {
    pub fn new(agent_id: impl Into<String>, profile_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            profile_id: profile_id.into(),
            entries: Vec::new(),
        }
    }

    pub fn set(mut self, key: impl Into<String>, value: AgentConfigValue) -> Self {
        let key = key.into();
        let redaction_classification = value.redaction_classification();
        self.entries.retain(|entry| entry.key != key);
        self.entries.push(AgentConfigEntry {
            key,
            value,
            redaction_classification,
        });
        self
    }

    pub fn value(&self, key: &str) -> Option<&AgentConfigValue> {
        self.entries
            .iter()
            .find(|entry| entry.key == key)
            .map(|entry| &entry.value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConfigEntry {
    pub key: String,
    pub value: AgentConfigValue,
    pub redaction_classification: KernelEventRedaction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentConfigValue {
    String(String),
    Boolean(bool),
    Integer(i64),
    SecretRef(String),
    StringList(Vec<String>),
    Json(String),
}

impl AgentConfigValue {
    pub fn string(value: impl Into<String>) -> Self {
        Self::String(value.into())
    }

    pub fn boolean(value: bool) -> Self {
        Self::Boolean(value)
    }

    pub fn integer(value: i64) -> Self {
        Self::Integer(value)
    }

    pub fn secret_ref(value: impl Into<String>) -> Self {
        Self::SecretRef(value.into())
    }

    pub fn string_list(value: Vec<String>) -> Self {
        Self::StringList(value)
    }

    pub fn json(value: impl Into<String>) -> Self {
        Self::Json(value.into())
    }

    pub fn is_secret_ref(&self) -> bool {
        matches!(self, Self::SecretRef(_))
    }

    pub fn redaction_classification(&self) -> KernelEventRedaction {
        match self {
            Self::SecretRef(_) => KernelEventRedaction::Secret,
            _ => KernelEventRedaction::Public,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConfigurationValidation {
    pub agent_id: String,
    pub profile_id: String,
    pub valid: bool,
    pub missing_required_fields: Vec<String>,
    pub invalid_fields: Vec<AgentConfigurationInvalidField>,
}

impl AgentConfigurationValidation {
    pub fn new(agent_id: impl Into<String>, profile_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            profile_id: profile_id.into(),
            valid: false,
            missing_required_fields: Vec::new(),
            invalid_fields: Vec::new(),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.valid && self.missing_required_fields.is_empty() && self.invalid_fields.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConfigurationInvalidField {
    pub field_key: String,
    pub reason_code: String,
    pub safe_message: String,
}

impl AgentConfigurationInvalidField {
    pub fn new(
        field_key: impl Into<String>,
        reason_code: impl Into<String>,
        safe_message: impl Into<String>,
    ) -> Self {
        Self {
            field_key: field_key.into(),
            reason_code: reason_code.into(),
            safe_message: safe_message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConfigurationProfile {
    pub profile_id: String,
    pub agent_id: String,
    pub configuration_version: String,
    pub status: AgentConfigurationProfileStatus,
    pub configuration: AgentConfiguration,
    pub secret_bindings: Vec<AgentSecretBinding>,
}

impl AgentConfigurationProfile {
    pub fn new(
        profile_id: impl Into<String>,
        agent_id: impl Into<String>,
        configuration_version: impl Into<String>,
        configuration: AgentConfiguration,
    ) -> Self {
        Self {
            profile_id: profile_id.into(),
            agent_id: agent_id.into(),
            configuration_version: configuration_version.into(),
            status: AgentConfigurationProfileStatus::Draft,
            configuration,
            secret_bindings: Vec::new(),
        }
    }

    pub fn from_json(input: &str) -> KernelResult<Self> {
        let manifest_type = extract_string(input, "manifest_type")?;
        if manifest_type != "agent_configuration_profile" {
            return Err(KernelError::validation(
                "manifest_type must be agent_configuration_profile",
            ));
        }

        let profile_id = extract_string(input, "profile_id")?;
        let agent_id = extract_string(input, "agent_id")?;
        let configuration_version = extract_string(input, "configuration_version")?;
        let status = parse_profile_status(&extract_string(input, "status")?)?;
        let configuration_body = extract_object_body(input, "configuration")?;

        let mut configuration = AgentConfiguration::new(agent_id.clone(), profile_id.clone());
        for entry_body in extract_object_array(&configuration_body, "entries")? {
            let key = extract_string(&entry_body, "key")?;
            let value_kind = extract_string(&entry_body, "value_kind")?;
            let value = parse_config_value(&entry_body, &value_kind)?;
            configuration = configuration.set(key, value);
        }

        let mut profile = Self::new(profile_id, agent_id, configuration_version, configuration);
        profile.status = status;

        for binding_body in extract_object_array(input, "secret_bindings")? {
            profile = profile.add_secret_binding(parse_secret_binding(&binding_body)?);
        }

        for entry in &profile.configuration.entries {
            if entry.value.is_secret_ref() && !profile.requires_secret(&entry.key) {
                return Err(KernelError::validation(format!(
                    "secret binding is required for field: {}",
                    entry.key
                )));
            }
        }

        Ok(profile)
    }

    pub fn activate(mut self) -> Self {
        self.status = AgentConfigurationProfileStatus::Active;
        self
    }

    pub fn deprecate(mut self) -> Self {
        self.status = AgentConfigurationProfileStatus::Deprecated;
        self
    }

    pub fn archive(mut self) -> Self {
        self.status = AgentConfigurationProfileStatus::Archived;
        self
    }

    pub fn add_secret_binding(mut self, binding: AgentSecretBinding) -> Self {
        self.secret_bindings
            .retain(|existing| existing.field_key != binding.field_key);
        self.secret_bindings.push(binding);
        self
    }

    pub fn requires_secret(&self, field_key: &str) -> bool {
        self.secret_bindings
            .iter()
            .any(|binding| binding.field_key == field_key)
    }

    pub fn validate_against(&self, spec: &AgentConfigurationSpec) -> AgentConfigurationValidation {
        let mut validation = spec.validate(&self.configuration);

        if self.agent_id != spec.agent_id {
            validation
                .invalid_fields
                .push(AgentConfigurationInvalidField::new(
                    "agent_id",
                    "agent_id_mismatch",
                    "profile belongs to a different agent",
                ));
        }

        if self.configuration.profile_id != self.profile_id {
            validation
                .invalid_fields
                .push(AgentConfigurationInvalidField::new(
                    "profile_id",
                    "profile_id_mismatch",
                    "configuration belongs to a different profile",
                ));
        }

        for entry in &self.configuration.entries {
            if entry.value.is_secret_ref() && !self.requires_secret(&entry.key) {
                validation
                    .invalid_fields
                    .push(AgentConfigurationInvalidField::new(
                        entry.key.clone(),
                        "secret_binding_missing",
                        "secret reference must have a profile secret binding",
                    ));
            }
        }

        validation.valid =
            validation.missing_required_fields.is_empty() && validation.invalid_fields.is_empty();
        validation
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentProfileArchiveRequest {
    pub request_id: String,
    pub agent_id: String,
    pub profile_id: String,
    pub requested_by: Option<String>,
    pub preserve_secret_bindings: bool,
}

impl AgentProfileArchiveRequest {
    pub fn new(
        request_id: impl Into<String>,
        agent_id: impl Into<String>,
        profile_id: impl Into<String>,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            agent_id: agent_id.into(),
            profile_id: profile_id.into(),
            requested_by: None,
            preserve_secret_bindings: true,
        }
    }

    pub fn requested_by(mut self, requested_by: impl Into<String>) -> Self {
        self.requested_by = Some(requested_by.into());
        self
    }

    pub fn remove_secret_bindings(mut self) -> Self {
        self.preserve_secret_bindings = false;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConfigurationStoreRecord {
    pub profile: AgentConfigurationProfile,
    pub action: AgentConfigurationStoreAction,
    pub migration_plan_id: Option<String>,
    pub request_id: Option<String>,
}

impl AgentConfigurationStoreRecord {
    pub fn created(profile: AgentConfigurationProfile) -> Self {
        Self {
            profile,
            action: AgentConfigurationStoreAction::Created,
            migration_plan_id: None,
            request_id: None,
        }
    }

    pub fn migrated(
        profile: AgentConfigurationProfile,
        migration_plan_id: impl Into<String>,
    ) -> Self {
        Self {
            profile,
            action: AgentConfigurationStoreAction::Migrated,
            migration_plan_id: Some(migration_plan_id.into()),
            request_id: None,
        }
    }

    pub fn archived(profile: AgentConfigurationProfile, request_id: impl Into<String>) -> Self {
        Self {
            profile,
            action: AgentConfigurationStoreAction::Archived,
            migration_plan_id: None,
            request_id: Some(request_id.into()),
        }
    }

    pub fn to_event(&self, event_id: impl Into<String>) -> KernelEvent {
        KernelEvent::new(
            event_id,
            self.action.event_type(),
            KernelEventSeverity::Info,
            format!(
                "agent_id={};profile_id={};configuration_version={};status={};migration_plan_id={};request_id={}",
                self.profile.agent_id,
                self.profile.profile_id,
                self.profile.configuration_version,
                self.profile.status.as_str(),
                self.migration_plan_id.as_deref().unwrap_or(""),
                self.request_id.as_deref().unwrap_or("")
            ),
        )
        .from_source(KernelEventSource::Runtime)
        .with_redaction(KernelEventRedaction::Internal)
        .with_payload_schema("sdkwork.agent.configuration.profile.v1")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentConfigurationStoreAction {
    Created,
    Migrated,
    Archived,
}

impl AgentConfigurationStoreAction {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Created => "agent.configure.profile.created",
            Self::Migrated => "agent.configure.profile.migrated",
            Self::Archived => "agent.configure.profile.archived",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentConfigurationProfileStatus {
    Draft,
    Active,
    Deprecated,
    Archived,
}

impl AgentConfigurationProfileStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::Deprecated => "deprecated",
            Self::Archived => "archived",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSecretBinding {
    pub field_key: String,
    pub binding_kind: AgentSecretBindingKind,
    pub secret_ref: String,
    pub provider_hint: Option<String>,
}

impl AgentSecretBinding {
    pub fn new(
        field_key: impl Into<String>,
        binding_kind: AgentSecretBindingKind,
        secret_ref: impl Into<String>,
    ) -> Self {
        Self {
            field_key: field_key.into(),
            binding_kind,
            secret_ref: secret_ref.into(),
            provider_hint: None,
        }
    }

    pub fn login_password(field_key: impl Into<String>, secret_ref: impl Into<String>) -> Self {
        Self::new(field_key, AgentSecretBindingKind::LoginPassword, secret_ref)
    }

    pub fn llm_api_key(
        field_key: impl Into<String>,
        provider_hint: impl Into<String>,
        secret_ref: impl Into<String>,
    ) -> Self {
        Self::new(field_key, AgentSecretBindingKind::LlmApiKey, secret_ref)
            .with_provider_hint(provider_hint)
    }

    pub fn with_provider_hint(mut self, provider_hint: impl Into<String>) -> Self {
        self.provider_hint = Some(provider_hint.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSecretBindingKind {
    LoginPassword,
    LoginToken,
    OAuthCredential,
    LlmApiKey,
    CustomSecret,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConfigurationUpgradePlan {
    pub plan_id: String,
    pub agent_id: String,
    pub profile_id: String,
    pub from_configuration_version: String,
    pub to_configuration_version: String,
    pub steps: Vec<ConfigurationMigrationStep>,
    pub required_policy_categories: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConfigurationUpgradeRequest {
    pub request_id: String,
    pub agent_id: String,
    pub profile_id: String,
    pub from_configuration_version: String,
    pub to_configuration_version: String,
    pub current_profile: Option<AgentConfigurationProfile>,
    pub requested_by: Option<String>,
}

impl AgentConfigurationUpgradeRequest {
    pub fn new(
        request_id: impl Into<String>,
        agent_id: impl Into<String>,
        profile_id: impl Into<String>,
        from_configuration_version: impl Into<String>,
        to_configuration_version: impl Into<String>,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            agent_id: agent_id.into(),
            profile_id: profile_id.into(),
            from_configuration_version: from_configuration_version.into(),
            to_configuration_version: to_configuration_version.into(),
            current_profile: None,
            requested_by: None,
        }
    }

    pub fn with_current_profile(mut self, current_profile: AgentConfigurationProfile) -> Self {
        self.current_profile = Some(current_profile);
        self
    }

    pub fn requested_by(mut self, requested_by: impl Into<String>) -> Self {
        self.requested_by = Some(requested_by.into());
        self
    }
}

impl AgentConfigurationUpgradePlan {
    pub fn new(
        plan_id: impl Into<String>,
        agent_id: impl Into<String>,
        profile_id: impl Into<String>,
        from_configuration_version: impl Into<String>,
        to_configuration_version: impl Into<String>,
    ) -> Self {
        Self {
            plan_id: plan_id.into(),
            agent_id: agent_id.into(),
            profile_id: profile_id.into(),
            from_configuration_version: from_configuration_version.into(),
            to_configuration_version: to_configuration_version.into(),
            steps: Vec::new(),
            required_policy_categories: Vec::new(),
        }
    }

    pub fn from_json(input: &str) -> KernelResult<Self> {
        let manifest_type = extract_string(input, "manifest_type")?;
        if manifest_type != "agent_configuration_migration" {
            return Err(KernelError::validation(
                "manifest_type must be agent_configuration_migration",
            ));
        }

        let mut plan = Self::new(
            extract_string(input, "plan_id")?,
            extract_string(input, "agent_id")?,
            extract_string(input, "profile_id")?,
            extract_string(input, "from_configuration_version")?,
            extract_string(input, "to_configuration_version")?,
        );

        for category in extract_string_array(input, "required_policy_categories")? {
            if category == PolicyCategory::AgentConfigure.as_str() {
                plan = plan.require_policy(PolicyCategory::AgentConfigure);
            } else if !plan.required_policy_categories.contains(&category) {
                plan.required_policy_categories.push(category);
            }
        }

        for step_body in extract_object_array(input, "steps")? {
            plan = plan.add_step(parse_migration_step(&step_body)?);
        }

        if plan.steps.is_empty() {
            return Err(KernelError::validation(
                "configuration migration must declare at least one step",
            ));
        }

        Ok(plan)
    }

    pub fn add_step(mut self, step: ConfigurationMigrationStep) -> Self {
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
        let category = self
            .required_policy_categories
            .first()
            .cloned()
            .unwrap_or_else(|| PolicyCategory::AgentConfigure.as_str().to_string());

        PolicyRequest::new(
            policy_request_id,
            category,
            format!("{}/{}", self.agent_id, self.profile_id),
        )
        .with_category(PolicyCategory::AgentConfigure)
        .with_action("agent.configure.migrate")
        .with_side_effect_level(SideEffectLevel::SideEffectful)
        .with_redaction(KernelEventRedaction::Internal)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigurationMigrationStep {
    pub step_id: String,
    pub kind: ConfigurationMigrationStepKind,
    pub field_key: String,
    pub secret_binding_kind: Option<AgentSecretBindingKind>,
    pub provider_hint: Option<String>,
    pub side_effect_level: SideEffectLevel,
}

impl ConfigurationMigrationStep {
    pub fn preserve_secret_ref(
        field_key: impl Into<String>,
        secret_binding_kind: AgentSecretBindingKind,
    ) -> Self {
        Self::new(
            format!("preserve.{}", field_key.into()),
            ConfigurationMigrationStepKind::PreserveSecretRef,
            "",
        )
        .with_field_key_from_step_id()
        .with_secret_binding_kind(secret_binding_kind)
    }

    pub fn rebind_secret_ref(
        field_key: impl Into<String>,
        secret_binding_kind: AgentSecretBindingKind,
        provider_hint: impl Into<String>,
    ) -> Self {
        Self::new(
            format!("rebind.{}", field_key.into()),
            ConfigurationMigrationStepKind::RebindSecretRef,
            "",
        )
        .with_field_key_from_step_id()
        .with_secret_binding_kind(secret_binding_kind)
        .with_provider_hint(provider_hint)
    }

    pub fn new(
        step_id: impl Into<String>,
        kind: ConfigurationMigrationStepKind,
        field_key: impl Into<String>,
    ) -> Self {
        let side_effect_level = kind.side_effect_level();
        Self {
            step_id: step_id.into(),
            kind,
            field_key: field_key.into(),
            secret_binding_kind: None,
            provider_hint: None,
            side_effect_level,
        }
    }

    pub fn with_secret_binding_kind(mut self, kind: AgentSecretBindingKind) -> Self {
        self.secret_binding_kind = Some(kind);
        self
    }

    pub fn with_provider_hint(mut self, provider_hint: impl Into<String>) -> Self {
        self.provider_hint = Some(provider_hint.into());
        self
    }

    fn with_field_key_from_step_id(mut self) -> Self {
        self.field_key = self
            .step_id
            .split_once('.')
            .map(|(_, field_key)| field_key.to_string())
            .unwrap_or_default();
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigurationMigrationStepKind {
    PreserveValue,
    RenameField,
    SetDefault,
    RemoveField,
    PreserveSecretRef,
    RebindSecretRef,
}

impl ConfigurationMigrationStepKind {
    pub fn side_effect_level(&self) -> SideEffectLevel {
        match self {
            Self::PreserveValue | Self::PreserveSecretRef => SideEffectLevel::ReadOnly,
            Self::RemoveField | Self::RebindSecretRef => SideEffectLevel::SideEffectful,
            Self::RenameField | Self::SetDefault => SideEffectLevel::SideEffectful,
        }
    }
}

fn parse_config_section(input: &str) -> KernelResult<AgentConfigSection> {
    let kind_name = extract_string(input, "kind")?;
    let kind = parse_config_section_kind(
        &kind_name,
        extract_optional_string(input, "custom_namespace")?,
    )?;

    let mut section = AgentConfigSection::new(
        extract_string(input, "section_id")?,
        extract_string(input, "title")?,
        kind,
    );

    for field_body in extract_object_array(input, "fields")? {
        section = section.add_field(parse_config_field(&field_body)?);
    }

    if section.fields.is_empty() {
        return Err(KernelError::validation(format!(
            "configuration section must declare at least one field: {}",
            section.section_id
        )));
    }

    Ok(section)
}

fn parse_config_section_kind(
    input: &str,
    custom_namespace: Option<String>,
) -> KernelResult<AgentConfigSectionKind> {
    match input {
        "base" => Ok(AgentConfigSectionKind::Base),
        "login_auth" => Ok(AgentConfigSectionKind::LoginAuth),
        "llm_api_key" => Ok(AgentConfigSectionKind::LlmApiKey),
        "runtime" => Ok(AgentConfigSectionKind::Runtime),
        "security" => Ok(AgentConfigSectionKind::Security),
        "custom" => Ok(AgentConfigSectionKind::Custom(
            custom_namespace.unwrap_or_else(|| "custom".to_string()),
        )),
        kind if kind.starts_with("custom:") => Ok(AgentConfigSectionKind::Custom(
            kind.trim_start_matches("custom:").to_string(),
        )),
        kind => Err(KernelError::validation(format!(
            "unsupported configuration section kind: {kind}"
        ))),
    }
}

fn parse_config_field(input: &str) -> KernelResult<AgentConfigField> {
    let value_kind_name = extract_string(input, "value_kind")?;
    let value_kind = parse_config_value_kind(&value_kind_name)?;
    let mut field = AgentConfigField::new(
        extract_string(input, "key")?,
        extract_string(input, "label")?,
        value_kind.clone(),
    );

    if extract_optional_bool(input, "required")?.unwrap_or(false) {
        field = field.required();
    }

    let secret_ref_required = extract_optional_bool(input, "secret_ref_required")?
        .unwrap_or(matches!(value_kind, AgentConfigValueKind::SecretRef));
    if matches!(value_kind, AgentConfigValueKind::SecretRef) && !secret_ref_required {
        return Err(KernelError::validation(format!(
            "secret_ref_required must be true for secret_ref field: {}",
            field.key
        )));
    }
    if secret_ref_required {
        field = field.secret_ref_required();
    }

    let redaction = match extract_optional_string(input, "redaction_classification")? {
        Some(redaction) => parse_redaction_classification(&redaction)?,
        None if matches!(value_kind, AgentConfigValueKind::SecretRef) => {
            KernelEventRedaction::Secret
        }
        None => KernelEventRedaction::Public,
    };

    if matches!(value_kind, AgentConfigValueKind::SecretRef) && !redaction.is_sensitive() {
        return Err(KernelError::validation(format!(
            "redaction_classification must be sensitive for secret_ref field: {}",
            field.key
        )));
    }

    field = field.with_redaction(redaction);

    if let Some(description) = extract_optional_string(input, "description")? {
        field = field.with_description(description);
    }

    if let Some(default_value_body) = extract_optional_object_body(input, "default_value")? {
        let default_value_kind =
            extract_optional_string(&default_value_body, "value_kind")?.unwrap_or(value_kind_name);
        let default_value = parse_config_value(&default_value_body, &default_value_kind)?;
        if !field.value_kind.matches(&default_value) {
            return Err(KernelError::validation(format!(
                "default_value kind does not match field schema: {}",
                field.key
            )));
        }
        field = field.with_default(default_value);
    }

    Ok(field)
}

fn parse_config_value_kind(input: &str) -> KernelResult<AgentConfigValueKind> {
    match input {
        "string" => Ok(AgentConfigValueKind::String),
        "boolean" => Ok(AgentConfigValueKind::Boolean),
        "integer" => Ok(AgentConfigValueKind::Integer),
        "secret_ref" => Ok(AgentConfigValueKind::SecretRef),
        "string_list" => Ok(AgentConfigValueKind::StringList),
        "json" => Ok(AgentConfigValueKind::Json),
        kind => Err(KernelError::validation(format!(
            "unsupported configuration value kind: {kind}"
        ))),
    }
}

fn parse_redaction_classification(input: &str) -> KernelResult<KernelEventRedaction> {
    match input {
        "public" => Ok(KernelEventRedaction::Public),
        "internal" => Ok(KernelEventRedaction::Internal),
        "tenant_sensitive" => Ok(KernelEventRedaction::TenantSensitive),
        "personal_data" => Ok(KernelEventRedaction::PersonalData),
        "secret" => Ok(KernelEventRedaction::Secret),
        "regulated" => Ok(KernelEventRedaction::Regulated),
        "unknown" => Ok(KernelEventRedaction::Unknown),
        redaction => Err(KernelError::validation(format!(
            "unsupported redaction classification: {redaction}"
        ))),
    }
}

fn parse_profile_status(input: &str) -> KernelResult<AgentConfigurationProfileStatus> {
    match input {
        "draft" => Ok(AgentConfigurationProfileStatus::Draft),
        "active" => Ok(AgentConfigurationProfileStatus::Active),
        "deprecated" => Ok(AgentConfigurationProfileStatus::Deprecated),
        "archived" => Ok(AgentConfigurationProfileStatus::Archived),
        status => Err(KernelError::validation(format!(
            "unsupported configuration profile status: {status}"
        ))),
    }
}

fn parse_config_value(input: &str, value_kind: &str) -> KernelResult<AgentConfigValue> {
    match value_kind {
        "string" => Ok(AgentConfigValue::string(extract_string(input, "value")?)),
        "boolean" => Ok(AgentConfigValue::boolean(extract_bool(input, "value")?)),
        "integer" => Ok(AgentConfigValue::integer(extract_i64(input, "value")?)),
        "secret_ref" => Ok(AgentConfigValue::secret_ref(extract_string(
            input, "value",
        )?)),
        "string_list" => Ok(AgentConfigValue::string_list(extract_string_array(
            input, "value",
        )?)),
        "json" => Ok(AgentConfigValue::json(extract_raw_json_value(
            input, "value",
        )?)),
        kind => Err(KernelError::validation(format!(
            "unsupported configuration value kind: {kind}"
        ))),
    }
}

fn parse_secret_binding(input: &str) -> KernelResult<AgentSecretBinding> {
    let mut binding = AgentSecretBinding::new(
        extract_string(input, "field_key")?,
        parse_secret_binding_kind(&extract_string(input, "kind")?)?,
        extract_string(input, "secret_ref")?,
    );

    if let Some(provider_hint) = extract_optional_string(input, "provider_hint")? {
        binding = binding.with_provider_hint(provider_hint);
    }

    Ok(binding)
}

fn parse_secret_binding_kind(input: &str) -> KernelResult<AgentSecretBindingKind> {
    match input {
        "login_password" => Ok(AgentSecretBindingKind::LoginPassword),
        "login_token" => Ok(AgentSecretBindingKind::LoginToken),
        "oauth_credential" => Ok(AgentSecretBindingKind::OAuthCredential),
        "llm_api_key" => Ok(AgentSecretBindingKind::LlmApiKey),
        "custom_secret" => Ok(AgentSecretBindingKind::CustomSecret),
        kind => Err(KernelError::validation(format!(
            "unsupported secret binding kind: {kind}"
        ))),
    }
}

fn parse_migration_step(input: &str) -> KernelResult<ConfigurationMigrationStep> {
    let mut step = ConfigurationMigrationStep::new(
        extract_string(input, "step_id")?,
        parse_migration_step_kind(&extract_string(input, "kind")?)?,
        extract_string(input, "field_key")?,
    );

    if let Some(kind) = extract_optional_string(input, "secret_binding_kind")? {
        step = step.with_secret_binding_kind(parse_secret_binding_kind(&kind)?);
    }

    if let Some(provider_hint) = extract_optional_string(input, "provider_hint")? {
        step = step.with_provider_hint(provider_hint);
    }

    Ok(step)
}

fn parse_migration_step_kind(input: &str) -> KernelResult<ConfigurationMigrationStepKind> {
    match input {
        "preserve_value" => Ok(ConfigurationMigrationStepKind::PreserveValue),
        "rename_field" => Ok(ConfigurationMigrationStepKind::RenameField),
        "set_default" => Ok(ConfigurationMigrationStepKind::SetDefault),
        "remove_field" => Ok(ConfigurationMigrationStepKind::RemoveField),
        "preserve_secret_ref" => Ok(ConfigurationMigrationStepKind::PreserveSecretRef),
        "rebind_secret_ref" => Ok(ConfigurationMigrationStepKind::RebindSecretRef),
        kind => Err(KernelError::validation(format!(
            "unsupported configuration migration step kind: {kind}"
        ))),
    }
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
    match value.trim() {
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

fn extract_i64(input: &str, key: &str) -> KernelResult<i64> {
    extract_raw_json_value(input, key)?
        .trim()
        .parse::<i64>()
        .map_err(|_| KernelError::validation(format!("field is not an integer: {key}")))
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
    let mut values = Vec::new();
    let mut depth = 0usize;
    let mut start = None;

    for (index, ch) in array.char_indices() {
        if ch == '{' {
            if depth == 0 {
                start = Some(index + 1);
            }
            depth += 1;
        } else if ch == '}' {
            depth = depth.checked_sub(1).ok_or_else(|| {
                KernelError::validation(format!("unterminated object array field: {key}"))
            })?;
            if depth == 0 {
                let object_start = start.ok_or_else(|| {
                    KernelError::validation(format!("unterminated object array field: {key}"))
                })?;
                values.push(array[object_start..index].to_string());
                start = None;
            }
        }
    }

    if depth != 0 {
        return Err(KernelError::validation(format!(
            "unterminated object array field: {key}"
        )));
    }

    Ok(values)
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
    let body_start = after_key
        .find(open)
        .ok_or_else(|| KernelError::validation(format!("field has no body: {key}")))?;
    let mut depth = 0usize;
    let mut end = None;

    for (index, ch) in after_key[body_start..].char_indices() {
        if ch == open {
            depth += 1;
        } else if ch == close {
            depth -= 1;
            if depth == 0 {
                end = Some(body_start + index);
                break;
            }
        }
    }

    let end = end.ok_or_else(|| KernelError::validation(format!("unterminated body: {key}")))?;
    Ok(after_key[body_start + 1..end].to_string())
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

    let value_end = after_colon
        .find([',', '\n', '\r'])
        .unwrap_or(after_colon.len());
    Ok(after_colon[..value_end].trim().to_string())
}
