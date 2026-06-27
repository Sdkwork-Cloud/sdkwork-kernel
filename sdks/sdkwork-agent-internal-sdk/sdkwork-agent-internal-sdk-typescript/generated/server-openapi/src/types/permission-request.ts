export interface PermissionRequest {
  permissionRequestId: string;
  sessionId: string;
  capability: string;
  status: 'pending' | 'allow' | 'deny' | 'needs_approval';
  detail?: string | null;
  requestedAt?: string | null;
  decidedAt?: string | null;
}
