import type { KernelEventView } from '@sdkwork/kernel-ui-types';
import type { EventSeverityTone, TelemetrySummary } from '../types/telemetry-ui.types';

export function summarizeTelemetry(events: KernelEventView[]): TelemetrySummary {
  return {
    eventCount: events.length,
    errorCount: events.filter((event) => event.severity === 'error').length,
    warningCount: events.filter((event) => event.severity === 'warn').length
  };
}

export function eventSeverityTone(event: KernelEventView): EventSeverityTone {
  if (event.severity === 'error') {
    return 'bad';
  }

  if (event.severity === 'warn') {
    return 'warn';
  }

  return 'neutral';
}
