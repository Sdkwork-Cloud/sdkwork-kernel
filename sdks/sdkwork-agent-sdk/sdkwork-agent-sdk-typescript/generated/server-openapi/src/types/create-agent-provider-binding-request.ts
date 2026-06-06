import type { AgentImplementationKind } from './agent-implementation-kind';

export interface CreateAgentProviderBindingRequest {
  bindingId: string;
  providerId: string;
  implementationKind: AgentImplementationKind;
  configurationProfileId: string;
  capabilities?: string[];
  makeDefault?: boolean;
  requestedAt: string;
}
