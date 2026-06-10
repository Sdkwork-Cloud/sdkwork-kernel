import type { AgentVisibility } from './agent-visibility';

export interface CreateMemoryProfileRequest {
  memoryProfileId: string;
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
