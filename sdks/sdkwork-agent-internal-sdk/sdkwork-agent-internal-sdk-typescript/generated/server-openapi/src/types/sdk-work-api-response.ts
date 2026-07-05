export interface SdkWorkApiResponse {
  /** Numeric success result code. MUST be 0 on HTTP 2xx JSON bodies. */
  code: 0;
  /** Operation-specific payload. */
  data: unknown;
  /** Server-owned request correlation id. */
  traceId: string;
}
