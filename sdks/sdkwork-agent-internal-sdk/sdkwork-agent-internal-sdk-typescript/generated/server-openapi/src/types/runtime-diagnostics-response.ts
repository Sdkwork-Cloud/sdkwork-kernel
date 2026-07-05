import type { RuntimeDiagnostics } from './runtime-diagnostics';
import type { SdkWorkResourceData } from './sdk-work-resource-data';

/** Runtime diagnostics SdkWorkApiResponse envelope. */
export interface RuntimeDiagnosticsResponse {
  /** Numeric success result code. MUST be 0 on HTTP 2xx JSON bodies. */
  code: 0;
  data: unknown & SdkWorkResourceData & Record<string, unknown>;
  /** Server-owned request correlation id. */
  traceId: string;
}
