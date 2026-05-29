import type { KernelRuntimeSnapshot } from '@sdkwork/kernel-ui-types';
import type { AgentRuntimeSummary } from '../types/agent-runtime-ui.types';

export function summarizeAgentRuntime(runtime: KernelRuntimeSnapshot): AgentRuntimeSummary {
  return {
    stateTone: runtime.state === 'ready' ? 'good' : runtime.state === 'degraded' ? 'warn' : 'bad',
    capabilityCount: runtime.capabilities.length,
    missingRequiredCount: runtime.missingRequiredCapabilities.length
  };
}
