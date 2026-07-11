export interface KernelEvent {
  eventId: string;
  eventType: string;
  severity: string;
  summary: string;
  sequence: number;
  traceId?: string | null;
}
