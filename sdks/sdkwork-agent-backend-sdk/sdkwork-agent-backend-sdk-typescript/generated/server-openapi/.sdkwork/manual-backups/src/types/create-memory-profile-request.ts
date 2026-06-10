import type { AgentVisibility } from './agent-visibility';
import type { Int64String } from './int64-string';

export interface CreateMemoryProfileRequest {
  memoryProfileId: string;
  organizationId: Int64String;
  ownerUserId: Int64String;
  code: string;
  displayName: string;
  description?: string | null;
  writePolicy?: Record<string, unknown>;
  retrievalPolicy?: Record<string, unknown>;
  compactionPolicy?: Record<string, unknown>;
  retentionPolicy?: Record<string, unknown>;
  privacyPolicy?: Record<string, unknown>;
  visibility: AgentVisibility;
  requestedAt: string;
}
