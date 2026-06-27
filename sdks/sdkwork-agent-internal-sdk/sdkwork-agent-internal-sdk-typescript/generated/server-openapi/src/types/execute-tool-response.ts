export interface ExecuteToolResponse {
  toolCallId: string;
  toolId: string;
  input: string;
  status: string;
  output?: string | null;
}
