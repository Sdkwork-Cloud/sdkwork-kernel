import type { TerminalCommandView } from '@sdkwork/kernel-ui-types';
import type { TerminalSummary } from '../types/terminal-ui.types';

export function summarizeTerminal(commands: TerminalCommandView[]): TerminalSummary {
  const latestCommand = commands.at(-1);

  return {
    runningCount: commands.filter((command) => command.status === 'running').length,
    failedCount: commands.filter((command) => command.status === 'failed' || command.status === 'timed_out').length,
    latestCommandLabel: latestCommand ? latestCommand.command : 'none'
  };
}

export function terminalStatusTone(status: TerminalCommandView['status']) {
  if (status === 'passed') {
    return 'good';
  }

  if (status === 'failed' || status === 'timed_out') {
    return 'bad';
  }

  if (status === 'running' || status === 'queued') {
    return 'warn';
  }

  return 'neutral';
}
