import type { MessagePart } from './message-part';

export interface MessageResponse {
  messageId: string;
  sessionId: string;
  role: string;
  parts: MessagePart[];
  createdAt?: string | null;
  metadata?: Record<string, string>;
}
