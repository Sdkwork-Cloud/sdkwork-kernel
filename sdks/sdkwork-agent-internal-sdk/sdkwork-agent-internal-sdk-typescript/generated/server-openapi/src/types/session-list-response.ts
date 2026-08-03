import type { SdkWorkPageData } from './sdk-work-page-data';
import type { SessionResponse } from './session-response';

/** Paginated runtime session list following SdkWorkApiResponse envelope. */
export interface SessionListResponse {
  /** Numeric success result code. MUST be 0 on HTTP 2xx JSON bodies. */
  code: 0;
  data: unknown & SdkWorkPageData & { items: SessionResponse[]; };
  /** Server-owned request correlation id. */
  traceId: string;
}
