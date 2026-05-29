import { useMemo } from 'react';
import type { PermissionRequestView } from '@sdkwork/kernel-ui-types';
import { summarizePermissions } from '../service/permission-ui.service';

export function usePermissionSummary(permissions: PermissionRequestView[]) {
  return useMemo(() => summarizePermissions(permissions), [permissions]);
}
