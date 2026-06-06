import type { Int64String } from './int64-string';
import type { MemorySourceKind } from './memory-source-kind';

export interface MemorySourceRecord {
  id: Int64String;
  tenantId: Int64String;
  memorySourceId: string;
  memoryId: string;
  sourceKind: MemorySourceKind;
  sourceRef: string;
  sourceHash: string;
  evidence: Record<string, unknown>;
  capturedAt: string;
  createdAt: string;
}
