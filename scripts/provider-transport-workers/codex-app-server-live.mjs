#!/usr/bin/env node
import { spawn as spawnProcess } from 'node:child_process';
import { EventEmitter } from 'node:events';

import {
  probeCodexCli,
  resolveLaunchCommand,
  terminateProcessTree,
} from './codex-cli-live.mjs';

const DEFAULT_REQUEST_TIMEOUT_MS = 30_000;
const DEFAULT_CLOSE_TIMEOUT_MS = 2_000;
const DEFAULT_MAX_LINE_BYTES = 8 * 1024 * 1024;
const MAX_LEDGER_ENTRIES = 512;
const WIRE_SESSION_START_METHOD = 'thread/start';
const WIRE_SESSION_RESUME_METHOD = 'thread/resume';
const WIRE_SESSION_READ_METHOD = 'thread/read';
const WIRE_SESSION_COMPACT_METHOD = 'thread/compact/start';
const WIRE_SESSION_FORK_METHOD = 'thread/fork';
const WIRE_TURN_START_METHOD = 'turn/start';
const WIRE_TURN_INTERRUPT_METHOD = 'turn/interrupt';
const WIRE_REQUEST_RESOLVED_METHOD = 'serverRequest/resolved';
const WIRE_INITIALIZE_METHOD = 'initialize';
const WIRE_INITIALIZED_METHOD = 'initialized';

const SESSION_START_KEYS = [
  'allowProviderModelFallback',
  'approvalPolicy',
  'approvalsReviewer',
  'baseInstructions',
  'config',
  'cwd',
  'developerInstructions',
  'dynamicTools',
  'environments',
  'ephemeral',
  'experimentalRawEvents',
  'historyMode',
  'model',
  'modelProvider',
  'multiAgentMode',
  'permissions',
  'personality',
  'runtimeWorkspaceRoots',
  'sandbox',
  'selectedCapabilityRoots',
  'serviceName',
  'serviceTier',
  'sessionStartSource',
];

const SESSION_RESUME_KEYS = [
  'approvalPolicy',
  'approvalsReviewer',
  'baseInstructions',
  'config',
  'cwd',
  'developerInstructions',
  'excludeTurns',
  'history',
  'initialTurnsPage',
  'model',
  'modelProvider',
  'path',
  'permissions',
  'personality',
  'runtimeWorkspaceRoots',
  'sandbox',
  'serviceTier',
];

const TURN_START_KEYS = [
  'additionalContext',
  'approvalPolicy',
  'approvalsReviewer',
  'clientUserMessageId',
  'collaborationMode',
  'cwd',
  'effort',
  'environments',
  'model',
  'multiAgentMode',
  'outputSchema',
  'permissions',
  'personality',
  'responsesapiClientMetadata',
  'runtimeWorkspaceRoots',
  'sandboxPolicy',
  'serviceTier',
  'summary',
];

let connectionSequence = 0;

export class CodexAppServerTransportError extends Error {
  constructor(code, message, details = undefined, cause = undefined) {
    super(`${code}: ${message}`, cause ? { cause } : undefined);
    this.name = 'CodexAppServerTransportError';
    this.code = code;
    this.details = details;
  }
}

export function buildCodexAppServerArgs() {
  return ['app-server', '--listen', 'stdio://'];
}

export function probeCodexAppServer(environment = process.env) {
  const probe = probeCodexCli(environment);
  return {
    available: probe.available,
    executable: probe.executable,
    mode: probe.available ? 'app_server' : null,
  };
}

export function createCodexAppServerTransport(options = {}) {
  return new CodexAppServerLiveTransport(options);
}

export function createCodexAppServerLiveTransport(options = {}) {
  return createCodexAppServerTransport(options);
}

export class CodexAppServerLiveTransport extends EventEmitter {
  #options;
  #state = 'disconnected';
  #child = null;
  #connectPromise = null;
  #exitPromise = null;
  #resolveExit = null;
  #connectionId = null;
  #requestSequence = 0;
  #requestLedger = new Map();
  #pendingRequests = new Map();
  #serverRequestLedger = new Map();
  #serverRequestWaiters = new Set();
  #turnLedger = new Map();
  #stdoutDecoder = new TextDecoder();
  #stdoutBuffer = '';
  #stderr = '';
  #initializeResult = null;
  #terminalError = null;

  constructor(options = {}) {
    super();
    if (!isRecord(options)) {
      throw transportError('codex_app_server_invalid_options', 'options must be an object');
    }
    this.#options = {
      args: options.args ?? options.commandArgs ?? null,
      capabilities: options.capabilities ?? null,
      clientInfo: normalizeClientInfo(options.clientInfo),
      closeTimeoutMs: positiveInteger(
        options.closeTimeoutMs,
        DEFAULT_CLOSE_TIMEOUT_MS,
        'closeTimeoutMs',
      ),
      cwd: options.cwd ?? process.cwd(),
      env: options.env ?? process.env,
      executable: options.executable ?? options.command ?? null,
      maxLineBytes: positiveInteger(
        options.maxLineBytes,
        DEFAULT_MAX_LINE_BYTES,
        'maxLineBytes',
      ),
      requestTimeoutMs: positiveInteger(
        options.requestTimeoutMs,
        DEFAULT_REQUEST_TIMEOUT_MS,
        'requestTimeoutMs',
      ),
      spawn: options.spawn ?? spawnProcess,
    };
    if (typeof this.#options.spawn !== 'function') {
      throw transportError('codex_app_server_invalid_options', 'spawn must be a function');
    }
    if (this.#options.args != null && !Array.isArray(this.#options.args)) {
      throw transportError('codex_app_server_invalid_options', 'args must be an array');
    }
    if (this.#options.capabilities != null && !isRecord(this.#options.capabilities)) {
      throw transportError(
        'codex_app_server_invalid_options',
        'capabilities must be an object',
      );
    }
  }

  get state() {
    return this.#state;
  }

  get pid() {
    return this.#child?.pid ?? null;
  }

  get connectionId() {
    return this.#connectionId;
  }

  get initializeResult() {
    return this.#initializeResult;
  }

  get isReady() {
    return this.#state === 'ready';
  }

  async connect() {
    if (this.#state === 'ready') {
      return this.#initializeResult;
    }
    if (this.#connectPromise) {
      return this.#connectPromise;
    }
    if (this.#state === 'closing' || this.#state === 'closed') {
      throw transportError('codex_app_server_closed', 'transport is closed');
    }
    if (this.#state === 'failed') {
      throw this.#terminalError ?? transportError(
        'codex_app_server_failed',
        'transport is in a failed state',
      );
    }

    this.#connectPromise = this.#openConnection();
    try {
      return await this.#connectPromise;
    } finally {
      if (this.#state !== 'connecting') {
        this.#connectPromise = null;
      }
    }
  }

  async startSession(options = {}) {
    const command = normalizeObjectCommand(options, 'startSession options');
    const params = buildSessionStartParams(command);
    await this.connect();
    const response = await this.#sendRequest(
      WIRE_SESSION_START_METHOD,
      params,
    );
    return this.#normalizeSessionResult(response, command.sessionId, false, null);
  }

  async resumeSession(commandOrProviderSessionId, options = {}) {
    const command = normalizeProviderSessionCommand(
      commandOrProviderSessionId,
      options,
      'resumeSession',
    );
    const params = buildSessionResumeParams(command);
    params.threadId = command.providerSessionId;
    await this.connect();
    const response = await this.#sendRequest(WIRE_SESSION_RESUME_METHOD, params);
    return this.#normalizeSessionResult(
      response,
      command.sessionId,
      true,
      command.providerSessionId,
    );
  }

  async readSession(commandOrProviderSessionId, options = {}) {
    const command = normalizeProviderSessionCommand(
      commandOrProviderSessionId,
      options,
      'readSession',
    );
    const includeTurns = command.includeTurns ?? false;
    if (typeof includeTurns !== 'boolean') {
      throw transportError(
        'codex_app_server_invalid_command',
        'readSession includeTurns must be a boolean',
      );
    }
    await this.connect();
    const response = await this.#sendRequest(WIRE_SESSION_READ_METHOD, {
      threadId: command.providerSessionId,
      includeTurns,
    });
    return this.#normalizeSessionResult(
      response,
      command.sessionId,
      true,
      command.providerSessionId,
    );
  }

  async compactSession(commandOrProviderSessionId, options = {}) {
    const command = normalizeProviderSessionCommand(
      commandOrProviderSessionId,
      options,
      'compactSession',
    );
    await this.connect();
    await this.#sendRequest(WIRE_SESSION_COMPACT_METHOD, {
      threadId: command.providerSessionId,
    });
    return Object.freeze({
      compacted: true,
      providerSessionId: command.providerSessionId,
      sessionId: optionalNonBlankString(command.sessionId, 'sessionId'),
    });
  }

  async forkSession(commandOrProviderSessionId, options = {}) {
    const command = normalizeProviderSessionCommand(
      commandOrProviderSessionId,
      options,
      'forkSession',
    );
    const params = { threadId: command.providerSessionId };
    const cwd = optionalNonBlankString(command.cwd, 'cwd');
    if (cwd) {
      params.cwd = cwd;
    }
    await this.connect();
    const response = await this.#sendRequest(WIRE_SESSION_FORK_METHOD, params);
    const forked = this.#normalizeSessionResult(response, command.sessionId, false, null);
    if (forked.providerSessionId === command.providerSessionId) {
      const error = transportError(
        'codex_app_server_fork_identity_mismatch',
        'thread/fork returned the source provider Session id',
      );
      this.#protocolViolation(error);
      throw error;
    }
    return Object.freeze({
      ...forked,
      sourceProviderSessionId: command.providerSessionId,
    });
  }

  async startTurn(commandOrProviderSessionId, options = {}) {
    const command = normalizeTurnCommand(commandOrProviderSessionId, options);
    const params = buildTurnStartParams(command);
    params.threadId = command.providerSessionId;
    await this.connect();
    const response = await this.#sendRequest(WIRE_TURN_START_METHOD, params);
    const wireTurn = response?.turn;
    let turnId;
    try {
      turnId = requireNonBlankString(
        wireTurn?.id,
        'codex_app_server_invalid_turn_response',
        'turn/start response did not include a provider turn id',
      );
    } catch (error) {
      const wrapped = asTransportError(error, 'codex_app_server_invalid_turn_response');
      this.#protocolViolation(wrapped);
      throw wrapped;
    }
    const tracker = this.#ensureTurnTracker(command.providerSessionId, turnId);
    return Object.freeze({
      providerSessionId: command.providerSessionId,
      sessionId: optionalNonBlankString(command.sessionId, 'sessionId'),
      turn: normalizeProviderWireValue(wireTurn),
      turnId,
      completion: tracker.promise,
    });
  }

  async interruptTurn(commandOrProviderSessionId, turnIdOrOptions, options = {}) {
    const command = normalizeInterruptCommand(
      commandOrProviderSessionId,
      turnIdOrOptions,
      options,
    );
    await this.connect();
    await this.#sendRequest(WIRE_TURN_INTERRUPT_METHOD, {
      threadId: command.providerSessionId,
      turnId: command.turnId,
    });
    const tracker = this.#ensureTurnTracker(command.providerSessionId, command.turnId);
    return Object.freeze({
      accepted: true,
      providerSessionId: command.providerSessionId,
      sessionId: optionalNonBlankString(command.sessionId, 'sessionId'),
      turnId: command.turnId,
      completion: tracker.promise,
    });
  }

  async waitForTurnCompletion(turnOrId, options = {}) {
    const { providerSessionId, turnId } = normalizeTurnWait(turnOrId, options);
    const tracker = this.#findTurnTracker(providerSessionId, turnId);
    if (!tracker) {
      throw transportError(
        'codex_app_server_unknown_turn',
        `no observed turn matches ${turnId}`,
      );
    }
    return withTimeout(
      tracker.promise,
      positiveInteger(options.timeoutMs, this.#options.requestTimeoutMs, 'timeoutMs'),
      () => transportError(
        'codex_app_server_turn_timeout',
        `turn ${turnId} did not complete before the timeout`,
      ),
    );
  }

  async waitForServerRequest(criteria = {}) {
    const normalized = normalizeServerRequestCriteria(criteria);
    if (this.#state === 'failed') {
      throw this.#terminalError ?? transportError(
        'codex_app_server_failed',
        'transport is in a failed state',
      );
    }
    if (this.#state === 'closing' || this.#state === 'closed') {
      throw transportError('codex_app_server_closed', 'transport is closed');
    }
    const existing = [...this.#serverRequestLedger.values()].find((record) =>
      record.status === 'pending' && serverRequestMatches(record.event, normalized),
    );
    if (existing) {
      return existing.event;
    }

    return new Promise((resolve, reject) => {
      const waiter = {
        onRequest: null,
        reject: null,
        timer: null,
      };
      const cleanup = () => {
        clearTimeout(waiter.timer);
        this.off('serverRequest', waiter.onRequest);
        this.#serverRequestWaiters.delete(waiter);
      };
      waiter.onRequest = (event) => {
        if (!serverRequestMatches(event, normalized)) {
          return;
        }
        cleanup();
        resolve(event);
      };
      waiter.reject = (error) => {
        cleanup();
        reject(error);
      };
      waiter.timer = setTimeout(() => {
        cleanup();
        reject(transportError(
          'codex_app_server_server_request_timeout',
          'no matching server request arrived before the timeout',
        ));
      }, normalized.timeoutMs ?? this.#options.requestTimeoutMs);
      waiter.timer.unref?.();
      this.#serverRequestWaiters.add(waiter);
      this.on('serverRequest', waiter.onRequest);
    });
  }

  async respondToServerRequest(requestOrId, response, affinity = {}) {
    const command = normalizeServerRequestResponse(requestOrId, response, affinity);
    const key = wireIdKey(command.requestId);
    const record = this.#serverRequestLedger.get(key);
    if (!record) {
      throw transportError(
        'codex_app_server_unknown_server_request',
        `server request ${String(command.requestId)} does not belong to this connection`,
      );
    }
    assertServerRequestAffinity(record, command.affinity, this.#connectionId);
    if (record.status !== 'pending') {
      throw transportError(
        'codex_app_server_server_request_already_settled',
        `server request ${String(command.requestId)} is already ${record.status}`,
        { status: record.status },
      );
    }

    const outbound = command.error
      ? { id: record.id, error: normalizeRpcError(command.error) }
      : { id: record.id, result: command.result };
    record.status = 'responding';
    record.respondingAt = new Date().toISOString();
    record.responseKind = command.error ? 'error' : 'result';
    try {
      await this.#writeWire(outbound);
      record.respondedAt = new Date().toISOString();
      if (record.status === 'responding') {
        record.status = 'responded';
      }
    } catch (error) {
      if (record.status === 'responding') {
        record.status = 'writeFailed';
      }
      throw error;
    }
    return Object.freeze({
      requestId: record.id,
      status: record.status,
      providerSessionId: record.event.providerSessionId,
      turnId: record.event.turnId,
    });
  }

  onNotification(listener) {
    assertListener(listener);
    this.on('notification', listener);
    return () => this.off('notification', listener);
  }

  onServerRequest(listener) {
    assertListener(listener);
    this.on('serverRequest', listener);
    return () => this.off('serverRequest', listener);
  }

  getRequestLedgerSnapshot() {
    return [...this.#requestLedger.values()].map((record) => Object.freeze({
      completedAt: record.completedAt ?? null,
      id: record.id,
      method: canonicalMethod(record.method),
      sentAt: record.sentAt,
      status: record.status,
    }));
  }

  getServerRequestLedgerSnapshot() {
    return [...this.#serverRequestLedger.values()].map((record) => Object.freeze({
      cancelledAt: record.cancelledAt ?? null,
      connectionId: record.connectionId,
      method: record.method,
      providerSessionId: record.event.providerSessionId,
      receivedAt: record.receivedAt,
      requestId: record.id,
      providerClearedAt: record.providerClearedAt ?? null,
      resolvedAt: record.resolvedAt ?? null,
      resolutionUnknownAt: record.resolutionUnknownAt ?? null,
      respondedAt: record.respondedAt ?? null,
      respondingAt: record.respondingAt ?? null,
      status: record.status,
      turnId: record.event.turnId,
    }));
  }

  async close() {
    if (this.#state === 'closed') {
      return;
    }
    const closeError = transportError(
      'codex_app_server_closed',
      'app-server transport was closed',
    );
    this.#rejectInFlight(closeError, 'closed');
    if (!this.#child) {
      this.#state = 'closed';
      return;
    }

    this.#state = 'closing';
    const child = this.#child;
    try {
      if (child.stdin.writable) {
        child.stdin.end();
      }
    } catch {
      terminateProcessTree(child);
    }

    const exited = await Promise.race([
      this.#exitPromise.then(() => true),
      new Promise((resolve) => {
        const timer = setTimeout(() => resolve(false), this.#options.closeTimeoutMs);
        timer.unref?.();
      }),
    ]);
    if (!exited && this.#child) {
      terminateProcessTree(this.#child);
      await this.#exitPromise;
    }
    this.#state = 'closed';
  }

  async [Symbol.asyncDispose]() {
    await this.close();
  }

  async #openConnection() {
    this.#state = 'connecting';
    this.#terminalError = null;
    this.#stdoutDecoder = new TextDecoder();
    this.#stdoutBuffer = '';
    this.#stderr = '';
    this.#connectionId = `${process.pid}:${++connectionSequence}`;

    let launch;
    try {
      launch = resolveAppServerLaunch(this.#options);
      this.#child = this.#options.spawn(launch.command, launch.args, {
        cwd: this.#options.cwd,
        env: this.#options.env,
        windowsHide: true,
        windowsVerbatimArguments: launch.windowsVerbatimArguments ?? false,
        stdio: ['pipe', 'pipe', 'pipe'],
      });
    } catch (error) {
      const wrapped = transportError(
        'codex_app_server_spawn_failed',
        error instanceof Error ? error.message : String(error),
        undefined,
        error,
      );
      this.#fail(wrapped, false);
      throw wrapped;
    }

    this.#exitPromise = new Promise((resolve) => {
      this.#resolveExit = resolve;
    });
    this.#attachProcessListeners(this.#child);

    try {
      const params = { clientInfo: this.#options.clientInfo };
      if (this.#options.capabilities) {
        params.capabilities = this.#options.capabilities;
      }
      const result = await this.#sendRequest(WIRE_INITIALIZE_METHOD, params, true);
      await this.#writeWire({ method: WIRE_INITIALIZED_METHOD });
      if (this.#state !== 'connecting') {
        throw this.#terminalError ?? transportError(
          'codex_app_server_connection_failed',
          'connection left the initializing state during the handshake',
        );
      }
      this.#initializeResult = normalizeProviderWireValue(result);
      this.#state = 'ready';
      this.emit('ready', this.#initializeResult);
      return this.#initializeResult;
    } catch (error) {
      const wrapped = asTransportError(error, 'codex_app_server_initialize_failed');
      this.#fail(wrapped, true);
      throw wrapped;
    }
  }

  #attachProcessListeners(child) {
    child.stdout.on('data', (chunk) => this.#handleStdoutData(chunk));
    child.stderr.on('data', (chunk) => {
      if (this.#stderr.length < 64 * 1024) {
        this.#stderr += String(chunk).slice(0, 64 * 1024 - this.#stderr.length);
      }
      this.emit('stderr', String(chunk));
    });
    child.stdin.on('error', (error) => {
      if (this.#state !== 'closing' && this.#state !== 'closed') {
        this.#fail(transportError(
          'codex_app_server_stdin_failed',
          error.message,
          undefined,
          error,
        ), true);
      }
    });
    child.once('error', (error) => {
      this.#fail(transportError(
        'codex_app_server_spawn_failed',
        error.message,
        undefined,
        error,
      ), false);
      this.#resolveExit?.({ code: null, signal: null, error });
    });
    child.once('close', (code, signal) => this.#handleProcessClose(code, signal));
  }

  #handleProcessClose(code, signal) {
    const expected = this.#state === 'closing' || this.#state === 'closed';
    const exit = { code, signal };
    this.#child = null;
    this.#resolveExit?.(exit);
    this.#resolveExit = null;
    if (expected) {
      this.#state = 'closed';
    } else if (this.#state !== 'failed') {
      const status = code == null ? `signal ${signal ?? 'unknown'}` : `status ${code}`;
      const detail = this.#stderr.trim();
      this.#fail(transportError(
        'codex_app_server_process_exited',
        `app-server exited with ${status}${detail ? `: ${detail}` : ''}`,
        exit,
      ), false);
    }
    this.emit('close', Object.freeze({ ...exit, expected }));
  }

  #handleStdoutData(chunk) {
    if (this.#state === 'failed' || this.#state === 'closed') {
      return;
    }
    this.#stdoutBuffer += this.#stdoutDecoder.decode(chunk, { stream: true });

    let newline = this.#stdoutBuffer.indexOf('\n');
    while (newline >= 0) {
      const line = this.#stdoutBuffer.slice(0, newline).replace(/\r$/u, '');
      this.#stdoutBuffer = this.#stdoutBuffer.slice(newline + 1);
      if (Buffer.byteLength(line, 'utf8') > this.#options.maxLineBytes) {
        this.#protocolViolation(transportError(
          'codex_app_server_line_too_large',
          `app-server emitted a line larger than ${this.#options.maxLineBytes} bytes`,
        ));
        return;
      }
      if (line.trim()) {
        this.#handleWireLine(line);
        if (this.#state === 'failed') {
          return;
        }
      }
      newline = this.#stdoutBuffer.indexOf('\n');
    }
    if (Buffer.byteLength(this.#stdoutBuffer, 'utf8') > this.#options.maxLineBytes) {
      this.#protocolViolation(transportError(
        'codex_app_server_line_too_large',
        `app-server emitted a line larger than ${this.#options.maxLineBytes} bytes`,
      ));
    }
  }

  #handleWireLine(line) {
    let message;
    try {
      message = JSON.parse(line);
    } catch (error) {
      this.#protocolViolation(transportError(
        'codex_app_server_invalid_json',
        error instanceof Error ? error.message : String(error),
        undefined,
        error,
      ));
      return;
    }
    if (!isRecord(message)) {
      this.#protocolViolation(transportError(
        'codex_app_server_invalid_message',
        'app-server messages must be JSON objects',
      ));
      return;
    }

    const hasId = Object.prototype.hasOwnProperty.call(message, 'id');
    const hasMethod = typeof message.method === 'string' && message.method.length > 0;
    if (hasMethod && hasId) {
      this.#handleServerRequest(message);
      return;
    }
    if (hasMethod) {
      this.#handleNotification(message);
      return;
    }
    if (hasId) {
      this.#handleResponse(message);
      return;
    }
    this.#protocolViolation(transportError(
      'codex_app_server_invalid_message',
      'app-server message had neither a method nor a response id',
    ));
  }

  #handleResponse(message) {
    let key;
    try {
      key = wireIdKey(message.id);
    } catch (error) {
      this.#protocolViolation(asTransportError(error, 'codex_app_server_invalid_response_id'));
      return;
    }
    const pending = this.#pendingRequests.get(key);
    if (!pending) {
      this.#protocolViolation(transportError(
        'codex_app_server_response_id_mismatch',
        `received a response for unknown request id ${String(message.id)}`,
        { pendingIds: [...this.#pendingRequests.values()].map((entry) => entry.id) },
      ));
      return;
    }
    if (Object.prototype.hasOwnProperty.call(message, 'result')
      && Object.prototype.hasOwnProperty.call(message, 'error')) {
      this.#protocolViolation(transportError(
        'codex_app_server_invalid_response',
        `response ${String(message.id)} included both result and error`,
      ));
      return;
    }
    if (!Object.prototype.hasOwnProperty.call(message, 'result')
      && !Object.prototype.hasOwnProperty.call(message, 'error')) {
      this.#protocolViolation(transportError(
        'codex_app_server_invalid_response',
        `response ${String(message.id)} included neither result nor error`,
      ));
      return;
    }

    clearTimeout(pending.timer);
    this.#pendingRequests.delete(key);
    pending.record.completedAt = new Date().toISOString();
    if (Object.prototype.hasOwnProperty.call(message, 'error')) {
      pending.record.status = 'failed';
      pending.reject(transportError(
        'codex_app_server_request_failed',
        rpcErrorMessage(message.error),
        { method: canonicalMethod(pending.method), providerError: message.error },
      ));
    } else {
      pending.record.status = 'completed';
      pending.resolve(message.result);
    }
    trimSettledLedger(this.#requestLedger);
  }

  #handleServerRequest(message) {
    let key;
    try {
      key = wireIdKey(message.id);
    } catch (error) {
      this.#protocolViolation(asTransportError(error, 'codex_app_server_invalid_server_request_id'));
      return;
    }
    if (this.#serverRequestLedger.has(key)) {
      this.#protocolViolation(transportError(
        'codex_app_server_duplicate_server_request',
        `server request id ${String(message.id)} was reused on one connection`,
      ));
      return;
    }

    const rawProviderSessionId = readWireProviderSessionId(message.params);
    const turnId = readWireTurnId(message.params);
    const event = Object.freeze({
      affinity: Object.freeze({
        connectionId: this.#connectionId,
        providerSessionId: rawProviderSessionId,
        requestId: message.id,
        turnId,
      }),
      connectionId: this.#connectionId,
      method: message.method,
      params: normalizeProviderWireValue(message.params ?? {}),
      providerSessionId: rawProviderSessionId,
      receivedAt: new Date().toISOString(),
      requestId: message.id,
      turnId,
    });
    this.#serverRequestLedger.set(key, {
      connectionId: this.#connectionId,
      event,
      id: message.id,
      method: message.method,
      rawProviderSessionId,
      receivedAt: event.receivedAt,
      status: 'pending',
      turnId,
    });
    trimServerRequestLedger(this.#serverRequestLedger);
    this.emit('serverRequest', event);
  }

  #handleNotification(message) {
    if (message.method === WIRE_REQUEST_RESOLVED_METHOD) {
      this.#reconcileResolvedServerRequest(message.params ?? {});
    }

    const providerSessionId = readWireProviderSessionId(message.params);
    const turnId = readWireTurnId(message.params);
    const event = Object.freeze({
      method: canonicalMethod(message.method),
      params: normalizeProviderWireValue(message.params ?? {}),
      providerSessionId,
      receivedAt: new Date().toISOString(),
      turnId,
    });
    this.#recordTurnNotification(message.method, providerSessionId, turnId, event);
    this.emit('notification', event);
  }

  #reconcileResolvedServerRequest(params) {
    let key;
    try {
      key = wireIdKey(params.requestId);
    } catch {
      this.#protocolViolation(transportError(
        'codex_app_server_invalid_resolved_request',
        'serverRequest/resolved did not include a valid request id',
      ));
      return;
    }
    const record = this.#serverRequestLedger.get(key);
    if (!record) {
      this.emit('serverRequestResolved', Object.freeze({
        known: false,
        providerSessionId: readWireProviderSessionId(params),
        requestId: params.requestId,
      }));
      return;
    }
    const providerSessionId = readWireProviderSessionId(params);
    if (record.rawProviderSessionId && providerSessionId
      && record.rawProviderSessionId !== providerSessionId) {
      this.#protocolViolation(transportError(
        'codex_app_server_server_request_affinity_mismatch',
        `resolved request ${String(record.id)} changed provider session affinity`,
      ));
      return;
    }
    if (record.status === 'resolved' || record.status === 'providerCleared') {
      return;
    }
    const providerClearedAt = new Date().toISOString();
    if (record.status === 'responded' || record.status === 'resolutionUnknown') {
      record.status = 'resolved';
      record.resolvedAt = providerClearedAt;
    } else {
      record.status = 'providerCleared';
      record.providerClearedAt = providerClearedAt;
    }
    this.emit('serverRequestResolved', Object.freeze({
      known: true,
      providerSessionId: record.event.providerSessionId,
      requestId: record.id,
      status: record.status,
      turnId: record.event.turnId,
    }));
  }

  #recordTurnNotification(method, providerSessionId, turnId, event) {
    if (!providerSessionId || !turnId) {
      return;
    }
    const tracker = this.#ensureTurnTracker(providerSessionId, turnId);
    tracker.events.push(event);
    if (method !== 'turn/completed' || tracker.status === 'completed') {
      return;
    }
    tracker.status = 'completed';
    tracker.completedAt = event.receivedAt;
    tracker.resolve(event);
    trimTurnLedger(this.#turnLedger);
  }

  #ensureTurnTracker(providerSessionId, turnId) {
    const key = turnLedgerKey(providerSessionId, turnId);
    const existing = this.#turnLedger.get(key);
    if (existing) {
      return existing;
    }
    let resolve;
    let reject;
    const promise = new Promise((resolvePromise, rejectPromise) => {
      resolve = resolvePromise;
      reject = rejectPromise;
    });
    promise.catch(() => {});
    const tracker = {
      completedAt: null,
      events: [],
      promise,
      providerSessionId,
      reject,
      resolve,
      status: 'active',
      turnId,
    };
    this.#turnLedger.set(key, tracker);
    return tracker;
  }

  #findTurnTracker(providerSessionId, turnId) {
    if (providerSessionId) {
      return this.#turnLedger.get(turnLedgerKey(providerSessionId, turnId)) ?? null;
    }
    const matches = [...this.#turnLedger.values()].filter((entry) => entry.turnId === turnId);
    if (matches.length > 1) {
      throw transportError(
        'codex_app_server_ambiguous_turn',
        `turn ${turnId} exists in more than one provider session`,
      );
    }
    return matches[0] ?? null;
  }

  #normalizeSessionResult(response, sessionIdValue, resumed, expectedProviderSessionId) {
    const wireSession = response?.thread;
    let providerSessionId;
    try {
      providerSessionId = requireNonBlankString(
        wireSession?.id,
        'codex_app_server_invalid_session_response',
        `${resumed ? 'session resume' : 'session start'} response omitted provider identity`,
      );
    } catch (error) {
      const wrapped = asTransportError(error, 'codex_app_server_invalid_session_response');
      this.#protocolViolation(wrapped);
      throw wrapped;
    }
    if (expectedProviderSessionId && providerSessionId !== expectedProviderSessionId) {
      const error = transportError(
        'codex_app_server_provider_session_mismatch',
        `provider resumed ${providerSessionId} instead of ${expectedProviderSessionId}`,
      );
      this.#protocolViolation(error);
      throw error;
    }
    const sessionId = optionalNonBlankString(sessionIdValue, 'sessionId');
    const normalized = normalizeProviderWireValue(response);
    const session = Object.freeze({
      ...(normalized.session ?? {}),
      ...(sessionId ? { sessionId } : {}),
      providerSessionId,
    });
    return Object.freeze({
      ...normalized,
      providerSessionId,
      resumed,
      session,
      sessionId,
    });
  }

  #sendRequest(method, params, allowBeforeReady = false) {
    if (!allowBeforeReady && this.#state !== 'ready') {
      return Promise.reject(transportError(
        'codex_app_server_not_ready',
        `cannot call ${canonicalMethod(method)} while transport state is ${this.#state}`,
      ));
    }
    if (allowBeforeReady && this.#state !== 'connecting') {
      return Promise.reject(transportError(
        'codex_app_server_not_ready',
        `cannot initialize while transport state is ${this.#state}`,
      ));
    }

    const id = ++this.#requestSequence;
    const key = wireIdKey(id);
    const sentAt = new Date().toISOString();
    const record = { id, method, sentAt, status: 'pending' };
    this.#requestLedger.set(key, record);
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.#pendingRequests.delete(key);
        record.completedAt = new Date().toISOString();
        record.status = 'timedOut';
        reject(transportError(
          'codex_app_server_request_timeout',
          `${canonicalMethod(method)} request ${id} timed out`,
        ));
      }, this.#options.requestTimeoutMs);
      timer.unref?.();
      this.#pendingRequests.set(key, { id, method, record, reject, resolve, timer });
      this.#writeWire({ id, method, params }).catch((error) => {
        const pending = this.#pendingRequests.get(key);
        if (!pending) {
          return;
        }
        clearTimeout(pending.timer);
        this.#pendingRequests.delete(key);
        record.completedAt = new Date().toISOString();
        record.status = 'writeFailed';
        reject(error);
      });
    });
  }

  #writeWire(message) {
    const child = this.#child;
    if (!child || !child.stdin?.writable) {
      return Promise.reject(this.#terminalError ?? transportError(
        'codex_app_server_closed',
        'app-server stdin is not writable',
      ));
    }
    let payload;
    try {
      payload = `${JSON.stringify(message)}\n`;
    } catch (error) {
      return Promise.reject(transportError(
        'codex_app_server_invalid_outbound_message',
        error instanceof Error ? error.message : String(error),
        undefined,
        error,
      ));
    }
    return new Promise((resolve, reject) => {
      try {
        child.stdin.write(payload, 'utf8', (error) => {
          if (error) {
            reject(transportError(
              'codex_app_server_stdin_failed',
              error.message,
              undefined,
              error,
            ));
          } else {
            resolve();
          }
        });
      } catch (error) {
        reject(transportError(
          'codex_app_server_stdin_failed',
          error instanceof Error ? error.message : String(error),
          undefined,
          error,
        ));
      }
    });
  }

  #protocolViolation(error) {
    this.emit('protocolError', error);
    this.#fail(error, true);
  }

  #fail(error, terminate) {
    if (this.#state === 'closed' || this.#state === 'closing' || this.#state === 'failed') {
      return;
    }
    this.#state = 'failed';
    this.#terminalError = error;
    this.#rejectInFlight(error, 'connectionFailed');
    if (terminate && this.#child) {
      terminateProcessTree(this.#child);
    }
  }

  #rejectInFlight(error, requestStatus) {
    for (const waiter of this.#serverRequestWaiters) {
      waiter.reject(error);
    }
    this.#serverRequestWaiters.clear();
    for (const pending of this.#pendingRequests.values()) {
      clearTimeout(pending.timer);
      pending.record.completedAt = new Date().toISOString();
      pending.record.status = requestStatus;
      pending.reject(error);
    }
    this.#pendingRequests.clear();
    const settledAt = new Date().toISOString();
    for (const record of this.#serverRequestLedger.values()) {
      if (record.status === 'pending') {
        record.status = 'cancelled';
        record.cancelledAt = settledAt;
      } else if (record.status === 'responding' || record.status === 'responded') {
        record.status = 'resolutionUnknown';
        record.resolutionUnknownAt = settledAt;
      }
    }
    trimServerRequestLedger(this.#serverRequestLedger);
    for (const tracker of this.#turnLedger.values()) {
      if (tracker.status === 'active') {
        tracker.status = 'failed';
        tracker.reject(error);
      }
    }
  }
}

export const CodexAppServerTransport = CodexAppServerLiveTransport;

function resolveAppServerLaunch(options) {
  const executable = options.executable
    ? String(options.executable)
    : probeCodexAppServer(options.env).executable;
  if (!executable) {
    throw transportError(
      'codex_app_server_unavailable',
      'no real codex executable was found',
    );
  }
  const args = options.args ? options.args.map(String) : buildCodexAppServerArgs();
  return resolveLaunchCommand(executable, args, options.env);
}

function buildSessionStartParams(command) {
  const params = mergeProviderOptions(command, 'startSession');
  copyDefined(params, command, SESSION_START_KEYS);
  if (command.sessionSource !== undefined) {
    params.threadSource = command.sessionSource;
  }
  return params;
}

function buildSessionResumeParams(command) {
  const params = mergeProviderOptions(command, 'resumeSession');
  copyDefined(params, command, SESSION_RESUME_KEYS);
  return params;
}

function buildTurnStartParams(command) {
  const params = mergeProviderOptions(command, 'startTurn');
  copyDefined(params, command, TURN_START_KEYS);
  params.input = normalizeTurnInput(command);
  return params;
}

function mergeProviderOptions(command, commandName) {
  const providerOptions = command.providerOptions ?? {};
  if (!isRecord(providerOptions)) {
    throw transportError(
      'codex_app_server_invalid_command',
      `${commandName}.providerOptions must be an object`,
    );
  }
  for (const key of Object.keys(providerOptions)) {
    if (key === 'thread' || key === 'threadId' || key === 'threadSource') {
      throw transportError(
        'codex_app_server_noncanonical_session_name',
        `${commandName}.providerOptions.${key} must use canonical Session naming`,
      );
    }
  }
  return { ...providerOptions };
}

function copyDefined(target, source, keys) {
  for (const key of keys) {
    if (source[key] !== undefined) {
      target[key] = source[key];
    }
  }
}

function normalizeTurnInput(command) {
  const candidates = [command.input, command.message, command.messages]
    .filter((value) => value !== undefined);
  if (candidates.length !== 1) {
    throw transportError(
      'codex_app_server_invalid_turn_input',
      'startTurn requires exactly one of input, message, or messages',
    );
  }
  const value = candidates[0];
  if (typeof value === 'string') {
    return [{ type: 'text', text: value, text_elements: [] }];
  }
  if (!Array.isArray(value) || value.length === 0) {
    throw transportError(
      'codex_app_server_invalid_turn_input',
      'turn input must be a non-empty string or array',
    );
  }
  if (value.every((entry) => typeof entry === 'string')) {
    return value.map((text) => ({ type: 'text', text, text_elements: [] }));
  }
  if (!value.every(isRecord)) {
    throw transportError(
      'codex_app_server_invalid_turn_input',
      'turn input entries must be objects or all strings',
    );
  }
  return value.map((entry) => ({ ...entry }));
}

function normalizeObjectCommand(value, label) {
  if (!isRecord(value)) {
    throw transportError('codex_app_server_invalid_command', `${label} must be an object`);
  }
  return value;
}

function normalizeProviderSessionCommand(value, options, commandName) {
  const command = typeof value === 'string'
    ? { ...normalizeObjectCommand(options, `${commandName} options`), providerSessionId: value }
    : normalizeObjectCommand(value, `${commandName} command`);
  return {
    ...command,
    providerSessionId: requireNonBlankString(
      command.providerSessionId,
      'codex_app_server_invalid_provider_session_id',
      `${commandName} requires providerSessionId`,
    ),
  };
}

function normalizeTurnCommand(value, options) {
  if (typeof value === 'string' && (typeof options === 'string' || Array.isArray(options))) {
    return {
      input: options,
      providerSessionId: requireNonBlankString(
        value,
        'codex_app_server_invalid_provider_session_id',
        'startTurn requires providerSessionId',
      ),
    };
  }
  return normalizeProviderSessionCommand(value, options, 'startTurn');
}

function normalizeInterruptCommand(value, turnIdOrOptions, options) {
  if (isRecord(value)) {
    return {
      ...value,
      providerSessionId: requireNonBlankString(
        value.providerSessionId,
        'codex_app_server_invalid_provider_session_id',
        'interruptTurn requires providerSessionId',
      ),
      turnId: requireNonBlankString(
        value.turnId,
        'codex_app_server_invalid_turn_id',
        'interruptTurn requires turnId',
      ),
    };
  }
  const commandOptions = isRecord(turnIdOrOptions)
    ? turnIdOrOptions
    : normalizeObjectCommand(options, 'interruptTurn options');
  const turnId = isRecord(turnIdOrOptions) ? turnIdOrOptions.turnId : turnIdOrOptions;
  return {
    ...commandOptions,
    providerSessionId: requireNonBlankString(
      value,
      'codex_app_server_invalid_provider_session_id',
      'interruptTurn requires providerSessionId',
    ),
    turnId: requireNonBlankString(
      turnId,
      'codex_app_server_invalid_turn_id',
      'interruptTurn requires turnId',
    ),
  };
}

function normalizeTurnWait(value, options) {
  if (isRecord(value)) {
    return {
      providerSessionId: optionalNonBlankString(value.providerSessionId, 'providerSessionId'),
      turnId: requireNonBlankString(
        value.turnId,
        'codex_app_server_invalid_turn_id',
        'turnId is required',
      ),
    };
  }
  return {
    providerSessionId: optionalNonBlankString(options.providerSessionId, 'providerSessionId'),
    turnId: requireNonBlankString(
      value,
      'codex_app_server_invalid_turn_id',
      'turnId is required',
    ),
  };
}

function normalizeServerRequestCriteria(criteria) {
  if (!isRecord(criteria)) {
    throw transportError(
      'codex_app_server_invalid_server_request_criteria',
      'server request criteria must be an object',
    );
  }
  return {
    method: optionalNonBlankString(criteria.method, 'method'),
    providerSessionId: optionalNonBlankString(
      criteria.providerSessionId,
      'providerSessionId',
    ),
    timeoutMs: criteria.timeoutMs == null
      ? null
      : positiveInteger(criteria.timeoutMs, null, 'timeoutMs'),
    turnId: optionalNonBlankString(criteria.turnId, 'turnId'),
  };
}

function serverRequestMatches(event, criteria) {
  return (!criteria.method || event.method === criteria.method)
    && (!criteria.providerSessionId || event.providerSessionId === criteria.providerSessionId)
    && (!criteria.turnId || event.turnId === criteria.turnId);
}

function normalizeServerRequestResponse(requestOrId, response, affinity) {
  let requestId = requestOrId;
  let inheritedAffinity = null;
  let result = response;
  let error = null;
  if (isRecord(requestOrId)) {
    requestId = requestOrId.requestId;
    inheritedAffinity = requestOrId.affinity ?? requestOrId;
    if (response === undefined && Object.prototype.hasOwnProperty.call(requestOrId, 'result')) {
      result = requestOrId.result;
    }
    if (response === undefined && Object.prototype.hasOwnProperty.call(requestOrId, 'error')) {
      error = requestOrId.error;
    }
  }
  if (isRecord(response)
    && Object.keys(response).every((key) => key === 'result' || key === 'error')
    && (Object.prototype.hasOwnProperty.call(response, 'result')
      || Object.prototype.hasOwnProperty.call(response, 'error'))) {
    result = response.result;
    error = response.error ?? null;
  }
  if (result === undefined && error == null) {
    throw transportError(
      'codex_app_server_invalid_server_request_response',
      'server request response requires a result or error',
    );
  }
  wireIdKey(requestId);
  const explicitAffinity = isRecord(affinity) ? affinity : {};
  return {
    affinity: { ...(inheritedAffinity ?? {}), ...explicitAffinity },
    error,
    requestId,
    result,
  };
}

function assertServerRequestAffinity(record, affinity, connectionId) {
  if (affinity.connectionId && affinity.connectionId !== connectionId) {
    throw transportError(
      'codex_app_server_server_request_affinity_mismatch',
      'server request belongs to a different app-server connection',
    );
  }
  if (affinity.requestId !== undefined
    && wireIdKey(affinity.requestId) !== wireIdKey(record.id)) {
    throw transportError(
      'codex_app_server_server_request_affinity_mismatch',
      'server request affinity carries a different request id',
    );
  }
  if (affinity.providerSessionId
    && affinity.providerSessionId !== record.event.providerSessionId) {
    throw transportError(
      'codex_app_server_server_request_affinity_mismatch',
      'server request belongs to a different provider session',
    );
  }
  if (affinity.turnId && affinity.turnId !== record.event.turnId) {
    throw transportError(
      'codex_app_server_server_request_affinity_mismatch',
      'server request belongs to a different turn',
    );
  }
}

function normalizeProviderWireValue(value, context = null) {
  if (Array.isArray(value)) {
    return value.map((entry) => normalizeProviderWireValue(entry, context));
  }
  if (!isRecord(value)) {
    return value;
  }
  const normalized = {};
  for (const [key, entry] of Object.entries(value)) {
    if (context === 'session' && key === 'id') {
      normalized.providerSessionId = entry;
      continue;
    }
    const mappedKey = canonicalProviderKey(key);
    const childContext = key === 'thread' || key === 'threads' ? 'session' : null;
    normalized[mappedKey] = normalizeProviderWireValue(entry, childContext);
  }
  return normalized;
}

function canonicalProviderKey(key) {
  const keys = {
    forkedFromId: 'forkedFromProviderSessionId',
    parentThreadId: 'parentProviderSessionId',
    parent_thread_id: 'parent_provider_session_id',
    thread: 'session',
    threadId: 'providerSessionId',
    threadName: 'sessionName',
    thread_id: 'provider_session_id',
    threads: 'sessions',
  };
  return keys[key] ?? key;
}

function canonicalMethod(method) {
  return method.startsWith('thread/') ? `session/${method.slice('thread/'.length)}` : method;
}

function readWireProviderSessionId(params) {
  const value = params?.threadId ?? params?.thread?.id;
  return typeof value === 'string' && value.trim() ? value.trim() : null;
}

function readWireTurnId(params) {
  const value = params?.turnId ?? params?.turn?.id;
  return typeof value === 'string' && value.trim() ? value.trim() : null;
}

function normalizeRpcError(error) {
  if (!isRecord(error)) {
    throw transportError(
      'codex_app_server_invalid_rpc_error',
      'server request error response must be an object',
    );
  }
  if (!Number.isInteger(error.code) || typeof error.message !== 'string') {
    throw transportError(
      'codex_app_server_invalid_rpc_error',
      'server request error response requires integer code and string message',
    );
  }
  return {
    code: error.code,
    message: error.message,
    ...(error.data === undefined ? {} : { data: error.data }),
  };
}

function rpcErrorMessage(error) {
  if (typeof error === 'string') {
    return error;
  }
  if (typeof error?.message === 'string') {
    return error.message;
  }
  return 'app-server returned an unknown JSON-RPC error';
}

function normalizeClientInfo(value) {
  if (value == null) {
    return {
      name: 'sdkwork_birdcoder',
      title: 'SDKWork BirdCoder',
      version: '0.1.0',
    };
  }
  if (!isRecord(value)) {
    throw transportError('codex_app_server_invalid_options', 'clientInfo must be an object');
  }
  return {
    name: requireNonBlankString(
      value.name,
      'codex_app_server_invalid_options',
      'clientInfo.name is required',
    ),
    title: requireNonBlankString(
      value.title,
      'codex_app_server_invalid_options',
      'clientInfo.title is required',
    ),
    version: requireNonBlankString(
      value.version,
      'codex_app_server_invalid_options',
      'clientInfo.version is required',
    ),
  };
}

function positiveInteger(value, fallback, fieldName) {
  if (value == null) {
    return fallback;
  }
  if (!Number.isSafeInteger(value) || value <= 0) {
    throw transportError(
      'codex_app_server_invalid_options',
      `${fieldName} must be a positive safe integer`,
    );
  }
  return value;
}

function optionalNonBlankString(value, fieldName) {
  if (value == null) {
    return null;
  }
  if (typeof value !== 'string') {
    throw transportError('codex_app_server_invalid_command', `${fieldName} must be a string`);
  }
  const normalized = value.trim();
  return normalized || null;
}

function requireNonBlankString(value, code, message) {
  if (typeof value !== 'string' || !value.trim()) {
    throw transportError(code, message);
  }
  return value.trim();
}

function assertListener(listener) {
  if (typeof listener !== 'function') {
    throw transportError('codex_app_server_invalid_listener', 'listener must be a function');
  }
}

function isRecord(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function wireIdKey(id) {
  if (typeof id === 'string') {
    return `string:${id}`;
  }
  if (typeof id === 'number' && Number.isSafeInteger(id)) {
    return `number:${id}`;
  }
  throw transportError(
    'codex_app_server_invalid_rpc_id',
    'JSON-RPC ids must be strings or safe integers',
  );
}

function turnLedgerKey(providerSessionId, turnId) {
  return `${providerSessionId}\u0000${turnId}`;
}

function trimSettledLedger(ledger) {
  if (ledger.size <= MAX_LEDGER_ENTRIES) {
    return;
  }
  for (const [key, record] of ledger) {
    if (record.status !== 'pending') {
      ledger.delete(key);
    }
    if (ledger.size <= MAX_LEDGER_ENTRIES) {
      break;
    }
  }
}

function trimServerRequestLedger(ledger) {
  if (ledger.size <= MAX_LEDGER_ENTRIES) {
    return;
  }
  for (const [key, record] of ledger) {
    if (!['pending', 'responding', 'responded'].includes(record.status)) {
      ledger.delete(key);
    }
    if (ledger.size <= MAX_LEDGER_ENTRIES) {
      break;
    }
  }
}

function trimTurnLedger(ledger) {
  if (ledger.size <= MAX_LEDGER_ENTRIES) {
    return;
  }
  for (const [key, record] of ledger) {
    if (record.status !== 'active') {
      ledger.delete(key);
    }
    if (ledger.size <= MAX_LEDGER_ENTRIES) {
      break;
    }
  }
}

function withTimeout(promise, timeoutMs, createError) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(createError()), timeoutMs);
    timer.unref?.();
    promise.then(
      (value) => {
        clearTimeout(timer);
        resolve(value);
      },
      (error) => {
        clearTimeout(timer);
        reject(error);
      },
    );
  });
}

function transportError(code, message, details, cause) {
  return new CodexAppServerTransportError(code, message, details, cause);
}

function asTransportError(error, fallbackCode) {
  if (error instanceof CodexAppServerTransportError) {
    return error;
  }
  return transportError(
    fallbackCode,
    error instanceof Error ? error.message : String(error),
    undefined,
    error,
  );
}
