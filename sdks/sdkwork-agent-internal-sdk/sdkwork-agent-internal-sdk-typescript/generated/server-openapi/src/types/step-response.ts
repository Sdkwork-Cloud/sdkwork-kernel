export interface StepResponse {
  stepId: string;
  sequenceNo: string;
  actionKind: 'model_call' | 'tool_call' | 'memory_read' | 'memory_write' | 'host_operation' | 'protocol_send' | 'handoff' | 'wait_for_user' | 'internal';
  state: 'created' | 'ready' | 'running' | 'awaiting_permission' | 'completed' | 'failed' | 'skipped' | 'cancelled';
  providerId?: string | null;
  startedAt?: string | null;
  finishedAt?: string | null;
  errorKind?: string | null;
  errorCode?: string | null;
  errorDetail?: string | null;
}
