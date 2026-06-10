import type { MemoryRecord } from './memory-record';
import type { PageInfo } from './page-info';

export interface MemoryRecordListResponse {
  data: Record<string, unknown>;
  requestId?: string;
}
