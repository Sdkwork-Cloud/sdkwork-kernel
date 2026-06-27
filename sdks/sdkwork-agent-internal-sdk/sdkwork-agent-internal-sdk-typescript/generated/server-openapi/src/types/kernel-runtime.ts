import type { Capability } from './capability';

export interface KernelRuntime {
  runtimeId: string;
  agentId: string;
  kernelVersion: string;
  state: string;
  health: string;
  capabilities: Capability[];
  missingRequiredCapabilities: string[];
  degradedCapabilities: string[];
}
