import type { SdkWorkAsyncData } from './sdk-work-async-data';

/** Accepted asynchronous operation following SdkWorkApiResponse envelope. */
export interface AsyncOperationResponse {
  /** Numeric success result code. MUST be 0 on HTTP 2xx JSON bodies. */
  code: 0;
  data: unknown & SdkWorkAsyncData;
  /** Server-owned request correlation id. */
  traceId: string;
}
