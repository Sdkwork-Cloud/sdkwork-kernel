import { KernelPanel, MetricStrip, StatusBadge } from '@sdkwork/kernel-ui-commons';
import { useTerminalSummary } from '../hooks/useTerminalSummary';
import { terminalStatusTone } from '../service/terminal-ui.service';
import type { TerminalKernelPanelProps } from '../types/terminal-ui.types';

export function TerminalKernelPanel({ commands, output }: TerminalKernelPanelProps) {
  const summary = useTerminalSummary(commands);

  return (
    <KernelPanel title="Terminal" eyebrow={`${commands.length} commands`}>
      <MetricStrip
        items={[
          { label: 'running', value: summary.runningCount, tone: summary.runningCount ? 'warn' : 'good' },
          { label: 'failed', value: summary.failedCount, tone: summary.failedCount ? 'bad' : 'good' },
          { label: 'latest', value: summary.latestCommandLabel },
          { label: 'chunks', value: output.length }
        ]}
      />
      <div className="kernel-grid">
        <section>
          <h3>Commands</h3>
          {commands.map((command) => (
            <div className="list-row" key={command.commandId}>
              <span>{command.command}</span>
              <StatusBadge tone={terminalStatusTone(command.status)}>{command.status}</StatusBadge>
            </div>
          ))}
        </section>
        <section>
          <h3>Output</h3>
          {output.map((chunk) => (
            <div className="terminal-output-row" key={`${chunk.commandId}.${chunk.sequence}`}>
              <span className="event-stream__sequence">#{chunk.sequence}</span>
              <span>{chunk.channel}</span>
              <span>{chunk.content}</span>
            </div>
          ))}
        </section>
      </div>
    </KernelPanel>
  );
}
