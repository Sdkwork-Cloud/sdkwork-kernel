import type { KernelUiClient } from '@sdkwork/kernel-ui-types';
import type { KernelUiRuntime } from '../types/kernel-ui-runtime.types';

export function createKernelUiRuntime(client: KernelUiClient): KernelUiRuntime {
  return {
    client,
    loadSnapshot: () => client.loadSnapshot()
  };
}
