import type { ProviderDiagnostic } from './provider-diagnostic';

export interface RuntimeDiagnostics {
  runtimeId: string;
  agentId: string;
  state: string;
  providerCount: number;
  capabilityCount: number;
  typedProviderCount: number;
  manifestOnlyProviderCount: number;
  missingRequiredCapabilities: string[];
  degradedCapabilities: string[];
  providerDiagnostics: ProviderDiagnostic[];
}
