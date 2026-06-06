export interface CreateAgentPromptOptimizationRequest {
  executionId: string;
  prompt: string;
  inputPayload?: Record<string, unknown>;
  requestedAt: string;
}
