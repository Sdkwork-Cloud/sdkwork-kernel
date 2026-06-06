import type { KnowledgeIndexKind } from './knowledge-index-kind';

/** vector knowledge index requires embeddingModelId and vectorDimension */
export interface UpsertKnowledgeIndexRequest {
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
  requestedAt: string;
}
