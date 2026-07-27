#!/usr/bin/env node
import { spawn, spawnSync } from 'node:child_process';
import { accessSync, constants, existsSync, statSync } from 'node:fs';
import path from 'node:path';

const CODEX_CLI_BIN_ENV = 'SDKWORK_CODEX_CLI_BIN';
const DEFAULT_TIMEOUT_MS = 300_000;
const MAX_TIMEOUT_MS = 3_600_000;
const DEFAULT_MAX_OUTPUT_BYTES = 8 * 1024 * 1024;
const MAX_CAPTURE_BYTES = 64 * 1024 * 1024;

export function isCodexPackage(packageName) {
  return packageName === '@openai/codex-sdk' || packageName === '@openai/codex';
}

export function probeCodexCli(environment = process.env) {
  const configured = environment[CODEX_CLI_BIN_ENV]?.trim();
  const executable = configured
    ? resolveConfiguredExecutable(configured)
    : resolveExecutableOnPath('codex', environment);
  return {
    available: Boolean(executable),
    executable,
    mode: executable ? 'sdk_cli' : null,
  };
}

export function buildCodexCliArgs(operation) {
  const executionOptions = readExecutionOptions(operation);
  const args = ['exec', '--json'];

  if (optionalBoolean(executionOptions.skip_git_repo_check, 'skip_git_repo_check')) {
    args.push('--skip-git-repo-check');
  }
  if (optionalBoolean(executionOptions.ephemeral, 'ephemeral')) {
    args.push('--ephemeral');
  }

  const modelId = optionalNonBlankString(operation.model_id, 'model_id');
  if (modelId) {
    args.push('--model', modelId);
  }

  const fullAuto = optionalBoolean(executionOptions.full_auto, 'full_auto');
  const configuredSandbox = normalizeSandboxMode(executionOptions.sandbox_mode);
  const sandboxMode = configuredSandbox ?? (fullAuto ? 'workspace-write' : null);
  if (sandboxMode) {
    args.push('--sandbox', sandboxMode);
  }

  const configuredApproval = normalizeApprovalPolicy(executionOptions.approval_policy);
  const approvalPolicy = configuredApproval ?? (fullAuto ? 'on-failure' : null);
  if (approvalPolicy) {
    args.push('--config', `approval_policy="${approvalPolicy}"`);
  }

  const workingDirectory = resolveWorkingDirectory(operation.working_directory);
  if (workingDirectory) {
    args.push('--cd', workingDirectory);
  }

  const sessionId = optionalNonBlankString(operation.session_id, 'session_id');
  if (sessionId) {
    args.push('resume', sessionId);
  }
  args.push('-');
  return args;
}

export function parseCodexCliJsonl(stdout) {
  const messages = [];
  let providerSessionId = null;
  let turnError = null;

  for (const [index, rawLine] of String(stdout ?? '').split(/\r?\n/u).entries()) {
    const line = rawLine.trim();
    if (!line) {
      continue;
    }

    let event;
    try {
      event = JSON.parse(line);
    } catch (error) {
      throw new Error(
        `codex_cli_invalid_jsonl: line ${index + 1}: ${error instanceof Error ? error.message : String(error)}`,
      );
    }

    if (event?.type === 'thread.started' && typeof event.thread_id === 'string') {
      providerSessionId = event.thread_id.trim() || providerSessionId;
      continue;
    }

    if (event?.type === 'item.completed' && event.item?.type === 'agent_message') {
      const text = extractAgentMessageText(event.item);
      if (text) {
        messages.push(text);
      }
      continue;
    }

    if (event?.type === 'turn.failed') {
      turnError = extractErrorMessage(event.error) ?? 'Codex CLI turn failed';
      continue;
    }

    if (event?.type === 'error') {
      turnError = extractErrorMessage(event) ?? 'Codex CLI emitted an error event';
    }
  }

  return {
    messages,
    provider_session_id: providerSessionId,
    finish_reason: turnError ? 'error' : 'stop',
    error: turnError,
  };
}

export async function invokeCodexCliModelChat(operation, options = {}) {
  const environment = options.env ?? process.env;
  const probe = options.probe ?? probeCodexCli(environment);
  if (!probe.available || !probe.executable) {
    throw new Error('codex_cli_unavailable: no real codex executable was found');
  }

  const prompt = String(options.prompt ?? resolvePrompt(operation));
  const args = buildCodexCliArgs(operation);
  const cwd = resolveWorkingDirectory(operation.working_directory) ?? process.cwd();
  const timeoutMs = resolveTimeoutMs(operation.timeout_ms);
  const maxOutputBytes = resolveMaxOutputBytes(operation.execution_options?.max_output_bytes);
  const captureLimit = Math.min(
    MAX_CAPTURE_BYTES,
    Math.max(DEFAULT_MAX_OUTPUT_BYTES, maxOutputBytes * 8),
  );

  const processResult = await runCodexProcess({
    executable: probe.executable,
    args,
    cwd,
    environment,
    prompt,
    timeoutMs,
    captureLimit,
    onEvent: options.onEvent,
  });
  const parsed = parseCodexCliJsonl(processResult.stdout);
  if (parsed.error) {
    throw new Error(`codex_cli_turn_failed: ${parsed.error}`);
  }
  if (parsed.messages.length === 0) {
    throw new Error('codex_cli_empty_response: no completed agent message was emitted');
  }

  const assistantBytes = Buffer.byteLength(parsed.messages.join('\n'), 'utf8');
  if (assistantBytes > maxOutputBytes) {
    throw new Error(
      `codex_cli_output_limit_exceeded: assistant output ${assistantBytes} bytes exceeds ${maxOutputBytes}`,
    );
  }

  return {
    ok: true,
    mode: 'sdk_cli',
    messages: parsed.messages,
    finish_reason: parsed.finish_reason,
    model_request_id: operation.model_request_id ?? null,
    provider_session_id: parsed.provider_session_id,
    package: options.packageName ?? '@openai/codex-sdk',
  };
}

function readExecutionOptions(operation) {
  const options = operation?.execution_options;
  if (options == null) {
    return {};
  }
  if (typeof options !== 'object' || Array.isArray(options)) {
    throw new Error('execution_options must be an object');
  }
  return options;
}

function resolvePrompt(operation) {
  return Array.isArray(operation?.messages) ? operation.messages.join('\n') : '';
}

function optionalNonBlankString(value, fieldName) {
  if (value == null) {
    return null;
  }
  if (typeof value !== 'string') {
    throw new Error(`${fieldName} must be a string`);
  }
  const normalized = value.trim();
  return normalized || null;
}

function optionalBoolean(value, fieldName) {
  if (value == null) {
    return false;
  }
  if (typeof value !== 'boolean') {
    throw new Error(`execution_options.${fieldName} must be a boolean`);
  }
  return value;
}

function normalizeSandboxMode(value) {
  const normalized = optionalNonBlankString(value, 'execution_options.sandbox_mode');
  if (!normalized) {
    return null;
  }
  const compact = normalized.toLowerCase().replace(/[_\s]/gu, '-');
  if (compact === 'read-only' || compact === 'readonly') {
    return 'read-only';
  }
  if (compact === 'workspace-write' || compact === 'workspacewrite') {
    return 'workspace-write';
  }
  if (compact === 'danger-full-access' || compact === 'dangerfullaccess') {
    return 'danger-full-access';
  }
  throw new Error(`unsupported Codex sandbox mode: ${normalized}`);
}

function normalizeApprovalPolicy(value) {
  const normalized = optionalNonBlankString(value, 'execution_options.approval_policy');
  if (!normalized) {
    return null;
  }
  const compact = normalized.toLowerCase().replace(/[-_\s]/gu, '');
  const aliases = new Map([
    ['onrequest', 'on-request'],
    ['restricted', 'untrusted'],
    ['untrusted', 'untrusted'],
    ['unlesstrusted', 'untrusted'],
    ['onfailure', 'on-failure'],
    ['releaseonly', 'on-failure'],
    ['autoallow', 'on-failure'],
    ['never', 'on-failure'],
  ]);
  const mapped = aliases.get(compact);
  if (!mapped) {
    throw new Error(`unsupported Codex approval policy: ${normalized}`);
  }
  return mapped;
}

function resolveWorkingDirectory(value) {
  const configured = optionalNonBlankString(value, 'working_directory');
  if (!configured) {
    return null;
  }
  const resolved = path.resolve(configured);
  let metadata;
  try {
    metadata = statSync(resolved);
  } catch (error) {
    throw new Error(
      `codex_cli_invalid_working_directory: ${resolved}: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
  if (!metadata.isDirectory()) {
    throw new Error(`codex_cli_invalid_working_directory: not a directory: ${resolved}`);
  }
  return resolved;
}

function resolveTimeoutMs(value) {
  if (value == null) {
    return DEFAULT_TIMEOUT_MS;
  }
  if (!Number.isSafeInteger(value) || value <= 0) {
    throw new Error('timeout_ms must be a positive safe integer');
  }
  return Math.min(value, MAX_TIMEOUT_MS);
}

function resolveMaxOutputBytes(value) {
  if (value == null) {
    return DEFAULT_MAX_OUTPUT_BYTES;
  }
  if (!Number.isSafeInteger(value) || value <= 0) {
    throw new Error('execution_options.max_output_bytes must be a positive safe integer');
  }
  return Math.min(value, MAX_CAPTURE_BYTES);
}

function extractAgentMessageText(item) {
  if (typeof item.text === 'string') {
    return item.text;
  }
  if (typeof item.content === 'string') {
    return item.content;
  }
  if (!Array.isArray(item.content)) {
    return '';
  }
  return item.content
    .map((part) => {
      if (typeof part === 'string') {
        return part;
      }
      return typeof part?.text === 'string' ? part.text : '';
    })
    .join('');
}

function extractErrorMessage(error) {
  if (typeof error === 'string') {
    return error;
  }
  if (typeof error?.message === 'string') {
    return error.message;
  }
  return null;
}

function resolveConfiguredExecutable(configured) {
  const resolved = path.resolve(configured);
  return isExecutableFile(resolved) ? resolved : null;
}

export function resolveExecutableOnPath(commandName, environment) {
  const pathValue = environment.PATH ?? environment.Path ?? '';
  const directories = pathValue.split(path.delimiter).filter(Boolean);
  const candidates = process.platform === 'win32'
    ? [`${commandName}.exe`, `${commandName}.cmd`, `${commandName}.bat`, `${commandName}.ps1`]
    : [commandName];
  for (const directory of directories) {
    for (const candidate of candidates) {
      const executable = path.join(directory.replace(/^"|"$/gu, ''), candidate);
      if (isExecutableFile(executable)) {
        return executable;
      }
    }
  }
  return null;
}

export function isExecutableFile(filePath) {
  try {
    if (!existsSync(filePath) || !statSync(filePath).isFile()) {
      return false;
    }
    accessSync(filePath, process.platform === 'win32' ? constants.F_OK : constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

function runCodexProcess({
  executable,
  args,
  cwd,
  environment,
  prompt,
  timeoutMs,
  captureLimit,
  onEvent,
}) {
  return new Promise((resolve, reject) => {
    let launch;
    try {
      launch = resolveLaunchCommand(executable, args, environment);
    } catch (error) {
      reject(error);
      return;
    }

    const child = spawn(launch.command, launch.args, {
      cwd,
      env: environment,
      windowsHide: true,
      windowsVerbatimArguments: launch.windowsVerbatimArguments ?? false,
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    const stdout = [];
    const stderr = [];
    let stdoutBytes = 0;
    let stderrBytes = 0;
    let settledReason = null;
    const liveEvents = createLiveJsonlDispatcher(onEvent, (lineNumber, error) =>
      new Error(
        `codex_cli_invalid_jsonl: line ${lineNumber}: ${error instanceof Error ? error.message : String(error)}`,
      ),
    );

    const timer = setTimeout(() => {
      settledReason = new Error(`codex_cli_timeout: exceeded ${timeoutMs} ms`);
      terminateProcessTree(child);
    }, timeoutMs);

    child.stdout.on('data', (chunk) => {
      stdoutBytes += chunk.length;
      if (stdoutBytes > captureLimit) {
        settledReason = new Error(
          `codex_cli_output_limit_exceeded: stdout exceeded ${captureLimit} bytes`,
        );
        terminateProcessTree(child);
        return;
      }
      stdout.push(chunk);
      liveEvents.push(chunk, (error) => {
        if (!settledReason) {
          settledReason = error;
          terminateProcessTree(child);
        }
      });
    });

    child.stderr.on('data', (chunk) => {
      stderrBytes += chunk.length;
      if (stderrBytes <= DEFAULT_MAX_OUTPUT_BYTES) {
        stderr.push(chunk);
      }
    });

    child.once('error', (error) => {
      clearTimeout(timer);
      reject(new Error(`codex_cli_spawn_failed: ${error.message}`));
    });

    child.once('close', async (code, signal) => {
      clearTimeout(timer);
      await liveEvents.finish((error) => {
        settledReason ??= error;
      });
      if (settledReason) {
        reject(settledReason);
        return;
      }
      const stdoutText = Buffer.concat(stdout).toString('utf8');
      const stderrText = Buffer.concat(stderr).toString('utf8').trim();
      if (code !== 0) {
        const status = code == null ? `signal ${signal ?? 'unknown'}` : `status ${code}`;
        const stdoutError = extractCodexCliFailureFromJsonl(stdoutText);
        const detail = stdoutError || stderrText;
        reject(
          new Error(
            `codex_cli_exit_failed: ${status}${detail ? `: ${detail}` : ''}`,
          ),
        );
        return;
      }
      resolve({ stdout: stdoutText, stderr: stderrText });
    });

    child.stdin.on('error', (error) => {
      if (!settledReason) {
        settledReason = new Error(`codex_cli_stdin_failed: ${error.message}`);
        terminateProcessTree(child);
      }
    });
    child.stdin.end(prompt, 'utf8');
  });
}

function createLiveJsonlDispatcher(onEvent, invalidJsonError) {
  if (onEvent == null) {
    return {
      push() {},
      async finish() {},
    };
  }
  if (typeof onEvent !== 'function') {
    throw new Error('CLI onEvent must be a function');
  }

  const decoder = new TextDecoder();
  let buffer = '';
  let lineNumber = 0;
  let eventError = null;
  let pending = Promise.resolve();

  const queueLine = (rawLine, onError) => {
    lineNumber += 1;
    const currentLineNumber = lineNumber;
    const line = rawLine.trim();
    if (!line) {
      return;
    }
    pending = pending
      .then(async () => {
        if (eventError) {
          return;
        }
        let event;
        try {
          event = JSON.parse(line);
        } catch (error) {
          throw invalidJsonError(currentLineNumber, error);
        }
        await onEvent(event);
      })
      .catch((error) => {
        eventError = error instanceof Error ? error : new Error(String(error));
        onError(eventError);
      });
  };
  const drainLines = (onError) => {
    let newline = buffer.indexOf('\n');
    while (newline >= 0) {
      const line = buffer.slice(0, newline).replace(/\r$/u, '');
      buffer = buffer.slice(newline + 1);
      queueLine(line, onError);
      newline = buffer.indexOf('\n');
    }
  };

  return {
    push(chunk, onError) {
      buffer += decoder.decode(chunk, { stream: true });
      drainLines(onError);
    },
    async finish(onError) {
      buffer += decoder.decode();
      drainLines(onError);
      if (buffer) {
        queueLine(buffer.replace(/\r$/u, ''), onError);
        buffer = '';
      }
      await pending;
      if (eventError) {
        onError(eventError);
      }
    },
  };
}

function extractCodexCliFailureFromJsonl(stdout) {
  try {
    const parsed = parseCodexCliJsonl(stdout);
    return parsed.error || null;
  } catch {
    return null;
  }
}

export function resolveLaunchCommand(executable, args, environment) {
  if (process.platform !== 'win32') {
    return { command: executable, args };
  }

  const extension = path.extname(executable).toLowerCase();
  if (extension === '.cmd' || extension === '.bat') {
    const commandLine = [executable, ...args].map(quoteCmdToken).join(' ');
    return {
      command: environment.ComSpec || environment.COMSPEC || 'cmd.exe',
      args: ['/d', '/s', '/c', `"${commandLine}"`],
      windowsVerbatimArguments: true,
    };
  }
  if (extension === '.ps1') {
    return {
      command: resolvePowerShellExecutable(environment),
      args: ['-NoLogo', '-NoProfile', '-NonInteractive', '-File', executable, ...args],
    };
  }
  return { command: executable, args };
}

function quoteCmdToken(value) {
  if (/[\0\r\n]/u.test(value)) {
    throw new Error('codex_cli_invalid_argument: command arguments cannot contain line breaks');
  }
  const escaped = value.replace(/%/gu, '%%').replace(/"/gu, '""');
  return `"${escaped}"`;
}

function resolvePowerShellExecutable(environment) {
  const systemRoot = environment.SystemRoot ?? environment.SYSTEMROOT;
  const windowsPowerShell = systemRoot
    ? path.join(systemRoot, 'System32', 'WindowsPowerShell', 'v1.0', 'powershell.exe')
    : null;
  if (windowsPowerShell && isExecutableFile(windowsPowerShell)) {
    return windowsPowerShell;
  }
  return 'powershell.exe';
}

export function terminateProcessTree(child) {
  if (!child.pid) {
    return;
  }
  if (process.platform === 'win32') {
    spawnSync('taskkill.exe', ['/PID', String(child.pid), '/T', '/F'], {
      windowsHide: true,
      stdio: 'ignore',
    });
    return;
  }
  child.kill('SIGTERM');
  const killTimer = setTimeout(() => child.kill('SIGKILL'), 500);
  killTimer.unref();
}
