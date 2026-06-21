import type {
  KernelUiClient,
  KernelUiSnapshot,
  KernelUiAuthProvider,
  PermissionDecisionValue,
  PermissionRequestView,
  SessionConfig,
  SessionView,
  MessageView,
  TaskView,
  ModelDescriptorView,
  ModelResponseView,
  ToolDescriptorView,
  ToolCallView,
  StreamEventView,
  EventSubscription
} from '@sdkwork/kernel-ui-types';
import { buildKernelUiAuthHeaders } from './kernel-ui-auth.provider';

export interface KernelUiClientConfig {
  baseUrl: string;
  fetch?: typeof globalThis.fetch;
  auth?: KernelUiAuthProvider;
}

export function createKernelUiClient(config: KernelUiClientConfig): KernelUiClient {
  return new HttpKernelUiClient(config);
}

class HttpKernelUiClient implements KernelUiClient {
  constructor(private config: KernelUiClientConfig) {}

  // =========================================================================
  // Existing
  // =========================================================================

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

  // =========================================================================
  // Session management
  // =========================================================================

  async createSession(config: SessionConfig): Promise<SessionView> {
    const response = await this.request('POST', '/api/kernel/sessions', config);
    return response as SessionView;
  }

  async getSession(sessionId: string): Promise<SessionView> {
    const response = await this.request('GET', `/api/kernel/sessions/${encodeURIComponent(sessionId)}`);
    return response as SessionView;
  }

  async listSessions(): Promise<SessionView[]> {
    const response = await this.request('GET', '/api/kernel/sessions');
    return response as SessionView[];
  }

  async closeSession(sessionId: string): Promise<SessionView> {
    const response = await this.request('POST', `/api/kernel/sessions/${encodeURIComponent(sessionId)}/close`);
    return response as SessionView;
  }

  async deleteSession(sessionId: string): Promise<void> {
    await this.request('DELETE', `/api/kernel/sessions/${encodeURIComponent(sessionId)}`);
  }

  // =========================================================================
  // Message operations
  // =========================================================================

  async sendMessage(sessionId: string, content: string): Promise<MessageView> {
    const response = await this.request(
      'POST',
      `/api/kernel/sessions/${encodeURIComponent(sessionId)}/messages`,
      { content }
    );
    return response as MessageView;
  }

  async getMessages(sessionId: string, limit?: number, offset?: number): Promise<MessageView[]> {
    const params = new URLSearchParams();
    if (limit !== undefined) params.set('limit', String(limit));
    if (offset !== undefined) params.set('offset', String(offset));
    const query = params.toString() ? `?${params.toString()}` : '';

    const response = await this.request(
      'GET',
      `/api/kernel/sessions/${encodeURIComponent(sessionId)}/messages${query}`
    );
    return response as MessageView[];
  }

  // =========================================================================
  // Task operations
  // =========================================================================

  async submitTask(sessionId: string, instruction: string): Promise<TaskView> {
    const response = await this.request(
      'POST',
      `/api/kernel/sessions/${encodeURIComponent(sessionId)}/tasks`,
      { instruction }
    );
    return response as TaskView;
  }

  async getTask(taskId: string): Promise<TaskView> {
    const response = await this.request('GET', `/api/kernel/tasks/${encodeURIComponent(taskId)}`);
    return response as TaskView;
  }

  async listTasks(sessionId: string): Promise<TaskView[]> {
    const response = await this.request(
      'GET',
      `/api/kernel/sessions/${encodeURIComponent(sessionId)}/tasks`
    );
    return response as TaskView[];
  }

  async cancelTask(taskId: string): Promise<TaskView> {
    const response = await this.request(
      'POST',
      `/api/kernel/tasks/${encodeURIComponent(taskId)}/cancel`
    );
    return response as TaskView;
  }

  // =========================================================================
  // Model operations
  // =========================================================================

  async listModels(): Promise<ModelDescriptorView[]> {
    const response = await this.request('GET', '/api/kernel/models');
    return response as ModelDescriptorView[];
  }

  async invokeModel(sessionId: string, modelId?: string): Promise<ModelResponseView> {
    const response = await this.request(
      'POST',
      `/api/kernel/sessions/${encodeURIComponent(sessionId)}/model/invoke`,
      { modelId }
    );
    return response as ModelResponseView;
  }

  // =========================================================================
  // Tool operations
  // =========================================================================

  async listTools(sessionId: string): Promise<ToolDescriptorView[]> {
    const response = await this.request(
      'GET',
      `/api/kernel/sessions/${encodeURIComponent(sessionId)}/tools`
    );
    return response as ToolDescriptorView[];
  }

  async executeTool(sessionId: string, toolName: string, args: string): Promise<ToolCallView> {
    const response = await this.request(
      'POST',
      `/api/kernel/sessions/${encodeURIComponent(sessionId)}/tools/${encodeURIComponent(toolName)}/execute`,
      { input: args }
    );
    return response as ToolCallView;
  }

  // =========================================================================
  // Streaming (fetch + SSE so auth headers are preserved)
  // =========================================================================

  subscribeEvents(sessionId: string, callback: (event: StreamEventView) => void): EventSubscription {
    const controller = new AbortController();

    void (async () => {
      const headers: Record<string, string> = {
        Accept: 'text/event-stream',
        ...(await buildKernelUiAuthHeaders(this.config.auth))
      };
      const url = `${this.config.baseUrl}/api/kernel/sessions/${encodeURIComponent(sessionId)}/events/stream`;
      const f = this.config.fetch ?? globalThis.fetch;

      try {
        const response = await f(url, { headers, signal: controller.signal });
        if (!response.ok || !response.body) {
          return;
        }

        const reader = response.body.getReader();
        const decoder = new TextDecoder();
        let buffer = '';

        while (!controller.signal.aborted) {
          const { done, value } = await reader.read();
          if (done) {
            break;
          }
          buffer += decoder.decode(value, { stream: true });
          buffer = this.consumeSseBuffer(buffer, callback);
        }
      } catch {
        // Ignore abort and transport teardown during unsubscribe.
      }
    })();

    return {
      unsubscribe: () => {
        controller.abort();
      }
    };
  }

  private consumeSseBuffer(buffer: string, callback: (event: StreamEventView) => void): string {
    const frames = buffer.split('\n\n');
    const remainder = frames.pop() ?? '';

    for (const frame of frames) {
      const dataLine = frame
        .split('\n')
        .find((line) => line.startsWith('data:'));
      if (!dataLine) {
        continue;
      }
      const payload = dataLine.slice('data:'.length).trim();
      if (!payload) {
        continue;
      }
      try {
        callback(JSON.parse(payload) as StreamEventView);
      } catch {
        // Ignore malformed frames.
      }
    }

    return remainder;
  }

  // =========================================================================
  // Internal
  // =========================================================================

  private async request(method: string, path: string, body?: unknown): Promise<unknown> {
    const f = this.config.fetch ?? globalThis.fetch;
    const url = `${this.config.baseUrl}${path}`;

    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      Accept: 'application/json',
      ...(await buildKernelUiAuthHeaders(this.config.auth))
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
