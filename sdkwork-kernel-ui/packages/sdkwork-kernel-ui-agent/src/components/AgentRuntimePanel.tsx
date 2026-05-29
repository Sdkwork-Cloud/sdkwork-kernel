import { KernelPanel, MetricStrip, StatusBadge } from '@sdkwork/kernel-ui-commons';
import { useAgentRuntimeSummary } from '../hooks/useAgentRuntimeSummary';
import type { AgentRuntimePanelProps } from '../types/agent-runtime-ui.types';

export function AgentRuntimePanel({ runtime }: AgentRuntimePanelProps) {
  const summary = useAgentRuntimeSummary(runtime);

  return (
    <KernelPanel
      title="Agent Runtime"
      eyebrow={runtime.runtimeId}
      actions={<StatusBadge tone={summary.stateTone}>{runtime.state}</StatusBadge>}
    >
      <MetricStrip
        items={[
          { label: 'agent', value: runtime.agentId },
          { label: 'kernel', value: runtime.kernelVersion },
          { label: 'capabilities', value: summary.capabilityCount },
          {
            label: 'missing',
            value: summary.missingRequiredCount,
            tone: summary.missingRequiredCount ? 'bad' : 'good'
          }
        ]}
      />
      <div className="capability-list">
        {runtime.capabilities.map((capability) => (
          <div className="capability-list__row" key={capability.capabilityId}>
            <span>{capability.capabilityId}</span>
            <span>{capability.providerId}</span>
            <StatusBadge tone={capability.status === 'available' ? 'good' : 'warn'}>{capability.status}</StatusBadge>
          </div>
        ))}
      </div>
    </KernelPanel>
  );
}
