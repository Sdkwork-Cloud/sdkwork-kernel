import type { AgentProviderBindingRecord } from './agent-provider-binding-record';
import type { PageInfo } from './page-info';

export interface AgentProviderBindingListResponse {
  data: Record<string, unknown>;
  requestId?: string;
}
