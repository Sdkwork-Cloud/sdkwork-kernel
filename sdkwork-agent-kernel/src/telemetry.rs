use crate::{
    KernelEvent, KernelEventRedaction, KernelEventSeverity, KernelEventSource, KernelResult,
    PolicyDecision, PolicyRequest, ProviderHealth, ProviderManifest, TraceContext,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditRecord {
    pub audit_id: String,
    pub event_type: String,
    pub actor: Option<String>,
    pub subject: Option<String>,
    pub action: String,
    pub resource: String,
    pub policy_decision_id: Option<String>,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub step_id: Option<String>,
    pub created_at: Option<String>,
    pub trace_context: Option<TraceContext>,
    pub redaction_classification: KernelEventRedaction,
    pub metadata: Vec<(String, String)>,
}

impl AuditRecord {
    pub fn new(
        audit_id: impl Into<String>,
        event_type: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        Self {
            audit_id: audit_id.into(),
            event_type: event_type.into(),
            actor: None,
            subject: None,
            action: action.into(),
            resource: String::new(),
            policy_decision_id: None,
            session_id: None,
            task_id: None,
            run_id: None,
            step_id: None,
            created_at: None,
            trace_context: None,
            redaction_classification: KernelEventRedaction::Unknown,
            metadata: Vec::new(),
        }
    }

    pub fn from_policy_decision(
        audit_id: impl Into<String>,
        decision: &PolicyDecision,
        request: &PolicyRequest,
    ) -> Self {
        let mut record = Self::new(
            audit_id,
            "agent.audit.policy_decision",
            decision.decision.as_str(),
        )
        .with_resource(request.resource.clone())
        .with_policy_decision(decision.decision_id.clone())
        .with_redaction(request.redaction_classification)
        .with_metadata("sdkwork.policy.request_id", decision.request_id.clone())
        .with_metadata(
            "sdkwork.policy.provider_id",
            decision.policy_provider_id.clone(),
        )
        .with_metadata("sdkwork.policy.reason_code", decision.reason_code.clone())
        .with_metadata(
            "sdkwork.policy.audit_required",
            decision.audit_required.to_string(),
        );

        if let Some(session_id) = &request.session_id {
            record = record.for_session(session_id.clone());
        }

        if let Some(task_id) = &request.task_id {
            record = record.for_task(task_id.clone());
        }

        if let Some(run_id) = &request.run_id {
            record = record.for_run(run_id.clone());
        }

        if let Some(action) = &request.action {
            record = record.with_metadata("sdkwork.policy.action", action.clone());
        }

        record
    }

    pub fn with_actor(mut self, actor: impl Into<String>) -> Self {
        self.actor = Some(actor.into());
        self
    }

    pub fn with_subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = action.into();
        self
    }

    pub fn with_resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = resource.into();
        self
    }

    pub fn with_policy_decision(mut self, policy_decision_id: impl Into<String>) -> Self {
        self.policy_decision_id = Some(policy_decision_id.into());
        self
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

    pub fn created_at(mut self, created_at: impl Into<String>) -> Self {
        self.created_at = Some(created_at.into());
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

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }

    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        self.metadata
            .iter()
            .find(|(metadata_key, _)| metadata_key == key)
            .map(|(_, value)| value.as_str())
    }

    pub fn to_event(&self, event_id: impl Into<String>) -> KernelEvent {
        let mut event = KernelEvent::new(
            event_id,
            "agent.audit.recorded",
            KernelEventSeverity::Info,
            format!(
                "audit_id={};event_type={};action={};resource={};decision_id={}",
                self.audit_id,
                self.event_type,
                self.action,
                self.resource,
                self.policy_decision_id.as_deref().unwrap_or("")
            ),
        )
        .from_source(KernelEventSource::Telemetry)
        .with_redaction(self.redaction_classification)
        .with_payload_schema("sdkwork.agent.audit.record.v1");

        if let Some(session_id) = &self.session_id {
            event = event.for_session(session_id.clone());
        }

        if let Some(task_id) = &self.task_id {
            event = event.for_task(task_id.clone());
        }

        if let Some(run_id) = &self.run_id {
            event = event.for_run(run_id.clone());
        }

        if let Some(step_id) = &self.step_id {
            event = event.for_step(step_id.clone());
        }

        if let Some(trace_context) = &self.trace_context {
            event = event.with_trace_context(trace_context.clone());
        }

        event
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryMetricKind {
    Counter,
    Gauge,
    Histogram,
}

impl TelemetryMetricKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Counter => "counter",
            Self::Gauge => "gauge",
            Self::Histogram => "histogram",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TelemetryMetric {
    pub metric_id: String,
    pub name: String,
    pub kind: TelemetryMetricKind,
    pub value: f64,
    pub unit: Option<String>,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub observed_at: Option<String>,
    pub labels: Vec<(String, String)>,
    pub redaction_classification: KernelEventRedaction,
}

impl TelemetryMetric {
    pub fn new(
        metric_id: impl Into<String>,
        name: impl Into<String>,
        kind: TelemetryMetricKind,
        value: f64,
    ) -> Self {
        Self {
            metric_id: metric_id.into(),
            name: name.into(),
            kind,
            value,
            unit: None,
            session_id: None,
            task_id: None,
            run_id: None,
            observed_at: None,
            labels: Vec::new(),
            redaction_classification: KernelEventRedaction::Unknown,
        }
    }

    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
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

    pub fn observed_at(mut self, observed_at: impl Into<String>) -> Self {
        self.observed_at = Some(observed_at.into());
        self
    }

    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.push((key.into(), value.into()));
        self
    }

    pub fn with_redaction(mut self, redaction_classification: KernelEventRedaction) -> Self {
        self.redaction_classification = redaction_classification;
        self
    }

    pub fn label_value(&self, key: &str) -> Option<&str> {
        self.labels
            .iter()
            .find(|(label_key, _)| label_key == key)
            .map(|(_, value)| value.as_str())
    }
}

impl Eq for TelemetryMetric {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TelemetryLogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl TelemetryLogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryLogRecord {
    pub log_id: String,
    pub level: TelemetryLogLevel,
    pub message: String,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub step_id: Option<String>,
    pub created_at: Option<String>,
    pub trace_context: Option<TraceContext>,
    pub fields: Vec<(String, String)>,
    pub redaction_classification: KernelEventRedaction,
}

impl TelemetryLogRecord {
    pub fn new(
        log_id: impl Into<String>,
        level: TelemetryLogLevel,
        message: impl Into<String>,
    ) -> Self {
        Self {
            log_id: log_id.into(),
            level,
            message: message.into(),
            session_id: None,
            task_id: None,
            run_id: None,
            step_id: None,
            created_at: None,
            trace_context: None,
            fields: Vec::new(),
            redaction_classification: KernelEventRedaction::Unknown,
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

    pub fn created_at(mut self, created_at: impl Into<String>) -> Self {
        self.created_at = Some(created_at.into());
        self
    }

    pub fn with_trace_context(mut self, trace_context: TraceContext) -> Self {
        self.trace_context = Some(trace_context);
        self
    }

    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.push((key.into(), value.into()));
        self
    }

    pub fn with_redaction(mut self, redaction_classification: KernelEventRedaction) -> Self {
        self.redaction_classification = redaction_classification;
        self
    }

    pub fn field_value(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(field_key, _)| field_key == key)
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetrySpanStatus {
    Unset,
    Ok,
    Error,
}

impl TelemetrySpanStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unset => "unset",
            Self::Ok => "ok",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetrySpan {
    pub span_id: String,
    pub name: String,
    pub trace_context: Option<TraceContext>,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub status: TelemetrySpanStatus,
    pub attributes: Vec<(String, String)>,
    pub redaction_classification: KernelEventRedaction,
}

impl TelemetrySpan {
    pub fn new(span_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            span_id: span_id.into(),
            name: name.into(),
            trace_context: None,
            started_at: None,
            ended_at: None,
            duration_ms: None,
            status: TelemetrySpanStatus::Unset,
            attributes: Vec::new(),
            redaction_classification: KernelEventRedaction::Unknown,
        }
    }

    pub fn with_trace_context(mut self, trace_context: TraceContext) -> Self {
        self.trace_context = Some(trace_context);
        self
    }

    pub fn started_at(mut self, started_at: impl Into<String>) -> Self {
        self.started_at = Some(started_at.into());
        self
    }

    pub fn ended_at(mut self, ended_at: impl Into<String>) -> Self {
        self.ended_at = Some(ended_at.into());
        self
    }

    pub fn with_duration_ms(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    pub fn with_status(mut self, status: TelemetrySpanStatus) -> Self {
        self.status = status;
        self
    }

    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.push((key.into(), value.into()));
        self
    }

    pub fn with_redaction(mut self, redaction_classification: KernelEventRedaction) -> Self {
        self.redaction_classification = redaction_classification;
        self
    }

    pub fn attribute_value(&self, key: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(attribute_key, _)| attribute_key == key)
            .map(|(_, value)| value.as_str())
    }
}

pub trait TelemetryProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.telemetry.unspecified",
            "telemetry",
            "telemetry-provider",
            "0.0.0",
            vec!["telemetry.record".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth;

    fn record_event(&mut self, event: KernelEvent) -> KernelResult<()>;

    fn record_metric(&mut self, metric: TelemetryMetric) -> KernelResult<()>;

    fn record_log(&mut self, log: TelemetryLogRecord) -> KernelResult<()>;

    fn record_audit(&mut self, audit: AuditRecord) -> KernelResult<()>;

    fn start_span(&mut self, span: TelemetrySpan) -> KernelResult<()>;

    fn finish_span(&mut self, span: TelemetrySpan) -> KernelResult<()>;
}
