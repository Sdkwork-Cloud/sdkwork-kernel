import type { MemoryIndexKind } from './memory-index-kind';

/** vector memory index requires embeddingModelId and vectorDimension */
export interface UpsertMemoryRetrievalIndexRequest {
  memoryIndexId: string;
  memoryId: string;
  indexKind: MemoryIndexKind;
  indexProviderId: string;
  externalRef: string;
  embeddingModelId?: string | null;
  vectorDimension?: number | null;
  contentHash: string;
  requestedAt: string;
}
