import type { MessageResponse } from './message-response';

export interface MessageTurnResponse {
  userMessage: MessageResponse;
  assistantMessage?: MessageResponse | null;
  status: 'completed';
}
