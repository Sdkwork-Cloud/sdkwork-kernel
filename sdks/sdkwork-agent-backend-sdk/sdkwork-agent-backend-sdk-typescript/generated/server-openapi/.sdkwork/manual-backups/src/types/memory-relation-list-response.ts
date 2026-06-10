import type { MemoryRelationRecord } from './memory-relation-record';
import type { PageInfo } from './page-info';

export interface MemoryRelationListResponse {
  data: Record<string, unknown>;
  requestId?: string;
}
