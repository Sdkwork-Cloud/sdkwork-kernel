import type { Capability } from './capability';
import type { ProviderManifest } from './provider-manifest';

export interface RuntimeManifest {
  runtimeId: string;
  agentId: string;
  kernelVersion: string;
  securityProfile: string;
  capabilities: Capability[];
  providers: ProviderManifest[];
  missingRequiredCapabilities: string[];
  degradedCapabilities: string[];
}
