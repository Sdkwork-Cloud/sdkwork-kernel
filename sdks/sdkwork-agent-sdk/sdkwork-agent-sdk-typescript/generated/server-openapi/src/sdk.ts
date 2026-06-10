import { HttpClient, createHttpClient } from './http/client';
import type { SdkworkAgentConfig } from './types/common';
import type { AuthTokenManager } from '@sdkwork/sdk-common';

import { AiApi, createAiApi } from './api/ai';

export class SdkworkAgentClient {
  private httpClient: HttpClient;

  public readonly ai: AiApi;

  constructor(config: SdkworkAgentConfig) {
    this.httpClient = createHttpClient(config);
    this.ai = createAiApi(this.httpClient);
  }
  setAuthToken(token: string): this {
    this.httpClient.setAuthToken(token);
    return this;
  }

  setAccessToken(token: string): this {
    this.httpClient.setAccessToken(token);
    return this;
  }

  setTokenManager(manager: AuthTokenManager): this {
    this.httpClient.setTokenManager(manager);
    return this;
  }

  get http(): HttpClient {
    return this.httpClient;
  }
}

export function createClient(config: SdkworkAgentConfig): SdkworkAgentClient {
  return new SdkworkAgentClient(config);
}

export default SdkworkAgentClient;
