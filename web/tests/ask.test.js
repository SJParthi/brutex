// A REQUEST THAT CANNOT END IS A SPINNER THAT LIES.
//
// `AbortSignal.timeout` appeared zero times in `web/src`, so every fetch in
// this app waited as long as the other end held the socket. This drives the
// wrapper that fixed it, with a stub `fetch` — no server is started and no
// route is called.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { ask, ASK_MS } from '../src/lib/ask.js';

/**
 * Swap in a fake `fetch` for one call, and always put the real one back.
 *
 * @param {typeof globalThis.fetch} fake
 * @param {() => Promise<any>} body
 */
async function withFetch(fake, body) {
  const real = globalThis.fetch;
  globalThis.fetch = fake;
  try {
    return await body();
  } finally {
    globalThis.fetch = real;
  }
}

test('an ordinary answer passes straight through', async () => {
  const answer = await withFetch(async () => new Response('{}', { status: 200 }), () =>
    ask('/store.json')
  );
  assert.equal(answer.status, 200);
});

test('a request that never settles is given up, and the message names why', async () => {
  const err = await withFetch(
    /** @type {any} */ (
      (/** @type {any} */ _url, /** @type {RequestInit} */ init) =>
        new Promise((_resolve, reject) => {
          init.signal?.addEventListener('abort', () => reject(init.signal?.reason));
        })
    ),
    () => ask('/bars.json', { ms: 20 }).then(() => null).catch((e) => e)
  );
  assert.ok(err instanceof Error, 'it throws');
  assert.match(err.message, /\/bars\.json/, 'the URL is named');
  assert.match(err.message, /given up rather than left waiting/, 'the choice is named');
  assert.match(err.message, /wedged/, 'and it separates wedged from slow');
});

test("an operator's own cancel is reported as a cancel, not as a timeout", async () => {
  const mine = new AbortController();
  const err = await withFetch(
    /** @type {any} */ (
      (/** @type {any} */ _url, /** @type {RequestInit} */ init) =>
        new Promise((_resolve, reject) => {
          init.signal?.addEventListener('abort', () => reject(init.signal?.reason));
          mine.abort(new Error('cancelled by the operator'));
        })
    ),
    () => ask('/pull/spot', { signal: mine.signal }).then(() => null).catch((e) => e)
  );
  assert.match(err.message, /cancelled by the operator/);
  assert.doesNotMatch(err.message, /given up rather than left waiting/);
});

test('the ceiling is a local-read ceiling, not a vendor one', () => {
  assert.equal(ASK_MS, 15_000);
});
