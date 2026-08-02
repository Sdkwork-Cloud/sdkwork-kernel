use std::fmt::{Display, Formatter};

use crate::{
    KernelEvent, KernelEventRedaction, KernelEventSeverity, KernelEventSource, TraceContext,
};

pub type KernelResult<T> = Result<T, KernelError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelErrorKind {
    ValidationError,
    CapabilityMissing,
    ProviderUnavailable,
    ProviderError,
    PolicyDenied,
    PermissionRequired,
    Timeout,
    Cancelled,
    Conflict,
    RateLimited,
    ResourceExhausted,
    UnsafeContent,
    SecurityViolation,
    InternalError,
}

impl KernelErrorKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ValidationError => "validation_error",
            Self::CapabilityMissing => "capability_missing",
            Self::ProviderUnavailable => "provider_unavailable",
            Self::ProviderError => "provider_error",
            Self::PolicyDenied => "policy_denied",
            Self::PermissionRequired => "permission_required",
            Self::Timeout => "timeout",
            Self::Cancelled => "cancelled",
            Self::Conflict => "conflict",
            Self::RateLimited => "rate_limited",
            Self::ResourceExhausted => "resource_exhausted",
            Self::UnsafeContent => "unsafe_content",
            Self::SecurityViolation => "security_violation",
            Self::InternalError => "internal_error",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelErrorSource {
    Runtime,
    Provider,
    Model,
    Tool,
    Context,
    Memory,
    Policy,
    Host,
    ProtocolAdapter,
    KernelUi,
    CodeKernel,
    Telemetry,
    Unknown,
}

impl KernelErrorSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Runtime => "runtime",
            Self::Provider => "provider",
            Self::Model => "model",
            Self::Tool => "tool",
            Self::Context => "context",
            Self::Memory => "memory",
            Self::Policy => "policy",
            Self::Host => "host",
            Self::ProtocolAdapter => "protocol_adapter",
            Self::KernelUi => "kernel_ui",
            Self::CodeKernel => "code_kernel",
            Self::Telemetry => "telemetry",
            Self::Unknown => "unknown",
        }
    }

    fn as_event_source(&self) -> KernelEventSource {
        match self {
            Self::Runtime => KernelEventSource::Runtime,
            Self::Provider => KernelEventSource::Provider,
            Self::Model => KernelEventSource::Model,
            Self::Tool => KernelEventSource::Tool,
            Self::Context => KernelEventSource::Context,
            Self::Memory => KernelEventSource::Memory,
            Self::Policy => KernelEventSource::Policy,
            Self::Host => KernelEventSource::Host,
            Self::ProtocolAdapter => KernelEventSource::ProtocolAdapter,
            Self::KernelUi => KernelEventSource::KernelUi,
            Self::CodeKernel => KernelEventSource::CodeKernel,
            Self::Telemetry => KernelEventSource::Telemetry,
            Self::Unknown => KernelEventSource::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelErrorInfo {
    pub code: String,
    pub message: String,
    pub kind: KernelErrorKind,
    pub retryable: bool,
    pub safe_for_user: bool,
    pub safe_message: Option<String>,
    pub provider_id: Option<String>,
    pub source: KernelErrorSource,
    pub trace_context: Option<TraceContext>,
    pub details: Vec<(String, String)>,
    pub redaction_classification: KernelEventRedaction,
}

impl KernelErrorInfo {
    pub fn new(kind: KernelErrorKind, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            kind,
            retryable: default_retryable(kind),
            safe_for_user: default_safe_for_user(kind),
            safe_message: None,
            provider_id: None,
            source: KernelErrorSource::Unknown,
            trace_context: None,
            details: Vec::new(),
            redaction_classification: KernelEventRedaction::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelError {
    Validation { message: String },
    CapabilityMissing { capability_id: String },
    ProviderUnavailable { provider_id: String },
    PolicyDenied { reason_code: String },
    Internal { message: String },
    Structured { info: Box<KernelErrorInfo> },
}

impl KernelError {
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    /// Structured not-found marker for resources that do not exist.
    ///
    /// The `sdkwork.not_found` detail lets HTTP adapters map the error to a
    /// 404 status without parsing message text; the kind stays
    /// [`KernelErrorKind::ValidationError`] so existing consumers are
    /// unaffected.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::validation(message).with_detail("sdkwork.not_found", "true")
    }

    pub fn provider_error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::structured(KernelErrorKind::ProviderError, code, message)
            .from_source(KernelErrorSource::Provider)
    }

    pub fn permission_required(message: impl Into<String>) -> Self {
        Self::structured(
            KernelErrorKind::PermissionRequired,
            KernelErrorKind::PermissionRequired.as_str(),
            message,
        )
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self::structured(
            KernelErrorKind::Timeout,
            KernelErrorKind::Timeout.as_str(),
            message,
        )
    }

    pub fn cancelled(message: impl Into<String>) -> Self {
        Self::structured(
            KernelErrorKind::Cancelled,
            KernelErrorKind::Cancelled.as_str(),
            message,
        )
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::structured(
            KernelErrorKind::Conflict,
            KernelErrorKind::Conflict.as_str(),
            message,
        )
    }

    pub fn rate_limited(message: impl Into<String>) -> Self {
        Self::structured(
            KernelErrorKind::RateLimited,
            KernelErrorKind::RateLimited.as_str(),
            message,
        )
    }

    pub fn resource_exhausted(message: impl Into<String>) -> Self {
        Self::structured(
            KernelErrorKind::ResourceExhausted,
            KernelErrorKind::ResourceExhausted.as_str(),
            message,
        )
    }

    pub fn unsafe_content(message: impl Into<String>) -> Self {
        Self::structured(
            KernelErrorKind::UnsafeContent,
            KernelErrorKind::UnsafeContent.as_str(),
            message,
        )
    }

    pub fn security_violation(message: impl Into<String>) -> Self {
        Self::structured(
            KernelErrorKind::SecurityViolation,
            KernelErrorKind::SecurityViolation.as_str(),
            message,
        )
    }

    pub fn kind(&self) -> KernelErrorKind {
        match self {
            Self::Validation { .. } => KernelErrorKind::ValidationError,
            Self::CapabilityMissing { .. } => KernelErrorKind::CapabilityMissing,
            Self::ProviderUnavailable { .. } => KernelErrorKind::ProviderUnavailable,
            Self::PolicyDenied { .. } => KernelErrorKind::PolicyDenied,
            Self::Internal { .. } => KernelErrorKind::InternalError,
            Self::Structured { info } => info.kind,
        }
    }

    pub fn code(&self) -> &str {
        match self {
            Self::Structured { info } => info.code.as_str(),
            _ => self.kind().as_str(),
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::Validation { message } | Self::Internal { message } => message,
            Self::CapabilityMissing { capability_id } => capability_id,
            Self::ProviderUnavailable { provider_id } => provider_id,
            Self::PolicyDenied { reason_code } => reason_code,
            Self::Structured { info } => info.message.as_str(),
        }
    }

    pub fn safe_message(&self) -> &str {
        match self {
            Self::Validation { message } => message,
            Self::CapabilityMissing { .. } => "required capability is unavailable",
            Self::ProviderUnavailable { .. } => "provider is unavailable",
            Self::PolicyDenied { .. } => "request denied by policy",
            Self::Internal { .. } => "internal kernel error",
            Self::Structured { info } => info.safe_message.as_deref().unwrap_or_else(|| {
                if info.safe_for_user {
                    info.message.as_str()
                } else {
                    default_safe_message(info.kind)
                }
            }),
        }
    }

    pub fn retryable(&self) -> bool {
        match self {
            Self::ProviderUnavailable { .. } => true,
            Self::Structured { info } => info.retryable,
            _ => default_retryable(self.kind()),
        }
    }

    pub fn safe_for_user(&self) -> bool {
        match self {
            Self::Internal { .. } => false,
            Self::Structured { info } => info.safe_for_user,
            _ => default_safe_for_user(self.kind()),
        }
    }

    pub fn provider_id(&self) -> Option<&str> {
        match self {
            Self::ProviderUnavailable { provider_id } => Some(provider_id.as_str()),
            Self::Structured { info } => info.provider_id.as_deref(),
            _ => None,
        }
    }

    pub fn source(&self) -> KernelErrorSource {
        match self {
            Self::ProviderUnavailable { .. } => KernelErrorSource::Provider,
            Self::PolicyDenied { .. } => KernelErrorSource::Policy,
            Self::Structured { info } => info.source,
            _ => KernelErrorSource::Unknown,
        }
    }

    pub fn trace_context(&self) -> Option<&TraceContext> {
        match self {
            Self::Structured { info } => info.trace_context.as_ref(),
            _ => None,
        }
    }

    pub fn detail_value(&self, key: &str) -> Option<&str> {
        match self {
            Self::Structured { info } => info
                .details
                .iter()
                .find(|(detail_key, _)| detail_key == key)
                .map(|(_, value)| value.as_str()),
            _ => None,
        }
    }

    pub fn redaction_classification(&self) -> KernelEventRedaction {
        match self {
            Self::Structured { info } => info.redaction_classification,
            _ => KernelEventRedaction::Unknown,
        }
    }

    pub fn with_provider(self, provider_id: impl Into<String>) -> Self {
        self.with_info(|info| info.provider_id = Some(provider_id.into()))
    }

    pub fn from_source(self, source: KernelErrorSource) -> Self {
        self.with_info(|info| info.source = source)
    }

    pub fn with_trace_context(self, trace_context: TraceContext) -> Self {
        self.with_info(|info| info.trace_context = Some(trace_context))
    }

    pub fn with_detail(self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.with_info(|info| info.details.push((key.into(), value.into())))
    }

    pub fn with_redaction(self, redaction_classification: KernelEventRedaction) -> Self {
        self.with_info(|info| info.redaction_classification = redaction_classification)
    }

    pub fn with_retryable(self, retryable: bool) -> Self {
        self.with_info(|info| info.retryable = retryable)
    }

    pub fn with_safe_for_user(self, safe_for_user: bool) -> Self {
        self.with_info(|info| info.safe_for_user = safe_for_user)
    }

    pub fn with_safe_message(self, safe_message: impl Into<String>) -> Self {
        self.with_info(|info| info.safe_message = Some(safe_message.into()))
    }

    pub fn to_event(&self, event_id: impl Into<String>) -> KernelEvent {
        let mut event = KernelEvent::new(
            event_id,
            "agent.error.occurred",
            KernelEventSeverity::Error,
            format!(
                "kind={};code={};safe_message={};retryable={};safe_for_user={};provider_id={};source={}",
                self.kind().as_str(),
                self.code(),
                self.safe_message(),
                self.retryable(),
                self.safe_for_user(),
                self.provider_id().unwrap_or(""),
                self.source().as_str()
            ),
        )
        .from_source(self.source().as_event_source())
        .with_redaction(self.redaction_classification())
        .with_payload_schema("sdkwork.agent.error.v1");

        if let Some(trace_context) = self.trace_context() {
            event = event.with_trace_context(trace_context.clone());
        }

        event
    }

    fn structured(
        kind: KernelErrorKind,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::Structured {
            info: Box::new(KernelErrorInfo::new(kind, code, message)),
        }
    }

    fn with_info(self, update: impl FnOnce(&mut KernelErrorInfo)) -> Self {
        let mut info = self.into_info();
        update(&mut info);
        Self::Structured {
            info: Box::new(info),
        }
    }

    fn into_info(self) -> KernelErrorInfo {
        match self {
            Self::Validation { message } => KernelErrorInfo::new(
                KernelErrorKind::ValidationError,
                "validation_error",
                message,
            ),
            Self::CapabilityMissing { capability_id } => KernelErrorInfo::new(
                KernelErrorKind::CapabilityMissing,
                "capability_missing",
                capability_id,
            ),
            Self::ProviderUnavailable { provider_id } => {
                let mut info = KernelErrorInfo::new(
                    KernelErrorKind::ProviderUnavailable,
                    "provider_unavailable",
                    "provider is unavailable",
                );
                info.provider_id = Some(provider_id);
                info.source = KernelErrorSource::Provider;
                info
            }
            Self::PolicyDenied { reason_code } => {
                let mut info = KernelErrorInfo::new(
                    KernelErrorKind::PolicyDenied,
                    "policy_denied",
                    reason_code,
                );
                info.source = KernelErrorSource::Policy;
                info
            }
            Self::Internal { message } => {
                KernelErrorInfo::new(KernelErrorKind::InternalError, "internal_error", message)
            }
            Self::Structured { info } => *info,
        }
    }
}

impl Display for KernelError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation { message } => write!(f, "validation error: {message}"),
            Self::CapabilityMissing { capability_id } => {
                write!(f, "capability missing: {capability_id}")
            }
            Self::ProviderUnavailable { provider_id } => {
                write!(f, "provider unavailable: {provider_id}")
            }
            Self::PolicyDenied { reason_code } => write!(f, "policy denied: {reason_code}"),
            Self::Internal { message } => write!(f, "internal error: {message}"),
            Self::Structured { info } => write!(f, "{}: {}", info.kind.as_str(), info.message),
        }
    }
}

impl std::error::Error for KernelError {}

fn default_retryable(kind: KernelErrorKind) -> bool {
    matches!(
        kind,
        KernelErrorKind::ProviderUnavailable | KernelErrorKind::RateLimited
    )
}

fn default_safe_for_user(kind: KernelErrorKind) -> bool {
    !matches!(kind, KernelErrorKind::InternalError)
}

fn default_safe_message(kind: KernelErrorKind) -> &'static str {
    match kind {
        KernelErrorKind::ValidationError => "validation error",
        KernelErrorKind::CapabilityMissing => "required capability is unavailable",
        KernelErrorKind::ProviderUnavailable | KernelErrorKind::ProviderError => {
            "provider is unavailable"
        }
        KernelErrorKind::PolicyDenied => "request denied by policy",
        KernelErrorKind::PermissionRequired => "permission is required",
        KernelErrorKind::Timeout => "operation timed out",
        KernelErrorKind::Cancelled => "operation was cancelled",
        KernelErrorKind::Conflict => "resource conflict",
        KernelErrorKind::RateLimited => "rate limited",
        KernelErrorKind::ResourceExhausted => "resource exhausted",
        KernelErrorKind::UnsafeContent => "unsafe content detected",
        KernelErrorKind::SecurityViolation => "security policy violation",
        KernelErrorKind::InternalError => "internal kernel error",
    }
}
