const CURRENT_TIME_READ_METHOD = 'currentTime/read';

export class CodexHostRequestProtocolError extends Error {
  constructor(code, message) {
    super(`${code}: ${message}`);
    this.name = 'CodexHostRequestProtocolError';
    this.code = code;
  }
}

export function isCodexCurrentTimeRequest(event) {
  return event?.method === CURRENT_TIME_READ_METHOD;
}

export function buildCodexCurrentTimeResponse(event, { now = Date.now } = {}) {
  const request = requiredRecord(event, 'request event');
  if (request.method !== CURRENT_TIME_READ_METHOD) {
    throw protocolError(
      'codex_host_request_unsupported_method',
      `expected ${CURRENT_TIME_READ_METHOD}, received ${String(request.method)}`,
    );
  }
  providerRequestId(request.requestId);
  const params = requiredRecord(request.params ?? {}, 'params');
  const providerSessionId = requiredString(
    request.providerSessionId ?? params.providerSessionId,
    'providerSessionId',
  );
  if (
    request.providerSessionId != null
    && params.providerSessionId != null
    && requiredString(request.providerSessionId, 'providerSessionId')
      !== requiredString(params.providerSessionId, 'params.providerSessionId')
  ) {
    throw protocolError(
      'codex_host_request_affinity_mismatch',
      `current-time request changed provider Session affinity from ${providerSessionId}`,
    );
  }
  if (typeof now !== 'function') {
    throw protocolError('codex_host_request_invalid_clock', 'now must be a function');
  }
  const currentTimeMs = now();
  if (!Number.isSafeInteger(currentTimeMs) || currentTimeMs < 0) {
    throw protocolError(
      'codex_host_request_invalid_clock',
      'now must return a non-negative Unix timestamp in whole milliseconds',
    );
  }
  const currentTimeAt = Math.floor(currentTimeMs / 1000);
  if (!Number.isSafeInteger(currentTimeAt)) {
    throw protocolError(
      'codex_host_request_invalid_clock',
      'current time is outside the safe Unix-seconds range',
    );
  }
  return { currentTimeAt };
}

function providerRequestId(value) {
  if (typeof value === 'string' && value.trim()) return value;
  if (typeof value === 'number' && Number.isSafeInteger(value)) return value;
  throw protocolError(
    'codex_host_request_invalid_request',
    'requestId must be a non-empty string or safe integer',
  );
}

function requiredRecord(value, fieldName) {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    throw protocolError('codex_host_request_invalid_request', `${fieldName} must be an object`);
  }
  return value;
}

function requiredString(value, fieldName) {
  if (typeof value !== 'string' || !value.trim()) {
    throw protocolError(
      'codex_host_request_invalid_request',
      `${fieldName} must be a non-empty string`,
    );
  }
  return value.trim();
}

function protocolError(code, message) {
  return new CodexHostRequestProtocolError(code, message);
}
