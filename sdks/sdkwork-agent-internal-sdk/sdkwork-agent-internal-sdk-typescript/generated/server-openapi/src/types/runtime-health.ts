export interface RuntimeHealth {
  runtimeId: string;
  state: string;
  health: 'healthy' | 'degraded';
  persistenceHealthy: boolean;
  degradedCapabilities: string[];
}
