import type { AgentVisibility } from './agent-visibility';
import type { Int64String } from './int64-string';
import type { KnowledgeBaseKind } from './knowledge-base-kind';
import type { KnowledgeIndexKind } from './knowledge-index-kind';

export interface UpdateKnowledgeBaseRequest {
  expectedVersion?: Int64String;
  displayName?: string;
  description?: string | null;
  providerId?: string;
  baseKind?: KnowledgeBaseKind;
  retrievalModes?: KnowledgeIndexKind[];
  capabilityIds?: string[];
  configurationProfileId?: string;
  visibility?: AgentVisibility;
  requestedAt: string;
}
