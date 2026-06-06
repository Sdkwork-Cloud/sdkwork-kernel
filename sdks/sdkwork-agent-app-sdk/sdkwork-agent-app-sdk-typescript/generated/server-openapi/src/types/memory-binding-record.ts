import type { Int64String } from './int64-string';
import type { MemoryBindingScopeKind } from './memory-binding-scope-kind';

export interface MemoryBindingRecord {
  id: Int64String;
  tenantId: Int64String;
  organizationId: Int64String;
  memoryBindingId: string;
  memoryProfileId: string;
  agentId?: string | null;
  deploymentId?: string | null;
  scopeKind: MemoryBindingScopeKind;
  scopeRef: string;
  active: boolean;
  defaultBinding: boolean;
  version: Int64String;
  createdAt: string;
  updatedAt: string;
}
