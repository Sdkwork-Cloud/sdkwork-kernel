import type {
  KernelUiClient,
  KernelUiSnapshot,
  PermissionDecisionValue,
  PermissionRequestView,
  SessionConfig,
  SessionView,
  SessionState,
  SessionKind,
  SessionSource,
  MessageView,
  TaskView,
  TaskState,
  ModelDescriptorView,
  ModelResponseView,
  ToolDescriptorView,
  ToolCallView,
  StreamEventView,
  EventSubscription,
  EventSubscriptionOptions
} from '@sdkwork/kernel-ui-types';
import { kernelUiMockSnapshot } from './kernel-ui.mock';

let idCounter = 0;
function generateId(): string {
  return `${Date.now()}-${++idCounter}`;
}

export function createMockKernelUiClient(snapshot = kernelUiMockSnapshot): KernelUiClient {
  let currentSnapshot: KernelUiSnapshot = cloneSnapshot(snapshot);
  const sessions = new Map<string, SessionView>();
  const messages = new Map<string, MessageView[]>();
  const tasks = new Map<string, TaskView[]>();
  const eventCallbacks = new Map<string, Set<(event: StreamEventView) => void>>();

  return {
    // =========================================================================
    // Existing
    // =========================================================================

    async loadSnapshot() {
      return cloneSnapshot(currentSnapshot);
    },

    async decidePermission(permissionRequestId: string, decision: PermissionDecisionValue) {
      let updatedPermission: PermissionRequestView | undefined;
      currentSnapshot = {
        ...currentSnapshot,
        permissions: currentSnapshot.permissions.map((permission) => {
          if (permission.permissionRequestId !== permissionRequestId) {
            return permission;
          }
          updatedPermission = { ...permission, status: decision };
          return updatedPermission;
        })
      };

      if (!updatedPermission) {
        throw new Error(`permission request not found: ${permissionRequestId}`);
      }
      return updatedPermission;
    },

    // =========================================================================
    // Session management
    // =========================================================================

    async createSession(config: SessionConfig) {
      const sessionId = `session.${generateId()}`;
      const now = new Date().toISOString();

      const session: SessionView = {
        sessionId,
        agentId: config.agentId,
        tenantId: config.tenantId,
        userRef: config.userRef,
        source: (config.source || 'api') as SessionSource,
        kind: (config.kind || 'main') as SessionKind,
        title: config.title,
        goal: config.goal,
        state: 'active' as SessionState,
        createdAt: now,
        updatedAt: now,
        model: config.model,
        modelProvider: config.modelProvider,
        cwd: config.cwd,
        workspaceRoots: config.workspaceRoots || [],
        instructions: config.instructions,
        tokenUsage: { inputTokens: 0, outputTokens: 0, cachedTokens: 0, reasoningTokens: 0, totalTokens: 0 },
        messageCount: 0,
        toolCallCount: 0,
        compressionCount: 0,
        changeSummary: { additions: 0, deletions: 0, filesChanged: 0 },
        childSessionIds: [],
        timeoutMs: config.timeoutMs,
        metadata: config.metadata || {}
      };

      sessions.set(sessionId, session);
      messages.set(sessionId, []);
      tasks.set(sessionId, []);

      emitEvent(sessionId, {
        eventId: `evt.${generateId()}`,
        eventType: 'session.created',
        sequence: 0,
        payload: JSON.stringify({ sessionId }),
        timestamp: now
      });

      return session;
    },

    async getSession(sessionId: string) {
      const session = sessions.get(sessionId);
      if (!session) throw new Error(`session not found: ${sessionId}`);
      return session;
    },

    async listSessions() {
      return Array.from(sessions.values());
    },

    async closeSession(sessionId: string) {
      const session = sessions.get(sessionId);
      if (!session) throw new Error(`session not found: ${sessionId}`);

      const updated: SessionView = {
        ...session,
        state: 'closed',
        endedAt: new Date().toISOString()
      };
      sessions.set(sessionId, updated);
      return updated;
    },

    async deleteSession(sessionId: string) {
      sessions.delete(sessionId);
      messages.delete(sessionId);
      tasks.delete(sessionId);
    },

    // =========================================================================
    // Message operations
    // =========================================================================

    async sendMessage(sessionId: string, content: string) {
      const session = sessions.get(sessionId);
      if (!session) throw new Error(`session not found: ${sessionId}`);

      const messageId = `msg.${generateId()}`;
      const now = new Date().toISOString();

      const message: MessageView = {
        messageId,
        sessionId,
        role: 'user',
        parts: [{ partId: `part.${generateId()}`, kind: 'text', content }],
        createdAt: now,
        metadata: {}
      };

      const sessionMessages = messages.get(sessionId) || [];
      sessionMessages.push(message);
      messages.set(sessionId, sessionMessages);

      // Update session
      sessions.set(sessionId, {
        ...session,
        messageCount: session.messageCount + 1,
        updatedAt: now
      });

      emitEvent(sessionId, {
        eventId: `evt.${generateId()}`,
        eventType: 'message.sent',
        sequence: 0,
        payload: JSON.stringify({ messageId, sessionId, contentLength: content.length }),
        timestamp: now
      });

      return message;
    },

    async getMessages(sessionId: string, limit?: number, offset?: number) {
      const sessionMessages = messages.get(sessionId) || [];
      const start = offset || 0;
      const end = limit ? start + limit : undefined;
      return sessionMessages.slice(start, end);
    },

    // =========================================================================
    // Task operations
    // =========================================================================

    async submitTask(sessionId: string, instruction: string) {
      const session = sessions.get(sessionId);
      if (!session) throw new Error(`session not found: ${sessionId}`);

      const taskId = `task.${generateId()}`;
      const now = new Date().toISOString();

      const task: TaskView = {
        taskId,
        sessionId,
        instruction,
        state: 'created',
        createdAt: now,
        updatedAt: now
      };

      const sessionTasks = tasks.get(sessionId) || [];
      sessionTasks.push(task);
      tasks.set(sessionId, sessionTasks);

      emitEvent(sessionId, {
        eventId: `evt.${generateId()}`,
        eventType: 'task.submitted',
        sequence: 0,
        payload: JSON.stringify({ taskId, sessionId, instruction }),
        timestamp: now
      });

      return task;
    },

    async getTask(taskId: string) {
      for (const sessionTasks of tasks.values()) {
        const task = sessionTasks.find(t => t.taskId === taskId);
        if (task) return task;
      }
      throw new Error(`task not found: ${taskId}`);
    },

    async listTasks(sessionId: string) {
      return tasks.get(sessionId) || [];
    },

    async cancelTask(taskId: string) {
      for (const [sessionId, sessionTasks] of tasks.entries()) {
        const taskIndex = sessionTasks.findIndex(t => t.taskId === taskId);
        if (taskIndex !== -1) {
          const updated: TaskView = {
            ...sessionTasks[taskIndex],
            state: 'cancelled',
            updatedAt: new Date().toISOString()
          };
          sessionTasks[taskIndex] = updated;
          return updated;
        }
      }
      throw new Error(`task not found: ${taskId}`);
    },

    // =========================================================================
    // Model operations
    // =========================================================================

    async listModels() {
      return [
        { modelId: 'gpt-4', providerId: 'provider.openai', displayName: 'GPT-4', family: 'gpt', contextWindowTokens: 128000, maxOutputTokens: 4096, capabilities: ['model.chat', 'model.stream'] },
        { modelId: 'gpt-3.5-turbo', providerId: 'provider.openai', displayName: 'GPT-3.5 Turbo', family: 'gpt', contextWindowTokens: 16385, maxOutputTokens: 4096, capabilities: ['model.chat', 'model.stream'] },
        { modelId: 'claude-3-opus', providerId: 'provider.anthropic', displayName: 'Claude 3 Opus', family: 'claude', contextWindowTokens: 200000, maxOutputTokens: 4096, capabilities: ['model.chat', 'model.stream'] }
      ];
    },

    async invokeModel(sessionId: string, modelId?: string) {
      const session = sessions.get(sessionId);
      if (!session) throw new Error(`session not found: ${sessionId}`);

      const requestId = `req.${generateId()}`;
      const model = modelId || session.model || 'gpt-4';

      return {
        modelRequestId: requestId,
        providerId: `provider.${model.split('-')[0]}`,
        status: 'succeeded',
        messages: [`Mock response from ${model}`],
        toolCalls: [],
        usage: { inputTokens: 100, outputTokens: 50 }
      };
    },

    // =========================================================================
    // Tool operations
    // =========================================================================

    async listTools(_sessionId: string) {
      return [
        { toolId: 'tool.bash', providerId: 'provider.tool.builtin', name: 'bash', displayName: 'Bash', description: 'Execute shell commands', sideEffectLevel: 'side_effectful', policyCategories: ['host.process.execute'], timeoutMs: 30000 },
        { toolId: 'tool.read_file', providerId: 'provider.tool.builtin', name: 'read_file', displayName: 'Read File', description: 'Read file contents', sideEffectLevel: 'read_only', policyCategories: [] },
        { toolId: 'tool.write_file', providerId: 'provider.tool.builtin', name: 'write_file', displayName: 'Write File', description: 'Write file contents', sideEffectLevel: 'side_effectful', policyCategories: ['host.filesystem.write'] }
      ];
    },

    async executeTool(_sessionId: string, toolName: string, args: string) {
      const callId = `call.${generateId()}`;
      return {
        toolCallId: callId,
        toolId: toolName,
        input: args,
        status: 'succeeded',
        output: `Mock output from ${toolName}`,
        durationMs: 100
      };
    },

    // =========================================================================
    // Streaming
    // =========================================================================

    subscribeEvents(
      sessionId: string,
      callback: (event: StreamEventView) => void,
      _options?: EventSubscriptionOptions
    ) {
      if (!eventCallbacks.has(sessionId)) {
        eventCallbacks.set(sessionId, new Set());
      }
      eventCallbacks.get(sessionId)!.add(callback);

      return {
        unsubscribe: () => {
          eventCallbacks.get(sessionId)?.delete(callback);
        }
      };
    }
  };

  function emitEvent(sessionId: string, event: StreamEventView) {
    eventCallbacks.get(sessionId)?.forEach(cb => cb(event));
  }
}

function cloneSnapshot(snapshot: KernelUiSnapshot): KernelUiSnapshot {
  return JSON.parse(JSON.stringify(snapshot)) as KernelUiSnapshot;
}
