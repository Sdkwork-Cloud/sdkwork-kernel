import type { KernelUiAuthProvider, KernelUiAuthSession } from '@sdkwork/kernel-ui-types';

const KERNEL_UI_AUTH_SESSION_KEY = 'sdkwork.kernel-ui.auth.session';

export function createStaticKernelUiAuthProvider(
  session: KernelUiAuthSession | null
): KernelUiAuthProvider {
  return {
    async getSession() {
      return session;
    }
  };
}

export function createBrowserStorageKernelUiAuthProvider(
  storageKey = KERNEL_UI_AUTH_SESSION_KEY
): KernelUiAuthProvider {
  return {
    async getSession() {
      if (typeof globalThis.sessionStorage === 'undefined') {
        return null;
      }
      const raw = globalThis.sessionStorage.getItem(storageKey);
      if (!raw) {
        return null;
      }
      try {
        const parsed = JSON.parse(raw) as KernelUiAuthSession;
        if (!parsed.accessToken) {
          return null;
        }
        return parsed;
      } catch {
        return null;
      }
    }
  };
}

export function persistBrowserKernelUiAuthSession(
  session: KernelUiAuthSession,
  storageKey = KERNEL_UI_AUTH_SESSION_KEY
): void {
  if (typeof globalThis.sessionStorage === 'undefined') {
    return;
  }
  globalThis.sessionStorage.setItem(storageKey, JSON.stringify(session));
}

export function readBrowserKernelUiAuthSession(
  storageKey = KERNEL_UI_AUTH_SESSION_KEY
): KernelUiAuthSession | null {
  if (typeof globalThis.sessionStorage === 'undefined') {
    return null;
  }
  const raw = globalThis.sessionStorage.getItem(storageKey);
  if (!raw) {
    return null;
  }
  try {
    const parsed = JSON.parse(raw) as KernelUiAuthSession;
    if (!parsed.accessToken) {
      return null;
    }
    return parsed;
  } catch {
    return null;
  }
}

export function clearBrowserKernelUiAuthSession(storageKey = KERNEL_UI_AUTH_SESSION_KEY): void {
  if (typeof globalThis.sessionStorage === 'undefined') {
    return;
  }
  globalThis.sessionStorage.removeItem(storageKey);
}

export async function buildKernelUiAuthHeaders(
  auth?: KernelUiAuthProvider
): Promise<Record<string, string>> {
  if (!auth) {
    return {};
  }

  const session = await auth.getSession();
  if (!session?.accessToken) {
    return {};
  }

  const headers: Record<string, string> = {
    Authorization: `Bearer ${session.accessToken}`
  };

  if (session.tenantId) {
    headers['x-sdkwork-tenant-id'] = session.tenantId;
  }
  if (session.userId) {
    headers['x-sdkwork-user-id'] = session.userId;
  }

  return headers;
}
