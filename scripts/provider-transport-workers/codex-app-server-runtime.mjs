import {
  createCodexAppServerLiveTransport,
  probeCodexAppServer,
} from './codex-app-server-live.mjs';
import {
  buildCodexInteractionResponse,
  normalizeCodexInteractionRequest,
  projectCodexInteractionServerRequest,
} from './codex-app-server-interactions.mjs';
import {
  buildCodexCurrentTimeResponse,
  isCodexCurrentTimeRequest,
} from './codex-app-server-host-requests.mjs';

const activeExecutions = new Map();
const activeProviderSessions = new Map();
const MAX_PRE_TURN_EVENTS = 1_024;
let residentTransport = null;

export class CodexAppServerRuntimeError extends Error {
  constructor(code, message, { cause, fallbackSafe = false } = {}) {
    super(`${code}: ${message}`, cause ? { cause } : undefined);
    this.name = 'CodexAppServerRuntimeError';
    this.code = code;
    this.fallbackSafe = fallbackSafe;
  }
}

export function probeCodexAppServerRuntime(environment = process.env) {
  const probe = probeCodexAppServer(environment);
  return {
    app_server_available: probe.available,
    app_server_mode: probe.mode,
    executable: probe.executable,
  };
}

export function isCodexAppServerFallbackSafe(error) {
  return error instanceof CodexAppServerRuntimeError && error.fallbackSafe === true;
}

export async function invokeCodexAppServerModelChat(operation, options = {}) {
  const modelRequestId = requiredString(operation?.model_request_id, 'model_request_id');
  if (activeExecutions.has(modelRequestId)) {
    throw runtimeError(
      'codex_app_server_duplicate_model_request',
      `model request ${modelRequestId} is already active`,
    );
  }

  const sessionId = optionalString(operation?.session_id, 'session_id');
  const turnId = optionalString(operation?.turn_id ?? operation?.turnId, 'turn_id');
  const requestedProviderSessionId = optionalString(
    operation?.provider_session_id,
    'provider_session_id',
  );
  const entry = {
    activity: options.activity ?? null,
    assistantItems: new Map(),
    chunkSequence: 0,
    eventError: null,
    eventQueue: Promise.resolve(),
    eventSequence: 0,
    interruptPromise: null,
    modelRequestId,
    now: clock(options.now),
    onChunk: callback(options.onChunk, 'onChunk'),
    onEvent: callback(options.onEvent, 'onEvent'),
    pendingServerRequests: new Map(),
    preTurnEvents: [],
    preTurnOverflowed: false,
    providerSessionId: requestedProviderSessionId,
    providerTurnId: null,
    sessionId,
    transport: null,
    turnId,
  };
  activeExecutions.set(modelRequestId, entry);

  let sideEffectsStarted = false;
  let removeNotificationListener = null;
  let removeServerRequestListener = null;
  let removeResolvedListener = null;
  try {
    const transport = await connectResidentTransport();
    entry.transport = transport;

    sideEffectsStarted = true;
    const sessionOptions = { ...(options.sessionOptions ?? {}), sessionId };
    const session = requestedProviderSessionId
      ? await transport.resumeSession({
          ...sessionOptions,
          providerSessionId: requestedProviderSessionId,
        })
      : await transport.startSession(sessionOptions);
    entry.providerSessionId = requiredString(
      session.providerSessionId,
      'provider_session_id',
    );
    if (activeProviderSessions.has(entry.providerSessionId)) {
      throw runtimeError(
        'codex_app_server_provider_session_busy',
        `provider Session ${entry.providerSessionId} already has an active Turn`,
      );
    }
    activeProviderSessions.set(entry.providerSessionId, modelRequestId);
    await entry.activity?.establish(entry.providerSessionId);
    await emitKernelEvent(entry, {
      method: requestedProviderSessionId ? 'session/resumed' : 'session/started',
      params: session.session,
      providerSessionId: entry.providerSessionId,
      turnId: null,
    });

    const handleNotification = (event) => {
      if (!matchesExecution(entry, event)) return;
      assertProviderTurnAffinity(entry, event.turnId);
      enqueue(entry, async () => {
        await observeNotification(entry, event);
        await emitKernelEvent(entry, event);
      });
    };
    const handleServerRequest = (request) => {
      if (!matchesExecution(entry, request)) return;
      assertProviderTurnAffinity(entry, request.turnId);
      if (isCodexCurrentTimeRequest(request)) {
        enqueue(entry, async () => {
          await transport.respondToServerRequest(request, {
            result: buildCodexCurrentTimeResponse(request, { now: entry.now }),
          });
          await entry.activity?.working(entry.providerSessionId);
        });
        return;
      }
      enqueue(entry, async () => {
        const projection = projectCodexInteractionServerRequest(request, {
          modelRequestId: entry.modelRequestId,
          sessionId: entry.sessionId,
        });
        if (projection.disposition === 'automatic_response') {
          await transport.respondToServerRequest(request, { result: projection.result });
          await entry.activity?.working(entry.providerSessionId);
          return;
        }
        const interaction = projection.interaction;
        entry.pendingServerRequests.set(requestKey(request.requestId), request);
        await entry.activity?.waiting(
          entry.providerSessionId,
          interactionHint(request.method),
        );
        await emitKernelEvent(entry, {
          interaction,
          method: request.method,
          params: request.params,
          providerSessionId: request.providerSessionId,
          requestId: request.requestId,
          serverRequest: true,
          turnId: request.turnId,
        });
      });
    };
    removeNotificationListener = transport.onNotification((event) => {
      if (!matchesProviderSession(entry, event)) return;
      if (deferUntilTurnBinding(entry, 'notification', event)) return;
      handleNotification(event);
    });
    removeServerRequestListener = transport.onServerRequest((request) => {
      if (!matchesProviderSession(entry, request)) return;
      if (deferUntilTurnBinding(entry, 'server_request', request)) return;
      handleServerRequest(request);
    });
    const resolvedListener = (event) => {
      if (event.providerSessionId !== entry.providerSessionId) {
        return;
      }
      const request = entry.pendingServerRequests.get(requestKey(event.requestId));
      if (request) {
        entry.pendingServerRequests.delete(requestKey(event.requestId));
      }
      enqueue(entry, async () => {
        await entry.activity?.working(entry.providerSessionId);
      });
    };
    transport.on('serverRequestResolved', resolvedListener);
    removeResolvedListener = () => transport.off('serverRequestResolved', resolvedListener);

    const turn = await transport.startTurn({
      ...(options.turnOptions ?? {}),
      message: options.prompt ?? '',
      providerSessionId: entry.providerSessionId,
      sessionId,
    });
    establishProviderTurnId(entry, turn.turnId);
    for (const deferred of entry.preTurnEvents.splice(0)) {
      // A provider may deliver a late notification or server request from the
      // previous Turn while the new turn/start response is still in flight.
      // Re-check affinity after the authoritative provider Turn id exists;
      // otherwise flushing the buffer could resurrect stale output or expose
      // an interaction belonging to an older Turn.
      if (!matchesExecution(entry, deferred.event)) {
        continue;
      }
      assertProviderTurnAffinity(entry, deferred.event.turnId);
      if (deferred.type === 'notification') {
        handleNotification(deferred.event);
      } else {
        handleServerRequest(deferred.event);
      }
    }
    const completion = await waitForCompletion(entry, turn.completion, operation?.timeout_ms);
    await entry.eventQueue;
    if (entry.eventError) {
      throw entry.eventError;
    }

    const status = String(completion?.params?.turn?.status ?? 'completed').toLowerCase();
    if (status === 'failed') {
      const providerMessage = completion?.params?.turn?.error?.message;
      throw runtimeError(
        'codex_app_server_turn_failed',
        typeof providerMessage === 'string' && providerMessage.trim()
          ? providerMessage.trim()
          : 'Codex Turn failed',
      );
    }
    const messages = [...entry.assistantItems.values()].filter(Boolean);
    return {
      chunks: entry.onChunk ? [] : messages.map((content, sequence) => ({ sequence, content })),
      finish_reason: status === 'interrupted' || status === 'cancelled' ? 'cancelled' : 'stop',
      messages,
      mode: 'app_server',
      model_request_id: modelRequestId,
      ok: true,
      provider_session_id: entry.providerSessionId,
      provider_turn_id: entry.providerTurnId,
    };
  } catch (error) {
    if (!sideEffectsStarted) {
      throw runtimeError(
        'codex_app_server_connect_failed',
        error instanceof Error ? error.message : String(error),
        { cause: error, fallbackSafe: true },
      );
    }
    throw error;
  } finally {
    removeNotificationListener?.();
    removeServerRequestListener?.();
    removeResolvedListener?.();
    activeExecutions.delete(modelRequestId);
    if (entry.providerSessionId
      && activeProviderSessions.get(entry.providerSessionId) === modelRequestId) {
      activeProviderSessions.delete(entry.providerSessionId);
    }
  }
}

export async function respondToCodexAppServerRequest(command = {}) {
  const entry = activeExecution(command);
  assertExecutionAffinity(entry, command);
  const requestId = command.request_id ?? command.requestId;
  const request = entry.pendingServerRequests.get(requestKey(requestId));
  if (!request) {
    throw runtimeError(
      'codex_app_server_unknown_server_request',
      `request ${String(requestId)} is not pending for ${entry.modelRequestId}`,
    );
  }
  const canonicalResolution = command.resolution
    ?? command.interaction_resolution
    ?? command.interactionResolution;
  const interaction = canonicalResolution == null
    ? null
    : normalizeCodexInteractionRequest(request, {
        modelRequestId: entry.modelRequestId,
        sessionId: entry.sessionId,
      });
  const response = Object.hasOwn(command, 'error')
    ? { error: command.error }
    : {
        result: interaction
          ? buildCodexInteractionResponse(interaction, canonicalResolution)
          : Object.hasOwn(command, 'result')
            ? command.result
            : command.response,
      };
  const result = await entry.transport.respondToServerRequest(request, response);
  return {
    ok: true,
    model_request_id: entry.modelRequestId,
    provider_session_id: entry.providerSessionId,
    provider_turn_id: entry.providerTurnId,
    request_id: result.requestId,
    interaction_kind: interaction?.kind ?? null,
    status: result.status,
  };
}

export async function interruptCodexAppServerTurn(command = {}) {
  const entry = activeExecution(command);
  assertExecutionAffinity(entry, command);
  if (!entry.providerTurnId) {
    throw runtimeError(
      'codex_app_server_turn_not_started',
      `model request ${entry.modelRequestId} has not established a provider Turn`,
    );
  }
  if (!entry.interruptPromise) {
    entry.interruptPromise = entry.transport.interruptTurn({
      providerSessionId: entry.providerSessionId,
      sessionId: entry.sessionId,
      turnId: entry.providerTurnId,
    });
  }
  const result = await entry.interruptPromise;
  return {
    accepted: result.accepted === true,
    model_request_id: entry.modelRequestId,
    ok: true,
    provider_session_id: entry.providerSessionId,
    provider_turn_id: entry.providerTurnId,
  };
}

export async function controlCodexAppServerSession(operation = {}) {
  const operationName = requiredString(operation.operation, 'operation');
  if (![
    'session_interrupt',
    'session_compact',
    'session_fork',
  ].includes(operationName)) {
    throw runtimeError(
      'codex_app_server_unsupported_session_control',
      `unsupported Codex Session control operation: ${operationName}`,
    );
  }
  const controlRequestId = requiredString(
    operation.control_request_id,
    'control_request_id',
  );
  const sessionId = requiredString(operation.session_id, 'session_id');
  const providerSessionId = requiredString(
    operation.provider_session_id,
    'provider_session_id',
  );
  const policyDecisionId = requiredString(
    operation.policy_decision_id,
    'policy_decision_id',
  );
  const modelRequestId = optionalString(operation.model_request_id, 'model_request_id');
  const command = {
    ...operation,
    operation: operationName,
    control_request_id: controlRequestId,
    session_id: sessionId,
    provider_session_id: providerSessionId,
    policy_decision_id: policyDecisionId,
    ...(modelRequestId ? { model_request_id: modelRequestId } : {}),
  };

  if (operationName === 'session_compact' && operation.focus != null) {
    requiredString(operation.focus, 'focus');
    throw runtimeError(
      'codex_app_server_unsupported_compact_focus',
      'Codex thread/compact/start does not support a focus parameter',
    );
  }
  if (operationName === 'session_fork' && operation.before_message_id != null) {
    requiredString(operation.before_message_id, 'before_message_id');
    throw runtimeError(
      'codex_app_server_unsupported_fork_boundary',
      'before_message_id cannot be mapped to a Codex Turn id without authoritative identity evidence',
    );
  }

  if (operationName === 'session_interrupt' && modelRequestId) {
    const interrupted = await interruptCodexAppServerTurn({
      model_request_id: modelRequestId,
      provider_session_id: providerSessionId,
      session_id: sessionId,
    });
    return sessionControlResult(command, {
      status: interrupted.accepted ? 'applied' : 'no_op',
      modelRequestId,
    });
  }

  let transport;
  if (modelRequestId) {
    const entry = activeExecution({ model_request_id: modelRequestId });
    assertExecutionAffinity(entry, command);
    transport = entry.transport;
  } else {
    transport = await connectResidentTransport();
  }
  await transport.readSession({ providerSessionId, sessionId });

  if (operationName === 'session_interrupt') {
    return sessionControlResult(command, { status: 'no_op' });
  }
  if (operationName === 'session_compact') {
    await transport.compactSession({ providerSessionId, sessionId });
    return sessionControlResult(command, { status: 'applied', modelRequestId });
  }

  const forked = await transport.forkSession({
    cwd: optionalString(operation.working_directory, 'working_directory'),
    providerSessionId,
    sessionId,
  });
  const forkedProviderSessionId = requiredString(
    forked.providerSessionId,
    'forked_provider_session_id',
  );
  if (forkedProviderSessionId === providerSessionId) {
    throw runtimeError(
      'codex_app_server_fork_identity_mismatch',
      'thread/fork returned the source provider Session id',
    );
  }
  return sessionControlResult(command, {
    forkedProviderSessionId,
    modelRequestId,
    status: 'applied',
  });
}

function sessionControlResult(operation, {
  forkedProviderSessionId = null,
  modelRequestId = null,
  status,
}) {
  return {
    ok: true,
    mode: 'app_server',
    operation: operation.operation,
    control_request_id: operation.control_request_id,
    session_id: operation.session_id,
    provider_session_id: operation.provider_session_id,
    policy_decision_id: operation.policy_decision_id,
    status,
    ...(modelRequestId ? { model_request_id: modelRequestId } : {}),
    ...(forkedProviderSessionId
      ? { forked_provider_session_id: forkedProviderSessionId }
      : {}),
  };
}

export async function closeCodexAppServerRuntime() {
  const transport = residentTransport;
  residentTransport = null;
  if (transport) {
    await transport.close();
  }
}

export function buildCodexAppServerKernelEvent(providerEvent, operation, sequence) {
  const modelRequestId = requiredString(operation?.model_request_id, 'model_request_id');
  const sessionId = optionalString(operation?.session_id, 'session_id');
  const method = String(providerEvent?.method ?? 'unknown').trim() || 'unknown';
  const params = isRecord(providerEvent?.params) ? providerEvent.params : {};
  const providerSessionId = optionalString(
    providerEvent?.providerSessionId ?? params.providerSessionId,
    'provider_session_id',
  );
  const turnId = optionalString(operation?.turn_id ?? operation?.turnId, 'turn_id');
  const providerTurnId = optionalString(
    providerEvent?.turnId ?? params.turn?.id,
    'provider_turn_id',
  );
  const item = isRecord(params.item) ? params.item : null;
  const itemId = optionalString(
    item?.id ?? params.itemId ?? params.callId,
    'provider_item_id',
  );
  const normalizedSequence = Number.isSafeInteger(sequence) && sequence >= 0 ? sequence : 0;
  const interaction = providerEvent?.serverRequest === true
    ? providerEvent.interaction ?? normalizeCodexInteractionRequest(providerEvent, {
        modelRequestId,
        sessionId,
      })
    : null;
  return {
    event_id: `event.${modelRequestId}.${normalizedSequence}`,
    event_type: appServerKernelEventType(method, params, item, interaction),
    event_version: '1.0.0',
    occurred_at: providerEvent?.receivedAt ?? new Date().toISOString(),
    source: appServerKernelEventSource(method, item),
    severity: appServerKernelEventSeverity(method, params),
    session_id: sessionId,
    run_id: modelRequestId,
    step_id: turnId ?? itemId ?? providerTurnId,
    correlation_id: modelRequestId,
    redaction_classification: 'tenant_sensitive',
    payload_schema: 'sdkwork.agent.provider_stream_event.v1',
    payload: {
      schemaVersion: 1,
      providerId: 'codex',
      providerEventType: method,
      providerSessionId,
      providerTurnId,
      providerRequestId: providerEvent?.requestId ?? null,
      sequence: normalizedSequence,
      interaction,
      rawProviderPayload: params,
    },
    replay: false,
  };
}

async function connectResidentTransport() {
  const probe = probeCodexAppServerRuntime();
  if (!probe.app_server_available) {
    throw runtimeError(
      'codex_app_server_unavailable',
      'no real Codex app-server executable was found',
      { fallbackSafe: true },
    );
  }
  if (!residentTransport || ['closed', 'failed'].includes(residentTransport.state)) {
    residentTransport = createCodexAppServerLiveTransport();
  }
  const transport = residentTransport;
  try {
    await transport.connect();
    return transport;
  } catch (error) {
    if (residentTransport === transport) {
      residentTransport = null;
    }
    await transport.close().catch(() => {});
    throw runtimeError(
      'codex_app_server_connect_failed',
      error instanceof Error ? error.message : String(error),
      { cause: error, fallbackSafe: true },
    );
  }
}

async function observeNotification(entry, event) {
  const method = event.method;
  if (method === 'item/agentMessage/delta') {
    const itemId = optionalString(event.params?.itemId, 'provider_item_id') ?? 'agent-message';
    const delta = typeof event.params?.delta === 'string' ? event.params.delta : '';
    if (delta) {
      entry.assistantItems.set(itemId, `${entry.assistantItems.get(itemId) ?? ''}${delta}`);
      if (entry.onChunk) {
        await entry.onChunk({ sequence: entry.chunkSequence, content: delta });
      }
      entry.chunkSequence += 1;
    }
  } else if (method === 'item/completed' && isAgentMessage(event.params?.item)) {
    const itemId = optionalString(event.params.item.id, 'provider_item_id') ?? 'agent-message';
    const text = agentMessageText(event.params.item);
    if (text) {
      entry.assistantItems.set(itemId, text);
    }
  }
  if (method !== 'serverRequest/resolved') {
    await entry.activity?.working(entry.providerSessionId);
  }
}

async function emitKernelEvent(entry, providerEvent) {
  if (!entry.onEvent) {
    return;
  }
  const event = buildCodexAppServerKernelEvent(
    providerEvent,
    {
      model_request_id: entry.modelRequestId,
      session_id: entry.sessionId,
      turn_id: entry.turnId,
    },
    entry.eventSequence,
  );
  entry.eventSequence += 1;
  await entry.onEvent(event);
}

function enqueue(entry, task) {
  entry.eventQueue = entry.eventQueue.then(task).catch((error) => {
    entry.eventError ??= error instanceof Error ? error : new Error(String(error));
  });
}

async function waitForCompletion(entry, completion, timeoutMs) {
  if (timeoutMs == null) {
    return completion;
  }
  if (!Number.isSafeInteger(timeoutMs) || timeoutMs <= 0) {
    throw new Error('timeout_ms must be a positive safe integer');
  }
  let timer;
  try {
    return await Promise.race([
      completion,
      new Promise((_, reject) => {
        timer = setTimeout(
          () => reject(runtimeError(
            'codex_app_server_turn_timeout',
            `Turn ${entry.providerTurnId ?? entry.turnId ?? entry.modelRequestId} exceeded ${timeoutMs} ms`,
          )),
          timeoutMs,
        );
        timer.unref?.();
      }),
    ]);
  } catch (error) {
    if (error?.code === 'codex_app_server_turn_timeout' && entry.providerTurnId) {
      const interrupt = await entry.transport.interruptTurn({
        providerSessionId: entry.providerSessionId,
        sessionId: entry.sessionId,
        turnId: entry.providerTurnId,
      });
      await interrupt.completion;
    }
    throw error;
  } finally {
    clearTimeout(timer);
  }
}

function activeExecution(command) {
  const modelRequestId = requiredString(
    command?.model_request_id ?? command?.modelRequestId,
    'model_request_id',
  );
  const entry = activeExecutions.get(modelRequestId);
  if (!entry) {
    throw runtimeError(
      'codex_app_server_unknown_model_request',
      `model request ${modelRequestId} is not active`,
    );
  }
  return entry;
}

function assertExecutionAffinity(entry, command) {
  const sessionId = optionalString(command.session_id ?? command.sessionId, 'session_id');
  const providerSessionId = optionalString(
    command.provider_session_id ?? command.providerSessionId,
    'provider_session_id',
  );
  const turnId = optionalString(
    command.turn_id ?? command.turnId,
    'turn_id',
  );
  const providerTurnId = optionalString(
    command.provider_turn_id ?? command.providerTurnId,
    'provider_turn_id',
  );
  if (sessionId && sessionId !== entry.sessionId) {
    throw runtimeError('codex_app_server_control_affinity_mismatch', 'Session affinity changed');
  }
  if (providerSessionId && providerSessionId !== entry.providerSessionId) {
    throw runtimeError(
      'codex_app_server_control_affinity_mismatch',
      'provider Session affinity changed',
    );
  }
  if (turnId && turnId !== entry.turnId) {
    throw runtimeError('codex_app_server_control_affinity_mismatch', 'Turn affinity changed');
  }
  if (providerTurnId && providerTurnId !== entry.providerTurnId) {
    throw runtimeError(
      'codex_app_server_control_affinity_mismatch',
      'provider Turn affinity changed',
    );
  }
}

function matchesExecution(entry, event) {
  return matchesProviderSession(entry, event)
    && (!entry.providerTurnId || !event.turnId || event.turnId === entry.providerTurnId);
}

function matchesProviderSession(entry, event) {
  return event.providerSessionId === entry.providerSessionId;
}

function deferUntilTurnBinding(entry, type, event) {
  if (entry.providerTurnId || !event.turnId) return false;
  if (entry.preTurnEvents.length >= MAX_PRE_TURN_EVENTS) {
    if (!entry.preTurnOverflowed) {
      entry.preTurnOverflowed = true;
      enqueue(entry, async () => {
        throw runtimeError(
          'codex_app_server_pre_turn_buffer_overflow',
          `provider emitted more than ${MAX_PRE_TURN_EVENTS} events before Turn binding`,
        );
      });
    }
    return true;
  }
  entry.preTurnEvents.push({ event, type });
  return true;
}

function assertProviderTurnAffinity(entry, candidate) {
  const providerTurnId = optionalString(candidate, 'provider_turn_id');
  if (!providerTurnId) {
    return;
  }
  if (entry.providerTurnId && entry.providerTurnId !== providerTurnId) {
    throw runtimeError(
      'codex_app_server_turn_affinity_mismatch',
      `provider emitted Turn ${providerTurnId} for active provider Turn ${entry.providerTurnId}`,
    );
  }
}

function establishProviderTurnId(entry, candidate) {
  const providerTurnId = requiredString(candidate, 'provider_turn_id');
  assertProviderTurnAffinity(entry, providerTurnId);
  entry.providerTurnId = providerTurnId;
}

function appServerKernelEventType(method, params, item, interaction) {
  if (interaction?.category === 'approval') return 'agent.policy.paused';
  if (interaction) return 'agent.message.paused';
  if (method === 'session/started' || method === 'session/resumed') {
    return `agent.${method}`;
  }
  if (method === 'turn/completed') {
    const status = String(params?.turn?.status ?? '').toLowerCase();
    if (status === 'failed') return 'agent.turn.failed';
    if (status === 'interrupted' || status === 'cancelled') return 'agent.turn.cancelled';
    return 'agent.turn.completed';
  }
  if (method === 'turn/started') return 'agent.turn.started';
  if (method === 'item/agentMessage/delta') return 'agent.message.streamed';
  if (method.startsWith('item/reasoning/')) return 'agent.model.streamed';
  if (method.includes('requestApproval')) return 'agent.policy.paused';
  if (method.includes('requestUserInput') || method.includes('elicitation/request')) {
    return 'agent.message.paused';
  }
  if (method === 'item/started' || method === 'item/completed') {
    return itemEventType(item, method.endsWith('/started') ? 'started' : 'completed');
  }
  if (method.includes('outputDelta') || method.includes('progress')) {
    return 'agent.tool.streamed';
  }
  if (method === 'error') return 'agent.provider.failed';
  return 'agent.provider.updated';
}

function itemEventType(item, action) {
  const type = String(item?.type ?? '').toLowerCase();
  if (type.includes('message')) return `agent.message.${action}`;
  if (type.includes('reasoning') || type.includes('plan')) return `agent.model.${action}`;
  return `agent.tool.${action}`;
}

function appServerKernelEventSource(method, item) {
  if (method.includes('requestApproval')) return 'policy';
  if (method.includes('agentMessage')) return 'model';
  const itemType = String(item?.type ?? '').toLowerCase();
  if (itemType.includes('message') || itemType.includes('reasoning')) return 'model';
  if (method.startsWith('item/') || method.startsWith('mcpServer/')) return 'tool';
  return 'provider';
}

function appServerKernelEventSeverity(method, params) {
  return method === 'error' || String(params?.turn?.status ?? '').toLowerCase() === 'failed'
    ? 'error'
    : 'info';
}

function interactionHint(method) {
  return method.includes('requestApproval') ? 'approval_required' : 'user_input_required';
}

function isAgentMessage(item) {
  const type = String(item?.type ?? '').toLowerCase().replace(/[_-]/gu, '');
  return type === 'agentmessage';
}

function agentMessageText(item) {
  for (const key of ['text', 'content']) {
    if (typeof item?.[key] === 'string' && item[key]) {
      return item[key];
    }
  }
  return '';
}

function requestKey(requestId) {
  if (typeof requestId === 'string' && requestId) return `string:${requestId}`;
  if (typeof requestId === 'number' && Number.isSafeInteger(requestId)) {
    return `number:${requestId}`;
  }
  throw runtimeError('codex_app_server_invalid_request_id', 'request_id is required');
}

function callback(value, name) {
  if (value == null) return null;
  if (typeof value !== 'function') throw new Error(`${name} must be a function`);
  return value;
}

function clock(value) {
  if (value == null) return Date.now;
  if (typeof value !== 'function') throw new Error('now must be a function');
  return value;
}

function optionalString(value, fieldName) {
  if (value == null) return null;
  if (typeof value !== 'string') throw new Error(`${fieldName} must be a string`);
  return value.trim() || null;
}

function requiredString(value, fieldName) {
  const normalized = optionalString(value, fieldName);
  if (!normalized) throw new Error(`${fieldName} is required`);
  return normalized;
}

function isRecord(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function runtimeError(code, message, options) {
  return new CodexAppServerRuntimeError(code, message, options);
}
