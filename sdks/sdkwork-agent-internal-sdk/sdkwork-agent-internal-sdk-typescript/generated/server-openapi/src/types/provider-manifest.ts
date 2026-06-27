export interface ProviderManifest {
  providerId: string;
  providerFamily: string;
  name: string;
  version: string;
  capabilities: string[];
  healthStatus?: string | null;
}
