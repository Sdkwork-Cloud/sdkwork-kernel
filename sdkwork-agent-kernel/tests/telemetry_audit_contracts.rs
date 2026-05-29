use sdkwork_agent_kernel::{
    AuditRecord, KernelEvent, KernelEventRedaction, KernelEventSeverity, KernelResult,
    PolicyCategory, PolicyDecision, PolicyDecisionValue, PolicyRequest, ProviderHealth,
    TelemetryLogLevel, TelemetryLogRecord, TelemetryMetric, TelemetryMetricKind, TelemetryProvider,
    TelemetrySpan, TelemetrySpanStatus, TraceContext,
};

#[test]
fn audit_record_preserves_actor_subject_action_resource_policy_trace_and_redaction() {
    let record = AuditRecord::new("audit.1", "agent.policy.denied", "policy.deny")
        .with_actor("user.1")
        .with_subject("tenant.1")
        .with_action("deny")
        .with_resource("tool.shell.run")
        .with_policy_decision("decision.1")
        .for_session("session.1")
        .for_task("task.1")
        .for_run("run.1")
        .for_step("step.1")
        .created_at("2026-05-28T10:00:00Z")
        .with_trace_context(TraceContext::new("trace.1", "span.audit"))
        .with_redaction(KernelEventRedaction::TenantSensitive)
        .with_metadata("sdkwork.audit.reason_code", "policy_denied");

    assert_eq!(record.audit_id, "audit.1");
    assert_eq!(record.event_type, "agent.policy.denied");
    assert_eq!(record.actor.as_deref(), Some("user.1"));
    assert_eq!(record.subject.as_deref(), Some("tenant.1"));
    assert_eq!(record.action, "deny");
    assert_eq!(record.resource, "tool.shell.run");
    assert_eq!(record.policy_decision_id.as_deref(), Some("decision.1"));
    assert_eq!(record.session_id.as_deref(), Some("session.1"));
    assert_eq!(record.task_id.as_deref(), Some("task.1"));
    assert_eq!(record.run_id.as_deref(), Some("run.1"));
    assert_eq!(record.step_id.as_deref(), Some("step.1"));
    assert_eq!(record.created_at.as_deref(), Some("2026-05-28T10:00:00Z"));
    assert_eq!(record.trace_context.as_ref().unwrap().trace_id, "trace.1");
    assert_eq!(
        record.redaction_classification,
        KernelEventRedaction::TenantSensitive
    );
    assert_eq!(
        record.metadata_value("sdkwork.audit.reason_code"),
        Some("policy_denied")
    );
}

#[test]
fn audit_record_can_be_derived_from_policy_decision_and_request() {
    let request = PolicyRequest::new("policy-request.1", "host.process.execute", "tool.shell.run")
        .with_category(PolicyCategory::HostProcessExecute)
        .with_action("execute")
        .with_session("session.1")
        .with_task("task.1")
        .with_run("run.1")
        .with_redaction(KernelEventRedaction::Internal);
    let decision = PolicyDecision::deny(
        "decision.1",
        "policy-request.1",
        "provider.policy.fake",
        "command_denied",
    )
    .with_safe_reason("Command is not allowed")
    .require_audit();

    let record = AuditRecord::from_policy_decision("audit.1", &decision, &request);

    assert_eq!(record.event_type, "agent.audit.policy_decision");
    assert_eq!(record.action, PolicyDecisionValue::Deny.as_str());
    assert_eq!(record.resource, "tool.shell.run");
    assert_eq!(record.policy_decision_id.as_deref(), Some("decision.1"));
    assert_eq!(record.session_id.as_deref(), Some("session.1"));
    assert_eq!(record.task_id.as_deref(), Some("task.1"));
    assert_eq!(record.run_id.as_deref(), Some("run.1"));
    assert_eq!(
        record.redaction_classification,
        KernelEventRedaction::Internal
    );
    assert_eq!(
        record.metadata_value("sdkwork.policy.reason_code"),
        Some("command_denied")
    );
    assert_eq!(
        record.metadata_value("sdkwork.policy.audit_required"),
        Some("true")
    );
}

#[test]
fn audit_record_maps_to_kernel_event_without_losing_trace_or_policy_context() {
    let record = AuditRecord::new("audit.1", "agent.policy.allowed", "allow")
        .with_resource("tool.echo")
        .with_policy_decision("decision.1")
        .for_session("session.1")
        .for_task("task.1")
        .with_trace_context(TraceContext::new("trace.1", "span.audit"))
        .with_redaction(KernelEventRedaction::Internal);

    let event = record.to_event("event.audit.1");

    assert_eq!(event.event_type, "agent.audit.recorded");
    assert_eq!(event.severity, KernelEventSeverity::Info);
    assert_eq!(event.session_id.as_deref(), Some("session.1"));
    assert_eq!(event.task_id.as_deref(), Some("task.1"));
    assert_eq!(event.trace_context.as_ref().unwrap().span_id, "span.audit");
    assert_eq!(
        event.redaction_classification,
        KernelEventRedaction::Internal
    );
    assert_eq!(
        event.payload_schema.as_deref(),
        Some("sdkwork.agent.audit.record.v1")
    );
    assert!(event.payload.contains("audit_id=audit.1"));
    assert!(event.payload.contains("decision_id=decision.1"));
}

#[test]
fn telemetry_metric_preserves_kind_unit_context_and_labels_without_payload_leakage() {
    let metric = TelemetryMetric::new(
        "metric.1",
        "agent.model.tokens",
        TelemetryMetricKind::Counter,
        42.0,
    )
    .with_unit("tokens")
    .for_session("session.1")
    .for_task("task.1")
    .for_run("run.1")
    .observed_at("2026-05-28T10:00:00Z")
    .with_label("provider_id", "provider.model.fake")
    .with_redaction(KernelEventRedaction::Internal);

    assert_eq!(metric.metric_id, "metric.1");
    assert_eq!(metric.name, "agent.model.tokens");
    assert_eq!(metric.kind, TelemetryMetricKind::Counter);
    assert_eq!(metric.value, 42.0);
    assert_eq!(metric.unit.as_deref(), Some("tokens"));
    assert_eq!(metric.session_id.as_deref(), Some("session.1"));
    assert_eq!(metric.task_id.as_deref(), Some("task.1"));
    assert_eq!(metric.run_id.as_deref(), Some("run.1"));
    assert_eq!(
        metric.label_value("provider_id"),
        Some("provider.model.fake")
    );
    assert_eq!(
        metric.redaction_classification,
        KernelEventRedaction::Internal
    );
}

#[test]
fn telemetry_log_record_preserves_level_context_trace_and_safe_message() {
    let log = TelemetryLogRecord::new("log.1", TelemetryLogLevel::Warn, "tool invocation denied")
        .for_session("session.1")
        .for_task("task.1")
        .for_run("run.1")
        .for_step("step.1")
        .created_at("2026-05-28T10:00:00Z")
        .with_trace_context(TraceContext::new("trace.1", "span.log"))
        .with_field("reason_code", "policy_denied")
        .with_redaction(KernelEventRedaction::Internal);

    assert_eq!(log.log_id, "log.1");
    assert_eq!(log.level, TelemetryLogLevel::Warn);
    assert_eq!(log.message, "tool invocation denied");
    assert_eq!(log.session_id.as_deref(), Some("session.1"));
    assert_eq!(log.task_id.as_deref(), Some("task.1"));
    assert_eq!(log.run_id.as_deref(), Some("run.1"));
    assert_eq!(log.step_id.as_deref(), Some("step.1"));
    assert_eq!(log.trace_context.as_ref().unwrap().span_id, "span.log");
    assert_eq!(log.field_value("reason_code"), Some("policy_denied"));
    assert_eq!(log.redaction_classification, KernelEventRedaction::Internal);
}

#[test]
fn telemetry_span_preserves_parent_trace_status_timing_and_attributes() {
    let span = TelemetrySpan::new("span.model.1", "agent.model.invoke")
        .with_trace_context(TraceContext::new("trace.1", "span.model.1").with_parent_span("root"))
        .started_at("2026-05-28T10:00:00Z")
        .ended_at("2026-05-28T10:00:01Z")
        .with_duration_ms(1000)
        .with_status(TelemetrySpanStatus::Error)
        .with_attribute("model_request_id", "model-request.1")
        .with_redaction(KernelEventRedaction::Internal);

    assert_eq!(span.span_id, "span.model.1");
    assert_eq!(span.name, "agent.model.invoke");
    assert_eq!(
        span.trace_context
            .as_ref()
            .unwrap()
            .parent_span_id
            .as_deref(),
        Some("root")
    );
    assert_eq!(span.status, TelemetrySpanStatus::Error);
    assert_eq!(span.duration_ms, Some(1000));
    assert_eq!(
        span.attribute_value("model_request_id"),
        Some("model-request.1")
    );
    assert_eq!(
        span.redaction_classification,
        KernelEventRedaction::Internal
    );
}

#[test]
fn telemetry_provider_records_events_metrics_logs_audits_and_spans() {
    let mut provider = FakeTelemetryProvider::default();
    let event = KernelEvent::new(
        "event.1",
        "agent.task.started",
        KernelEventSeverity::Info,
        "task_id=task.1",
    );
    let metric = TelemetryMetric::new(
        "metric.1",
        "agent.task.started.count",
        TelemetryMetricKind::Counter,
        1.0,
    );
    let log = TelemetryLogRecord::new("log.1", TelemetryLogLevel::Info, "task started");
    let audit = AuditRecord::new("audit.1", "agent.task.started", "start");
    let span =
        TelemetrySpan::new("span.1", "agent.task.start").with_status(TelemetrySpanStatus::Ok);

    provider
        .record_event(event.clone())
        .expect("event recorded");
    provider
        .record_metric(metric.clone())
        .expect("metric recorded");
    provider.record_log(log.clone()).expect("log recorded");
    provider
        .record_audit(audit.clone())
        .expect("audit recorded");
    provider.start_span(span.clone()).expect("span started");
    provider
        .finish_span(
            TelemetrySpan::new("span.1", "agent.task.start")
                .with_status(TelemetrySpanStatus::Ok)
                .ended_at("2026-05-28T10:00:01Z"),
        )
        .expect("span finished");

    assert_eq!(provider.health(), ProviderHealth::available());
    assert_eq!(provider.events, vec![event]);
    assert_eq!(provider.metrics, vec![metric]);
    assert_eq!(provider.logs, vec![log]);
    assert_eq!(provider.audits, vec![audit]);
    assert_eq!(provider.started_spans, vec![span]);
    assert_eq!(provider.finished_spans[0].status, TelemetrySpanStatus::Ok);
}

#[derive(Default)]
struct FakeTelemetryProvider {
    events: Vec<KernelEvent>,
    metrics: Vec<TelemetryMetric>,
    logs: Vec<TelemetryLogRecord>,
    audits: Vec<AuditRecord>,
    started_spans: Vec<TelemetrySpan>,
    finished_spans: Vec<TelemetrySpan>,
}

impl TelemetryProvider for FakeTelemetryProvider {
    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn record_event(&mut self, event: KernelEvent) -> KernelResult<()> {
        self.events.push(event);
        Ok(())
    }

    fn record_metric(&mut self, metric: TelemetryMetric) -> KernelResult<()> {
        self.metrics.push(metric);
        Ok(())
    }

    fn record_log(&mut self, log: TelemetryLogRecord) -> KernelResult<()> {
        self.logs.push(log);
        Ok(())
    }

    fn record_audit(&mut self, audit: AuditRecord) -> KernelResult<()> {
        self.audits.push(audit);
        Ok(())
    }

    fn start_span(&mut self, span: TelemetrySpan) -> KernelResult<()> {
        self.started_spans.push(span);
        Ok(())
    }

    fn finish_span(&mut self, span: TelemetrySpan) -> KernelResult<()> {
        self.finished_spans.push(span);
        Ok(())
    }
}
