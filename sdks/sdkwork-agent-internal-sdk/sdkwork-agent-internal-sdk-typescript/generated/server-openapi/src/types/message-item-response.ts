import type { MessageResponse } from './message-response';
import type { SdkWorkResourceData } from './sdk-work-resource-data';

/** Message resource SdkWorkApiResponse envelope. */
export interface MessageItemResponse {
  /** Numeric success result code. MUST be 0 on HTTP 2xx JSON bodies. */
  code: 0;
  data: unknown & SdkWorkResourceData & { item?: MessageResponse; };
  /** Server-owned request correlation id. */
  traceId: string;
}
