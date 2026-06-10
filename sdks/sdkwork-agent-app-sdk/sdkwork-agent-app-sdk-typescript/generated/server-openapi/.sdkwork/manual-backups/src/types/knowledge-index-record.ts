import type { AgentStatus } from './agent-status';
import type { Int64String } from './int64-string';
import type { KnowledgeIndexKind } from './knowledge-index-kind';

export interface KnowledgeIndexRecord {
  id: Int64String;
  tenantId: Int64String;
  knowledgeIndexId: string;
  knowledgeBaseId: string;
  knowledgeDocumentId?: string | null;
  knowledgeChunkId?: string | null;
  indexKind: KnowledgeIndexKind;
  indexProviderId: string;
  externalRef: string;
  embeddingModelId?: string | null;
  vectorDimension?: number | null;
  contentHash: string;
  indexedAt: string;
  status: AgentStatus;
}
