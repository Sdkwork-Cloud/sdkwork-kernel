import assert from 'node:assert/strict';
import test from 'node:test';

import {
  buildCodexCurrentTimeResponse,
  isCodexCurrentTimeRequest,
} from './codex-app-server-host-requests.mjs';

test('builds whole Unix seconds for the canonical provider Session request', () => {
  const event = {
    method: 'currentTime/read',
    params: { providerSessionId: 'provider-session-1' },
    providerSessionId: 'provider-session-1',
    requestId: 73,
  };

  assert.equal(isCodexCurrentTimeRequest(event), true);
  assert.deepEqual(
    buildCodexCurrentTimeResponse(event, { now: () => 1_785_542_400_999 }),
    { currentTimeAt: 1_785_542_400 },
  );
  assert.equal(JSON.stringify(event).includes('thread'), false);
});

test('fails closed for invalid current-time request affinity and clocks', () => {
  assert.throws(
    () => buildCodexCurrentTimeResponse({
      method: 'currentTime/read',
      params: { providerSessionId: 'provider-session-2' },
      providerSessionId: 'provider-session-1',
      requestId: 'time-1',
    }),
    hasCode('codex_host_request_affinity_mismatch'),
  );
  for (const now of [null, () => -1, () => 1.5, () => Number.POSITIVE_INFINITY]) {
    assert.throws(
      () => buildCodexCurrentTimeResponse({
        method: 'currentTime/read',
        params: { providerSessionId: 'provider-session-1' },
        providerSessionId: 'provider-session-1',
        requestId: 'time-1',
      }, { now }),
      hasCode('codex_host_request_invalid_clock'),
    );
  }
});

function hasCode(code) {
  return (error) => error?.code === code;
}
