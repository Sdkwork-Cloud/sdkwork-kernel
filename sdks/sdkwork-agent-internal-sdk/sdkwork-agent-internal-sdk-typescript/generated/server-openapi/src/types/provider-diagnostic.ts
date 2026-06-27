export interface ProviderDiagnostic {
  providerId: string;
  providerFamily: string;
  providerVersion: string;
  typedRegistered: boolean;
  healthStatus?: string | null;
  capabilities: string[];
}
