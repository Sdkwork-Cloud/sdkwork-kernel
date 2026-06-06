import type { Int64String } from './int64-string';

export interface PageInfo {
  page: number;
  pageSize: number;
  totalItems: Int64String;
  totalPages: number;
}
