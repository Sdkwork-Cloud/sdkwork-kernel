import type { AgentStatus } from './agent-status';
import type { Int64String } from './int64-string';
import type { MemoryRecordKind } from './memory-record-kind';

export interface MemoryRecord {
  id: Int64String;
  tenantId: Int64String;
  organizationId: Int64String;
  memoryId: string;
  memoryNamespaceId: string;
  agentId?: string | null;
  memoryKind: MemoryRecordKind;
  contentFormat: string;
  content: Record<string, unknown>;
  summary?: string | null;
  salienceScore: number;
  confidenceScore: number;
  freshnessScore: number;
  sensitivityLevel: number;
  sourceCount: number;
  effectiveAt?: string;
  expiresAt?: string;
  lastUsedAt?: string;
  useCount: Int64String;
  status: AgentStatus;
  version: Int64String;
  createdAt: string;
  updatedAt: string;
  deletedAt?: string;
  redactedAt?: string;
}
