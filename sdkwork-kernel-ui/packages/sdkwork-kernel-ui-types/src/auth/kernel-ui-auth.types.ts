export interface KernelUiAuthSession {
  accessToken: string;
  tenantId?: string;
  userId?: string;
}

export interface KernelUiAuthProvider {
  getSession(): Promise<KernelUiAuthSession | null>;
}
