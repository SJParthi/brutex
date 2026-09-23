import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createReceiptBatch, receiptPlan, receiptObservation, MAX_RECEIPT_JOBS, STRICT_KNOBS } from '../src/lib/receipt-batch.js';

const rungs = ['1min', '2min', '3min', '5min', '10min', '15min', '30min', '60min'];
/** @param {Record<string,unknown>} [extra] */
const input = (extra = {}) => ({ feed: 'fixture-feed', symbols: ['NIFTY', 'BANKNIFTY'], rungs: ['60min', '2min'], supportedRungs: rungs,
  from: '2025-03', to: '2025-05', minHits: '21', knobs: { validate: '0', top: '5' }, ...extra });
/** @param {unknown} body @param {number} [status] */
const response = (body, status = 200) => new Response(JSON.stringify(body), { status });
/** @param {string} attempt */
const accepted = attempt => response({ accepted: true, refusal: null, attempt }, 202);
/** @param {import('../src/lib/receipt-batch.js').Request} request @param {string|null} attempt @param {Record<string,unknown>} [extra] */
const run = (request, attempt, extra = {}) => ({ where: 'browser', kind: 'command', feed: request.feed,
  underlying: request.underlying, from_year: request.from_year, from_month: request.from_month,
  to_year: request.to_year, to_month: request.to_month, attempt_key: attempt,
  in_flight: false, report: 'REAL MARKET DATA\nReceipt-checked research completed\n', refusal: null, ...extra });
/** @param {import('../src/lib/receipt-batch.js').Request} request @param {string} [attempt] @returns {import('../src/lib/receipt-batch.js').Job} */
const job = (request, attempt = '9007199254740993') => ({ request, attempt, phase: 'running', report: null, why: '' });

/** @param {(url:string,options?:RequestInit,count?:number)=>Response|Promise<Response>} handler */
function driver(handler) {
  /** @type {import('../src/lib/receipt-batch.js').BatchState[]} */
  const snapshots = [];
  /** @type {{url:string,body:import('../src/lib/receipt-batch.js').Request|null}[]} */
  const calls = [];
  const batch = createReceiptBatch({ changed: state => snapshots.push(state), wait: async () => {},
    request: async (url, options) => { calls.push({ url, body: options?.body ? JSON.parse(String(options.body)) : null }); return handler(url, options, calls.length); } });
  return { batch, calls, snapshots, latest: () => { const last = snapshots.at(-1); assert.ok(last); return last; } };
}

test('the plan honors every selected instrument, canonical rung order and exact u64 support', () => {
  const source = input({ symbols: ['NIFTY', 'BANKNIFTY', 'NIFTY'], rungs: ['60min', '2min', '60min'], minHits: '18446744073709551615' });
  const plan = receiptPlan(source);
  assert.deepEqual(plan.map(row => `${row.underlying}/${row.rung}`), ['NIFTY/2min', 'NIFTY/60min', 'BANKNIFTY/2min', 'BANKNIFTY/60min']);
  source.knobs.top = '99'; source.symbols[0] = 'REPLACED'; source.rungs[0] = '1day';
  for (const row of plan) {
    assert.equal(row.command, 'audit-audited-range'); assert.equal(Reflect.get(row, 'top'), '5');
    assert.equal(row.min_hits, '18446744073709551615');
    assert.equal(Object.hasOwn(row, 'support_ppm'), false); assert.ok(Object.isFrozen(row));
  }
  assert.ok(Object.isFrozen(plan));
});

test('invalid, absent, daily, overflowing and oversized plans refuse without truncation', () => {
  for (const extra of [
    { minHits: '' }, { minHits: '0' }, { minHits: '-1' }, { minHits: '1.5' }, { minHits: '1e3' },
    { minHits: '18446744073709551616' }, { from: '2025-13' }, { from: '0000-01' }, { to: '2024-12' },
    { feed: '' }, { symbols: [] }, { symbols: [''] }, { rungs: [] }, { rungs: ['1day'] }, { rungs: ['7min'] },
    { supportedRungs: [] }, { supportedRungs: ['1day'] }, { supportedRungs: ['2min', '2min'] },
    { knobs: { support_ppm: '1000' } }, { knobs: { sizing_rate_bp: '7500' } }, { knobs: { receipt_path: '/unexpected' } },
    { knobs: { exit_cutoff: '15:30' } }, { symbols: Array.from({ length: MAX_RECEIPT_JOBS + 1 }, (_, n) => `S${n}`), rungs: ['2min'] }
  ]) assert.throws(() => receiptPlan(input(extra)), Error, JSON.stringify(extra).slice(0, 90));
  assert.equal(receiptPlan(input({ symbols: Array.from({ length: MAX_RECEIPT_JOBS }, (_, n) => `S${n}`), rungs: ['2min'] })).length, MAX_RECEIPT_JOBS);
});

test('strict setting names match the shared HTTP contract, excluding absolute-support conflicts', () => {
  const rust = readFileSync(new URL('../../crates/api/src/sweeprun.rs', import.meta.url), 'utf8');
  const table = rust.slice(rust.indexOf('const KNOBS:'), rust.indexOf('const KNOBS:') + 2500);
  assert.notEqual(rust.indexOf('const KNOBS:'), -1);
  const keys = [...table.matchAll(/\("([a-z_]+)",\s*"BRUTEX_[A-Z_]+"\)/g)].map(match => match[1]);
  assert.equal(keys.length, 16);
  assert.deepEqual([...STRICT_KNOBS].sort(), keys.filter(key => !['support_ppm', 'sizing_rate_bp'].includes(key)).sort());
});

test('all four jobs wait for their own exact terminal status, including tokens above Number precision', async () => {
  const plan = receiptPlan(input());
  /** @type {import('../src/lib/receipt-batch.js').Request|null} */
  let current = null;
  let token = ''; let index = 0; let observations = 0;
  const testRun = driver((url, options) => {
    if (url === '/engine/command') {
      assert.equal(observations, index * 2, 'the prior job must have two observations before another POST');
      current = JSON.parse(String(options?.body)); token = String(9007199254740993n + BigInt(index++));
      return accepted(token);
    }
    observations += 1;
    assert.ok(current);
    assert.equal(url, `/backtest/run.json?attempt=${token}`, 'every poll selects the exact accepted decimal token');
    return response({ running: run(current, token, observations % 2 ? { in_flight: true, report: null } : {}) });
  });
  await testRun.batch.start(plan);
  assert.equal(index, 4); assert.equal(observations, 8);
  assert.deepEqual(testRun.latest().jobs.map(item => item.phase), ['done', 'done', 'done', 'done']);
  assert.deepEqual(testRun.latest().jobs.map(item => item.attempt), ['9007199254740993', '9007199254740994', '9007199254740995', '9007199254740996']);
  assert.equal(testRun.latest().phase, 'done');
  assert.deepEqual(testRun.calls.filter(call => call.body).map(call => call.body), plan);
});

test('every foreign context, CLI latest event, lossy token and unknown completion is rejected', () => {
  const expected = job(receiptPlan(input())[0]);
  assert.equal(receiptObservation(run(expected.request, expected.attempt), expected).phase, 'done');
  for (const changed of [
    { where: 'cli' }, { where: undefined }, { kind: 'sweep' }, { attempt_key: '9007199254740992' },
    { attempt_key: 9007199254740993 }, { attempt_key: '' }, { feed: 'other' }, { underlying: 'RELIANCE' },
    { from_year: 2024 }, { from_month: 4 }, { to_year: 2026 }, { to_month: 6 },
    { status: 'unknown' }, { report: null }, { in_flight: 'false' }
  ]) assert.throws(() => receiptObservation(run(expected.request, expected.attempt, changed), expected), Error, JSON.stringify(changed));
  assert.throws(() => receiptObservation(null, expected));
});

test('prestart configuration refusal is preserved and sends no remaining job', async () => {
  const reason = 'Required receipt configuration missing: BRUTEX_CHECKSUM_RECEIPTS';
  const testRun = driver(() => response({ accepted: false, started: false, refusal: reason, missing: ['BRUTEX_CHECKSUM_RECEIPTS'] }, 503));
  await testRun.batch.start(receiptPlan(input()));
  assert.equal(testRun.calls.length, 1); assert.equal(testRun.latest().why, reason);
  assert.equal(testRun.latest().phase, 'failed');
  assert.deepEqual(testRun.latest().jobs.map(item => item.phase), ['failed', 'not-started', 'not-started', 'not-started']);
});

test('poststart refusal preserves report/refusal and never advances the queue', async () => {
  const plan = receiptPlan(input());
  const testRun = driver(url => url === '/engine/command' ? accepted('91') : response({ running: run(plan[0], '91', { refusal: 'refused: required child receipt changed', report: null }) }));
  await testRun.batch.start(plan);
  assert.equal(testRun.latest().phase, 'failed'); assert.equal(testRun.latest().jobs[0].attempt, '91');
  assert.equal(testRun.latest().why, 'refused: required child receipt changed'); assert.equal(testRun.calls.length, 2);
});

test('ambiguous acceptance is unknown, never an automatic POST retry or a successful batch', async () => {
  for (const answer of [
    () => { throw new Error('Connection lost after submission'); },
    () => new Response('{', { status: 202 }),
    () => response({ accepted: true }, 202),
    () => response({ accepted: true, attempt: 9007199254740993 }, 202),
    () => response({ refusal: 'unknown shape' }, 500)
  ]) {
    const testRun = driver(answer);
    await testRun.batch.start(receiptPlan(input()));
    assert.equal(testRun.latest().phase, 'unknown'); assert.equal(testRun.calls.length, 1);
    assert.equal(testRun.latest().jobs[0].attempt, null);
    await testRun.batch.recheck(); assert.equal(testRun.calls.length, 1);
    await assert.rejects(() => testRun.batch.start(receiptPlan(input())), /resolved/);
  }
});

test('lost or foreign status stops the queue; recheck can resolve only the accepted attempt', async () => {
  for (const bad of [() => response({}, 503), () => response({ running: null }), () => { throw new Error('timeout'); },
    () => response({ running: { where: 'cli', attempt_key: '55', in_flight: false, report: 'completed' } })]) {
    const plan = receiptPlan(input()); let polls = 0;
    const testRun = driver(url => url === '/engine/command' ? accepted('55') : ++polls === 1 ? bad() : response({ running: run(plan[0], '55') }));
    await testRun.batch.start(plan);
    assert.equal(testRun.latest().phase, 'unknown'); assert.equal(testRun.calls.filter(call => call.body).length, 1);
    await testRun.batch.recheck();
    assert.equal(testRun.latest().phase, 'stopped'); assert.equal(testRun.latest().jobs[0].phase, 'done');
    assert.equal(testRun.latest().jobs[1].phase, 'not-started'); assert.equal(testRun.calls.filter(call => call.body).length, 1);
    assert.deepEqual(testRun.calls.filter(call => !call.body).map(call => call.url),
      ['/backtest/run.json?attempt=55', '/backtest/run.json?attempt=55'], 'recheck cannot switch to global latest status');
  }
});

test('a replaced exact browser slot remains unknown without a global fallback or another POST', async () => {
  const testRun = driver(url => url === '/engine/command' ? accepted('9007199254740993') : response({
    running: { where: 'browser', status: 'unknown', requested_attempt: '9007199254740993',
      in_flight: false, report: null, refusal: null, why: 'This exact browser attempt is not retained.' }
  }));
  await testRun.batch.start(receiptPlan(input()));
  await testRun.batch.recheck();
  assert.equal(testRun.latest().phase, 'unknown');
  assert.equal(testRun.latest().jobs[0].attempt, '9007199254740993');
  assert.equal(testRun.calls.filter(call => call.body).length, 1);
  assert.deepEqual(testRun.calls.filter(call => !call.body).map(call => call.url),
    ['/backtest/run.json?attempt=9007199254740993', '/backtest/run.json?attempt=9007199254740993']);
  await assert.rejects(() => testRun.batch.start(receiptPlan(input())), /resolved/);
});

test('stop waits for the accepted server job and disposal prevents any subsequent POST', async () => {
  for (const dispose of [false, true]) {
    const plan = receiptPlan(input());
    /** @type {ReturnType<typeof createReceiptBatch>} */
    let batch;
    const testRun = driver(url => {
      if (url === '/engine/command') return accepted('17');
      if (dispose) batch.dispose(); else batch.stop();
      return response({ running: run(plan[0], '17') });
    }); batch = testRun.batch;
    await batch.start(plan);
    assert.equal(testRun.calls.filter(call => call.body).length, 1);
    if (!dispose) { assert.equal(testRun.latest().phase, 'stopped'); assert.equal(testRun.latest().jobs[0].phase, 'done'); }
    else assert.notEqual(testRun.latest().phase, 'done', 'a disposed page cannot publish a terminal batch');
  }
});

test('a concurrent second press cannot replace or duplicate the first request', async () => {
  /** @type {()=>void} */
  let release = () => { throw new Error('The promise was not initialized'); };
  const held = new Promise(resolve => release = () => resolve(undefined));
  const plan = receiptPlan(input());
  const testRun = driver(async url => url === '/engine/command' ? (await held, accepted('8')) : response({ running: run(plan[0], '8', { refusal: 'stop fixture' }) }));
  const pending = testRun.batch.start(plan);
  await assert.rejects(() => testRun.batch.start(plan), /resolved/);
  release(); await pending;
  assert.equal(testRun.calls.filter(call => call.body).length, 1);
});

test('the actual page wires strict mode, supported rungs, exact settings and truthful intraday labels', () => {
  const page = readFileSync(new URL('../src/routes/backtest/+page.svelte', import.meta.url), 'utf8');
  const component = readFileSync(new URL('../src/lib/ReceiptBatch.svelte', import.meta.url), 'utf8');
  assert.match(page, /<ReceiptBatch[\s\S]*?symbols=\{\[\.\.\.pickedSymbols\]\} rungs=\{\[\.\.\.pickedRungs\]\}/);
  assert.match(page, /supportedRungs=\{signalRungSet \? \[\.\.\.signalRungSet\] : \[\]\}/);
  assert.match(page, /KNOB_FIELDS\.filter\(field => STRICT_KNOBS\.includes\(field\.key\)\)/);
  assert.match(page, /Only the first selected instrument runs in ordinary mode/);
  assert.match(component, /Forced exit at 15:10 IST/);
  assert.match(component, /cost-excluded research, not institutional admission/);
  assert.match(component, /Daily bars provide context, never a swept/);
  assert.match(component, /controller\.dispose\(\)/);
  assert.doesNotMatch(component, /\/backtest\/run['"]|ledger-v6['"]|15:30/);
});
