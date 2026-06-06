import type { Int64String } from './int64-string';
import type { MemoryBindingScopeKind } from './memory-binding-scope-kind';

export interface CreateMemoryBindingRequest {
  memoryBindingId: string;
  organizationId: Int64String;
  agentId?: string | null;
  deploymentId?: string | null;
  scopeKind: MemoryBindingScopeKind;
  scopeRef: string;
  active?: boolean;
  defaultBinding?: boolean;
  requestedAt: string;
}
