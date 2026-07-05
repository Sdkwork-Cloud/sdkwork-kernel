import type { ModelDescriptor } from './model-descriptor';
import type { SdkWorkPageData } from './sdk-work-page-data';

/** Model catalog SdkWorkApiResponse page envelope. */
export interface ModelCatalogListResponse {
  /** Numeric success result code. MUST be 0 on HTTP 2xx JSON bodies. */
  code: 0;
  data: unknown & SdkWorkPageData & Record<string, unknown>;
  /** Server-owned request correlation id. */
  traceId: string;
}
