import type { MemoryRetrievalIndexRecord } from './memory-retrieval-index-record';
import type { PageInfo } from './page-info';

export interface MemoryRetrievalIndexListResponse {
  data: Record<string, unknown>;
  requestId?: string;
}
