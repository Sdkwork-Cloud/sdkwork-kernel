import type { StatusTone } from '@sdkwork/kernel-ui-commons';
import type { PermissionDecisionValue, PermissionRequestView } from '@sdkwork/kernel-ui-types';

export interface PermissionQueueProps {
  permissions: PermissionRequestView[];
  onDecision(permissionRequestId: string, decision: PermissionDecisionValue): void;
}

export interface PermissionSummary {
  totalCount: number;
  pendingCount: number;
  destructiveCount: number;
}

export type PermissionTone = StatusTone;
