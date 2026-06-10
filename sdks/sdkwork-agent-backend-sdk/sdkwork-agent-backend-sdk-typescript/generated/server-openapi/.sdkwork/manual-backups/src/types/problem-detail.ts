import type { FieldError } from './field-error';

export interface ProblemDetail {
  type: string;
  title: string;
  status: number;
  detail?: string;
  instance?: string;
  /** Error code, for example validation_error, permission_required, conflict, version_conflict, not_found, internal_error. */
  code?: string;
  /** Error category for client handling, for example validation, permission, business, concurrency, resource, internal. */
  errorCategory?: 'validation' | 'permission' | 'business' | 'concurrency' | 'resource' | 'internal';
  /** Whether the client may retry the same request without changing payload semantics. */
  retryable?: boolean;
  traceId?: string;
  errors?: FieldError[];
}
