import type { CodeWorkspaceView } from '@sdkwork/kernel-ui-types';

export interface WorkspaceSummary {
  trustTone: 'good' | 'warn' | 'bad' | 'neutral';
  changeCount: number;
  changedFilesLabel: string;
}

export interface WorkspaceKernelPanelProps {
  workspace: CodeWorkspaceView;
}
