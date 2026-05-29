use crate::{KernelError, KernelResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KernelEventSeverity {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
}

impl TraceContext {
    pub fn new(trace_id: impl Into<String>, span_id: impl Into<String>) -> Self {
        Self {
            trace_id: trace_id.into(),
            span_id: span_id.into(),
            parent_span_id: None,
        }
    }

    pub fn with_parent_span(mut self, parent_span_id: impl Into<String>) -> Self {
        self.parent_span_id = Some(parent_span_id.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelEventSource {
    Runtime,
    Manifest,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelEventRedaction {
    Public,
    Internal,
    TenantSensitive,
    PersonalData,
    Secret,
    Regulated,
    Unknown,
}

impl KernelEventRedaction {
    pub fn is_sensitive(&self) -> bool {
        !matches!(self, Self::Public)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelEvent {
    pub event_id: String,
    pub event_type: String,
    pub event_version: String,
    pub occurred_at: Option<String>,
    pub source: KernelEventSource,
    pub severity: KernelEventSeverity,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub step_id: Option<String>,
    pub payload: String,
    pub trace_context: Option<TraceContext>,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
    pub redaction_classification: KernelEventRedaction,
    pub payload_schema: Option<String>,
    pub replay: bool,
}

impl KernelEvent {
    pub fn new(
        event_id: impl Into<String>,
        event_type: impl Into<String>,
        severity: KernelEventSeverity,
        payload: impl Into<String>,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            event_type: event_type.into(),
            event_version: "0.1.0".to_string(),
            occurred_at: None,
            source: KernelEventSource::Unknown,
            severity,
            session_id: None,
            task_id: None,
            run_id: None,
            step_id: None,
            payload: payload.into(),
            trace_context: None,
            correlation_id: None,
            causation_id: None,
            redaction_classification: KernelEventRedaction::Unknown,
            payload_schema: None,
            replay: false,
        }
    }

    pub fn with_trace(mut self, trace_id: impl Into<String>, span_id: impl Into<String>) -> Self {
        self.trace_context = Some(TraceContext::new(trace_id, span_id));
        self
    }

    pub fn with_trace_context(mut self, trace_context: TraceContext) -> Self {
        self.trace_context = Some(trace_context);
        self
    }

    pub fn occurred_at(mut self, occurred_at: impl Into<String>) -> Self {
        self.occurred_at = Some(occurred_at.into());
        self
    }

    pub fn from_source(mut self, source: KernelEventSource) -> Self {
        self.source = source;
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

    pub fn with_correlation(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }

    pub fn caused_by(mut self, causation_id: impl Into<String>) -> Self {
        self.causation_id = Some(causation_id.into());
        self
    }

    pub fn with_redaction(mut self, redaction_classification: KernelEventRedaction) -> Self {
        self.redaction_classification = redaction_classification;
        self
    }

    pub fn with_payload_schema(mut self, payload_schema: impl Into<String>) -> Self {
        self.payload_schema = Some(payload_schema.into());
        self
    }

    pub fn mark_replay(mut self) -> Self {
        self.replay = true;
        self
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EventRecorder {
    events: Vec<KernelEvent>,
}

impl EventRecorder {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn record(&mut self, event: KernelEvent) {
        self.events.push(event);
    }

    pub fn events(&self) -> &[KernelEvent] {
        &self.events
    }

    pub fn by_session(&self, session_id: &str) -> Vec<&KernelEvent> {
        self.events
            .iter()
            .filter(|event| event.session_id.as_deref() == Some(session_id))
            .collect()
    }

    pub fn by_task(&self, task_id: &str) -> Vec<&KernelEvent> {
        self.events
            .iter()
            .filter(|event| event.task_id.as_deref() == Some(task_id))
            .collect()
    }

    pub fn by_min_severity(&self, severity: KernelEventSeverity) -> Vec<&KernelEvent> {
        self.events
            .iter()
            .filter(|event| event.severity >= severity)
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventStreamStatus {
    Open,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventStreamCursor {
    pub last_sequence: Option<u64>,
}

impl EventStreamCursor {
    pub fn from_start() -> Self {
        Self {
            last_sequence: None,
        }
    }

    pub fn after(last_sequence: u64) -> Self {
        Self {
            last_sequence: Some(last_sequence),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventStreamFilter {
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub step_id: Option<String>,
    pub source: Option<KernelEventSource>,
    pub min_severity: Option<KernelEventSeverity>,
    pub event_family: Option<String>,
}

impl EventStreamFilter {
    pub fn new() -> Self {
        Self {
            session_id: None,
            task_id: None,
            run_id: None,
            step_id: None,
            source: None,
            min_severity: None,
            event_family: None,
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

    pub fn from_source(mut self, source: KernelEventSource) -> Self {
        self.source = Some(source);
        self
    }

    pub fn with_min_severity(mut self, min_severity: KernelEventSeverity) -> Self {
        self.min_severity = Some(min_severity);
        self
    }

    pub fn with_event_family(mut self, event_family: impl Into<String>) -> Self {
        self.event_family = Some(event_family.into());
        self
    }

    pub fn matches(&self, event: &KernelEvent) -> bool {
        if let Some(session_id) = &self.session_id {
            if event.session_id.as_deref() != Some(session_id.as_str()) {
                return false;
            }
        }

        if let Some(task_id) = &self.task_id {
            if event.task_id.as_deref() != Some(task_id.as_str()) {
                return false;
            }
        }

        if let Some(run_id) = &self.run_id {
            if event.run_id.as_deref() != Some(run_id.as_str()) {
                return false;
            }
        }

        if let Some(step_id) = &self.step_id {
            if event.step_id.as_deref() != Some(step_id.as_str()) {
                return false;
            }
        }

        if let Some(source) = self.source {
            if event.source != source {
                return false;
            }
        }

        if let Some(min_severity) = self.min_severity {
            if event.severity < min_severity {
                return false;
            }
        }

        if let Some(event_family) = &self.event_family {
            if !event.event_type.starts_with(event_family) {
                return false;
            }
        }

        true
    }
}

impl Default for EventStreamFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventStreamItem {
    pub sequence: u64,
    pub event: KernelEvent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventSubscription {
    pub subscription_id: String,
    pub filter: EventStreamFilter,
    pub cursor: EventStreamCursor,
    pub batch_limit: usize,
}

impl EventSubscription {
    pub fn new(
        subscription_id: impl Into<String>,
        filter: EventStreamFilter,
        cursor: EventStreamCursor,
        batch_limit: usize,
    ) -> Self {
        Self {
            subscription_id: subscription_id.into(),
            filter,
            cursor,
            batch_limit,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventStreamBatch {
    pub stream_id: String,
    pub subscription_id: String,
    pub events: Vec<EventStreamItem>,
    pub next_cursor: EventStreamCursor,
    pub has_more: bool,
    pub status: EventStreamStatus,
    pub completion_event_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventStream {
    stream_id: String,
    events: Vec<EventStreamItem>,
    next_sequence: u64,
    status: EventStreamStatus,
    completion_event_id: Option<String>,
    failure: Option<KernelError>,
}

impl EventStream {
    pub fn new(stream_id: impl Into<String>) -> Self {
        Self {
            stream_id: stream_id.into(),
            events: Vec::new(),
            next_sequence: 1,
            status: EventStreamStatus::Open,
            completion_event_id: None,
            failure: None,
        }
    }

    pub fn from_recorder(stream_id: impl Into<String>, recorder: &EventRecorder) -> Self {
        let mut stream = Self::new(stream_id);
        for event in recorder.events() {
            stream.publish(event.clone());
        }
        stream
    }

    pub fn publish(&mut self, event: KernelEvent) -> u64 {
        let sequence = self.next_sequence;
        self.next_sequence += 1;
        self.events.push(EventStreamItem { sequence, event });
        sequence
    }

    pub fn mark_replay(mut self) -> Self {
        self.events = self
            .events
            .into_iter()
            .map(|item| EventStreamItem {
                sequence: item.sequence,
                event: item.event.mark_replay(),
            })
            .collect();
        self
    }

    pub fn complete(&mut self) {
        self.status = EventStreamStatus::Completed;
        self.completion_event_id = Some(format!("event.{}.completed", self.stream_id));
    }

    pub fn fail(&mut self, error: KernelError) {
        self.status = EventStreamStatus::Failed;
        self.failure = Some(error);
    }

    pub fn status(&self) -> EventStreamStatus {
        self.status
    }

    pub fn subscribe(
        &self,
        subscription_id: impl Into<String>,
        filter: EventStreamFilter,
        cursor: EventStreamCursor,
        batch_limit: usize,
    ) -> KernelResult<EventStreamBatch> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }

        let subscription_id = subscription_id.into();
        let limit = batch_limit.max(1);
        let after_sequence = cursor.last_sequence.unwrap_or(0);
        let matching: Vec<EventStreamItem> = self
            .events
            .iter()
            .filter(|item| item.sequence > after_sequence && filter.matches(&item.event))
            .cloned()
            .collect();

        let has_more = matching.len() > limit;
        let events: Vec<EventStreamItem> = matching.into_iter().take(limit).collect();
        let next_cursor = events
            .last()
            .map(|item| EventStreamCursor::after(item.sequence))
            .unwrap_or(cursor);

        Ok(EventStreamBatch {
            stream_id: self.stream_id.clone(),
            subscription_id,
            events,
            next_cursor,
            has_more,
            status: self.status,
            completion_event_id: self.completion_event_id.clone(),
        })
    }
}
