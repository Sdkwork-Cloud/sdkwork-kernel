import type { StatusTone } from '@sdkwork/kernel-ui-commons';
import type { KernelRuntimeSnapshot } from '@sdkwork/kernel-ui-types';

export interface AgentRuntimePanelProps {
  runtime: KernelRuntimeSnapshot;
}

export interface AgentRuntimeSummary {
  stateTone: StatusTone;
  capabilityCount: number;
  missingRequiredCount: number;
}
