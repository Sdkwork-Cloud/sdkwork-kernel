import type { TerminalCommandView, TerminalOutputChunkView } from '@sdkwork/kernel-ui-types';

export interface TerminalSummary {
  runningCount: number;
  failedCount: number;
  latestCommandLabel: string;
}

export interface TerminalKernelPanelProps {
  commands: TerminalCommandView[];
  output: TerminalOutputChunkView[];
}
