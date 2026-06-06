import type { MemorySourceKind } from './memory-source-kind';

export interface CreateMemorySourceRequest {
  memorySourceId: string;
  sourceKind: MemorySourceKind;
  sourceRef: string;
  sourceHash: string;
  evidence?: Record<string, unknown>;
  capturedAt: string;
  requestedAt: string;
}
