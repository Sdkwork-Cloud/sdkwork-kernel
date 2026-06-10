import type { MemoryRecord } from './memory-record';

export interface MemoryRecordResponse {
  data: MemoryRecord;
  requestId?: string;
}
