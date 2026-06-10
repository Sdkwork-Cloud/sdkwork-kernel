import type { KnowledgeChunkRecord } from './knowledge-chunk-record';

export interface KnowledgeChunkResponse {
  data: KnowledgeChunkRecord;
  requestId?: string;
}
