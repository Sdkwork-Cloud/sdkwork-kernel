use crate::{
    AgentConfigSectionKind, AgentInstallRequest, AgentPackageSource, AgentUninstallRequest,
    AgentUpgradeRequest, KernelError, KernelResult,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentPackageManifest {
    pub schema_version: String,
    pub agent_id: String,
    pub version: String,
    pub source: AgentPackageSource,
    pub lifecycle: AgentPackageLifecycle,
    pub provider_binding: AgentPackageProviderBinding,
    pub kernel_compatibility: AgentPackageVersionCompatibility,
    pub required_configuration_sections: Vec<AgentConfigSectionKind>,
    pub default_profile_id: Option<String>,
}

impl AgentPackageManifest {
    pub fn new(
        agent_id: impl Into<String>,
        version: impl Into<String>,
        source: AgentPackageSource,
    ) -> Self {
        Self {
            schema_version: "0.1.0".to_string(),
            agent_id: agent_id.into(),
            version: version.into(),
            source,
            lifecycle: AgentPackageLifecycle::default(),
            provider_binding: AgentPackageProviderBinding::default(),
            kernel_compatibility: AgentPackageVersionCompatibility::new("0.1.0", None::<String>),
            required_configuration_sections: Vec::new(),
            default_profile_id: None,
        }
    }

    pub fn from_json(input: &str) -> KernelResult<Self> {
        let manifest_type = extract_string(input, "manifest_type")?;
        if manifest_type != "agent_package" {
            return Err(KernelError::validation(
                "manifest_type must be agent_package",
            ));
        }

        let source = parse_package_source(&extract_object_body(input, "source")?)?;
        let lifecycle = parse_lifecycle(&extract_object_body(input, "lifecycle")?)?;
        let provider_binding =
            parse_provider_binding(&extract_object_body(input, "provider_binding")?)?;
        let kernel_compatibility =
            parse_kernel_compatibility(&extract_object_body(input, "kernel_compatibility")?)?;
        let required_configuration_sections =
            parse_required_configuration_sections(input, "required_configuration_sections")?;

        if required_configuration_sections.is_empty() {
            return Err(KernelError::validation(
                "at least one required configuration section must be declared",
            ));
        }

        let mut manifest = Self::new(
            extract_string(input, "agent_id")?,
            extract_string(input, "version")?,
            source,
        )
        .with_lifecycle(lifecycle)?
        .with_provider_binding(provider_binding)?
        .with_kernel_compatibility(kernel_compatibility);

        manifest.schema_version = extract_string(input, "schema_version")?;
        for section in required_configuration_sections {
            manifest = manifest.require_configuration_section(section);
        }

        if let Some(default_profile_id) = extract_optional_string(input, "default_profile_id")? {
            manifest = manifest.with_default_profile(default_profile_id);
        }

        Ok(manifest)
    }

    pub fn with_lifecycle(mut self, lifecycle: AgentPackageLifecycle) -> KernelResult<Self> {
        lifecycle.validate()?;
        self.lifecycle = lifecycle;
        Ok(self)
    }

    pub fn with_provider_binding(
        mut self,
        provider_binding: AgentPackageProviderBinding,
    ) -> KernelResult<Self> {
        provider_binding.validate()?;
        self.provider_binding = provider_binding;
        Ok(self)
    }

    pub fn with_kernel_compatibility(
        mut self,
        kernel_compatibility: AgentPackageVersionCompatibility,
    ) -> Self {
        self.kernel_compatibility = kernel_compatibility;
        self
    }

    pub fn require_configuration_section(mut self, section: AgentConfigSectionKind) -> Self {
        if !self.required_configuration_sections.contains(&section) {
            self.required_configuration_sections.push(section);
        }
        self
    }

    pub fn with_default_profile(mut self, profile_id: impl Into<String>) -> Self {
        self.default_profile_id = Some(profile_id.into());
        self
    }

    pub fn required_configuration_sections(&self) -> Vec<AgentConfigSectionKind> {
        self.required_configuration_sections.clone()
    }

    pub fn requires_login_auth(&self) -> bool {
        self.required_configuration_sections
            .contains(&AgentConfigSectionKind::LoginAuth)
    }

    pub fn requires_llm_api_key(&self) -> bool {
        self.required_configuration_sections
            .contains(&AgentConfigSectionKind::LlmApiKey)
    }

    pub fn is_compatible_with_agent_kernel(&self, version: &str) -> bool {
        self.kernel_compatibility.matches(version)
    }

    pub fn install_request(&self, request_id: impl Into<String>) -> AgentInstallRequest {
        let request = AgentInstallRequest::new(
            request_id,
            self.agent_id.clone(),
            self.version.clone(),
            self.source.clone(),
        );

        match &self.default_profile_id {
            Some(profile_id) => request.with_profile(profile_id.clone()),
            None => request,
        }
    }

    pub fn upgrade_request(
        &self,
        request_id: impl Into<String>,
        from_version: impl Into<String>,
    ) -> AgentUpgradeRequest {
        AgentUpgradeRequest::new(
            request_id,
            self.agent_id.clone(),
            from_version,
            self.version.clone(),
        )
    }

    pub fn uninstall_request(&self, request_id: impl Into<String>) -> AgentUninstallRequest {
        AgentUninstallRequest::new(request_id, self.agent_id.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentPackageLifecycle {
    pub supports_install: bool,
    pub supports_uninstall: bool,
    pub supports_upgrade: bool,
}

impl AgentPackageLifecycle {
    pub fn installable() -> Self {
        Self {
            supports_install: true,
            supports_uninstall: true,
            supports_upgrade: true,
        }
    }

    pub fn validate(&self) -> KernelResult<()> {
        if !self.supports_install {
            return Err(KernelError::validation(
                "agent package lifecycle must support install",
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentPackageProviderBinding {
    pub installer_provider_id: String,
    pub configuration_provider_id: String,
}

impl AgentPackageProviderBinding {
    pub fn new(
        installer_provider_id: impl Into<String>,
        configuration_provider_id: impl Into<String>,
    ) -> Self {
        Self {
            installer_provider_id: installer_provider_id.into(),
            configuration_provider_id: configuration_provider_id.into(),
        }
    }

    pub fn validate(&self) -> KernelResult<()> {
        if self.installer_provider_id.trim().is_empty() {
            return Err(KernelError::validation("installer provider id is required"));
        }

        if self.configuration_provider_id.trim().is_empty() {
            return Err(KernelError::validation(
                "configuration provider id is required",
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentPackageVersionCompatibility {
    pub min_agent_kernel_version: String,
    pub max_agent_kernel_version_exclusive: Option<String>,
}

impl AgentPackageVersionCompatibility {
    pub fn new(
        min_agent_kernel_version: impl Into<String>,
        max_agent_kernel_version_exclusive: Option<impl Into<String>>,
    ) -> Self {
        Self {
            min_agent_kernel_version: min_agent_kernel_version.into(),
            max_agent_kernel_version_exclusive: max_agent_kernel_version_exclusive
                .map(std::convert::Into::into),
        }
    }

    pub fn matches(&self, version: &str) -> bool {
        version >= self.min_agent_kernel_version.as_str()
            && self
                .max_agent_kernel_version_exclusive
                .as_deref()
                .is_none_or(|max_version| version < max_version)
    }
}

fn parse_package_source(input: &str) -> KernelResult<AgentPackageSource> {
    match extract_string(input, "type")?.as_str() {
        "local_path" => Ok(AgentPackageSource::local_path(extract_string(
            input, "path",
        )?)),
        "registry" => Ok(AgentPackageSource::registry(
            extract_string(input, "registry_id")?,
            extract_string(input, "package_id")?,
            extract_string(input, "version")?,
        )),
        "remote_archive" => Ok(AgentPackageSource::remote_archive(
            extract_string(input, "url")?,
            extract_string(input, "checksum")?,
        )),
        source_type => Err(KernelError::validation(format!(
            "unsupported agent package source type: {source_type}"
        ))),
    }
}

fn parse_lifecycle(input: &str) -> KernelResult<AgentPackageLifecycle> {
    let lifecycle = AgentPackageLifecycle {
        supports_install: extract_bool(input, "install")?,
        supports_uninstall: extract_bool(input, "uninstall")?,
        supports_upgrade: extract_bool(input, "upgrade")?,
    };
    lifecycle.validate()?;
    Ok(lifecycle)
}

fn parse_provider_binding(input: &str) -> KernelResult<AgentPackageProviderBinding> {
    let binding = AgentPackageProviderBinding::new(
        extract_string(input, "installer_provider_id")?,
        extract_string(input, "configuration_provider_id")?,
    );
    binding.validate()?;
    Ok(binding)
}

fn parse_kernel_compatibility(input: &str) -> KernelResult<AgentPackageVersionCompatibility> {
    Ok(AgentPackageVersionCompatibility::new(
        extract_string(input, "agent_kernel_min")?,
        extract_optional_string(input, "agent_kernel_max_exclusive")?,
    ))
}

fn parse_required_configuration_sections(
    input: &str,
    key: &str,
) -> KernelResult<Vec<AgentConfigSectionKind>> {
    extract_string_array(input, key)?
        .into_iter()
        .map(|section| parse_configuration_section_kind(&section))
        .collect()
}

fn parse_configuration_section_kind(input: &str) -> KernelResult<AgentConfigSectionKind> {
    match input {
        "base" => Ok(AgentConfigSectionKind::Base),
        "login_auth" => Ok(AgentConfigSectionKind::LoginAuth),
        "llm_api_key" => Ok(AgentConfigSectionKind::LlmApiKey),
        "runtime" => Ok(AgentConfigSectionKind::Runtime),
        "security" => Ok(AgentConfigSectionKind::Security),
        custom if custom.starts_with("custom.") => Ok(AgentConfigSectionKind::Custom(
            custom["custom.".len()..].to_string(),
        )),
        section => Err(KernelError::validation(format!(
            "unsupported configuration section: {section}"
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
    let pattern = format!("\"{key}\"");
    let key_start = input
        .find(&pattern)
        .ok_or_else(|| KernelError::validation(format!("missing field: {key}")))?;
    let after_key = &input[key_start + pattern.len()..];
    let colon = after_key
        .find(':')
        .ok_or_else(|| KernelError::validation(format!("missing ':' after field: {key}")))?;
    let after_colon = after_key[colon + 1..].trim_start();

    if after_colon.starts_with("true") {
        Ok(true)
    } else if after_colon.starts_with("false") {
        Ok(false)
    } else {
        Err(KernelError::validation(format!(
            "field is not a boolean: {key}"
        )))
    }
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

fn extract_array_body(input: &str, key: &str) -> KernelResult<String> {
    extract_delimited_body(input, key, '[', ']')
}

fn extract_object_body(input: &str, key: &str) -> KernelResult<String> {
    extract_delimited_body(input, key, '{', '}')
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
