import type { MessageTurnResponse } from './message-turn-response';
import type { SdkWorkResourceData } from './sdk-work-resource-data';

/** Completed message turn SdkWorkApiResponse envelope. */
export interface MessageTurnItemResponse {
  /** Numeric success result code. MUST be 0 on HTTP 2xx JSON bodies. */
  code: 0;
  data: unknown & SdkWorkResourceData & { item?: MessageTurnResponse; };
  /** Server-owned request correlation id. */
  traceId: string;
}
