#!/usr/bin/env node
import { once } from 'node:events';
import readline from 'node:readline';
import {
  buildModelChatStreamResult,
  closeSdkLiveRuntimes,
  buildStubModelChatResult,
  interruptSdkLiveTurn,
  invokeModelChatStreamRuntime,
  invokeModelChatRuntime,
  mockProviderInvocationAllowed,
  probePackage,
  probeModelChatRuntime,
  respondToSdkLiveServerRequest,
  VERIFIED_PROVIDER_SESSION_ID,
} from './engine-sdk-live.mjs';

const packageIndex = process.argv.indexOf('--package');
const packageName =
  packageIndex >= 0 && packageIndex + 1 < process.argv.length
    ? process.argv[packageIndex + 1]
    : 'unknown';

async function writeResponse(response) {
  if (!process.stdout.write(`${JSON.stringify(response)}\n`)) {
    await once(process.stdout, 'drain');
  }
}

async function writeStreamChunk(requestId, chunk, modelRequestId) {
  await writeResponse({
    jsonrpc: '2.0',
    id: requestId,
    result: {
      event: 'stream.chunk',
      sequence: chunk.sequence ?? 0,
      content: chunk.content ?? chunk.delta ?? '',
      model_request_id: modelRequestId,
    },
  });
}

async function writeStreamEvent(requestId, kernelEvent, modelRequestId) {
  await writeResponse({
    jsonrpc: '2.0',
    id: requestId,
    result: {
      event: 'stream.event',
      model_request_id: modelRequestId,
      kernel_event: kernelEvent,
    },
  });
}

async function writeActivity(requestId, activity) {
  await writeResponse({
    jsonrpc: '2.0',
    id: requestId,
    result: {
      event: 'session.activity',
      ...activity,
    },
  });
}

async function writeInvokeDone(requestId, payload) {
  await writeResponse({
    jsonrpc: '2.0',
    id: requestId,
    result: {
      event: 'invoke.done',
      payload,
    },
  });
}

async function writeStreamDone(requestId, result) {
  const terminalResult = {
    event: 'stream.done',
    finish_reason: result.finish_reason ?? 'stop',
    model_request_id: result.model_request_id ?? null,
  };
  const providerSessionId = verifiedProviderSessionId(result);
  if (providerSessionId) {
    terminalResult.provider_session_id = providerSessionId;
  }
  await writeResponse({
    jsonrpc: '2.0',
    id: requestId,
    result: terminalResult,
  });
}

async function writeStreamResult(requestId, result) {
  const chunks = Array.isArray(result.chunks) ? result.chunks : [];
  const modelRequestId = result.model_request_id ?? null;
  for (const chunk of chunks) {
    await writeStreamChunk(requestId, chunk, modelRequestId);
  }
  await writeStreamDone(requestId, result);
}

function verifiedProviderSessionId(result) {
  if (result?.[VERIFIED_PROVIDER_SESSION_ID] !== true) {
    return null;
  }
  const providerSessionId = result?.provider_session_id;
  if (typeof providerSessionId !== 'string') {
    return null;
  }
  const normalized = providerSessionId.trim();
  return normalized || null;
}

function failClosedSyntheticOperation(operationName, packageProbe, modelRequestId = null) {
  return {
    ok: false,
    mode: 'sdk_live_failed',
    package: packageName,
    package_resolved: packageProbe.resolved,
    operation: operationName,
    error: packageProbe.resolved
      ? `${operationName} requires a live provider SDK implementation and mock fallback is disabled`
      : `official sdk package is not resolved for ${operationName} and mock fallback is disabled: ${packageName}`,
    model_request_id: modelRequestId,
  };
}

function syntheticProviderOperationAllowed() {
  return mockProviderInvocationAllowed();
}

function operationRequiresLiveProvider(operation) {
  const value = operation?.execution_options?.require_live_provider;
  if (value == null) {
    return false;
  }
  if (typeof value !== 'boolean') {
    throw new Error('execution_options.require_live_provider must be a boolean');
  }
  return value;
}

async function handleCapabilityInvoke(params, streamOptions = {}) {
  const operation = params.operation ?? {};
  const op = operation.operation ?? operation;
  const packageProbe = probePackage(packageName);
  const runtimeProbe = probeModelChatRuntime(packageName);

  if (op === 'ping') {
    return {
      ok: true,
      backend: 'typescript_node',
      package: packageName,
      package_resolved: packageProbe.resolved,
      app_server_available: runtimeProbe.app_server_available,
      cli_available: runtimeProbe.cli_available,
      runtime_available: runtimeProbe.runtime_available,
      runtime_mode: runtimeProbe.runtime_mode,
    };
  }

  if (op === 'session_create') {
    if (!syntheticProviderOperationAllowed()) {
      return failClosedSyntheticOperation(op, packageProbe);
    }
    return {
      ok: true,
      mode: packageProbe.resolved ? 'sdk_probe' : 'stub',
      agent_id: operation.agent_id ?? null,
      user_ref: operation.user_ref ?? null,
      package: packageName,
      package_resolved: packageProbe.resolved,
    };
  }

  if (op === 'model_chat' || op === 'model_chat_stream') {
    const fallbackAllowed =
      mockProviderInvocationAllowed() && !operationRequiresLiveProvider(operation);

    if (
      op === 'model_chat_stream' &&
      !fallbackAllowed &&
      runtimeProbe.runtime_available &&
      packageName.startsWith('@openai/codex')
    ) {
      try {
        return await invokeModelChatStreamRuntime(packageName, operation, streamOptions);
      } catch (error) {
        if (!fallbackAllowed) {
          return {
            ok: false,
            mode: 'sdk_live_failed',
            package: packageName,
            package_resolved: packageProbe.resolved,
            cli_available: runtimeProbe.cli_available,
            runtime_available: runtimeProbe.runtime_available,
            runtime_mode: runtimeProbe.runtime_mode,
            error: error instanceof Error ? error.message : String(error),
            model_request_id: operation.model_request_id ?? null,
          };
        }
      }
    }

    const handleResult = (result) =>
      op === 'model_chat_stream' ? buildModelChatStreamResult(result) : result;

    if (!fallbackAllowed) {
      try {
        return handleResult(await invokeModelChatRuntime(packageName, operation, streamOptions));
      } catch (error) {
        return {
          ok: false,
          mode: 'sdk_live_failed',
          package: packageName,
          package_resolved: packageProbe.resolved,
          cli_available: runtimeProbe.cli_available,
          runtime_available: runtimeProbe.runtime_available,
          runtime_mode: runtimeProbe.runtime_mode,
          error: error instanceof Error ? error.message : String(error),
          model_request_id: operation.model_request_id ?? null,
        };
      }
    }

    if (!fallbackAllowed) {
      return {
        ok: false,
        mode: 'sdk_live_failed',
        package: packageName,
        error: packageProbe.resolved
          ? 'official sdk live invoke failed and mock fallback is disabled'
          : `official sdk package is not resolved: ${packageName}`,
        model_request_id: operation.model_request_id ?? null,
      };
    }

    return handleResult(buildStubModelChatResult(packageName, operation, packageProbe));
  }

  return {
    ...failClosedSyntheticOperation(op, packageProbe),
    mode: 'unsupported_operation',
    error: `operation is not implemented by the official provider SDK adapter: ${op}`,
  };
}

async function handleStreamingCapabilityInvoke(requestId, params) {
  let emittedChunkCount = 0;
  const result = await handleCapabilityInvoke(params, {
    onActivity:
      params.activity_stream === true
        ? async (activity) => writeActivity(requestId, activity)
        : undefined,
    onChunk: async (chunk) => {
      await writeStreamChunk(requestId, chunk, params.operation?.model_request_id ?? null);
      emittedChunkCount += 1;
    },
    onEvent: async (event) => {
      await writeStreamEvent(
        requestId,
        event,
        params.operation?.model_request_id ?? null,
      );
    },
  });

  if (result?.ok === false) {
    await writeResponse({
      jsonrpc: '2.0',
      id: requestId,
      result,
    });
    return;
  }

  if (emittedChunkCount === 0) {
    await writeStreamResult(requestId, result);
    return;
  }

  await writeStreamDone(requestId, result);
}

async function handleActivityCapabilityInvoke(requestId, params) {
  const result = await handleCapabilityInvoke(params, {
    onActivity: async (activity) => writeActivity(requestId, activity),
    onEvent: async (event) => {
      await writeStreamEvent(
        requestId,
        event,
        params.operation?.model_request_id ?? null,
      );
    },
  });
  await writeInvokeDone(requestId, result);
}

async function handleRequest(request) {
  if (request.method === 'sdkwork/ping') {
    const probe = probeModelChatRuntime(packageName);
    await writeResponse({
      jsonrpc: '2.0',
      id: request.id,
      result: {
        ok: true,
        backend: 'typescript_node',
        package: packageName,
        package_resolved: probe.resolved,
        app_server_available: probe.app_server_available,
        cli_available: probe.cli_available,
        runtime_available: probe.runtime_available,
        runtime_mode: probe.runtime_mode,
      },
    });
    return;
  }

  if (request.method === 'sdkwork/capability.invoke') {
    const params = request.params ?? {};
    const operation = params.operation ?? {};
    const op = operation.operation ?? operation;
    if (op === 'model_chat_stream') {
      await handleStreamingCapabilityInvoke(request.id, params);
      return;
    }
    if (op === 'model_chat' && params.activity_stream === true) {
      await handleActivityCapabilityInvoke(request.id, params);
      return;
    }
    const result = await handleCapabilityInvoke(params);
    await writeResponse({
      jsonrpc: '2.0',
      id: request.id,
      result,
    });
    return;
  }

  if (request.method === 'sdkwork/serverRequest.respond') {
    const result = await respondToSdkLiveServerRequest(request.params ?? {});
    await writeResponse({ jsonrpc: '2.0', id: request.id, result });
    return;
  }

  if (request.method === 'sdkwork/turn.interrupt') {
    const result = await interruptSdkLiveTurn(request.params ?? {});
    await writeResponse({ jsonrpc: '2.0', id: request.id, result });
    return;
  }

  await writeResponse({
    jsonrpc: '2.0',
    id: request.id,
    error: {
      code: -32601,
      message: `Method not found: ${request.method}`,
    },
  });
}

const rl = readline.createInterface({ input: process.stdin });

rl.once('close', () => {
  void closeSdkLiveRuntimes();
});

rl.on('line', (line) => {
  const trimmed = line.trim();
  if (!trimmed) {
    return;
  }

  let request;
  try {
    request = JSON.parse(trimmed);
  } catch (error) {
    void writeResponse({
      jsonrpc: '2.0',
      id: null,
      error: {
        code: -32700,
        message: `Parse error: ${error.message}`,
      },
    });
    return;
  }

  void handleRequest(request).catch((error) => {
    void writeResponse({
      jsonrpc: '2.0',
      id: request.id ?? null,
      error: {
        code: -32000,
        message: error instanceof Error ? error.message : String(error),
      },
    });
  });
});
