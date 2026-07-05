export interface ProblemDetail {
  type: string;
  title: string;
  status: number;
  detail?: string;
  /** Request endpoint occurrence in the form `{METHOD} {routeTemplate}`. */
  instance?: string;
  operationId?: string;
  /** Numeric error result code. MUST be non-zero. See API_SPEC.md §15.3. */
  code: number;
  /** Server-owned request correlation id. */
  traceId: string;
  errorCategory?: string;
  retryable?: boolean;
}
