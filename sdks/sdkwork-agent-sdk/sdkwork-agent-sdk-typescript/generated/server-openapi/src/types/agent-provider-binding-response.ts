import type { AgentProviderBindingRecord } from './agent-provider-binding-record';

export interface AgentProviderBindingResponse {
  data: AgentProviderBindingRecord;
  requestId?: string;
}
