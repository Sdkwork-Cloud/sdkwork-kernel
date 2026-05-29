import type {
  KernelUiClient,
  KernelUiSnapshot,
  PermissionDecisionValue,
  PermissionRequestView
} from '@sdkwork/kernel-ui-types';
import { kernelUiMockSnapshot } from './kernel-ui.mock';

export function createMockKernelUiClient(snapshot = kernelUiMockSnapshot): KernelUiClient {
  let currentSnapshot: KernelUiSnapshot = cloneSnapshot(snapshot);

  return {
    async loadSnapshot() {
      return cloneSnapshot(currentSnapshot);
    },
    async decidePermission(permissionRequestId: string, decision: PermissionDecisionValue) {
      let updatedPermission: PermissionRequestView | undefined;
      currentSnapshot = {
        ...currentSnapshot,
        permissions: currentSnapshot.permissions.map((permission) => {
          if (permission.permissionRequestId !== permissionRequestId) {
            return permission;
          }

          updatedPermission = {
            ...permission,
            status: decision
          };
          return updatedPermission;
        })
      };

      if (!updatedPermission) {
        throw new Error(`permission request not found: ${permissionRequestId}`);
      }

      return updatedPermission;
    }
  };
}

function cloneSnapshot(snapshot: KernelUiSnapshot): KernelUiSnapshot {
  return JSON.parse(JSON.stringify(snapshot)) as KernelUiSnapshot;
}
