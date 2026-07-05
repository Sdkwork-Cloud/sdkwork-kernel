import type { RuntimeManifest } from './runtime-manifest';
import type { SdkWorkResourceData } from './sdk-work-resource-data';

/** Runtime manifest SdkWorkApiResponse envelope. */
export interface RuntimeManifestResponse {
  /** Numeric success result code. MUST be 0 on HTTP 2xx JSON bodies. */
  code: 0;
  data: unknown & SdkWorkResourceData & Record<string, unknown>;
  /** Server-owned request correlation id. */
  traceId: string;
}
