import type { SdkWorkPageData } from './sdk-work-page-data';
import type { SessionItemResponse } from './session-item-response';

/** Paginated runtime session list following SdkWorkApiResponse envelope. */
export interface SessionListResponse {
  /** Numeric success result code. MUST be 0 on HTTP 2xx JSON bodies. */
  code: 0;
  data: unknown & SdkWorkPageData & Record<string, unknown>;
  /** Server-owned request correlation id. */
  traceId: string;
}
