import test from 'node:test';
import assert from 'node:assert/strict';
import { fetchWithBusyRetry, waitForRetry } from '../saved-backtest/requests.js';

/** @param {number} status @param {string | null} [after] */
const reply = (status, after = null) => new Response('{"status":"refused","refusal":"exact saved refusal","rows":[]}', {
  status, headers: after === null ? {} : { 'Retry-After': after }
});
/** @param {Response[]} replies */
function transport(replies) {
  let calls = 0;
  return { get calls() { return calls; }, request: async () => {
    const response = replies[calls++];
    assert.ok(response, 'no unexpected extra attempt');
    return response;
  } };
}
/** @param {number[]} waits */
const instant = waits => ({ wait: async (/** @type {number} */ ms, /** @type {AbortSignal} */ signal) => { signal.throwIfAborted(); waits.push(ms); }, now: () => Date.UTC(2026, 8, 8, 0, 0, 0) });

test('success and ordinary failures return the same unconsumed response without retry', async () => {
  for (const status of [200, 400, 403, 404, 500, 503]) {
    const expected = reply(status), stub = transport([expected]);
    const actual = await fetchWithBusyRetry('/saved', { signal: new AbortController().signal }, stub.request);
    assert.equal(actual, expected); assert.equal(stub.calls, 1); assert.equal(actual.bodyUsed, false);
    assert.match(await actual.text(), /exact saved refusal/);
  }
});

test('explicit busy replies wait sequentially and report each retry before it happens', async () => {
  const result = reply(200), stub = transport([reply(429), reply(429), result]);
  /** @type {any[]} */ const events = [];
  /** @type {number[]} */ const waits = [];
  const clock = instant(waits), signal = new AbortController().signal;
  const actual = await fetchWithBusyRetry('/exact?completion=pinned', { signal, onBusy: info => events.push(info) }, async (url, options) => {
    assert.equal(url, '/exact?completion=pinned'); assert.equal(options?.signal, signal); assert.equal(options?.cache, 'no-store');
    return stub.request();
  }, { ...clock, wait: async (ms, current) => {
    assert.equal(events.length, waits.length + 1, 'a retry is announced before its wait');
    await clock.wait(ms, current);
  } });
  assert.equal(actual, result); assert.equal(stub.calls, 3);
  assert.deepEqual(waits, [250, 500]);
  assert.deepEqual(events, [{ attempt: 1, nextAttempt: 2, delayMs: 250, status: 429 }, { attempt: 2, nextAttempt: 3, delayMs: 500, status: 429 }]);
});

test('the third busy response remains the exact final refusal, with no fourth attempt', async () => {
  const last = reply(429), stub = transport([reply(429), reply(429), last]);
  /** @type {number[]} */ const waits = [];
  const actual = await fetchWithBusyRetry('/saved', { signal: new AbortController().signal }, stub.request, instant(waits));
  assert.equal(actual, last); assert.equal(stub.calls, 3); assert.deepEqual(waits, [250, 500]);
  assert.equal(actual.bodyUsed, false); assert.equal((await actual.json()).refusal, 'exact saved refusal');
});

test('503 needs explicit valid Retry-After; ordinary stored-evidence refusals are never retried', async () => {
  for (const header of [null, '', '-1', '0.5', 'later', '2026', 'Tue, 31 Feb 2026 00:00:00 GMT']) {
    const expected = reply(503, header), stub = transport([expected]);
    const actual = await fetchWithBusyRetry('/saved', { signal: new AbortController().signal }, stub.request, instant([]));
    assert.equal(actual, expected); assert.equal(stub.calls, 1);
  }
  const invalid503 = new Response('{"status":"refused","refusal":"busy-looking text in a generic failure","rows":[]}', { status: 503 });
  const stub = transport([invalid503]);
  assert.equal(await fetchWithBusyRetry('/saved', { signal: new AbortController().signal }, stub.request), invalid503);
  assert.equal(stub.calls, 1);
});

test('valid seconds and HTTP-date delays are honored exactly, including zero and expired dates', async () => {
  for (const [header, expected] of [['1', 1000], ['02', 2000], ['0', 0], ['Tue, 08 Sep 2026 00:00:01 GMT', 1000], ['Mon, 07 Sep 2026 23:59:59 GMT', 0]]) {
    const result = reply(200), stub = transport([reply(503, String(header)), result]);
    /** @type {number[]} */ const waits = [];
    assert.equal(await fetchWithBusyRetry('/saved', { signal: new AbortController().signal }, stub.request, instant(waits)), result);
    assert.deepEqual(waits, [expected]); assert.equal(stub.calls, 2);
  }
});

test('a server delay beyond the bound is not shortened or converted into an endless wait', async () => {
  for (const status of [429, 503]) for (const header of ['3', '9'.repeat(128), '9'.repeat(129), 'Tue, 08 Sep 2026 00:00:03 GMT']) {
    const expected = reply(status, header), stub = transport([expected]);
    /** @type {number[]} */ const waits = [];
    const actual = await fetchWithBusyRetry('/saved', { signal: new AbortController().signal }, stub.request, instant(waits));
    assert.equal(actual, expected); assert.equal(stub.calls, 1); assert.deepEqual(waits, []);
  }
});

test('an unreadable supplied Retry-After is never replaced by an earlier guessed time', async () => {
  const result = reply(429, 'soon'), stub = transport([result]);
  /** @type {number[]} */ const waits = [];
  assert.equal(await fetchWithBusyRetry('/saved', { signal: new AbortController().signal }, stub.request, instant(waits)), result);
  assert.deepEqual(waits, []); assert.equal(stub.calls, 1);
});

test('aborting before a request or while a request resolves prevents retry and publication', async () => {
  const before = new AbortController(); before.abort(); const empty = transport([]);
  await assert.rejects(fetchWithBusyRetry('/saved', { signal: before.signal }, empty.request), { name: 'AbortError' });
  assert.equal(empty.calls, 0);
  const during = new AbortController(); let calls = 0;
  await assert.rejects(fetchWithBusyRetry('/saved', { signal: during.signal }, async () => {
    calls++; during.abort(); return reply(429);
  }), { name: 'AbortError' });
  assert.equal(calls, 1);
});

test('aborting the retry notice or the real timer starts no successor request', async () => {
  const before = new AbortController(), one = transport([reply(429)]);
  await assert.rejects(fetchWithBusyRetry('/saved', { signal: before.signal, onBusy: () => before.abort() }, one.request), { name: 'AbortError' });
  assert.equal(one.calls, 1);
  const during = new AbortController(), two = transport([reply(429)]);
  await assert.rejects(fetchWithBusyRetry('/saved', {
    signal: during.signal, onBusy: () => setTimeout(() => during.abort(), 1)
  }, two.request), { name: 'AbortError' });
  assert.equal(two.calls, 1);
});

test('a wait that resolves after cancellation still cannot start another read', async () => {
  const controller = new AbortController(), stub = transport([reply(429)]);
  await assert.rejects(fetchWithBusyRetry('/saved', { signal: controller.signal }, stub.request, {
    wait: async () => { controller.abort(); }
  }), { name: 'AbortError' });
  assert.equal(stub.calls, 1);
});

test('network errors propagate once and the timer handles both completion and already-aborted signals', async () => {
  const error = new Error('connection refused'); let calls = 0;
  await assert.rejects(fetchWithBusyRetry('/saved', { signal: new AbortController().signal }, async () => {
    calls++; throw error;
  }), why => why === error);
  assert.equal(calls, 1);
  await waitForRetry(0, new AbortController().signal);
  const cancelled = new AbortController(); cancelled.abort();
  assert.throws(() => waitForRetry(0, cancelled.signal), { name: 'AbortError' });
});
