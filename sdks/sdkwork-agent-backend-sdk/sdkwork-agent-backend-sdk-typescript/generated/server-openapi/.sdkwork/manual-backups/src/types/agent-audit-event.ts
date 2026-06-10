export interface AgentAuditEvent {
  eventId: string;
  eventType: string;
  severity: string;
  payload: string;
  occurredAt: string;
}
