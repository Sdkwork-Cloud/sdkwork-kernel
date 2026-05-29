import { useMemo } from 'react';
import type { CodeWorkspaceView } from '@sdkwork/kernel-ui-types';
import { summarizeWorkspace } from '../service/workspace-ui.service';

export function useWorkspaceSummary(workspace: CodeWorkspaceView) {
  return useMemo(() => summarizeWorkspace(workspace), [workspace]);
}
