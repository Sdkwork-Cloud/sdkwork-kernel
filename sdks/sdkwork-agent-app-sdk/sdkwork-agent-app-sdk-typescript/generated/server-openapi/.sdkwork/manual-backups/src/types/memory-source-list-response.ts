import type { MemorySourceRecord } from './memory-source-record';
import type { PageInfo } from './page-info';

export interface MemorySourceListResponse {
  data: Record<string, unknown>;
  requestId?: string;
}
