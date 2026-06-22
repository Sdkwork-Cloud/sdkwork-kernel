export interface ProblemDetail {
  type: string;
  title: string;
  status: number;
  detail?: string;
  code?: string;
  errorCategory?: string;
  retryable?: boolean;
}
