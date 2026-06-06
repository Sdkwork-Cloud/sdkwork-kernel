import type { Int64String } from './int64-string';

export interface DeleteAgentRequest {
  expectedVersion?: Int64String;
  requestedAt: string;
}
