import type { CodeWorkspaceView } from '@sdkwork/kernel-ui-types';
import type { WorkspaceSummary } from '../types/workspace-ui.types';

export function summarizeWorkspace(workspace: CodeWorkspaceView): WorkspaceSummary {
  return {
    trustTone: workspace.dirty ? 'warn' : 'good',
    changeCount: workspace.changedFiles.length,
    changedFilesLabel: workspace.changedFiles.length === 1 ? '1 file' : `${workspace.changedFiles.length} files`
  };
}
