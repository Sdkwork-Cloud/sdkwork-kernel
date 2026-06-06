import type { AgentVisibility } from './agent-visibility';
import type { Int64String } from './int64-string';

export interface UpdateAgentRequest {
  displayName?: string;
  description?: string | null;
  manifest?: Record<string, unknown>;
  visibility?: AgentVisibility;
  tags?: string[];
  defaultCodeTaskIntent?: Record<string, unknown>;
  expectedVersion?: Int64String;
  requestedAt: string;
}
