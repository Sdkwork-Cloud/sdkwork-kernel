import { useMemo } from 'react';
import type { KernelRuntimeSnapshot } from '@sdkwork/kernel-ui-types';
import { summarizeAgentRuntime } from '../service/agent-runtime-ui.service';

export function useAgentRuntimeSummary(runtime: KernelRuntimeSnapshot) {
  return useMemo(() => summarizeAgentRuntime(runtime), [runtime]);
}
