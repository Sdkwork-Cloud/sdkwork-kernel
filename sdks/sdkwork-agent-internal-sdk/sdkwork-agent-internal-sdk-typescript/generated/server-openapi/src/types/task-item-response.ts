import type { SdkWorkResourceData } from './sdk-work-resource-data';
import type { TaskResponse } from './task-response';

/** Task resource SdkWorkApiResponse envelope. */
export interface TaskItemResponse {
  /** Numeric success result code. MUST be 0 on HTTP 2xx JSON bodies. */
  code: 0;
  data: unknown & SdkWorkResourceData & { item?: TaskResponse; };
  /** Server-owned request correlation id. */
  traceId: string;
}
