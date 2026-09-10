import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createCatalogueLoader } from '../src/lib/catalogue-loader.js';

/** @returns {{rows:{symbol?:string}[],ready:boolean,feed:string|null,error:string|null}} */
const state = () => ({ rows: [], ready: false, feed: null, error: null });
function deferred() {
  /** @type {(value:any)=>void} */ let resolve = () => {};
  const promise = /** @type {Promise<any>} */ (new Promise((done) => { resolve = done; }));
  return { promise, resolve };
}

test('duplicate catalogue loads share one read; the held feed is reused and explicit refresh re-reads', async () => {
  const reply = deferred(), held = state();
  let calls = 0;
  const loader = createCatalogueLoader(held, async () => { calls++; return calls === 1 ? reply.promise : Response.json([{ symbol: 'NEW' }]); });
  const first = loader.load('feed');
  assert.strictEqual(loader.load('feed'), first);
  assert.strictEqual(loader.load('feed', true), first);
  reply.resolve(Response.json([{ symbol: 'NIFTY' }]));
  await first;
  assert.equal(calls, 1);
  assert.deepEqual(loader.search('NIF'), [{ symbol: 'NIFTY' }]);
  const original = held.rows;
  for (let n = 0; n < 1000; n++) await loader.load('feed');
  assert.equal(calls, 1);
  assert.strictEqual(held.rows, original);
  await loader.load('feed', true);
  assert.equal(calls, 2);
  assert.deepEqual(loader.search('NEW'), [{ symbol: 'NEW' }]);
});

test('A → B → A aborts replaced requests and only the newest A response can publish', async () => {
  const held = state();
  /** @type {{url:string,signal:AbortSignal|null|undefined,reply:ReturnType<typeof deferred>}[]} */
  const requests = [];
  const loader = createCatalogueLoader(held, (url, options = {}) => {
    const reply = deferred(); requests.push({ url, signal: options.signal, reply }); return reply.promise;
  });
  const first = loader.load('A'), second = loader.load('B'), third = loader.load('A');
  assert.equal(requests[0].signal?.aborted, true);
  assert.equal(requests[1].signal?.aborted, true);
  assert.equal(requests[2].signal?.aborted, false);
  requests[2].reply.resolve(Response.json([{ symbol: 'CURRENT' }]));
  await third;
  requests[0].reply.resolve(Response.json([{ symbol: 'STALE' }]));
  requests[1].reply.resolve(new Response(null, { status: 503 }));
  await Promise.all([first, second]);
  assert.equal(held.feed, 'A');
  assert.deepEqual(held.rows, [{ symbol: 'CURRENT' }]);
  assert.equal(held.error, null);
});

test('clearing during the body read invalidates its pending index and refuses blank-feed requests', async () => {
  const held = state(), body = deferred();
  let calls = 0;
  const loader = createCatalogueLoader(held, async () => {
    calls++;
    const response = Response.json([]);
    response.json = () => body.promise;
    return response;
  });
  const first = loader.load('A');
  await new Promise((done) => setImmediate(done));
  await loader.load(null);
  body.resolve([{ symbol: 'STALE' }]);
  await first;
  assert.equal(calls, 1);
  assert.equal(held.ready, false);
  assert.equal(held.feed, null);
  assert.deepEqual(loader.search(''), []);
  assert.match(held.error ?? '', /No feed was named/);
});

test('malformed catalogue and server refusals remain visible and are retryable', async () => {
  for (const failed of [Response.json({}), Response.json([null]), Response.json([[]]),
    new Response(null, { status: 503, headers: { 'x-brutex-master-state': 'absent', 'x-brutex-master-note': 'master not held' } })]) {
    const held = state();
    let calls = 0;
    const loader = createCatalogueLoader(held, async () => ++calls === 1 ? failed : Response.json([]));
    await loader.load('A');
    assert.equal(held.ready, false);
    assert.match(held.error ?? '', /instrument row list|absent — master not held/);
    await loader.load('A');
    assert.equal(held.ready, true);
    assert.equal(held.error, null);
    assert.equal(held.feed, 'A');
  }
});
