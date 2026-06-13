import type {
  KernelUiClient,
  KernelUiSnapshot,
  PermissionDecisionValue,
  PermissionRequestView
} from '@sdkwork/kernel-ui-types';

export interface KernelUiClientConfig {
  baseUrl: string;
  fetch?: typeof globalThis.fetch;
}

export function createKernelUiClient(config: KernelUiClientConfig): KernelUiClient {
  return new HttpKernelUiClient(config);
}

class HttpKernelUiClient implements KernelUiClient {
  constructor(private config: KernelUiClientConfig) {}

  async loadSnapshot(): Promise<KernelUiSnapshot> {
    const response = await this.request('GET', '/api/kernel/snapshot');
    return response as KernelUiSnapshot;
  }

  async decidePermission(
    permissionRequestId: string,
    decision: PermissionDecisionValue
  ): Promise<PermissionRequestView> {
    const response = await this.request(
      'POST',
      `/api/kernel/permissions/${encodeURIComponent(permissionRequestId)}`,
      { decision }
    );
    return response as PermissionRequestView;
  }

  private async request(method: string, path: string, body?: unknown): Promise<unknown> {
    const f = this.config.fetch ?? globalThis.fetch;
    const url = `${this.config.baseUrl}${path}`;

    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      Accept: 'application/json'
    };

    const init: RequestInit = { method, headers };
    if (body !== undefined) {
      init.body = JSON.stringify(body);
    }

    const response = await f(url, init);

    if (!response.ok) {
      const text = await response.text().catch(() => '');
      throw new Error(
        `Kernel UI request failed: ${response.status} ${response.statusText}${text ? ` - ${text}` : ''}`
      );
    }

    return response.json();
  }
}
