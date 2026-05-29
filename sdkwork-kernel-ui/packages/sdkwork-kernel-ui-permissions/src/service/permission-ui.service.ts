import type { PermissionRequestView } from '@sdkwork/kernel-ui-types';
import type { PermissionSummary, PermissionTone } from '../types/permission-ui.types';

export function summarizePermissions(permissions: PermissionRequestView[]): PermissionSummary {
  return {
    totalCount: permissions.length,
    pendingCount: permissions.filter((permission) => permission.status === 'pending').length,
    destructiveCount: permissions.filter((permission) => permission.sideEffectLevel === 'destructive').length
  };
}

export function permissionStatusTone(permission: PermissionRequestView): PermissionTone {
  if (permission.status === 'deny' || permission.sideEffectLevel === 'destructive') {
    return 'bad';
  }

  if (permission.status === 'pending' || permission.status === 'needs_approval') {
    return 'warn';
  }

  return 'neutral';
}
