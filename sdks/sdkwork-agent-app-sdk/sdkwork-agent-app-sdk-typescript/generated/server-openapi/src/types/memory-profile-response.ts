import type { MemoryProfileRecord } from './memory-profile-record';

export interface MemoryProfileResponse {
  data: MemoryProfileRecord;
  requestId?: string;
}
