import type { CancelModelResponse } from './cancel-model-response';
import type { SdkWorkResourceData } from './sdk-work-resource-data';

/** Model cancel SdkWorkApiResponse envelope. */
export interface CancelModelItemResponse {
  /** Numeric success result code. MUST be 0 on HTTP 2xx JSON bodies. */
  code: 0;
  data: unknown & SdkWorkResourceData & { item?: CancelModelResponse; };
  /** Server-owned request correlation id. */
  traceId: string;
}
