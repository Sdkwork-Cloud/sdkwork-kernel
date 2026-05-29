import { KernelPanel, MetricStrip, StatusBadge } from '@sdkwork/kernel-ui-commons';
import { useTelemetrySummary } from '../hooks/useTelemetrySummary';
import { eventSeverityTone } from '../service/telemetry-ui.service';
import type { TelemetryEventStreamProps } from '../types/telemetry-ui.types';

export function TelemetryEventStream({ events }: TelemetryEventStreamProps) {
  const summary = useTelemetrySummary(events);

  return (
    <KernelPanel title="Telemetry" eyebrow={`${summary.eventCount} events`}>
      <MetricStrip
        items={[
          { label: 'events', value: summary.eventCount },
          { label: 'warnings', value: summary.warningCount, tone: summary.warningCount ? 'warn' : 'good' },
          { label: 'errors', value: summary.errorCount, tone: summary.errorCount ? 'bad' : 'good' },
          { label: 'stream', value: 'ordered' }
        ]}
      />
      <div className="event-stream">
        {events.map((event) => (
          <div className="event-stream__row" key={event.eventId}>
            <span className="event-stream__sequence">#{event.sequence}</span>
            <span>{event.eventType}</span>
            <span>{event.summary}</span>
            <StatusBadge tone={eventSeverityTone(event)}>{event.severity}</StatusBadge>
          </div>
        ))}
      </div>
    </KernelPanel>
  );
}
