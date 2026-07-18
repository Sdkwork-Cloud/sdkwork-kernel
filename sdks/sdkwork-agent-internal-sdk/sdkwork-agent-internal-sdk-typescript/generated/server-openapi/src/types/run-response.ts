import type { StepResponse } from './step-response';

export interface RunResponse {
  runId: string;
  taskId: string;
  sessionId: string;
  attempt: string;
  state: 'created' | 'planning' | 'executing' | 'awaiting_permission' | 'paused' | 'completed' | 'failed' | 'cancelled';
  fencingToken: string;
  cancelRequestedAt?: string | null;
  startedAt?: string | null;
  finishedAt?: string | null;
  errorKind?: string | null;
  errorCode?: string | null;
  errorDetail?: string | null;
  createdAt: string;
  updatedAt: string;
  steps: StepResponse[];
}
