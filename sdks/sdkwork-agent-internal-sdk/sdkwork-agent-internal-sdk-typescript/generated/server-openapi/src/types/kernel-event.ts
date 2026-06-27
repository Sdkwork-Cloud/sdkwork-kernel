export interface KernelEvent {
  eventId: string;
  eventType: string;
  occurredAt: string;
  payload?: Record<string, unknown>;
}
