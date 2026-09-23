import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { parse } from 'svelte/compiler';
import { fetchWithBusyRetry } from '../saved-backtest/requests.js';
import { validateLedgerPayload } from '../src/lib/comparison.js';

const page = readFileSync(new URL('../src/routes/backtest/+page.svelte', import.meta.url), 'utf8');
const ast = parse(page);
const declarations = ['fetchLedger', 'cancelLedger'].map(name => {
  const node = ast.instance?.content.body.find((/** @type {any} */ row) => row.type === 'FunctionDeclaration' && row.id?.name === name);
  assert.ok(node, `The actual ${name} function must remain testable.`);
  return page.slice(node.start, node.end);
}).join('\n');

// A refusal fixture exercises transport and envelope validation. It is not
// market data or a recorded sweep result.
const unavailable = () => ({ path: '', total: 0, scanned: 0, hit_scan_cap: false,
  partial_tail: false, max_runs: 500, halted: 0, unsealed: 0,
  signal_rungs: ['1min', '2min', '3min', '5min', '10min', '15min', '30min', '60min'],
  best_complete: null, refusal: 'fixture: no configured store', runs: [] });
/** @param {number} status @param {unknown} [body] @param {Record<string,string>} [headers] */
const reply = (status, body = unavailable(), headers = {}) => new Response(JSON.stringify(body), { status, headers });

/** @param {(url:string,options:any)=>Promise<Response>} request
 * @param {(ms:number,signal:AbortSignal)=>Promise<any>} [wait] */
function mounted(request, wait = async () => {}) {
  const create = new Function('ask_', 'fetchWithBusyRetry', 'validateLedgerPayload', `
    let ledgerSeq = 0, ledgerAbort = null;
    let load = { phase: 'failed', body: null, why: 'older failure' };
    const LIMIT = 500;
    ${declarations}
    return { fetchLedger, cancelLedger, state: () => load, owner: () => ledgerAbort };
  `);
  return create(request, (/** @type {string} */ url, /** @type {any} */ options, /** @type {any} */ transport) =>
    fetchWithBusyRetry(url, options, transport, { wait }), validateLedgerPayload);
}

test('the actual ledger loader recovers from server backpressure with a visible, exact GET retry', async () => {
  /** @type {any[]} */ const calls = [];
  const replies = [reply(429, {}, { 'Retry-After': '1' }), reply(200)];
  const app = mounted(async (url, options) => {
    calls.push({ url, options });
    const response = replies.shift(); assert.ok(response); return response;
  }, async (ms, signal) => {
    assert.equal(ms, 1000); assert.equal(signal.aborted, false);
    assert.equal(app.state().phase, 'loading');
    assert.match(app.state().why, /busy \(429\).*1\.0 s/);
    assert.equal(app.state().body, null);
  });
  await app.fetchLedger();
  assert.equal(calls.length, 2);
  assert.ok(calls.every(c => c.url === '/backtest.json?limit=500' && c.options.cache === 'no-store' && !c.options.method && !c.options.body));
  assert.strictEqual(calls[0].options.signal, calls[1].options.signal);
  assert.deepEqual(app.state(), { phase: 'ready', body: unavailable(), why: '' });
  assert.equal(app.owner(), null);
});

test('persistent busy replies stop; generic 503 evidence is validated once and malformed success is refused', async () => {
  for (const [status, body, expectedCalls, phase] of [
    [429, {}, 3, 'failed'], [503, unavailable(), 1, 'ready'], [200, {}, 1, 'failed']
  ]) {
    let calls = 0;
    const app = mounted(async () => { calls++; return reply(Number(status), body); });
    await app.fetchLedger();
    assert.equal(calls, expectedCalls); assert.equal(app.state().phase, phase);
    if (phase === 'failed') assert.equal(app.state().body, null);
    assert.equal(app.owner(), null);
  }
});

test('replacing a ledger read aborts its pending wait and an old completion cannot replace the new result', async () => {
  /** @type {((value?:unknown)=>void) | undefined} */ let release;
  /** @type {AbortSignal | undefined} */ let oldSignal;
  let calls = 0;
  const app = mounted(async (_url, options) => {
    calls++;
    if (calls === 1) { oldSignal = options.signal; return reply(429); }
    return reply(200);
  }, async () => { await new Promise(resolve => { release = resolve; }); });
  const old = app.fetchLedger();
  while (!release) await Promise.resolve();
  await app.fetchLedger();
  assert.equal(oldSignal?.aborted, true);
  const accepted = app.state();
  release(); await old;
  assert.strictEqual(app.state(), accepted);
  assert.equal(calls, 2); assert.equal(app.owner(), null);
});

test('component cancellation aborts the waiting read and revokes any late reply', async () => {
  /** @type {((value?:unknown)=>void) | undefined} */ let release;
  let calls = 0;
  const app = mounted(async () => { calls++; return reply(429); }, async () => {
    await new Promise(resolve => { release = resolve; });
  });
  const pending = app.fetchLedger();
  while (!release) await Promise.resolve();
  const owner = app.owner();
  app.cancelLedger();
  assert.equal(owner.signal.aborted, true);
  const before = app.state();
  release(); await pending;
  assert.strictEqual(app.state(), before);
  assert.equal(calls, 1); assert.equal(app.owner(), null);
});
