import type { SdkWorkPageData } from './sdk-work-page-data';
import type { ToolDescriptor } from './tool-descriptor';

/** Tool catalog SdkWorkApiResponse page envelope. */
export interface ToolCatalogListResponse {
  /** Numeric success result code. MUST be 0 on HTTP 2xx JSON bodies. */
  code: 0;
  data: unknown & SdkWorkPageData & { items?: ToolDescriptor[]; };
  /** Server-owned request correlation id. */
  traceId: string;
}
