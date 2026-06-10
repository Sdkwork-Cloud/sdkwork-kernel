export interface CreateKnowledgeChunkRequest {
  knowledgeChunkId: string;
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
