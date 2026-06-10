import type { Int64String } from './int64-string';
import type { KnowledgeBindingScopeKind } from './knowledge-binding-scope-kind';

export interface CreateKnowledgeBindingRequest {
  knowledgeBindingId: string;
  organizationId: Int64String;
  agentId?: string | null;
  deploymentId?: string | null;
  scopeKind: KnowledgeBindingScopeKind;
  /** Agent scopes require scopeRef to match agentId; deployment scopes require scopeRef to match deploymentId and include agentId. */
  scopeRef: string;
  active?: boolean;
  defaultBinding?: boolean;
  requestedAt: string;
}
