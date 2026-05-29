import { KernelPanel, MetricStrip, StatusBadge } from '@sdkwork/kernel-ui-commons';
import { useWorkspaceSummary } from '../hooks/useWorkspaceSummary';
import type { WorkspaceKernelPanelProps } from '../types/workspace-ui.types';

export function WorkspaceKernelPanel({ workspace }: WorkspaceKernelPanelProps) {
  const summary = useWorkspaceSummary(workspace);

  return (
    <KernelPanel
      title="Workspace"
      eyebrow={workspace.workspaceId}
      actions={<StatusBadge tone={summary.trustTone}>{workspace.branch}</StatusBadge>}
    >
      <MetricStrip
        items={[
          { label: 'root', value: workspace.root },
          { label: 'state', value: workspace.dirty ? 'dirty' : 'clean', tone: summary.trustTone },
          { label: 'changed files', value: summary.changedFilesLabel, tone: summary.trustTone },
          { label: 'vcs', value: workspace.branch }
        ]}
      />
      <div className="file-change-list">
        {workspace.changedFiles.map((filePath) => (
          <div className="list-row" key={filePath}>
            <span>{filePath}</span>
            <StatusBadge tone="neutral">changed</StatusBadge>
          </div>
        ))}
      </div>
    </KernelPanel>
  );
}
