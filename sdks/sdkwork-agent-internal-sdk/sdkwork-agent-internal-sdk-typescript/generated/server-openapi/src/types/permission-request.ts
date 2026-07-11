export interface PermissionRequest {
  permissionRequestId: string;
  category: string;
  resource: string;
  sideEffectLevel: string;
  reason: string;
  status: 'pending' | 'allow' | 'deny' | 'needs_approval';
}
