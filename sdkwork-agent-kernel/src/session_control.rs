use crate::{KernelError, KernelResult, ProviderHealth, ProviderManifest, TraceContext};

const MAX_IDENTIFIER_BYTES: usize = 512;
const MAX_TEXT_BYTES: usize = 4 * 1024;
const MAX_METADATA_ENTRIES: usize = 64;

/// Stable provider-session actions shared by agent runtimes.
///
/// The kernel keeps these semantics distinct from provider wire methods. A
/// provider advertises the exact `session.control.*` capabilities it supports
/// and maps the action to its own SDK or protocol at the L3 boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderSessionControlActionKind {
    Interrupt,
    Compact,
    Fork,
}

impl ProviderSessionControlActionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Interrupt => "interrupt",
            Self::Compact => "compact",
            Self::Fork => "fork",
        }
    }

    pub fn capability_id(self) -> &'static str {
        match self {
            Self::Interrupt => "session.control.interrupt",
            Self::Compact => "session.control.compact",
            Self::Fork => "session.control.fork",
        }
    }
}

/// One provider-neutral mutation of an existing provider session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderSessionControlAction {
    Interrupt { reason: Option<String> },
    Compact { focus: Option<String> },
    Fork { before_message_id: Option<String> },
}

impl ProviderSessionControlAction {
    pub fn kind(&self) -> ProviderSessionControlActionKind {
        match self {
            Self::Interrupt { .. } => ProviderSessionControlActionKind::Interrupt,
            Self::Compact { .. } => ProviderSessionControlActionKind::Compact,
            Self::Fork { .. } => ProviderSessionControlActionKind::Fork,
        }
    }

    fn validate(&self) -> KernelResult<()> {
        match self {
            Self::Interrupt { reason } => validate_optional_text("reason", reason),
            Self::Compact { focus } => validate_optional_text("focus", focus),
            Self::Fork { before_message_id } => {
                validate_optional_identifier("before_message_id", before_message_id)
            }
        }
    }
}

/// Correlated request to control one provider-owned session.
///
/// `session_id` is the canonical SDKWork identity. `provider_session_id` is
/// the opaque identity returned by the provider. They are intentionally never
/// substituted for one another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderSessionControlRequest {
    pub control_request_id: String,
    pub session_id: String,
    pub provider_session_id: String,
    pub policy_decision_id: String,
    pub action: ProviderSessionControlAction,
    pub working_directory: Option<String>,
    pub timeout_ms: Option<u64>,
    pub trace_context: Option<TraceContext>,
    pub metadata: Vec<(String, String)>,
}

impl ProviderSessionControlRequest {
    pub fn new(
        control_request_id: impl Into<String>,
        session_id: impl Into<String>,
        provider_session_id: impl Into<String>,
        policy_decision_id: impl Into<String>,
        action: ProviderSessionControlAction,
    ) -> Self {
        Self {
            control_request_id: control_request_id.into(),
            session_id: session_id.into(),
            provider_session_id: provider_session_id.into(),
            policy_decision_id: policy_decision_id.into(),
            action,
            working_directory: None,
            timeout_ms: None,
            trace_context: None,
            metadata: Vec::new(),
        }
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    pub fn with_working_directory(mut self, working_directory: impl Into<String>) -> Self {
        self.working_directory = Some(working_directory.into());
        self
    }

    pub fn with_trace_context(mut self, trace_context: TraceContext) -> Self {
        self.trace_context = Some(trace_context);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }

    pub fn validate(&self) -> KernelResult<()> {
        validate_identifier("control_request_id", &self.control_request_id)?;
        validate_identifier("session_id", &self.session_id)?;
        validate_identifier("provider_session_id", &self.provider_session_id)?;
        validate_identifier("policy_decision_id", &self.policy_decision_id)?;
        self.action.validate()?;

        if let Some(working_directory) = &self.working_directory {
            if working_directory.trim().is_empty()
                || working_directory.len() > MAX_TEXT_BYTES
                || working_directory.chars().any(char::is_control)
            {
                return Err(KernelError::validation(
                    "provider session control working_directory is invalid",
                ));
            }
        }

        if self.timeout_ms == Some(0) {
            return Err(KernelError::validation(
                "provider session control timeout_ms must be greater than zero",
            ));
        }
        if self.metadata.len() > MAX_METADATA_ENTRIES {
            return Err(KernelError::validation(format!(
                "provider session control metadata exceeded entry limit ({MAX_METADATA_ENTRIES})"
            )));
        }
        for (key, value) in &self.metadata {
            if !key.contains('.') || key.trim() != key || key.chars().any(char::is_control) {
                return Err(KernelError::validation(
                    "provider session control metadata keys must be namespaced",
                ));
            }
            if value.len() > MAX_TEXT_BYTES {
                return Err(KernelError::validation(format!(
                    "provider session control metadata value exceeded byte limit ({MAX_TEXT_BYTES})"
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderSessionControlStatus {
    Applied,
    NoOp,
}

impl ProviderSessionControlStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::NoOp => "no_op",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderSessionControlOutput {
    Acknowledged,
    Forked { provider_session_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderSessionControlResult {
    pub control_request_id: String,
    pub session_id: String,
    pub provider_session_id: String,
    pub action: ProviderSessionControlActionKind,
    pub status: ProviderSessionControlStatus,
    pub output: ProviderSessionControlOutput,
    pub metadata: Vec<(String, String)>,
}

impl ProviderSessionControlResult {
    pub fn acknowledged(request: &ProviderSessionControlRequest) -> Self {
        Self {
            control_request_id: request.control_request_id.clone(),
            session_id: request.session_id.clone(),
            provider_session_id: request.provider_session_id.clone(),
            action: request.action.kind(),
            status: ProviderSessionControlStatus::Applied,
            output: ProviderSessionControlOutput::Acknowledged,
            metadata: Vec::new(),
        }
    }

    pub fn no_op(request: &ProviderSessionControlRequest) -> Self {
        Self {
            status: ProviderSessionControlStatus::NoOp,
            ..Self::acknowledged(request)
        }
    }

    pub fn forked(
        request: &ProviderSessionControlRequest,
        provider_session_id: impl Into<String>,
    ) -> Self {
        Self {
            output: ProviderSessionControlOutput::Forked {
                provider_session_id: provider_session_id.into(),
            },
            ..Self::acknowledged(request)
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }
}

/// Optional L0 extension for controlling a provider-owned live session.
pub trait ProviderSessionControlProvider {
    fn provider_manifest(&self) -> ProviderManifest;

    fn control(
        &self,
        request: ProviderSessionControlRequest,
    ) -> KernelResult<ProviderSessionControlResult>;

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

fn validate_identifier(field: &str, value: &str) -> KernelResult<()> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(KernelError::validation(format!(
            "provider session control {field} must not be empty"
        )));
    }
    if trimmed != value || value.len() > MAX_IDENTIFIER_BYTES || value.chars().any(char::is_control)
    {
        return Err(KernelError::validation(format!(
            "provider session control {field} is invalid"
        )));
    }
    Ok(())
}

fn validate_optional_identifier(field: &str, value: &Option<String>) -> KernelResult<()> {
    match value {
        Some(value) => validate_identifier(field, value),
        None => Ok(()),
    }
}

fn validate_optional_text(field: &str, value: &Option<String>) -> KernelResult<()> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.trim().is_empty()
        || value.len() > MAX_TEXT_BYTES
        || value.chars().any(char::is_control)
    {
        return Err(KernelError::validation(format!(
            "provider session control {field} is invalid"
        )));
    }
    Ok(())
}
