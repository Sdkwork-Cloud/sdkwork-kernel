#!/usr/bin/env node
import readline from 'node:readline';
import {
  buildModelChatStreamResult,
  buildStubModelChatResult,
  invokeModelChatLive,
  mockProviderInvocationAllowed,
  probePackage,
} from './engine-sdk-live.mjs';

const packageIndex = process.argv.indexOf('--package');
const packageName =
  packageIndex >= 0 && packageIndex + 1 < process.argv.length
    ? process.argv[packageIndex + 1]
    : 'unknown';

function writeResponse(response) {
  process.stdout.write(`${JSON.stringify(response)}\n`);
}

function writeStreamResult(requestId, result) {
  const chunks = Array.isArray(result.chunks) ? result.chunks : [];
  const modelRequestId = result.model_request_id ?? null;
  for (const chunk of chunks) {
    writeResponse({
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
  writeResponse({
    jsonrpc: '2.0',
    id: requestId,
    result: {
      event: 'stream.done',
      finish_reason: result.finish_reason ?? 'stop',
      model_request_id: modelRequestId,
    },
  });
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

async function handleCapabilityInvoke(params) {
  const operation = params.operation ?? {};
  const op = operation.operation ?? operation;
  const packageProbe = probePackage(packageName);

  if (op === 'ping') {
    return {
      ok: true,
      backend: 'typescript_node',
      package: packageName,
      package_resolved: packageProbe.resolved,
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
    const handleResult = (result) =>
      op === 'model_chat_stream' ? buildModelChatStreamResult(result) : result;

    if (packageProbe.resolved) {
      try {
        return handleResult(await invokeModelChatLive(packageName, operation));
      } catch (error) {
        if (!mockProviderInvocationAllowed()) {
          return {
            ok: false,
            mode: 'sdk_live_failed',
            package: packageName,
            error: error instanceof Error ? error.message : String(error),
            model_request_id: operation.model_request_id ?? null,
          };
        }
      }
    }

    if (!mockProviderInvocationAllowed()) {
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

  if (op === 'tool_invoke') {
    if (!syntheticProviderOperationAllowed()) {
      return failClosedSyntheticOperation(op, packageProbe);
    }
    return {
      ok: true,
      mode: packageProbe.resolved ? 'sdk_probe' : 'stub',
      output: JSON.stringify({
        tool_id: operation.tool_id ?? null,
        arguments: operation.arguments ?? null,
        package: packageName,
      }),
      package: packageName,
      tool_call_id: operation.tool_call_id ?? null,
    };
  }

  if (op === 'skill_invoke') {
    if (!syntheticProviderOperationAllowed()) {
      return failClosedSyntheticOperation(op, packageProbe);
    }
    return {
      ok: true,
      mode: packageProbe.resolved ? 'sdk_probe' : 'stub',
      output: JSON.stringify({
        skill_id: operation.skill_id ?? null,
        arguments: operation.arguments ?? null,
        package: packageName,
      }),
      package: packageName,
    };
  }

  if (!syntheticProviderOperationAllowed()) {
    return failClosedSyntheticOperation(op, packageProbe);
  }

  return {
    ok: true,
    mode: 'unknown_operation',
    operation: op,
    package: packageName,
  };
}

function handleRequest(request) {
  if (request.method === 'sdkwork/ping') {
    const probe = probePackage(packageName);
    writeResponse({
      jsonrpc: '2.0',
      id: request.id,
      result: {
        ok: true,
        backend: 'typescript_node',
        package: packageName,
        package_resolved: probe.resolved,
      },
    });
    return;
  }

  if (request.method === 'sdkwork/capability.invoke') {
    const params = request.params ?? {};
    Promise.resolve(handleCapabilityInvoke(params))
      .then((result) => {
        const operation = params.operation ?? {};
        const op = operation.operation ?? operation;
        if (op === 'model_chat_stream' && result?.ok !== false) {
          writeStreamResult(request.id, result);
          return;
        }
        writeResponse({
          jsonrpc: '2.0',
          id: request.id,
          result,
        });
      })
      .catch((error) => {
        writeResponse({
          jsonrpc: '2.0',
          id: request.id,
          error: {
            code: -32000,
            message: error instanceof Error ? error.message : String(error),
          },
        });
      });
    return;
  }

  writeResponse({
    jsonrpc: '2.0',
    id: request.id,
    error: {
      code: -32601,
      message: `Method not found: ${request.method}`,
    },
  });
}

const rl = readline.createInterface({ input: process.stdin });

rl.on('line', (line) => {
  const trimmed = line.trim();
  if (!trimmed) {
    return;
  }

  let request;
  try {
    request = JSON.parse(trimmed);
  } catch (error) {
    writeResponse({
      jsonrpc: '2.0',
      id: null,
      error: {
        code: -32700,
        message: `Parse error: ${error.message}`,
      },
    });
    return;
  }

  handleRequest(request);
});
