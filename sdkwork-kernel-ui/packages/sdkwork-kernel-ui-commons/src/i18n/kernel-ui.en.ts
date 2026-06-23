export const kernelUiEn = {
  'app.title': 'SDKWork Kernel UI',
  'app.loading': 'Loading kernel UI',
  'auth.title': 'Kernel session',
  'auth.description': 'Provide the kernel ingress token plus tenant and user identity for signed internal-api access.',
  'auth.accessToken': 'Ingress token',
  'auth.tenantId': 'Tenant ID',
  'auth.userId': 'User ID',
  'auth.save': 'Save session',
  'auth.clear': 'Clear session',
  'permission.pending': 'Pending approval',
  'permission.allow': 'Allow',
  'permission.deny': 'Deny',
  'session.create': 'Create session',
  'session.closed': 'Session closed',
  'error.requestFailed': 'Kernel UI request failed'
} as const;

export type KernelUiMessageKey = keyof typeof kernelUiEn;

export function translateKernelUi(key: KernelUiMessageKey, locale = 'en'): string {
  if (locale !== 'en') {
    return kernelUiEn[key];
  }
  return kernelUiEn[key];
}
