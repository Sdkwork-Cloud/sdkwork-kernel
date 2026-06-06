import type { KnowledgeChunkRecord } from './knowledge-chunk-record';
import type { PageInfo } from './page-info';

export interface KnowledgeChunkListResponse {
  data: Record<string, unknown>;
  requestId?: string;
}
