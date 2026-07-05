import type { RuntimeHealth } from './runtime-health';
import type { SdkWorkResourceData } from './sdk-work-resource-data';

/** Runtime health SdkWorkApiResponse envelope. */
export interface RuntimeHealthResponse {
  /** Numeric success result code. MUST be 0 on HTTP 2xx JSON bodies. */
  code: 0;
  data: unknown & SdkWorkResourceData & Record<string, unknown>;
  /** Server-owned request correlation id. */
  traceId: string;
}
