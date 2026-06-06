import type { AgentStatus } from './agent-status';
import type { AgentVisibility } from './agent-visibility';
import type { Int64String } from './int64-string';
import type { KnowledgeBaseKind } from './knowledge-base-kind';
import type { KnowledgeIndexKind } from './knowledge-index-kind';

export interface KnowledgeBaseRecord {
  id: Int64String;
  tenantId: Int64String;
  organizationId: Int64String;
  ownerUserId: Int64String;
  knowledgeBaseId: string;
  code: string;
  displayName: string;
  description?: string | null;
  providerId: string;
  baseKind: KnowledgeBaseKind;
  retrievalModes: KnowledgeIndexKind[];
  capabilityIds: string[];
  configurationProfileId: string;
  status: AgentStatus;
  visibility: AgentVisibility;
  version: Int64String;
  createdAt: string;
  updatedAt: string;
  deletedAt?: string;
}
