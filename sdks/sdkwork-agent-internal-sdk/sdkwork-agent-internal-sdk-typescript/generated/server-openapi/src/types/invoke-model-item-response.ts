import type { InvokeModelResponse } from './invoke-model-response';
import type { SdkWorkResourceData } from './sdk-work-resource-data';

/** Model invoke SdkWorkApiResponse envelope. */
export interface InvokeModelItemResponse {
  /** Numeric success result code. MUST be 0 on HTTP 2xx JSON bodies. */
  code: 0;
  data: unknown & SdkWorkResourceData & Record<string, unknown>;
  /** Server-owned request correlation id. */
  traceId: string;
}
