import { Check, X } from 'lucide-react';
import { KernelPanel, MetricStrip, StatusBadge } from '@sdkwork/kernel-ui-commons';
import { usePermissionSummary } from '../hooks/usePermissionSummary';
import { permissionStatusTone } from '../service/permission-ui.service';
import type { PermissionQueueProps } from '../types/permission-ui.types';

export function PermissionQueue({ permissions, onDecision }: PermissionQueueProps) {
  const summary = usePermissionSummary(permissions);

  return (
    <KernelPanel title="Permissions" eyebrow={`${summary.totalCount} requests`}>
      <MetricStrip
        items={[
          { label: 'pending', value: summary.pendingCount, tone: summary.pendingCount ? 'warn' : 'good' },
          { label: 'destructive', value: summary.destructiveCount, tone: summary.destructiveCount ? 'bad' : 'good' },
          { label: 'total', value: summary.totalCount },
          { label: 'policy', value: 'kernel' }
        ]}
      />
      <div className="permission-list">
        {permissions.map((permission) => (
          <div className="permission-list__row" key={permission.permissionRequestId}>
            <div>
              <strong>{permission.category}</strong>
              <p>{permission.resource}</p>
            </div>
            <StatusBadge tone={permissionStatusTone(permission)}>{permission.sideEffectLevel}</StatusBadge>
            <div className="permission-list__actions">
              <button
                aria-label={`Allow ${permission.permissionRequestId}`}
                className="icon-button icon-button--allow"
                onClick={() => onDecision(permission.permissionRequestId, 'allow')}
                type="button"
              >
                <Check size={16} />
              </button>
              <button
                aria-label={`Deny ${permission.permissionRequestId}`}
                className="icon-button icon-button--deny"
                onClick={() => onDecision(permission.permissionRequestId, 'deny')}
                type="button"
              >
                <X size={16} />
              </button>
            </div>
          </div>
        ))}
      </div>
    </KernelPanel>
  );
}
