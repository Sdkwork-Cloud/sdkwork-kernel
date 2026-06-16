import {
  createBrowserStorageKernelUiAuthProvider,
  createKernelUiClient,
  createMockKernelUiClient,
  createStaticKernelUiAuthProvider,
  readBrowserKernelUiAuthSession
} from '@sdkwork/kernel-ui-services';
import type { KernelUiAuthProvider, KernelUiClient } from '@sdkwork/kernel-ui-types';

function hasStaticEnvAuth(): boolean {
  return Boolean(
    import.meta.env.VITE_KERNEL_ACCESS_TOKEN ||
      import.meta.env.VITE_KERNEL_TENANT_ID ||
      import.meta.env.VITE_KERNEL_USER_ID
  );
}

export function kernelApiUrl(): string | undefined {
  return import.meta.env.VITE_KERNEL_API_URL as string | undefined;
}

export function needsKernelUiSessionGate(): boolean {
  const apiUrl = kernelApiUrl();
  if (!apiUrl) {
    return false;
  }
  if (hasStaticEnvAuth()) {
    return false;
  }
  return readBrowserKernelUiAuthSession() === null;
}

function createAuthProvider(): KernelUiAuthProvider | undefined {
  const accessToken = import.meta.env.VITE_KERNEL_ACCESS_TOKEN;
  const tenantId = import.meta.env.VITE_KERNEL_TENANT_ID;
  const userId = import.meta.env.VITE_KERNEL_USER_ID;
  if (accessToken || tenantId || userId) {
    return createStaticKernelUiAuthProvider({
      accessToken: accessToken ?? '',
      tenantId,
      userId
    });
  }
  return createBrowserStorageKernelUiAuthProvider();
}

export function createKernelUiShellClient(): KernelUiClient {
  const apiUrl = kernelApiUrl();
  if (apiUrl) {
    return createKernelUiClient({ baseUrl: apiUrl, auth: createAuthProvider() });
  }
  return createMockKernelUiClient();
}
