import type { MemoryNamespaceRecord } from './memory-namespace-record';

export interface MemoryNamespaceResponse {
  data: MemoryNamespaceRecord;
  requestId?: string;
}
