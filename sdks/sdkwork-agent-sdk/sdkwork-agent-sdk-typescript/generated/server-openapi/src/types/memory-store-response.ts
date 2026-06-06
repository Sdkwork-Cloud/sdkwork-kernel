import type { MemoryStoreRecord } from './memory-store-record';

export interface MemoryStoreResponse {
  data: MemoryStoreRecord;
  requestId?: string;
}
