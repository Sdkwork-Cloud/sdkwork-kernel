import type { RuntimeSnapshot } from './runtime-snapshot';
import type { SdkWorkResourceData } from './sdk-work-resource-data';

/** Runtime aggregate snapshot SdkWorkApiResponse envelope. */
export interface RuntimeSnapshotResponse {
  /** Numeric success result code. MUST be 0 on HTTP 2xx JSON bodies. */
  code: 0;
  data: unknown & SdkWorkResourceData & { item?: RuntimeSnapshot; };
  /** Server-owned request correlation id. */
  traceId: string;
}
