import type { StatusTone } from '@sdkwork/kernel-ui-commons';
import type { KernelEventView } from '@sdkwork/kernel-ui-types';

export interface TelemetryEventStreamProps {
  events: KernelEventView[];
}

export interface TelemetrySummary {
  eventCount: number;
  errorCount: number;
  warningCount: number;
}

export type EventSeverityTone = StatusTone;
