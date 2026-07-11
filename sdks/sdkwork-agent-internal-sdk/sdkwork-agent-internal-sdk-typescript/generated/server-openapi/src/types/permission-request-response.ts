import type { PermissionRequest } from './permission-request';
import type { SdkWorkResourceData } from './sdk-work-resource-data';

/** Permission request SdkWorkApiResponse envelope. */
export interface PermissionRequestResponse {
  /** Numeric success result code. MUST be 0 on HTTP 2xx JSON bodies. */
  code: 0;
  data: unknown & SdkWorkResourceData & Record<string, unknown>;
  /** Server-owned request correlation id. */
  traceId: string;
}
