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

/**
 * Hold the event loop open while `body` runs (P19-07, D-2566).
 *
 * `AbortSignal.timeout` arms an UNREF'D timer, and the fake `fetch` below never
 * settles, so on Node 22 and earlier nothing kept the loop alive: it drained
 * before the deadline fired, `node --test` cancelled this test ("Promise
 * resolution is still pending but the event loop has already resolved") and the
 * two after it, and the run exited 1 for a non-defect (reproduced on Node 20, 21
 * and 22). Node 24, which CI pins, happened to keep it alive, and
 * `package.json` names no engine. A ref'd interval, always cleared, makes the
 * test independent of that runtime detail without touching what `ask` does.
 *
 * @template T
 * @param {() => Promise<T>} body
 * @returns {Promise<T>}
 */
async function holdingTheLoop(body) {
  const hold = setInterval(() => {}, 1_000);
  try {
    return await body();
  } finally {
    clearInterval(hold);
  }
}

/** A fetch that answers only to its abort signal. */
const hanging = /** @type {any} */ (
  (/** @type {any} */ _url, /** @type {RequestInit} */ init) =>
    new Promise((_resolve, reject) => {
      init.signal?.addEventListener('abort', () => reject(init.signal?.reason));
    })
);

test('a timeout names the missed deadline without inventing a server diagnosis', async () => {
  const err = await holdingTheLoop(() =>
    withFetch(hanging, () => ask('/bars.json', { ms: 20 }).then(() => null).catch((e) => e))
  );
  assert.ok(err instanceof Error, 'it throws');
  assert.match(err.message, /\/bars\.json/, 'the URL is named');
  assert.match(err.message, /timed out and was cancelled/, 'the measured outcome is named');
  assert.match(err.message, /does not identify.*browser, connection, or server/, 'the cause is left unknown');
  assert.doesNotMatch(err.message, /accepted the connection|wedged|store is locked/, 'a timeout proves none of these');
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
  assert.doesNotMatch(err.message, /timed out and was cancelled/);
});

test('the ceiling is a local-read ceiling, not a vendor one', () => {
  assert.equal(ASK_MS, 15_000);
});

test('the shortest deadlines still end in a named timeout, and the loop hold is released (P19-07)', async () => {
  for (const ms of [0, 1, 999, 1_000, 1_499]) {
    const err = await holdingTheLoop(() =>
      withFetch(hanging, () => ask('/edge.json', { ms }).then(() => null).catch((e) => e))
    );
    assert.ok(err instanceof Error, `ms ${ms}`);
    assert.match(err.message, /timed out and was cancelled/, `ms ${ms}`);
    // The seconds are rounded for reading: 1,499 ms reads as 1 s, 999 as 1 s.
    assert.match(err.message, new RegExp(`within ${Math.round(ms / 1000)} s\\.`), `ms ${ms}`);
  }
  // A body that throws still releases the hold, or `node --test` would never exit.
  await assert.rejects(holdingTheLoop(async () => { throw new Error('body failed'); }), /body failed/);
});

test('the timeout test holds the loop itself rather than relying on the runtime to (P19-07)', async () => {
  const { readFileSync } = await import('node:fs');
  const self = readFileSync(new URL(import.meta.url), 'utf8');
  const timeoutTest = self.slice(self.indexOf("test('a timeout names the missed deadline"), self.indexOf("test(\"an operator's own cancel"));
  assert.match(timeoutTest, /holdingTheLoop\(/);
  assert.match(self, /const hold = setInterval\(\(\) => \{\}, 1_000\);[\s\S]*?finally \{\s*clearInterval\(hold\);/);
});
