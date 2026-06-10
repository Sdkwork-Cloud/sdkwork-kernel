import type { Int64String } from './int64-string';

export interface CreateKnowledgeChunkRequest {
  knowledgeChunkId: string;
  organizationId: Int64String;
  parentChunkId?: string | null;
  chunkOrdinal: number;
  heading?: string | null;
  contentRef: string;
  contentHash: string;
  tokenEstimate: number;
  summary?: string | null;
  metadata?: Record<string, unknown>;
  requestedAt: string;
}
