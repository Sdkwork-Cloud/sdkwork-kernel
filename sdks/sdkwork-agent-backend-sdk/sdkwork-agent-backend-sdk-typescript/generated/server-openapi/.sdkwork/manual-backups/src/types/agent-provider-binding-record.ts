import type { AgentImplementationKind } from './agent-implementation-kind';
import type { Int64String } from './int64-string';

export interface AgentProviderBindingRecord {
  tenantId: Int64String;
  agentId: string;
  bindingId: string;
  providerId: string;
  implementationKind: AgentImplementationKind;
  configurationProfileId: string;
  capabilities: string[];
  active: boolean;
  version: Int64String;
  createdAt: string;
  updatedAt: string;
}
