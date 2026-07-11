export interface ToolDescriptor {
  toolId: string;
  providerId: string;
  name?: string | null;
  displayName: string;
  description?: string | null;
  sideEffectLevel: string;
  policyCategories: string[];
  timeoutMs?: string | null;
}
