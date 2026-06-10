import type { MemorySourceRecord } from './memory-source-record';

export interface MemorySourceResponse {
  data: MemorySourceRecord;
  requestId?: string;
}
