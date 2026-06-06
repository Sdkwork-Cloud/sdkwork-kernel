import type { AgentStatus } from './agent-status';
import type { Int64String } from './int64-string';
import type { KnowledgeSourceKind } from './knowledge-source-kind';

export interface KnowledgeSourceRecord {
  id: Int64String;
  tenantId: Int64String;
  organizationId: Int64String;
  knowledgeSourceId: string;
  knowledgeBaseId: string;
  sourceKind: KnowledgeSourceKind;
  sourceRef: string;
  sourceHash: string;
  syncPolicy: Record<string, unknown>;
  metadata: Record<string, unknown>;
  status: AgentStatus;
  version: Int64String;
  createdAt: string;
  updatedAt: string;
  deletedAt?: string;
}
