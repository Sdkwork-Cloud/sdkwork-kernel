import type { SdkWorkResourceData } from './sdk-work-resource-data';
import type { SessionResponse } from './session-response';

/** Session resource SdkWorkApiResponse envelope. */
export interface SessionItemResponse {
  /** Numeric success result code. MUST be 0 on HTTP 2xx JSON bodies. */
  code: 0;
  data: unknown & SdkWorkResourceData & { item?: SessionResponse; };
  /** Server-owned request correlation id. */
  traceId: string;
}
