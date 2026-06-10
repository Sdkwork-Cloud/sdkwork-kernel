import type { MemoryRetrievalIndexRecord } from './memory-retrieval-index-record';

export interface MemoryRetrievalIndexResponse {
  data: MemoryRetrievalIndexRecord;
  requestId?: string;
}
