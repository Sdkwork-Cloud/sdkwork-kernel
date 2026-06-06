import type { MemoryRelationKind } from './memory-relation-kind';

export interface CreateMemoryRelationRequest {
  memoryRelationId: string;
  fromMemoryId: string;
  toMemoryId: string;
  relationKind: MemoryRelationKind;
  weight: number;
  validFrom?: string;
  validUntil?: string;
  requestedAt: string;
}
