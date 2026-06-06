import type { MemoryRelationRecord } from './memory-relation-record';

export interface MemoryRelationResponse {
  data: MemoryRelationRecord;
  requestId?: string;
}
