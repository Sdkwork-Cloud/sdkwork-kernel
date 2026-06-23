import { createClient, type SdkworkCustomClient } from '@sdkwork/agent-internal-sdk';
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
  EventSubscription,
  EventSubscriptionOptions
} from '@sdkwork/kernel-ui-types';
import { buildKernelUiAuthHeaders } from './kernel-ui-auth.provider';

export interface KernelUiClientConfig {
  baseUrl: string;
  auth?: KernelUiAuthProvider;
}

export function createKernelUiClient(config: KernelUiClientConfig): KernelUiClient {
  return new InternalSdkKernelUiClient(config);
}

function asKernelView<T>(value: unknown): T {
  return value as T;
}

class InternalSdkKernelUiClient implements KernelUiClient {
  constructor(private config: KernelUiClientConfig) {}

  async loadSnapshot(): Promise<KernelUiSnapshot> {
    const client = await this.buildSdk();
    return asKernelView<KernelUiSnapshot>(await client.intelligence.runtime.snapshot.load());
  }

  async decidePermission(
    permissionRequestId: string,
    decision: PermissionDecisionValue
  ): Promise<PermissionRequestView> {
    const client = await this.buildSdk();
    return asKernelView<PermissionRequestView>(
      await client.intelligence.runtime.permissions.decide(permissionRequestId, {
        decision
      })
    );
  }

  async createSession(config: SessionConfig): Promise<SessionView> {
    const client = await this.buildSdk();
    return asKernelView<SessionView>(
      await client.intelligence.runtime.sessions.create({
      agentId: config.agentId,
      tenantId: config.tenantId,
      userRef: config.userRef,
      model: config.model,
      modelProvider: config.modelProvider,
      title: config.title,
      goal: config.goal,
      instructions: config.instructions,
      cwd: config.cwd,
      workspaceRoots: config.workspaceRoots,
      source: config.source,
      kind: config.kind,
      timeoutMs: config.timeoutMs,
      metadata: config.metadata
      })
    );
  }

  async getSession(sessionId: string): Promise<SessionView> {
    const client = await this.buildSdk();
    return asKernelView<SessionView>(
      await client.intelligence.runtime.sessions.retrieve(sessionId)
    );
  }

  async listSessions(): Promise<SessionView[]> {
    const client = await this.buildSdk();
    const response = await client.intelligence.runtime.sessions.list();
    return asKernelView<SessionView[]>(response.items ?? []);
  }

  async closeSession(sessionId: string): Promise<SessionView> {
    const client = await this.buildSdk();
    return asKernelView<SessionView>(
      await client.intelligence.runtime.sessions.close(sessionId)
    );
  }

  async deleteSession(sessionId: string): Promise<void> {
    const client = await this.buildSdk();
    await client.intelligence.runtime.sessions.delete(sessionId);
  }

  async sendMessage(sessionId: string, content: string): Promise<MessageView> {
    const client = await this.buildSdk();
    return asKernelView<MessageView>(
      await client.intelligence.runtime.sessions.messages.send(sessionId, {
        content
      })
    );
  }

  async getMessages(sessionId: string, limit?: number, offset?: number): Promise<MessageView[]> {
    const client = await this.buildSdk();
    const response = await client.intelligence.runtime.sessions.messages.list(sessionId, {
      limit,
      offset
    });
    return asKernelView<MessageView[]>(response.items ?? []);
  }

  async submitTask(sessionId: string, instruction: string): Promise<TaskView> {
    const client = await this.buildSdk();
    return asKernelView<TaskView>(
      await client.intelligence.runtime.sessions.tasks.submit(sessionId, {
        instruction
      })
    );
  }

  async getTask(taskId: string): Promise<TaskView> {
    const client = await this.buildSdk();
    return asKernelView<TaskView>(await client.intelligence.runtime.tasks.retrieve(taskId));
  }

  async listTasks(sessionId: string): Promise<TaskView[]> {
    const client = await this.buildSdk();
    const response = await client.intelligence.runtime.sessions.tasks.list(sessionId);
    return asKernelView<TaskView[]>(response.items ?? []);
  }

  async cancelTask(taskId: string): Promise<TaskView> {
    const client = await this.buildSdk();
    return asKernelView<TaskView>(await client.intelligence.runtime.tasks.cancel(taskId));
  }

  async listModels(): Promise<ModelDescriptorView[]> {
    const client = await this.buildSdk();
    const response = await client.intelligence.runtime.models.list();
    return (response.items ?? []).map((row) => mapModelDescriptor(row));
  }

  async invokeModel(sessionId: string, modelId?: string): Promise<ModelResponseView> {
    const client = await this.buildSdk();
    return asKernelView<ModelResponseView>(
      await client.intelligence.runtime.sessions.model.invoke(sessionId, {
        modelId
      })
    );
  }

  async listTools(sessionId: string): Promise<ToolDescriptorView[]> {
    const client = await this.buildSdk();
    const response = await client.intelligence.runtime.sessions.tools.list(sessionId);
    return (response.items ?? []).map((row) => mapToolDescriptor(row));
  }

  async executeTool(sessionId: string, toolName: string, args: string): Promise<ToolCallView> {
    const client = await this.buildSdk();
    return asKernelView<ToolCallView>(
      await client.intelligence.runtime.sessions.tools.execute(sessionId, toolName, {
        input: args
      })
    );
  }

  subscribeEvents(
    sessionId: string,
    callback: (event: StreamEventView) => void,
    options?: EventSubscriptionOptions
  ): EventSubscription {
    const controller = new AbortController();

    void (async () => {
      try {
        const client = await this.buildSdk();
        const stream = await client.intelligence.runtime.sessions.events.stream(sessionId, {
          lastEventId: options?.lastEventId,
          live: options?.live
        });
        for await (const event of stream) {
          if (controller.signal.aborted) {
            break;
          }
          callback(asKernelView<StreamEventView>(event));
        }
      } catch (error) {
        if (!controller.signal.aborted) {
          options?.onError?.(
            error instanceof Error ? error : new Error(String(error))
          );
        }
      }
    })();

    return {
      unsubscribe: () => {
        controller.abort();
      }
    };
  }

  private async buildSdk(): Promise<SdkworkCustomClient> {
    const headers = await buildKernelUiAuthHeaders(this.config.auth);

    return createClient({
      baseUrl: this.config.baseUrl,
      headers: Object.keys(headers).length > 0 ? headers : undefined
    });
  }
}

function mapModelDescriptor(row: Record<string, unknown>): ModelDescriptorView {
  return {
    modelId: String(row.modelId ?? ''),
    providerId: String(row.providerId ?? ''),
    displayName: String(row.displayName ?? row.modelId ?? ''),
    family: String(row.family ?? ''),
    capabilities: Array.isArray(row.capabilities) ? row.capabilities.map(String) : []
  };
}

function mapToolDescriptor(row: Record<string, unknown>): ToolDescriptorView {
  return {
    toolId: String(row.toolId ?? ''),
    providerId: String(row.providerId ?? ''),
    name: row.name != null ? String(row.name) : undefined,
    displayName: String(row.displayName ?? row.name ?? row.toolId ?? ''),
    description: row.description != null ? String(row.description) : undefined,
    sideEffectLevel: (row.sideEffectLevel ?? 'read_only') as ToolDescriptorView['sideEffectLevel'],
    policyCategories: Array.isArray(row.policyCategories)
      ? row.policyCategories.map(String)
      : [],
    timeoutMs: typeof row.timeoutMs === 'number' ? row.timeoutMs : undefined
  };
}
