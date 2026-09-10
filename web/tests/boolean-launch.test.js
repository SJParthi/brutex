import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { policy as indexPolicy } from './index-consistency-fixture.js';
// @ts-expect-error The installed internal runtime has no public declaration; this checks its real proxy behavior.
import { proxy } from 'svelte/internal/client';
import { BOOLEAN_SEARCH_COMMAND, validateBooleanLaunchMetadata, booleanLaunchPlan,
  booleanLaunchObservation, booleanPlanTimeframes, createBooleanLaunch } from '../src/lib/boolean-launch.js';

const U64 = '18446744073709551615', ATTEMPT = '9223372036854775809';
const SEARCH = '1a'.repeat(32), POLICY = 'e3'.repeat(32);
const rungs = ['1min', '2min', '3min', '5min', '10min', '15min', '30min', '60min'];
// Only transport fixtures: names come from the native table, with no browser
// policy threshold or alternative admission vocabulary introduced by a test.
const admission = readFileSync(new URL('../../crates/runner/src/admission.rs', import.meta.url), 'utf8');
const fieldTable = admission.slice(admission.indexOf('impl AdmissionFieldV1'), admission.indexOf('/// Why a policy draft'));
const policyNames = [...fieldTable.matchAll(/Self::\w+ => "([a-z_]+)"/g)].map(match => match[1]);
assert.equal(policyNames.length, 39);
const projection = readFileSync(new URL('../../crates/api/src/booleanadmission_projection.rs', import.meta.url), 'utf8');
const projectionPolicy = projection.slice(projection.search(/pub\((?:super|crate)\) fn policy\(/));
const numericPolicy = /fields!\(([\s\S]+?)\);/.exec(projectionPolicy);
assert.ok(numericPolicy);
const policyOrder = [...numericPolicy[1].matchAll(/\b([a-z][a-z_]+)\b/g)].map(match => match[1]);
const booleanNames = [...projectionPolicy.matchAll(/"(require_[a-z_]+)"/g)].map(match => match[1]);
policyOrder.push(...booleanNames);
assert.deepEqual([...policyOrder].sort(), [...policyNames].sort());

/** @returns {Record<string,any>} */
function metadata() {
  return { schema_version: 1, model: 'boolean-qualified-search-launch', command: BOOLEAN_SEARCH_COMMAND,
    timeframes: [...rungs], ready: true, refusal: null, index_consistency_policy: indexPolicy(),
    limits: { max_symbols: 3, request_bytes: 16384, horizon_bars_max: '4294967295', max_points_max: '184467440737095516',
      checksum_max_bytes: '8192', checksum_max_records: '32', observation_bytes: '4096', replay_nodes: '64' },
    configured: { horizon_bars: null, max_points: '20', batch_programs: null, node_allowance: null, batch_allowance: null },
    policy: { ready: true, digest: POLICY, values: policyOrder.map(name => ({ name, value: booleanNames.includes(name) ? false : '0' })), refusal: null }
  };
}
/** @param {Record<string,unknown>} [extra] */
const input = (extra = {}) => ({ feed: 'fixture-feed', symbols: ['NIFTY', 'RELIANCE'], from: '2025-01', to: '2025-03',
  laterFrom: '2025-04', laterTo: '2025-06', horizonBars: '7', maxPoints: '20', batchPrograms: '2', nodeAllowance: '64', batchAllowance: '3', ...extra });
/** @param {Record<string,unknown>} [extra] @param {Record<string,any>} [native] */
const planFor = (extra = {}, native = metadata()) => booleanLaunchPlan(input(extra), validateBooleanLaunchMetadata(native));
/** @param {unknown} body @param {number} [status] */
const response = (body, status = 200) => new Response(JSON.stringify(body), { status });
/** @param {string} [attempt] */
const accepted = (attempt = ATTEMPT) => response({ accepted: true, refusal: null, attempt }, 202);
/** @param {any} plan @param {string} [attempt] @param {Record<string,unknown>} [extra] */
const run = (plan, attempt = ATTEMPT, extra = {}) => ({ ...plan, where: 'browser', kind: 'command',
  attempt: Number(attempt), attempt_key: attempt, in_flight: false, finished_micros: 1750000000000000,
  report: 'Recorded Boolean research batch ended.', refusal: null,
  search_identity: SEARCH, exhausted: false, completed_batches: '1', ...extra });
/** @param {any} plan @param {Record<string,unknown>} [extra] */
const running = (plan, extra = {}) => run(plan, ATTEMPT, { in_flight: true, finished_micros: null, report: null, ...extra });

function latch() {
  /** @type {()=>void} */
  let release = () => { throw new Error('Latch is not initialized'); };
  const promise = new Promise(resolve => { release = () => resolve(undefined); });
  return { promise, release };
}
/** @param {(url:string,options?:RequestInit,count?:number)=>Response|Promise<Response>} handler
 * @param {{wait?:(signal:AbortSignal)=>Promise<unknown>,changed?:(state:import('../src/lib/boolean-launch.js').LaunchState)=>void}} [options] */
function driver(handler, options = {}) {
  /** @type {import('../src/lib/boolean-launch.js').LaunchState[]} */
  const snapshots = [];
  /** @type {{url:string,body:any,options:RequestInit|undefined}[]} */
  const calls = [];
  /** @type {string[]} */
  const searches = [];
  const launch = createBooleanLaunch({ changed: state => { snapshots.push(state); options.changed?.(state); },
    onSearch: identity => searches.push(identity), wait: options.wait ?? (async () => {}),
    request: async (url, init) => {
      calls.push({ url, body: init?.body ? JSON.parse(String(init.body)) : null, options: init });
      return handler(url, init, calls.length);
    } });
  return { launch, calls, snapshots, searches, latest: () => { const value = snapshots.at(-1); assert.ok(value); return value; } };
}

test('metadata preserves the native all-eight-rung contract, zero policy values and absent defaults', () => {
  const source = metadata();
  const checked = validateBooleanLaunchMetadata(source);
  assert.deepEqual(checked.timeframes, rungs);
  assert.deepEqual(checked.policy.values.map((/** @type {{name:string}} */ row) => row.name), policyOrder);
  assert.equal(checked.policy.values[0].value, '0', 'a legitimate zero is never replaced by a default');
  assert.equal(checked.configured.horizon_bars, null);
  assert.equal(checked.configured.batch_allowance, null);
  assert.ok(Object.isFrozen(checked)); assert.ok(Object.isFrozen(checked.policy.values[0]));
  source.policy.values[0].value = '99'; source.timeframes.reverse(); source.configured.max_points = '99';
  assert.equal(checked.policy.values[0].value, '0'); assert.equal(checked.configured.max_points, '20');
  assert.deepEqual(checked.timeframes, rungs);
});

test('every native policy name and wire type is required before Boolean metadata can authorize a launch', () => {
  for (let index = 0; index < policyOrder.length; index += 1) {
    const name = policyOrder[index];
    for (const defect of ['missing', 'unknown replacement', 'duplicate', 'wrong type']) {
      const body = metadata();
      if (defect === 'missing') body.policy.values.splice(index, 1);
      else if (defect === 'unknown replacement') body.policy.values[index].name = 'unknown_policy_field';
      else if (defect === 'duplicate') body.policy.values[index] = structuredClone(body.policy.values[(index + 1) % policyOrder.length]);
      else body.policy.values[index].value = booleanNames.includes(name) ? 'false' : 0;
      let branded = null;
      assert.throws(() => {
        branded = validateBooleanLaunchMetadata(body);
        booleanLaunchPlan(input(), branded);
      }, Error, `${name}: ${defect}`);
      assert.equal(branded, null, `${name}: ${defect} must refuse before metadata is branded`);
    }
  }
  const override = metadata();
  override.policy.values.find((/** @type {{name:string}} */ field) => field.name === 'min_worst_reward_risk_ppm').value = '2500000';
  const exact = validateBooleanLaunchMetadata(override);
  assert.equal(exact.policy.values.find((/** @type {{name:string}} */ field) => field.name === 'min_worst_reward_risk_ppm').value, '2500000');
  assert.equal(booleanLaunchPlan(input(), exact).expected_policy_digest, POLICY);
});

test('selected-timeframe support requires an explicit boolean capability; old metadata remains compatible', () => {
  for (const capability of [true, false]) {
    const body = { ...metadata(), selected_timeframes_supported: capability };
    assert.equal(validateBooleanLaunchMetadata(body).selected_timeframes_supported, capability);
  }
  for (const capability of [undefined, null, 1, 0, 'true', 'false', [], {}]) {
    assert.throws(() => validateBooleanLaunchMetadata({ ...metadata(), selected_timeframes_supported: capability }), /schema/);
  }
  assert.equal(Object.hasOwn(validateBooleanLaunchMetadata(metadata()), 'selected_timeframes_supported'), false);
});

test('all 255 nonempty canonical timeframe subsets retain their exact immutable request scope', () => {
  const checked = validateBooleanLaunchMetadata({ ...metadata(), selected_timeframes_supported: true });
  for (let mask = 1; mask < 1 << rungs.length; mask += 1) {
    const timeframes = rungs.filter((_, index) => mask & (1 << index));
    const expected = [...timeframes];
    const plan = booleanLaunchPlan(input({ timeframes }), checked);
    assert.deepEqual(plan.timeframes, expected);
    assert.deepEqual(booleanPlanTimeframes(plan), expected);
    assert.ok(Object.isFrozen(plan.timeframes));
    timeframes.splice(0, timeframes.length, '1day');
    assert.deepEqual(plan.timeframes, expected, 'editing the form cannot widen or replace the captured selection');
    assert.equal(booleanLaunchObservation(run(plan), plan, ATTEMPT).phase, 'done');
    if (expected.length > 1) assert.throws(() => booleanLaunchPlan(input({ timeframes: [...expected].reverse() }), checked), /server’s order/);
  }
  assert.deepEqual(booleanLaunchPlan(input(), checked).timeframes, rungs, 'omission retains the historical all-eight contract');
});

test('empty, duplicate, unknown, daily, sparse and reordered explicit timeframe selections are refused', () => {
  const native = { ...metadata(), selected_timeframes_supported: true };
  const invalid = [null, [], '1min', ['1day'], ['1min', '1day'], ['1min', '1min'],
    ['5min', '1min'], ['0min'], ['120min'], [' 1min'], ['1MIN'], ['1min', undefined], new Array(2), [...rungs, '1day']];
  for (const timeframes of invalid) assert.throws(() => planFor({ timeframes }, native), Error, JSON.stringify(timeframes));
});

test('an old server cannot silently turn a selected subset into all eight timeframes', async () => {
  for (const native of [metadata(), { ...metadata(), selected_timeframes_supported: false }]) {
    const testRun = driver(() => { throw new Error('An unsupported subset must never reach transport'); });
    await assert.rejects(async () => testRun.launch.start(planFor({ timeframes: ['5min'] }, native)), /cannot bind a selected timeframe subset/);
    assert.equal(testRun.calls.length, 0);
    const plan = planFor({ timeframes: [...rungs] }, native);
    assert.equal(Object.hasOwn(plan, 'timeframes'), false, 'legacy all-eight submissions omit the unsupported additive field');
    assert.deepEqual(booleanPlanTimeframes(plan), rungs);
    assert.equal(booleanLaunchObservation(run(plan), plan, ATTEMPT).phase, 'done');
    assert.equal(booleanLaunchObservation(run(plan, ATTEMPT, { timeframes: [...rungs] }), plan, ATTEMPT).phase, 'done');
    assert.throws(() => booleanLaunchObservation(run(plan, ATTEMPT, { timeframes: ['5min'] }), plan, ATTEMPT), /complete declaration/);
  }
});

test('selected timeframes must match status exactly before any completion is accepted', () => {
  const plan = planFor({ timeframes: ['2min', '15min'] }, { ...metadata(), selected_timeframes_supported: true });
  for (const timeframes of [undefined, null, [], ['2min'], ['15min', '2min'], ['2min', '15min', '60min'], rungs]) {
    assert.throws(() => booleanLaunchObservation(run(plan, ATTEMPT, { timeframes }), plan, ATTEMPT), /complete declaration/);
  }
  assert.equal(booleanLaunchObservation(run(plan, ATTEMPT, { timeframes: ['2min', '15min'] }), plan, ATTEMPT).phase, 'done');
});

test('subset submission sends one exact POST; a mismatched status stays unknown until a GET-only repair', async () => {
  const plan = planFor({ timeframes: ['3min', '30min'] }, { ...metadata(), selected_timeframes_supported: true });
  let reads = 0;
  const testRun = driver(url => url === '/engine/command' ? accepted()
    : response({ running: run(plan, ATTEMPT, { timeframes: ++reads === 1 ? rungs : ['3min', '30min'] }) }));
  await testRun.launch.start(plan);
  assert.equal(testRun.latest().phase, 'unknown');
  assert.deepEqual(testRun.calls[0].body.timeframes, ['3min', '30min']);
  assert.deepEqual(testRun.searches, []);
  await assert.rejects(() => testRun.launch.start(plan), /Resolve/);
  await testRun.launch.recheck();
  assert.equal(testRun.latest().phase, 'done');
  assert.equal(testRun.calls.filter(call => call.body).length, 1);
  assert.equal(testRun.calls.length, 3);
  assert.deepEqual(testRun.searches, [SEARCH]);
});

test('subset resume and reload retain exact selected timeframes without POST authorization', async () => {
  const native = { ...metadata(), selected_timeframes_supported: true };
  const plan = planFor({ timeframes: ['10min'] }, native);
  const resumed = driver(() => response({ running: run(plan) }));
  await resumed.launch.resume(plan, ATTEMPT);
  assert.equal(resumed.latest().phase, 'done');
  await assert.rejects(() => resumed.launch.resume(planFor({ timeframes: ['15min'] }, native), ATTEMPT), /same complete declaration/);
  assert.equal(resumed.calls.length, 1); assert.equal(resumed.calls[0].body, null);

  const restored = driver(() => response({ running: run(plan) }));
  await restored.launch.restore(running(plan));
  assert.equal(restored.latest().phase, 'done');
  assert.deepEqual(restored.latest().plan.timeframes, ['10min']);
  assert.ok(Object.isFrozen(restored.latest().plan.timeframes));
  await assert.rejects(() => restored.launch.start(restored.latest().plan), /validated/);
  assert.equal(restored.calls.length, 1); assert.equal(restored.calls[0].body, null);
  await restored.launch.restore(run(plan, ATTEMPT, { timeframes: ['15min'] }));
  assert.equal(restored.latest().phase, 'unknown');
  assert.deepEqual(restored.latest().plan.timeframes, ['10min']);
});

test('additive explicit all-eight status can restore or resume an equivalent legacy request', async () => {
  const oldPlan = planFor();
  const newPlan = planFor({ timeframes: rungs }, { ...metadata(), selected_timeframes_supported: true });
  const testRun = driver(() => response({ running: run(newPlan) }));
  await testRun.launch.resume(oldPlan, ATTEMPT);
  assert.equal(testRun.latest().phase, 'done');
  await testRun.launch.resume(newPlan, ATTEMPT);
  assert.equal(testRun.latest().phase, 'done');
  await testRun.launch.restore(run(oldPlan));
  assert.equal(testRun.latest().phase, 'done');
  assert.deepEqual(booleanPlanTimeframes(testRun.latest().plan), rungs);
  assert.equal(testRun.calls.length, 2); assert.ok(testRun.calls.every(call => call.body === null));
});

test('the common native policy preserves signed paisa and boolean types without numeric coercion', () => {
  const signed = ['min_weakest_period_return_paisa', 'min_pessimistic_profit_paisa', 'min_oos_pessimistic_return_paisa'];
  for (const name of signed) {
    for (const value of ['-9223372036854775808', '-1', '0', '9223372036854775807']) {
      const body = metadata(); const row = body.policy.values.find((/** @type {{name:string}} */ field) => field.name === name);
      row.value = value;
      assert.equal(validateBooleanLaunchMetadata(body).policy.values.find((/** @type {{name:string}} */ field) => field.name === name).value, value);
    }
    for (const value of ['-9223372036854775809', '9223372036854775808', '-0', '-01', true, 0]) {
      const body = metadata(); body.policy.values.find((/** @type {{name:string}} */ field) => field.name === name).value = value;
      assert.throws(() => validateBooleanLaunchMetadata(body), Error, `${name}=${String(value)}`);
    }
  }
  for (const name of booleanNames) {
    for (const value of [false, true]) {
      const body = metadata(); body.policy.values.find((/** @type {{name:string}} */ field) => field.name === name).value = value;
      assert.equal(validateBooleanLaunchMetadata(body).policy.values.find((/** @type {{name:string}} */ field) => field.name === name).value, value);
    }
    for (const value of ['false', 'true', '0', '1', 0, 1, null]) {
      const body = metadata(); body.policy.values.find((/** @type {{name:string}} */ field) => field.name === name).value = value;
      assert.throws(() => validateBooleanLaunchMetadata(body));
    }
  }
  const unknown = metadata(); unknown.policy.values.find((/** @type {{name:string}} */ field) => field.name === signed[0]).name = 'unknown_policy_field';
  assert.throws(() => validateBooleanLaunchMetadata(unknown), /unknown/);
});

test('unresolved policy and configuration preserve the exact refusal with no manufactured values', () => {
  const source = metadata(); source.ready = false; source.refusal = 'Required replay capacity is absent.';
  source.limits.replay_nodes = null;
  source.policy = { ready: false, digest: null, values: [], refusal: 'Policy is not configured.' };
  for (const name of Object.keys(source.configured)) source.configured[name] = null;
  const checked = validateBooleanLaunchMetadata(source);
  assert.equal(checked.ready, false); assert.equal(checked.refusal, source.refusal);
  assert.deepEqual(checked.policy, source.policy); assert.equal(checked.limits.replay_nodes, null);
  assert.deepEqual(Object.values(checked.configured), [null, null, null, null, null]);
  assert.throws(() => booleanLaunchPlan(input(), checked), /ready/);
});

test('malformed metadata schemas, unsafe capacities and contradictory readiness are rejected', () => {
  /** @type {((body:Record<string,any>)=>void)[]} */
  const changes = [
    body => { body.schema_version = 2; }, body => { body.command = 'sweep-all'; },
    body => { body.model = 'ordinary-launch'; }, body => { body.unchecked = true; },
    body => { delete body.configured; }, body => { body.configured = null; },
    body => { body.timeframes[0] = '1day'; }, body => { body.timeframes.reverse(); },
    body => { delete body.timeframes[2]; }, body => { body.timeframes.push('1day'); },
    body => { body.ready = 'true'; }, body => { body.refusal = 'Failure despite ready.'; },
    body => { body.ready = false; }, body => { body.limits.max_symbols = 0; },
    body => { body.limits.max_symbols = 1.5; }, body => { body.limits.max_symbols = Number.MAX_SAFE_INTEGER + 1; },
    body => { body.limits.request_bytes = 32768; }, body => { body.limits.horizon_bars_max = U64; },
    body => { body.limits.max_points_max = U64; }, body => { body.limits.replay_nodes = null; },
    body => { body.limits.observation_bytes = '0'; }, body => { body.limits.checksum_max_records = 4; },
    body => { body.configured.node_allowance = '01'; }, body => { body.configured.horizon_bars = '4294967296'; },
    body => { body.configured.max_points = '184467440737095517'; }, body => { body.configured.node_allowance = '18446744073709551616'; },
    body => { body.policy.ready = false; }, body => { body.policy.digest = '0'.repeat(64); },
    body => { body.policy.values.pop(); }, body => { body.policy.values[1].name = body.policy.values[0].name; },
    body => { delete body.policy.values[3]; }, body => { body.policy.values[0].value = '-1'; },
    body => { body.policy.values[0].value = Number.MAX_SAFE_INTEGER + 1; }, body => { body.policy.values[0].value = '18446744073709551616'; },
    body => { body.policy.values[0].name = 'a'.repeat(65); }, body => { body.policy.values[0].extra = true; },
    body => { body.refusal = 'x'.repeat(4097); }
  ];
  for (const change of changes) { const body = metadata(); change(body); assert.throws(() => validateBooleanLaunchMetadata(body)); }
  for (const body of [null, [], true, undefined, '{}']) assert.throws(() => validateBooleanLaunchMetadata(body));
});

test('the exact request freezes the declaration and relies on the server supplied universe size', () => {
  const native = metadata(); native.limits.max_symbols = 1;
  assert.throws(() => planFor({}, native), /1–1/);
  const source = input({ symbols: ['NIFTY'] });
  const plan = booleanLaunchPlan(source, validateBooleanLaunchMetadata(native));
  assert.deepEqual(plan, { command: BOOLEAN_SEARCH_COMMAND, feed: 'fixture-feed', symbols: ['NIFTY'], expected_policy_digest: POLICY,
    from_year: 2025, from_month: 1, to_year: 2025, to_month: 3,
    later_from_year: 2025, later_from_month: 4, later_to_year: 2025, later_to_month: 6,
    bits: 'all', horizon_bars: '7', max_points: '20', batch_programs: '2', node_allowance: '64', batch_allowance: '3' });
  source.symbols[0] = 'BANKNIFTY'; source.maxPoints = '50';
  assert.deepEqual(plan.symbols, ['NIFTY']); assert.equal(plan.max_points, '20');
  assert.ok(Object.isFrozen(plan)); assert.ok(Object.isFrozen(plan.symbols));
  assert.throws(() => { plan.symbols.push('BANKNIFTY'); }, TypeError);
  assert.throws(() => booleanLaunchPlan(input(), metadata()), /metadata/);
  assert.throws(() => booleanLaunchPlan(input(), structuredClone(validateBooleanLaunchMetadata(metadata()))), /metadata/);
});

test('the observed native policy digest is bound into the request and cannot be replaced by a form value', () => {
  const plan = planFor(); assert.equal(plan.expected_policy_digest, POLICY);
  assert.throws(() => planFor({ expected_policy_digest: '2b'.repeat(32) }), /Only the Boolean/);
  const other = metadata(); other.policy.digest = '2b'.repeat(32);
  const otherPlan = planFor({}, other);
  assert.equal(otherPlan.expected_policy_digest, other.policy.digest);
  assert.throws(() => booleanLaunchObservation(run(plan), otherPlan, ATTEMPT), /complete declaration/);
});

test('the installed Svelte proxy cannot silently replace the validated metadata or plan identity', async () => {
  const checked = validateBooleanLaunchMetadata(metadata());
  assert.notEqual(proxy(checked), checked, 'Object.freeze does not prevent the installed Svelte runtime from wrapping the root object');
  assert.throws(() => booleanLaunchPlan(input(), proxy(checked)), /metadata/);
  const plan = booleanLaunchPlan(input(), checked);
  const testRun = driver(() => accepted());
  await assert.rejects(() => testRun.launch.start(proxy(plan)), /validated/);
  assert.equal(testRun.calls.length, 0, 'the component must retain validated values in $state.raw before submission');
});

test('every explicit period is required and no reversed or overlapping holdout split is guessed', () => {
  for (const extra of [
    { from: undefined }, { to: undefined }, { laterFrom: undefined }, { laterTo: undefined },
    { from: '' }, { from: '0000-01' }, { from: '2025-1' }, { from: '2025-00' }, { to: '2025-13' },
    { from: ' 2025-01' }, { to: '10000-01' }, { to: '2024-12' },
    { laterFrom: '2025-03' }, { laterFrom: '2024-01' }, { laterTo: '2025-03' }
  ]) assert.throws(() => planFor(extra), Error, JSON.stringify(extra));
  assert.equal(planFor({ from: '0001-01', to: '0001-01', laterFrom: '0001-02', laterTo: '9999-12' }).later_to_year, 9999);
});

test('symbol duplicates, missing entries, aliases with whitespace and request byte overflow are refused without truncation', () => {
  for (const symbols of [[], ['NIFTY', 'NIFTY'], ['NIFTY', 'BANKNIFTY', 'RELIANCE', 'M&M'],
    [''], [' NIFTY'], ['nifty'], ['NIFTY/../../secret'], ['株'], ['A'.repeat(65)], ['NIFTY', undefined], new Array(2)]) {
    assert.throws(() => planFor({ symbols }), Error, JSON.stringify(symbols));
  }
  assert.deepEqual(planFor({ symbols: ['M&M', 'BAJAJ-AUTO', 'NSE-RELIANCE'] }).symbols, ['M&M', 'BAJAJ-AUTO', 'NSE-RELIANCE']);
  assert.equal(planFor({ symbols: ['A'.repeat(64)] }).symbols[0].length, 64);
  const native = metadata(); native.limits.max_symbols = 300;
  const symbols = Array.from({ length: 300 }, (_, index) => `S${String(index).padStart(3, '0')}${'A'.repeat(60)}`);
  assert.throws(() => planFor({ symbols }, native), /16,384-byte/);
});

test('only explicit canonical integer budgets and the exact policy max points can be launched', () => {
  for (const field of ['horizonBars', 'maxPoints', 'batchPrograms', 'nodeAllowance', 'batchAllowance']) {
    for (const value of [undefined, null, '', '0', '-1', '01', '1 ', '+1', '1e3', '1.5', 1, NaN, Infinity, 9007199254740992, '18446744073709551616']) {
      assert.throws(() => planFor({ [field]: value }), Error, `${field}=${String(value)}`);
    }
  }
  assert.throws(() => planFor({ horizonBars: '4294967296' }), /budget/);
  assert.throws(() => planFor({ maxPoints: '184467440737095517' }), /budget/);
  assert.throws(() => planFor({ maxPoints: '21' }), /exact max-points/);
  const native = metadata(); native.configured.max_points = null;
  assert.throws(() => planFor({}, native), /exact max-points/);
  native.configured.max_points = '184467440737095516';
  const plan = planFor({ maxPoints: native.configured.max_points, horizonBars: '4294967295',
    batchPrograms: U64, nodeAllowance: U64, batchAllowance: U64 }, native);
  assert.equal(plan.max_points, '184467440737095516'); assert.equal(plan.node_allowance, U64);
});

test('condition syntax is exact and no ordinary knob, depth, timeframe or server path enters a Boolean request', () => {
  assert.equal(planFor({ bits: '0,1,63,4294967295' }).bits, '0,1,63,4294967295');
  for (const value of [null, 0, '', 'ALL', '0,0', '1,0', ',1', '1,', '01', '1, 2', '-1', '4294967296', '1e2']) {
    assert.throws(() => planFor({ bits: value }), Error, String(value));
  }
  const oversizedBits = Array.from({ length: 1200 }, (_, index) => String(index)).join(',');
  assert.ok(oversizedBits.length > 4096);
  assert.throws(() => planFor({ bits: oversizedBits }), /4,096-byte/);
  for (const name of ['depth', 'k', 'rungs', 'timeframe', 'store_root', 'receipt_path', 'top', 'support_ppm', 'exit_cutoff', 'run_id']) {
    assert.throws(() => planFor({ [name]: 'unexpected' }), /Only the Boolean/);
  }
  for (const feed of [null, '', 'Zerodha', 'zerodha/path', 'a'.repeat(65)]) assert.throws(() => planFor({ feed }));
});

test('one POST is followed by exact upper-half u64 GETs with structured search progress', async () => {
  const plan = planFor(); let reads = 0;
  const testRun = driver((url, options) => {
    if (url === '/engine/command') {
      assert.equal(options?.method, 'POST'); assert.equal(new Headers(options?.headers).get('content-type'), 'application/json');
      assert.deepEqual(JSON.parse(String(options?.body)), plan); return accepted();
    }
    assert.equal(url, `/backtest/run.json?attempt=${ATTEMPT}`); assert.equal(options?.cache, 'no-store');
    assert.ok(options?.signal instanceof AbortSignal);
    return response({ running: ++reads === 1 ? running(plan, { completed_batches: '0' }) : run(plan) });
  });
  await testRun.launch.start(plan);
  assert.equal(reads, 2); assert.equal(testRun.calls.filter(call => call.body).length, 1);
  assert.equal(testRun.latest().phase, 'done'); assert.equal(testRun.latest().attempt, ATTEMPT);
  assert.equal(testRun.latest().searchIdentity, SEARCH); assert.equal(testRun.latest().exhausted, false);
  assert.equal(testRun.latest().completedBatches, '1'); assert.deepEqual(testRun.searches, [SEARCH]);
  assert.ok(testRun.snapshots.some(state => state.completedBatches === '0'));
  for (const state of testRun.snapshots) assert.ok(Object.isFrozen(state));
});

test('a command ending does not imply grammar exhaustion and report text cannot invent an identity', () => {
  const plan = planFor();
  const ended = booleanLaunchObservation(run(plan, ATTEMPT, { report: 'All done, exhausted=true. search=' + SEARCH,
    exhausted: null, completed_batches: null }), plan, ATTEMPT);
  assert.equal(ended.phase, 'done'); assert.equal(ended.exhausted, null); assert.equal(ended.completedBatches, null);
  assert.throws(() => booleanLaunchObservation(run(plan, ATTEMPT, { search_identity: null,
    exhausted: null, completed_batches: null, report: 'Search ' + SEARCH + ' completed.' }), plan, ATTEMPT));
  assert.equal(booleanLaunchObservation(run(plan, ATTEMPT, { exhausted: true }), plan, ATTEMPT).exhausted, true);
});

test('every declaration field, symbol order and exact ID is joined before interpreting a status', () => {
  const plan = planFor();
  for (const field of Object.keys(plan).filter(name => name !== 'symbols')) {
    const changed = typeof plan[field] === 'number' ? plan[field] + 1 : plan[field] + '_other';
    assert.throws(() => booleanLaunchObservation(run(plan, ATTEMPT, { [field]: changed }), plan, ATTEMPT), Error, field);
    assert.throws(() => booleanLaunchObservation(run(plan, ATTEMPT, { [field]: undefined }), plan, ATTEMPT), Error, field + ' missing');
  }
  for (const extra of [{ where: 'cli' }, { kind: 'sweep' }, { symbols: ['RELIANCE', 'NIFTY'] }, { symbols: ['NIFTY'] },
    { symbols: new Array(2) }, { attempt_key: '9223372036854775810' }, { attempt_key: ATTEMPT + '0' },
    { attempt_key: '0' }, { attempt_key: 1 }, { attempt_key: '01' }, { attempt_key: undefined }, { attempt: 7 }]) {
    assert.throws(() => booleanLaunchObservation(run(plan, ATTEMPT, extra), plan, ATTEMPT), Error, JSON.stringify(extra));
  }
  assert.equal(Number(ATTEMPT), Number('9223372036854775810'), 'the distinct exact aliases have the same rounded number');
  const legacy = run(plan, '17'); delete /** @type {Record<string,unknown>} */ (legacy).attempt_key;
  assert.equal(booleanLaunchObservation(legacy, plan, '17').attempt, '17');
  assert.throws(() => booleanLaunchObservation(legacy, structuredClone(plan), '17'), /complete declaration/);
});

test('missing, conflicting and unknown lifecycle evidence never becomes completion or a zero count', () => {
  const plan = planFor();
  for (const extra of [{ status: 'unknown' }, { status: 'unconfirmed' }, { status: 'completed', report: null },
    { in_flight: 'false' }, { in_flight: undefined }, { report: null }, { report: '' }, { report: '  ' },
    { finished_micros: null }, { finished_micros: '1750000000000000' }, { finished_micros: 1.5 },
    { report: undefined }, { refusal: undefined }, { refusal: 'failure' }, { status: 'running' },
    { status: 'refused' }, { status: 'cancelled' }, { search_identity: undefined }, { search_identity: '0'.repeat(64) },
    { search_identity: SEARCH.toUpperCase() }, { exhausted: undefined }, { exhausted: 'false' },
    { completed_batches: undefined }, { completed_batches: 0 }, { completed_batches: '01' },
    { completed_batches: '-1' }, { completed_batches: '18446744073709551616' }, { completed_batches: null },
    { exhausted: null, completed_batches: '0' }]) {
    assert.throws(() => booleanLaunchObservation(run(plan, ATTEMPT, extra), plan, ATTEMPT), Error, JSON.stringify(extra));
  }
  for (const extra of [{ report: 'ended' }, { refusal: 'refused' }, { finished_micros: 1 }]) {
    assert.throws(() => booleanLaunchObservation(running(plan, extra), plan, ATTEMPT));
  }
  assert.equal(booleanLaunchObservation(running(plan, { search_identity: null, exhausted: null, completed_batches: null }), plan, ATTEMPT).completedBatches, null);
  for (const value of [null, undefined, [], false]) assert.throws(() => booleanLaunchObservation(value, plan, ATTEMPT));
});

test('structured search identity, exhaustion and exact batch counts never move backwards', () => {
  const plan = planFor();
  const prior = booleanLaunchObservation(running(plan, { completed_batches: '9007199254740993' }), plan, ATTEMPT);
  for (const extra of [{ search_identity: '2a'.repeat(32) }, { search_identity: null, exhausted: null, completed_batches: null },
    { completed_batches: '9007199254740992' }, { completed_batches: null, exhausted: null }]) {
    assert.throws(() => booleanLaunchObservation(running(plan, extra), plan, ATTEMPT, prior));
  }
  const exhausted = booleanLaunchObservation(run(plan, ATTEMPT, { exhausted: true, completed_batches: U64 }), plan, ATTEMPT, prior);
  assert.equal(exhausted.completedBatches, U64);
  assert.throws(() => booleanLaunchObservation(run(plan, ATTEMPT, { exhausted: false, completed_batches: U64 }), plan, ATTEMPT, exhausted));
  assert.throws(() => booleanLaunchObservation(run(plan, ATTEMPT, { exhausted: null, completed_batches: U64 }), plan, ATTEMPT, exhausted));
  assert.throws(() => booleanLaunchObservation(running(plan, { exhausted: true, completed_batches: U64 }), plan, ATTEMPT, exhausted));
  const refused = booleanLaunchObservation(run(plan, ATTEMPT, { report: null, refusal: 'Receipt failed.' }), plan, ATTEMPT);
  assert.throws(() => booleanLaunchObservation(run(plan), plan, ATTEMPT, refused));
});

test('the smallest and largest canonical exact attempts survive acceptance without Number conversion', async () => {
  const plan = planFor();
  for (const attempt of ['1', U64]) {
    const testRun = driver(url => url === '/engine/command' ? accepted(attempt) : response({ running: run(plan, attempt) }));
    await testRun.launch.start(plan);
    assert.equal(testRun.latest().phase, 'done'); assert.equal(testRun.latest().attempt, attempt);
    assert.equal(testRun.calls[1].url, `/backtest/run.json?attempt=${attempt}`);
  }
});

test('a clear prestart refusal preserves its reason and does not invent an attempt', async () => {
  const testRun = driver(() => response({ accepted: false, started: false, attempt: null, refusal: 'Runtime receipt capacity is not configured.' }, 503));
  await testRun.launch.start(planFor());
  assert.equal(testRun.latest().phase, 'refused'); assert.equal(testRun.latest().attempt, null);
  assert.equal(testRun.latest().why, 'Runtime receipt capacity is not configured.');
  await testRun.launch.recheck(); assert.equal(testRun.calls.length, 1); assert.deepEqual(testRun.searches, []);
});

test('poststart refusal stays attached to its exact attempt and never claims a finished search', async () => {
  const plan = planFor();
  for (const status of [undefined, 'refused', 'failed', 'cancelled']) {
    const testRun = driver(url => url === '/engine/command' ? accepted() : response({ running: run(plan, ATTEMPT,
      { status, report: null, refusal: 'Stored input receipt changed.', search_identity: null, exhausted: null, completed_batches: null }) }));
    await testRun.launch.start(plan);
    assert.equal(testRun.latest().phase, 'refused'); assert.equal(testRun.latest().attempt, ATTEMPT);
    assert.equal(testRun.latest().why, 'Stored input receipt changed.'); assert.equal(testRun.latest().searchIdentity, null);
    assert.equal(testRun.calls.length, 2); assert.deepEqual(testRun.searches, []);
  }
});

test('ambiguous or malformed acceptance is unknown and can never trigger an automatic POST retry', async () => {
  /** @type {(()=>Response)[]} */
  const answers = [
    () => { throw new Error('Disconnected after submission'); }, () => new Response('{', { status: 202 }),
    () => response({ accepted: true }, 202), () => response({ accepted: true, refusal: null, attempt: Number(ATTEMPT) }, 202),
    () => response({ accepted: true, refusal: null, attempt: '0' }, 202),
    () => response({ accepted: true, refusal: null, attempt: '01' }, 202),
    () => response({ accepted: true, refusal: null, attempt: '18446744073709551616' }, 202),
    () => response({ accepted: false, refusal: 'Busy.' }, 202),
    () => response({ accepted: false, refusal: 'Busy.', attempt_key: ATTEMPT }, 409),
    () => response({ accepted: false, refusal: 'Busy.', started: true }, 409),
    () => response({ refusal: 'No authoritative admission receipt.' }, 500)
  ];
  const plan = planFor();
  for (const answer of answers) {
    const testRun = driver(answer); await testRun.launch.start(plan);
    assert.equal(testRun.latest().phase, 'unknown'); assert.equal(testRun.latest().attempt, null);
    await testRun.launch.recheck(); assert.equal(testRun.calls.length, 1);
    await assert.rejects(() => testRun.launch.start(plan), /Resolve/); assert.equal(testRun.calls.length, 1);
  }
});

test('conflicting acceptance keeps only a supplied exact attempt for a GET-only recheck', async () => {
  const plan = planFor();
  for (const [status, extra] of [[200, {}], [500, {}], [202, { attempt_key: '2' }], [202, { started: false }], [202, { refusal: 'conflicting refusal' }]]) {
    let calls = 0;
    const testRun = driver(() => ++calls === 1 ? response({ accepted: true, refusal: null, attempt: ATTEMPT, .../** @type {object} */ (extra) }, /** @type {number} */ (status)) : response({ running: run(plan) }));
    await testRun.launch.start(plan); assert.equal(testRun.latest().phase, 'unknown'); assert.equal(testRun.latest().attempt, ATTEMPT);
    assert.equal(testRun.calls.length, 1); await testRun.launch.recheck();
    assert.equal(testRun.latest().phase, 'done'); assert.equal(testRun.calls[1].url, `/backtest/run.json?attempt=${ATTEMPT}`);
    assert.equal(testRun.calls.filter(call => call.body).length, 1);
  }
});

test('bad HTTP, bad JSON and a foreign or missing run remain unknown until the exact GET succeeds', async () => {
  const plan = planFor();
  /** @type {(()=>Response)[]} */
  const failures = [() => response({}, 503), () => new Response('{', { status: 200 }), () => response({ running: null }),
    () => response({ running: run(plan, '9223372036854775810') }), () => { throw new Error('Timed out'); }];
  for (const failure of failures) {
    let reads = 0;
    const testRun = driver(url => url === '/engine/command' ? accepted() : ++reads === 1 ? failure() : response({ running: run(plan) }));
    await testRun.launch.start(plan);
    assert.equal(testRun.latest().phase, 'unknown'); assert.equal(testRun.latest().completedBatches, null);
    assert.equal(testRun.latest().searchIdentity, null); assert.deepEqual(testRun.searches, []);
    await testRun.launch.recheck(); assert.equal(testRun.latest().phase, 'done');
    assert.equal(testRun.calls.filter(call => call.body).length, 1);
    assert.deepEqual(testRun.calls.slice(1).map(call => call.url), Array(2).fill(`/backtest/run.json?attempt=${ATTEMPT}`));
  }
});

test('a concurrent second press and recheck cannot duplicate or replace an in-flight POST', async () => {
  const held = latch(), entered = latch(), plan = planFor();
  const testRun = driver(async url => {
    if (url === '/engine/command') { entered.release(); await held.promise; return accepted(); }
    return response({ running: run(plan) });
  });
  const pending = testRun.launch.start(plan); await entered.promise;
  await assert.rejects(() => testRun.launch.start(plan), /Resolve/);
  await testRun.launch.recheck(); assert.equal(testRun.calls.length, 1);
  held.release(); await pending;
  assert.equal(testRun.latest().phase, 'done'); assert.equal(testRun.calls.filter(call => call.body).length, 1);
});

test('resume performs no POST and requires the complete normalized declaration and exact attempt', async () => {
  const plan = planFor();
  const testRun = driver(() => response({ running: run(plan) }));
  for (const attempt of [Number(ATTEMPT), undefined, '0', '01', '18446744073709551616']) {
    await assert.rejects(() => testRun.launch.resume(plan, attempt), /Resume/);
  }
  await assert.rejects(() => testRun.launch.resume(structuredClone(plan), ATTEMPT), /validated/);
  assert.equal(testRun.calls.length, 0);
  await testRun.launch.resume(plan, ATTEMPT);
  assert.equal(testRun.latest().phase, 'done'); assert.equal(testRun.calls.length, 1); assert.equal(testRun.calls[0].body, null);
  await assert.rejects(() => testRun.launch.resume(plan, '9223372036854775810'), /same complete declaration/);
  await assert.rejects(() => testRun.launch.resume(planFor({ laterTo: '2025-07' }), ATTEMPT), /same complete declaration/);
  assert.equal(testRun.calls.length, 1);
});

test('reload restoration observes the historical full request without current metadata or a POST-authorized plan', async () => {
  const oldMetadata = metadata(); oldMetadata.policy.digest = '2b'.repeat(32); oldMetadata.limits.max_symbols = 4;
  const old = planFor({ symbols: ['NIFTY', 'BANKNIFTY', 'RELIANCE', 'M&M'] }, oldMetadata);
  const reported = running(old, { completed_batches: '9007199254740993' });
  const testRun = driver(url => {
    assert.equal(url, `/backtest/run.json?attempt=${ATTEMPT}`);
    return response({ running: run(old, ATTEMPT, { completed_batches: '9007199254740994' }) });
  });
  await testRun.launch.restore(reported);
  assert.equal(testRun.latest().phase, 'done'); assert.equal(testRun.latest().attempt, ATTEMPT);
  assert.deepEqual(testRun.latest().plan, old); assert.equal(testRun.latest().completedBatches, '9007199254740994');
  assert.equal(testRun.calls.length, 1); assert.equal(testRun.calls[0].body, null); assert.deepEqual(testRun.searches, [SEARCH]);
  assert.ok(Object.isFrozen(testRun.latest().plan)); assert.ok(Object.isFrozen(testRun.latest().plan.symbols));
  await assert.rejects(() => testRun.launch.start(testRun.latest().plan), /validated/);
  await assert.rejects(() => testRun.launch.resume(testRun.latest().plan, ATTEMPT), /validated/);
  assert.equal(testRun.calls.length, 1, 'restoring a request never authorizes resubmitting it');
});

test('restored terminal receipts hand off saved results immediately with no POST or polling timer', async () => {
  const plan = planFor();
  for (const extra of [{}, { exhausted: true }, { report: null, refusal: 'A later receipt failed.' }]) {
    const testRun = driver(() => { throw new Error('Terminal restoration must not send a request'); });
    await testRun.launch.restore(run(plan, ATTEMPT, extra));
    assert.equal(testRun.latest().phase, extra.refusal ? 'refused' : 'done');
    assert.equal(testRun.latest().exhausted, extra.exhausted ?? false);
    assert.deepEqual(testRun.searches, [SEARCH]); assert.equal(testRun.calls.length, 0);
    await testRun.launch.restore(run(plan, ATTEMPT, extra));
    assert.deepEqual(testRun.searches, [SEARCH], 'the same restored receipt does not restart the saved-viewer handoff');
  }
});

test('restore keeps acknowledged search identity and exact counts as the baseline for later status reads', async () => {
  const plan = planFor();
  for (const extra of [{ search_identity: '2b'.repeat(32) }, { completed_batches: '9007199254740992' },
    { expected_policy_digest: '2b'.repeat(32) }, { later_to_month: 7 }]) {
    const testRun = driver(() => response({ running: run(plan, ATTEMPT, { completed_batches: '9007199254740994', ...extra }) }));
    await testRun.launch.restore(running(plan, { completed_batches: '9007199254740993' }));
    assert.equal(testRun.latest().phase, 'unknown'); assert.equal(testRun.latest().searchIdentity, SEARCH);
    assert.equal(testRun.latest().completedBatches, '9007199254740993'); assert.deepEqual(testRun.searches, [SEARCH]);
    assert.equal(testRun.calls.length, 1); assert.equal(testRun.calls[0].body, null);
    await assert.rejects(() => testRun.launch.start(plan), /Resolve/);
  }
});

test('malformed restored declarations become unknown and cannot authorize a second launch', async () => {
  const plan = planFor();
  for (const extra of [{ command: 'sweep-stored' }, { where: 'cli' }, { symbols: [] }, { symbols: ['NIFTY', 'NIFTY'] },
    { expected_policy_digest: undefined }, { expected_policy_digest: '0'.repeat(64) }, { expected_policy_digest: POLICY.toUpperCase() },
    { from_year: '2025' }, { from_year: 0 }, { from_month: 13 }, { later_from_month: 1 },
    { bits: undefined }, { horizon_bars: 7 }, { max_points: '0' }, { batch_allowance: '18446744073709551616' },
    { report: null }, { exhausted: null, completed_batches: '0' }]) {
    const testRun = driver(() => { throw new Error('No request is permitted for an unjoined restoration'); });
    await testRun.launch.restore(run(plan, ATTEMPT, extra));
    assert.equal(testRun.latest().phase, 'unknown'); assert.equal(testRun.latest().attempt, ATTEMPT);
    assert.equal(testRun.latest().searchIdentity, null); assert.equal(testRun.calls.length, 0); assert.deepEqual(testRun.searches, []);
    await assert.rejects(() => testRun.launch.start(plan), /Resolve/);
  }
  for (const extra of [{ attempt_key: undefined }, { attempt_key: '0' }, { attempt_key: '01' },
    { attempt_key: '18446744073709551616' }, { attempt: 7 }]) {
    const testRun = driver(() => accepted()); await testRun.launch.restore(run(plan, ATTEMPT, extra));
    assert.equal(testRun.latest().phase, 'unknown'); assert.equal(testRun.latest().attempt, null);
    await testRun.launch.recheck(); assert.equal(testRun.calls.length, 0);
  }
});

test('an exact recheck can repair a malformed reload snapshot without current configuration or resubmission', async () => {
  const plan = planFor();
  const testRun = driver(() => response({ running: run(plan) }));
  await testRun.launch.restore(run(plan, ATTEMPT, { expected_policy_digest: undefined }));
  assert.equal(testRun.latest().phase, 'unknown'); assert.equal(testRun.latest().plan, null);
  await testRun.launch.recheck(); assert.equal(testRun.latest().phase, 'done');
  assert.deepEqual(testRun.latest().plan, plan); assert.deepEqual(testRun.searches, [SEARCH]);
  assert.equal(testRun.calls.length, 1); assert.equal(testRun.calls[0].body, null);
  await assert.rejects(() => testRun.launch.start(testRun.latest().plan), /validated/);
});

test('restore cannot replace a pending local launch or switch an already observed exact attempt', async () => {
  const held = latch(), entered = latch(), plan = planFor();
  const testRun = driver(async url => {
    if (url === '/engine/command') { entered.release(); await held.promise; return accepted(); }
    return response({ running: run(plan) });
  });
  const pending = testRun.launch.start(plan); await entered.promise;
  await testRun.launch.restore(run(plan, '9223372036854775810'));
  assert.equal(testRun.latest().phase, 'starting'); assert.equal(testRun.calls.length, 1);
  held.release(); await pending; assert.equal(testRun.latest().phase, 'done');
  await testRun.launch.restore(run(plan, '9223372036854775810'));
  assert.equal(testRun.latest().phase, 'unknown'); assert.equal(testRun.latest().attempt, ATTEMPT);
  assert.equal(testRun.calls.length, 2); assert.deepEqual(testRun.searches, [SEARCH]);
});

test('restoration after disposal is silent and a cancelled restored poll cannot publish late', async () => {
  const held = latch(), entered = latch(), plan = planFor();
  const testRun = driver(async () => { entered.release(); await held.promise; return response({ running: run(plan) }); });
  const pending = testRun.launch.restore(running(plan)); await entered.promise;
  testRun.launch.dispose(); const count = testRun.snapshots.length;
  held.release(); await pending; assert.equal(testRun.snapshots.length, count);
  await testRun.launch.restore(run(plan)); assert.equal(testRun.snapshots.length, count);
  assert.equal(testRun.calls.length, 1); assert.deepEqual(testRun.searches, [SEARCH]);
});

test('stopping during POST leaves an unknown outcome, suppresses its late reply and never resubmits', async () => {
  const held = latch(), entered = latch(), plan = planFor();
  const testRun = driver(async () => { entered.release(); await held.promise; return accepted(); });
  const pending = testRun.launch.start(plan); await entered.promise;
  testRun.launch.stop(); const stopped = testRun.snapshots.length;
  assert.equal(testRun.latest().phase, 'unknown'); assert.equal(testRun.latest().attempt, null);
  assert.match(testRun.latest().why, /may continue/); assert.equal(testRun.calls[0].options?.signal?.aborted, true);
  held.release(); await pending;
  assert.equal(testRun.snapshots.length, stopped); assert.equal(testRun.calls.length, 1);
  await assert.rejects(() => testRun.launch.start(plan), /Resolve/);
});

test('a stale status after stop cannot overwrite a fresh GET-only resume of that exact attempt', async () => {
  const held = latch(), entered = latch(), plan = planFor(); let reads = 0;
  const testRun = driver(async url => {
    if (url === '/engine/command') return accepted();
    if (++reads === 1) { entered.release(); await held.promise; return response({ running: run(plan, ATTEMPT, { search_identity: '2a'.repeat(32) }) }); }
    return response({ running: run(plan) });
  });
  const pending = testRun.launch.start(plan); await entered.promise;
  testRun.launch.stop(); await testRun.launch.resume(plan, ATTEMPT);
  const fresh = testRun.snapshots.length;
  held.release(); await pending;
  assert.equal(testRun.snapshots.length, fresh); assert.equal(testRun.latest().phase, 'done');
  assert.equal(testRun.latest().searchIdentity, SEARCH); assert.deepEqual(testRun.searches, [SEARCH]);
  assert.equal(testRun.calls.filter(call => call.body).length, 1);
});

test('late JSON parsing and delayed polling cannot publish after disposal', async () => {
  const plan = planFor(), entered = latch();
  /** @type {ReadableStreamDefaultController<Uint8Array>|null} */
  let stream = null;
  const body = new ReadableStream({ start(controller) { stream = controller; } });
  const testRun = driver(url => {
    if (url === '/engine/command') return accepted();
    entered.release(); return new Response(body, { status: 200 });
  });
  const pending = testRun.launch.start(plan); await entered.promise;
  // Let response.json begin and then remove the observing component.
  await Promise.resolve(); testRun.launch.dispose(); const count = testRun.snapshots.length;
  assert.ok(stream);
  /** @type {ReadableStreamDefaultController<Uint8Array>} */ (stream).enqueue(new TextEncoder().encode(JSON.stringify({ running: run(plan) })));
  /** @type {ReadableStreamDefaultController<Uint8Array>} */ (stream).close();
  await pending; assert.equal(testRun.snapshots.length, count); assert.deepEqual(testRun.searches, []);
  await testRun.launch.recheck(); assert.equal(testRun.calls.length, 2);

  const waiting = latch(), release = latch();
  const delayed = driver(url => url === '/engine/command' ? accepted() : response({ running: running(plan) }),
    { wait: async () => { waiting.release(); await release.promise; } });
  const polling = delayed.launch.start(plan); await waiting.promise;
  delayed.launch.dispose(); const snapshots = delayed.snapshots.length; release.release(); await polling;
  assert.equal(delayed.snapshots.length, snapshots); assert.equal(delayed.calls.length, 2);
});

test('disposal from a state observer sends no later request or saved-search notification', async () => {
  const plan = planFor();
  for (const phase of ['starting', 'running', 'done']) {
    /** @type {ReturnType<typeof createBooleanLaunch>|null} */
    let launch = null;
    const testRun = driver(url => url === '/engine/command' ? accepted() : response({ running: run(plan) }),
      { changed: state => { if (state.phase === phase) launch?.dispose(); } });
    launch = testRun.launch; await launch.start(plan);
    assert.equal(testRun.calls.length, phase === 'starting' ? 0 : phase === 'running' ? 1 : 2);
    assert.deepEqual(testRun.searches, []);
  }
});

test('new index declarations require the native daily policy while old evidence observation and cash remain usable', () => {
  const old = metadata(); delete old.index_consistency_policy;
  const checked = validateBooleanLaunchMetadata(old);
  for (const symbols of [['NIFTY'], ['BANKNIFTY'], ['RELIANCE', 'NIFTY']]) {
    assert.throws(() => booleanLaunchPlan(input({ symbols }), checked), /required index day\/week rule/);
  }
  assert.deepEqual(booleanLaunchPlan(input({ symbols: ['RELIANCE'] }), checked).symbols, ['RELIANCE']);
  const oldPlan = planFor();
  assert.equal(booleanLaunchObservation(run(oldPlan), oldPlan, ATTEMPT).phase, 'done', 'reading a saved declaration does not claim new policy adoption');
  for (const change of [(/** @type {any} */ v) => v.maximum_losing_day_streak = '3', (/** @type {any} */ v) => v.evaluated_scope = 'later_only', (/** @type {any} */ v) => v.costs_included = true]) {
    const body = metadata(); change(body.index_consistency_policy); assert.throws(() => validateBooleanLaunchMetadata(body), /approved rule/);
  }
});

test('native work sizing is additive, preserved exactly, and never converted to a full population or ETA', () => {
  const body = metadata(); body.work_model = { version: 1, state: 'lower_bound_only', scope: 'full_live_alphabet_per_timeframe', live_conditions: 384, max_instructions: 1151,
    conjunction_program_lower_bound: String((1n << 384n) - 1n), lower_bound_exceeds_cumulative_counter: true, timeframe_execution: 'sequential', total_programs: null, eta_seconds: null, refusal: null };
  assert.equal(validateBooleanLaunchMetadata(body).work_model.conjunction_program_lower_bound, String((1n << 384n) - 1n));
  const wrong = metadata(); wrong.work_model = { ...body.work_model, eta_seconds: 10 }; assert.throws(() => validateBooleanLaunchMetadata(wrong), /unmeasured total or ETA/);
});
