import type { KernelUiAuthProvider, KernelUiAuthSession } from '@sdkwork/kernel-ui-types';

const KERNEL_UI_AUTH_SESSION_KEY = 'sdkwork.kernel-ui.auth.session';
export const INGRESS_IDENTITY_MAC_HEADER = 'x-sdkwork-identity-mac';

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

export async function computeIngressIdentityMac(
  ingressToken: string,
  tenantId: string,
  userId: string
): Promise<string> {
  const encoder = new TextEncoder();
  const key = await crypto.subtle.importKey(
    'raw',
    encoder.encode(ingressToken),
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['sign']
  );
  const signature = await crypto.subtle.sign(
    'HMAC',
    key,
    encoder.encode(`${tenantId}\n${userId}`)
  );
  return Array.from(new Uint8Array(signature))
    .map((byte) => byte.toString(16).padStart(2, '0'))
    .join('');
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
    Authorization: `Bearer ${session.accessToken}`,
    'x-api-key': session.accessToken
  };

  if (session.tenantId) {
    headers['x-sdkwork-tenant-id'] = session.tenantId;
  }
  if (session.userId) {
    headers['x-sdkwork-user-id'] = session.userId;
  }
  if (session.tenantId && session.userId) {
    headers[INGRESS_IDENTITY_MAC_HEADER] = await computeIngressIdentityMac(
      session.accessToken,
      session.tenantId,
      session.userId
    );
  }

  return headers;
}
