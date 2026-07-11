import {
  createClient as createGeneratedInternalClient,
  SdkworkCustomClient,
} from '../generated/server-openapi/src/index';
import type { SdkworkCustomConfig } from '../generated/server-openapi/src/types/common';
import {
  createAgentInternalStreamingApi,
  type AgentInternalStreamingApi,
} from './streaming';

export { SdkworkCustomClient, createGeneratedInternalClient };
export type { SdkworkCustomConfig };
export * from './sse-parser';
export * from './streaming';
export * from '../generated/server-openapi/src/types';
export * from '../generated/server-openapi/src/api';
export * from '../generated/server-openapi/src/http';
export * from '../generated/server-openapi/src/auth';

/**
 * Stable composed client for the agent internal API.
 *
 * Generated resource methods remain available under `intelligence`; typed
 * Server-Sent Event methods live under `streaming` because the current
 * generator transport exposes SSE as raw protocol lines.
 */
export class SdkworkAgentInternalClient {
  private readonly generatedClient: SdkworkCustomClient;

  public readonly intelligence: SdkworkCustomClient['intelligence'];
  public readonly streaming: AgentInternalStreamingApi;

  public constructor(config: SdkworkCustomConfig) {
    this.generatedClient = createGeneratedInternalClient(config);
    this.intelligence = this.generatedClient.intelligence;
    this.streaming = createAgentInternalStreamingApi(this.generatedClient.http);
  }

  public setApiKey(apiKey: string): this {
    this.generatedClient.setApiKey(apiKey);
    return this;
  }

  public get http(): SdkworkCustomClient['http'] {
    return this.generatedClient.http;
  }

  public get generated(): SdkworkCustomClient {
    return this.generatedClient;
  }
}

export function createClient(config: SdkworkCustomConfig): SdkworkAgentInternalClient {
  return new SdkworkAgentInternalClient(config);
}
