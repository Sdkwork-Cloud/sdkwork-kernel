import { useMemo } from 'react';
import type { TerminalCommandView } from '@sdkwork/kernel-ui-types';
import { summarizeTerminal } from '../service/terminal-ui.service';

export function useTerminalSummary(commands: TerminalCommandView[]) {
  return useMemo(() => summarizeTerminal(commands), [commands]);
}
