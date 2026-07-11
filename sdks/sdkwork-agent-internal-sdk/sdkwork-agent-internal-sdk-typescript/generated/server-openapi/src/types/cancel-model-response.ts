export interface CancelModelResponse {
  modelRequestId: string;
  providerId: string;
  status: string;
  finishReason?: string | null;
}
