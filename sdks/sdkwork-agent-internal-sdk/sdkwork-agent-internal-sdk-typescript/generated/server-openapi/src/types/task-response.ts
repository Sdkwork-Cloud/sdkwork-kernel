export interface TaskResponse {
  taskId: string;
  sessionId: string;
  instruction: string;
  state: string;
  createdAt?: string | null;
  updatedAt?: string | null;
}
