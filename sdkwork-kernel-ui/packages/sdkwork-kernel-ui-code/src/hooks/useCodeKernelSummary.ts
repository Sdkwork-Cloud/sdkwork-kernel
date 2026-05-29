import { useMemo } from 'react';
import { summarizeCodeKernel } from '../service/code-kernel-ui.service';
import type { CodeKernelPanelProps } from '../types/code-kernel-ui.types';

export function useCodeKernelSummary(props: CodeKernelPanelProps) {
  return useMemo(() => summarizeCodeKernel(props), [props]);
}
