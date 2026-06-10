import type { Int64String } from './int64-string';
import type { MemoryRelationKind } from './memory-relation-kind';

export interface MemoryRelationRecord {
  id: Int64String;
  tenantId: Int64String;
  memoryRelationId: string;
  fromMemoryId: string;
  toMemoryId: string;
  relationKind: MemoryRelationKind;
  weight: number;
  validFrom?: string;
  validUntil?: string;
  createdAt: string;
}
