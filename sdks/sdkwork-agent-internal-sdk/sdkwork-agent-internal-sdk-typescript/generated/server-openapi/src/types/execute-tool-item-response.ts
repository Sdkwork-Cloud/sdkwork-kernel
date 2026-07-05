import type { ExecuteToolResponse } from './execute-tool-response';
import type { SdkWorkResourceData } from './sdk-work-resource-data';

/** Tool execution SdkWorkApiResponse envelope. */
export interface ExecuteToolItemResponse {
  /** Numeric success result code. MUST be 0 on HTTP 2xx JSON bodies. */
  code: 0;
  data: unknown & SdkWorkResourceData & Record<string, unknown>;
  /** Server-owned request correlation id. */
  traceId: string;
}
