import type { MemoryBindingRecord } from './memory-binding-record';

export interface MemoryBindingResponse {
  data: MemoryBindingRecord;
  requestId?: string;
}
