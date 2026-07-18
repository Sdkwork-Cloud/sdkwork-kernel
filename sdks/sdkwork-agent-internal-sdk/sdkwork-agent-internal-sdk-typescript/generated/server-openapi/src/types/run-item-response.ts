import type { RunResponse } from './run-response';
import type { SdkWorkResourceData } from './sdk-work-resource-data';

/** Runtime run resource SdkWorkApiResponse envelope. */
export interface RunItemResponse {
  /** Numeric success result code. MUST be 0 on HTTP 2xx JSON bodies. */
  code: 0;
  data: unknown & SdkWorkResourceData & Record<string, unknown>;
  /** Server-owned request correlation id. */
  traceId: string;
}
