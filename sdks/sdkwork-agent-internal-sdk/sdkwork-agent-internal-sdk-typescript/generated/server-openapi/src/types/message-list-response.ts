import type { MessageResponse } from './message-response';
import type { SdkWorkPageData } from './sdk-work-page-data';

/** Paginated session message list following SdkWorkApiResponse envelope. */
export interface MessageListResponse {
  /** Numeric success result code. MUST be 0 on HTTP 2xx JSON bodies. */
  code: 0;
  data: unknown & SdkWorkPageData & { items: MessageResponse[]; };
  /** Server-owned request correlation id. */
  traceId: string;
}
