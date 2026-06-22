import type { SdkworkCustomConfig } from '../types/common';
import type { RequestOptions, QueryParams } from '@sdkwork/sdk-common';
import { BaseHttpClient, withRetry } from '@sdkwork/sdk-common';

type HttpRequestOptions = RequestOptions & {
  method?: string;
  body?: unknown;
  headers?: Record<string, string>;
  contentType?: string;
};

export class HttpClient extends BaseHttpClient {
  private static readonly API_KEY_HEADER: string = 'X-API-Key';
  private static readonly API_KEY_USE_BEARER = false;

  constructor(config: SdkworkCustomConfig) {
    super(config as any);
  }

  private getInternalAuthConfig(): any {
    const self = this as any;
    self.authConfig = self.authConfig || {};
    return self.authConfig;
  }
  private buildRequestHeaders(
    headers?: Record<string, string>,
    contentType?: string,
  ): Record<string, string> | undefined {
    const mergedHeaders = {
      ...(headers ?? {}),
    };

    if (contentType && contentType.toLowerCase() !== 'multipart/form-data') {
      mergedHeaders['Content-Type'] = contentType;
    }

    return Object.keys(mergedHeaders).length > 0 ? mergedHeaders : undefined;
  }
  protected buildHeaders(config: any, skipAuth = false): Record<string, string> {
    const headers = super.buildHeaders(config, true);
    if (!skipAuth && !config?.skipAuth) {
      const apiKey = this.getInternalAuthConfig().apiKey;
      if (typeof apiKey === 'string' && apiKey.length > 0) {
        headers[HttpClient.API_KEY_HEADER] = HttpClient.API_KEY_USE_BEARER
          ? `Bearer ${apiKey}`
          : apiKey;
      }
    }
    return headers;
  }
  private buildRequestBody(body: unknown, contentType?: string): unknown {
    if (body == null) {
      return body;
    }

    const normalizedContentType = (contentType ?? '').toLowerCase();
    if (normalizedContentType === 'application/x-www-form-urlencoded') {
      return this.encodeFormBody(body);
    }
    if (normalizedContentType === 'multipart/form-data') {
      return this.encodeMultipartBody(body);
    }

    return body;
  }

  private encodeMultipartBody(body: unknown): FormData {
    if (body instanceof FormData) {
      return body;
    }

    const formData = new FormData();
    if (body instanceof Map) {
      for (const [key, value] of body.entries()) {
        this.appendMultipartValue(formData, String(key), value);
      }
      return formData;
    }
    if (typeof body === 'object') {
      const record = body as Record<string, unknown>;
      for (const [key, value] of Object.entries(record)) {
        if (this.isMultipartMetadataField(key)) {
          continue;
        }
        this.appendMultipartValue(formData, key, value, this.resolveMultipartFileName(record, key));
      }
      return formData;
    }

    this.appendMultipartValue(formData, 'value', body);
    return formData;
  }

  private appendMultipartValue(formData: FormData, key: string, value: unknown, fileName?: string): void {
    if (value == null) {
      return;
    }
    if (Array.isArray(value)) {
      value.forEach((item) => this.appendMultipartValue(formData, key, item, fileName));
      return;
    }
    if (value instanceof Blob) {
      if (fileName) {
        formData.append(key, value, fileName);
        return;
      }
      formData.append(key, value);
      return;
    }
    if (value instanceof Date) {
      formData.append(key, value.toISOString());
      return;
    }
    if (typeof value === 'object') {
      formData.append(key, JSON.stringify(value));
      return;
    }
    formData.append(key, String(value));
  }

  private resolveMultipartFileName(record: Record<string, unknown>, key: string): string | undefined {
    const fieldSpecificName = record[`${key}FileName`];
    if (typeof fieldSpecificName === 'string' && fieldSpecificName.trim()) {
      return fieldSpecificName.trim();
    }
    const genericName = record.fileName;
    if (key === 'file' && typeof genericName === 'string' && genericName.trim()) {
      return genericName.trim();
    }
    return undefined;
  }

  private isMultipartMetadataField(key: string): boolean {
    return key === 'fileName' || key.endsWith('FileName');
  }

  private encodeFormBody(body: unknown): string {
    if (body instanceof URLSearchParams) {
      return body.toString();
    }
    if (typeof body === 'string') {
      return body;
    }

    const params = new URLSearchParams();
    if (body instanceof Map) {
      for (const [key, value] of body.entries()) {
        this.appendFormValue(params, String(key), value);
      }
      return params.toString();
    }
    if (typeof body === 'object') {
      for (const [key, value] of Object.entries(body as Record<string, unknown>)) {
        this.appendFormValue(params, key, value);
      }
      return params.toString();
    }

    params.append('value', String(body));
    return params.toString();
  }

  private appendFormValue(params: URLSearchParams, key: string, value: unknown): void {
    if (value == null) {
      return;
    }
    if (Array.isArray(value)) {
      value.forEach((item) => this.appendFormValue(params, key, item));
      return;
    }
    if (value instanceof Date) {
      params.append(key, value.toISOString());
      return;
    }
    if (typeof value === 'object') {
      params.append(key, JSON.stringify(value));
      return;
    }
    params.append(key, String(value));
  }
  setApiKey(apiKey: string): void {
    this.getInternalAuthConfig().apiKey = apiKey;
  }
  async request<T>(path: string, options: HttpRequestOptions = {}): Promise<T> {
    const execute = (this as any).execute;
    if (typeof execute !== 'function') {
      throw new Error('BaseHttpClient execute method is not available');
    }
    const { body, headers, contentType, method = 'GET', skipAuth, ...rest } = options;
    const requestHeaders = headers;
    return withRetry(
      () => execute.call(this, {
        url: path,
        method,
        ...rest,
        skipAuth,
        body: this.buildRequestBody(body, contentType),
        headers: this.buildRequestHeaders(requestHeaders, body == null ? undefined : contentType),
      }),
      { maxRetries: 3 }
    );
  }

  async *streamJson<T>(path: string, options: HttpRequestOptions = {}): AsyncIterable<T> {
    const stream = (BaseHttpClient.prototype as any).stream;
    if (typeof stream !== 'function') {
      throw new Error('BaseHttpClient stream method is not available');
    }
    const { body, headers, contentType, method = 'GET', skipAuth, ...rest } = options;
    const requestHeaders = this.buildRequestHeaders(
      { Accept: 'text/event-stream', ...(headers ?? {}) },
      body == null ? undefined : contentType,
    );

    for await (const data of stream.call(this, path, {
      method,
      ...rest,
      skipAuth,
      body: this.buildRequestBody(body, contentType),
      headers: requestHeaders,
    })) {
      if (data === '[DONE]') {
        return;
      }
      if (typeof data !== 'string' || data.trim().length === 0) {
        continue;
      }
      yield JSON.parse(data) as T;
    }
  }

  async get<T>(path: string, params?: QueryParams, headers?: Record<string, string>): Promise<T> {
    return this.request<T>(path, { method: 'GET', params, headers });
  }

  async post<T>(
    path: string,
    body?: unknown,
    params?: QueryParams,
    headers?: Record<string, string>,
    contentType?: string,
  ): Promise<T> {
    return this.request<T>(path, { method: 'POST', body, params, headers, contentType });
  }

  async put<T>(
    path: string,
    body?: unknown,
    params?: QueryParams,
    headers?: Record<string, string>,
    contentType?: string,
  ): Promise<T> {
    return this.request<T>(path, { method: 'PUT', body, params, headers, contentType });
  }

  async delete<T>(path: string, params?: QueryParams, headers?: Record<string, string>): Promise<T> {
    return this.request<T>(path, { method: 'DELETE', params, headers });
  }

  async patch<T>(
    path: string,
    body?: unknown,
    params?: QueryParams,
    headers?: Record<string, string>,
    contentType?: string,
  ): Promise<T> {
    return this.request<T>(path, { method: 'PATCH', body, params, headers, contentType });
  }
}

export function createHttpClient(config: SdkworkCustomConfig): HttpClient {
  return new HttpClient(config);
}
