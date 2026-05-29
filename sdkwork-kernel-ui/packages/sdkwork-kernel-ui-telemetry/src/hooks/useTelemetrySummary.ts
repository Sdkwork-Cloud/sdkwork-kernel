import { useMemo } from 'react';
import type { KernelEventView } from '@sdkwork/kernel-ui-types';
import { summarizeTelemetry } from '../service/telemetry-ui.service';

export function useTelemetrySummary(events: KernelEventView[]) {
  return useMemo(() => summarizeTelemetry(events), [events]);
}
