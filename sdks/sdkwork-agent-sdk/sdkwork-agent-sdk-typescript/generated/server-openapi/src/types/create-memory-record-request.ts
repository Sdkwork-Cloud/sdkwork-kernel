import type { MemoryRecordKind } from './memory-record-kind';

export interface CreateMemoryRecordRequest {
  memoryId: string;
  agentId?: string | null;
  memoryKind: MemoryRecordKind;
  contentFormat: string;
  content: Record<string, unknown>;
  summary?: string | null;
  salienceScore: number;
  confidenceScore: number;
  freshnessScore: number;
  sensitivityLevel: number;
  effectiveAt?: string;
  expiresAt?: string;
  requestedAt: string;
}
