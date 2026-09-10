import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { parse } from 'svelte/compiler';
import { createInspectionReader, inspectionState, savedResultsUrl, INITIAL_INSPECTION, INSPECTION_MESSAGE } from '../src/lib/runtime-inspection.js';

const declaration = () => ({ schema: 1, mode: 'read-only-main-app', can_sweep: false,
  can_pull: false, can_write: false, release_cleared: false,
  automatic_recovery: false, automatic_acquisition: false });

test('the native application names itself without claiming sweep readiness', async () => {
  const native = { schema: 2, mode: 'application', can_sweep: true, can_pull: true,
    can_write: true, release_cleared: false, commit: 'a'.repeat(40) };
  const reader = createInspectionReader(async () => Response.json(native), () => {});
  const state = await reader.load();
  assert.equal(state.mode, 'application');
  assert.equal(state.phase, 'ready');
  assert.equal(state.canSweep, true);
  assert.doesNotMatch(state.why, /Legacy/);
  assert.match(state.why, /checked before/);
  const unstamped = inspectionState({ ...native, can_sweep: false, commit: null });
  assert.equal(unstamped.canSweep, false);
  assert.match(unstamped.why, /no verified source identity/);
  for (const changed of [
    { commit: null }, { commit: 'A'.repeat(40) }, { commit: undefined },
    { can_sweep: 'true' }, { can_sweep: false }, { release_cleared: true },
    { can_pull: false }, { can_write: false }, { schema: 3 }
  ]) assert.throws(() => inspectionState({ ...native, ...changed }), Error);
});

test('only the complete explicit read-only declaration earns an inspection label', () => {
  assert.equal(INITIAL_INSPECTION.canSweep, false);
  assert.deepEqual(inspectionState(declaration()), { phase: 'ready', mode: 'read-only-main-app',
    canSweep: false, canPull: false, savedResultsUrl: null, why: INSPECTION_MESSAGE });
  for (const key of Object.keys(declaration())) {
    for (const bad of [undefined, null, true, 'false', 0, 2]) {
      const value = { ...declaration(), [key]: bad };
      assert.throws(() => inspectionState(value), /recognised inspection-mode declaration/);
    }
  }
  for (const bad of [null, [], 'read-only-main-app', false]) {
    assert.throws(() => inspectionState(bad), /recognised inspection-mode declaration/);
  }
});

test('saved-results links require literal loopback, an exact checkpoint and the viewer query contract', () => {
  const query = `identity=${'a'.repeat(64)}&pin=${'b'.repeat(64)}&batch=1&rung=0&offset=480&setting=480`;
  const valid = `http://127.0.0.1:8081/backtest?${query}`;
  assert.equal(savedResultsUrl(valid), valid);
  assert.equal(savedResultsUrl(`http://[::1]:8081/backtest?${query}`), `http://[::1]:8081/backtest?${query}`);
  assert.equal(inspectionState({ ...declaration(), saved_results_url: valid }).savedResultsUrl, valid);
  assert.equal(savedResultsUrl(null), null); assert.equal(savedResultsUrl(undefined), null);
  for (const bad of [
    valid.replace('127.0.0.1', 'example.com'), valid.replace('127.0.0.1', '127.1'),
    valid.replace('http:', 'https:'), valid.replace('127.0.0.1', 'user@127.0.0.1'),
    valid.replace(':8081', ''), valid.replace('/backtest', '/db'), valid + '#fragment',
    valid + '&store=/somewhere', valid + '&rung=1', valid.replace('setting=480', 'setting=479'),
    valid.replace(/&pin=[^&]+/, ''), valid.replace('&batch=1', ''), valid.replace('&rung=0', ''),
    valid.replace('&offset=480', ''), valid.replace('identity=a', 'identity=%61'), '', 7
  ]) assert.throws(() => savedResultsUrl(bad), Error, `refuse ${String(bad)}`);
});

test('layout and page consumers share one inspection read and retain the verified restrictions', async () => {
  let calls = 0;
  /** @type {(response:Response)=>void} */ let resolve = () => { throw new Error('Request missing'); };
  /** @type {import('../src/lib/runtime-inspection.js').InspectionState[]} */ const seen = [];
  const reader = createInspectionReader(async (url, options) => {
    calls++;
    assert.equal(url, '/inspection.json');
    assert.equal(options?.cache, 'no-store');
    return new Promise(done => { resolve = done; });
  }, state => seen.push(state));
  const first = reader.load(), second = reader.load();
  assert.strictEqual(first, second);
  assert.equal(calls, 1);
  assert.equal(seen[0].phase, 'reading');
  assert.equal(seen[0].canSweep, false);
  resolve(Response.json(declaration()));
  await first;
  await reader.load();
  assert.equal(calls, 1);
  assert.equal(seen.at(-1)?.mode, 'read-only-main-app');
  assert.equal(seen.every(state => !state.canSweep && !state.canPull), true);
});

test('failed, malformed, non-JSON and unknown-mode responses keep commands disabled and may retry', async () => {
  const cases = [
    () => new Response('unavailable', { status: 503 }),
    () => new Response('<html>fallback</html>', { headers: { 'content-type': 'text/html' } }),
    () => new Response('{', { headers: { 'content-type': 'application/json' } }),
    () => Response.json({ ...declaration(), mode: 'unrecognised' }),
    () => { throw new Error('connection unavailable'); }
  ];
  for (const failed of cases) {
    let calls = 0;
    const reader = createInspectionReader(async () => ++calls === 1 ? failed() : Response.json(declaration()), () => {});
    const first = await reader.load();
    assert.equal(first.phase, 'failed');
    assert.equal(first.canSweep, false); assert.equal(first.canPull, false);
    assert.match(first.why, /could not be verified/);
    assert.equal((await reader.load()).phase, 'ready');
    assert.equal(calls, 2);
  }
});

test('a legacy 404 is explicitly identified and never described as inspection or release clearance', async () => {
  const reader = createInspectionReader(async () => new Response(null, { status: 404 }), () => {});
  const state = await reader.load();
  assert.equal(state.phase, 'legacy'); assert.equal(state.mode, 'legacy');
  assert.equal(state.canSweep, true); assert.equal(state.canPull, true);
  assert.match(state.why, /not sweep-readiness clearance/);
  assert.notEqual(state.why, INSPECTION_MESSAGE);
});

test('actual Run and Descend functions stop before any sweep state or transport when execution is disabled', async () => {
  const page = readFileSync(new URL('../src/routes/backtest/+page.svelte', import.meta.url), 'utf8');
  const ast = parse(page);
  for (const name of ['startSweep', 'startDescent']) {
    const fn = ast.instance?.content.body.find((/** @type {any} */ row) => row.type === 'FunctionDeclaration' && row.id?.name === name);
    assert.ok(fn, `${name} must remain a callable page function`);
    const body = page.slice(fn.start, fn.end);
    const run = new Function('runtimeInspection', body + `\nreturn ${name};`)({ value: INITIAL_INSPECTION });
    assert.equal(await run(), undefined, 'no remaining page state is in scope unless the guard is bypassed');
  }
  assert.match(page, /blockedReason=\{!runtimeInspection.value.canSweep \? runtimeInspection.value.why/);
  assert.match(page, /disabled=\{!runtimeInspection.value.canSweep \|\| indexWorkflow \|\| receiptBusy/);
  assert.match(page, /disabled=\{!runtimeInspection.value.canSweep \|\| indexWorkflow \|\| inFlight/);
  const layout = readFileSync(new URL('../src/routes/+layout.svelte', import.meta.url), 'utf8');
  assert.match(layout, /runtimeInspection.value.why/);
  assert.match(layout, /void loadInspection\(\)/);
});
