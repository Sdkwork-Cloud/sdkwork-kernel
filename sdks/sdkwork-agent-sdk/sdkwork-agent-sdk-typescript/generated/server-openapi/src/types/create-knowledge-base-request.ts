import type { AgentVisibility } from './agent-visibility';
import type { KnowledgeBaseKind } from './knowledge-base-kind';
import type { KnowledgeIndexKind } from './knowledge-index-kind';

export interface CreateKnowledgeBaseRequest {
  knowledgeBaseId: string;
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
