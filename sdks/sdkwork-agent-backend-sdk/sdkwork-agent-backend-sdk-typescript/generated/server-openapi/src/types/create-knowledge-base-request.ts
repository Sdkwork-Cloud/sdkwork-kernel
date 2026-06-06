import type { AgentVisibility } from './agent-visibility';
import type { Int64String } from './int64-string';
import type { KnowledgeBaseKind } from './knowledge-base-kind';
import type { KnowledgeIndexKind } from './knowledge-index-kind';

export interface CreateKnowledgeBaseRequest {
  knowledgeBaseId: string;
  organizationId: Int64String;
  ownerUserId: Int64String;
  code: string;
  displayName: string;
  description?: string | null;
  providerId: string;
  baseKind: KnowledgeBaseKind;
  retrievalModes: KnowledgeIndexKind[];
  capabilityIds?: string[];
  configurationProfileId: string;
  visibility: AgentVisibility;
  requestedAt: string;
}
