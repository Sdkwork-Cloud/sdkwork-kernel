import type { Int64String } from './int64-string';
import type { KnowledgeBindingScopeKind } from './knowledge-binding-scope-kind';

export interface KnowledgeBindingRecord {
  id: Int64String;
  tenantId: Int64String;
  organizationId: Int64String;
  knowledgeBindingId: string;
  knowledgeBaseId: string;
  agentId?: string | null;
  deploymentId?: string | null;
  scopeKind: KnowledgeBindingScopeKind;
  /** Agent scopes require scopeRef to match agentId; deployment scopes require scopeRef to match deploymentId and include agentId. */
  scopeRef: string;
  active: boolean;
  defaultBinding: boolean;
  version: Int64String;
  createdAt: string;
  updatedAt: string;
}
