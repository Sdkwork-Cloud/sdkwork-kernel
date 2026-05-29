import type { KernelUiClient, KernelUiSnapshot } from '@sdkwork/kernel-ui-types';

export interface KernelUiRuntime {
  client: KernelUiClient;
  loadSnapshot(): Promise<KernelUiSnapshot>;
}
