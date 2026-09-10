import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { parse } from 'svelte/compiler';
import { INDEX_STOP_COMMAND } from '../src/lib/index-stop-launch.js';
import { BOOLEAN_SEARCH_COMMAND } from '../src/lib/boolean-launch.js';
import { sweepOutcome as realSweepOutcome } from '../src/lib/sweep.js';
import { sweepAdmission } from '../src/lib/sweep-admission.js';

// Execute the page's real routing functions. Payload validation and transport
// ownership are exercised separately by boolean-launch.test.js.
const page = readFileSync(new URL('../src/routes/backtest/+page.svelte', import.meta.url), 'utf8');
const ast = parse(page);
const launchSource = readFileSync(new URL('../src/lib/BooleanLaunch.svelte', import.meta.url), 'utf8');
const launchAst = parse(launchSource);
/** @param {ReturnType<typeof parse>} tree @param {string} name @returns {any} */
function variable(tree, name) {
  const declaration = tree.instance?.content.body.flatMap((/** @type {any} */ row) =>
    row.type === 'VariableDeclaration' ? row.declarations : []).find((/** @type {any} */ row) => row.id?.name === name);
  assert.ok(declaration, `The actual ${name} declaration must be tested.`);
  return declaration;
}
const declarations = ['observedRunning', 'adoptIndexStop', 'adoptBoolean', 'adoptRunning', 'pollSweep', 'handoffBooleanSearch'].map(name => {
  const node = ast.instance?.content.body.find((/** @type {any} */ row) => row.type === 'FunctionDeclaration' && row.id?.name === name);
  assert.ok(node, `The actual ${name} function must remain available to the integration check.`);
  return page.slice(node.start, node.end);
}).join('\n');

test('the normal form defaults to full search and exposes narrower workflows only through advanced tools', () => {
  const mode = variable(ast, 'sweepMode').init;
  assert.equal(mode.callee.name, '$state');
  assert.equal(mode.arguments[0].value, 'boolean');
  assert.doesNotMatch(page, /<select[^>]*[\s\S]{0,200}bind:value=\{sweepMode\}/);
  assert.match(page, /<summary>Advanced tools: AND-only sweeps and receipt checks<\/summary>/);
  assert.match(page, /<details class="research-mode-guide">\s*<summary>Advanced: descend an AND-only timeframe<\/summary>/);
  assert.match(launchSource, /<details>\s*<summary>Advanced settings and acceptance checks<\/summary>/);
  assert.doesNotMatch(launchSource, /<details\s+open=/, 'invalid or missing input must not force every advanced control open');
});

test('the actual launch preparation sends only selected timeframes, in server order, without mutating the picker', () => {
  const arrow = variable(launchAst, 'prepared').init.arguments[0];
  const prepare = new Function('rungs', 'config', 'booleanLaunchPlan', `
    const feed = 'fixture-feed', symbols = ['NIFTY'], from = '2025-01', to = '2025-02';
    const laterFrom = '2025-03', laterTo = '2025-04', settings = { horizonBars: '7' };
    return (${launchSource.slice(arrow.start, arrow.end)})();
  `);
  const allowed = ['1min', '2min', '3min', '5min', '10min', '15min', '30min', '60min'];
  const selected = ['30min', '1min', '5min'];
  const calls = /** @type {any[]} */ ([]);
  const config = { body: { timeframes: allowed } };
  const result = prepare(selected, config, (/** @type {any} */ input, /** @type {any} */ metadata) => {
    calls.push({ input, metadata }); return input;
  });
  assert.deepEqual(result.plan.timeframes, ['1min', '5min', '30min']);
  assert.equal(result.plan.bits, 'all'); assert.deepEqual(result.plan.symbols, ['NIFTY']);
  assert.deepEqual(selected, ['30min', '1min', '5min']);
  assert.strictEqual(calls[0].metadata, config.body);
  for (const selection of [[], ['1min', '1min'], ['1day']]) {
    const refusal = prepare(selection, config, (/** @type {any} */ input) => {
      assert.deepEqual(input.timeframes, selection, 'preparation must not drop invalid choices or silently substitute all eight');
      throw new Error('Selection refused by the declaration validator');
    });
    assert.equal(refusal.plan, null); assert.match(refusal.why, /Selection refused/);
  }
});

test('the actual picker seeding uses server timeframes until an operator changes or clears the selection', () => {
  const effect = ast.instance?.content.body.find((/** @type {any} */ row) => row.type === 'ExpressionStatement' &&
    row.expression?.callee?.name === '$effect' && page.slice(row.start, row.end).includes('const supportedTimeframes'));
  assert.ok(effect);
  const arrow = /** @type {any} */ (effect).expression.arguments[0];
  const seed = new Function('selected', 'rungsTouched', 'booleanTimeframes', `
    const catalog = { phase: 'ready', held: [{ leaf: 'NIFTY', from: '2025-01', to: '2025-04', rungs: [{name: '1min'}] }] };
    const sweepSymbol = 'NIFTY', sweepMode = 'boolean', spanTouched = false, symbolTouched = true, allRuns = [];
    let pickedRungs = new Set(selected), pickedSymbols = new Set(['NIFTY']), ask = {};
    const untrack = fn => fn();
    (${page.slice(arrow.start, arrow.end)})();
    return { rungs: [...pickedRungs], ask };
  `);
  const supported = ['1min', '5min', '30min'];
  assert.deepEqual(seed([], false, supported).rungs, supported);
  assert.deepEqual(seed(['1min'], false, supported).rungs, supported, 'an early census seed is updated when metadata arrives');
  assert.deepEqual(seed(['5min'], true, supported).rungs, ['5min']);
  assert.deepEqual(seed([], true, supported).rungs, [], 'Clear all remains an empty refused selection');
  assert.deepEqual(seed([], false, []).rungs, ['1min'], 'before metadata, the existing census seed remains usable');
});

test('advanced ratio examples use hundredths while win-rate examples use basis points, with no assumed defaults', () => {
  const definition = variable(ast, 'KNOB_FIELDS').init;
  const fields = new Function(`return (${page.slice(definition.start, definition.end)});`)();
  const byKey = Object.fromEntries(fields.map((/** @type {any} */ field) => [field.key, field]));
  assert.match(byKey.min_rr_bp.label, /hundredths/); assert.match(byKey.min_rr_bp.note, /125.*1\.25×/);
  assert.match(byKey.min_ret_over_dd_bp.label, /hundredths/); assert.match(byKey.min_ret_over_dd_bp.note, /500.*5×/);
  assert.match(byKey.min_win_rate_bp.label, /basis points/); assert.match(byKey.min_win_rate_bp.note, /5000.*50%/);
  assert.ok(fields.every((/** @type {any} */ field) => !Object.hasOwn(field, 'fallback')));
  assert.doesNotMatch(JSON.stringify(fields), /220,000|430,000|625-variant|does not finish/);
});

/** @param {any} running @param {{busy?:boolean,current?:boolean,adoptWhy?:string,outcome?:any,body?:any}} [options] */
function mounted(running, options = {}) {
  const create = new Function('BOOLEAN_SEARCH_COMMAND', 'INDEX_STOP_COMMAND', 'running', 'options', 'realSweepOutcome', 'sweepAdmission', `
    const calls = [];
    let sweep = { phase: 'idle', run: null, why: '' }, adoptWhy = options.adoptWhy ?? '';
    let indexStopBusy = false, initialIndexStopRun = null;
    let booleanBusy = !!options.busy, sweepMode = 'ordinary', initialBooleanRun = null;
    let launchedSearchIdentity = '', launchedSearchRevision = 0;
    let launchAdmission = { available: false, why: 'Checking' }, unconfirmedSubmission = false, submittedAttempt = '';
    const statusRequests = { cancel: () => calls.push('cancel ordinary observer'), schedule: () => calls.push('schedule ordinary observer') };
    const invalidateLive = () => calls.push('invalidate ordinary progress');
    const ask_ = async url => { calls.push(url); return { ok: true, status: 200, json: async () => Object.hasOwn(options, 'body') ? options.body : ({ running }) }; };
    const sweepOutcome = value => { calls.push('ordinary outcome'); return options.outcome ?? realSweepOutcome(value); };
    const liveRunKey = () => 'unused';
    const fetchLive = () => calls.push('ordinary progress'), fetchLiveTop = () => calls.push('ordinary live ranking');
    const fetchLedger = () => calls.push('ordinary ledger'), fetchTop = () => calls.push('ordinary ranking');
    ${declarations}
    return { adoptRunning, pollSweep, handoffBooleanSearch, calls,
      ticket: { signal: new AbortController().signal, current: () => options.current !== false },
      state: () => ({ sweep, adoptWhy, booleanBusy, sweepMode, initialBooleanRun, initialIndexStopRun, indexStopBusy, launchedSearchIdentity, launchedSearchRevision }) };
  `);
  return create(BOOLEAN_SEARCH_COMMAND, INDEX_STOP_COMMAND, running, options, realSweepOutcome, sweepAdmission);
}

test('actual startup and retry routes hand Boolean jobs to their exact observer before ordinary outcome handling', async () => {
  for (const method of ['adoptRunning', 'pollSweep']) {
    for (const in_flight of [true, false]) {
      const running = { command: BOOLEAN_SEARCH_COMMAND, where: 'browser', kind: 'command', in_flight, attempt_key: '9223372036854775809' };
      const app = mounted(running);
      await app[method](app.ticket);
      assert.equal(app.state().sweepMode, 'boolean');
      assert.equal(app.state().booleanBusy, true, 'remain blocked until the exact observer validates the declaration');
      assert.strictEqual(app.state().initialBooleanRun, running);
      assert.equal(app.state().sweep.phase, 'idle');
      assert.deepEqual(app.calls, ['/backtest/run.json', 'cancel ordinary observer', 'invalidate ordinary progress']);
    }
  }
});

test('a late global read cannot replace a Boolean observer this page already owns', async () => {
  const app = mounted({ command: BOOLEAN_SEARCH_COMMAND }, { busy: true });
  await app.adoptRunning(app.ticket);
  assert.equal(app.state().initialBooleanRun, null);
  assert.equal(app.state().booleanBusy, true);
  assert.ok(!app.calls.includes('ordinary outcome'));
  assert.ok(!app.calls.includes('schedule ordinary observer'));
});

test('startup and retry restore the exact single-stop observer without ordinary ledger or automatic launch', async () => {
  for (const method of ['adoptRunning', 'pollSweep']) {
    const running = { command: INDEX_STOP_COMMAND, where: 'browser', kind: 'command', in_flight: true, attempt_key: '9223372036854775809' };
    const app = mounted(running);
    await app[method](app.ticket);
    assert.strictEqual(app.state().initialIndexStopRun, running);
    assert.equal(app.state().indexStopBusy, true);
    assert.equal(app.state().initialBooleanRun, null);
    assert.equal(app.state().booleanBusy, false);
    assert.equal(app.state().sweepMode, 'boolean');
    assert.deepEqual(app.calls, ['/backtest/run.json', 'cancel ordinary observer', 'invalidate ordinary progress']);
  }
  assert.match(page, /IndexStopLaunch active=\{sweepMode === 'boolean' && indexWorkflow\}/);
  assert.match(page, /BooleanLaunch active=\{sweepMode === 'boolean' && !indexWorkflow\}/);
  assert.match(page, /index_stop_qualification/);
  assert.match(page, /initialCompletion=\{routePage.url.searchParams.get\('completion'\)/);
});

test('stale replies cannot adopt work and non-Boolean work retains the existing outcome path', async () => {
  for (const method of ['adoptRunning', 'pollSweep']) {
    const stale = mounted({ command: BOOLEAN_SEARCH_COMMAND }, { current: false });
    await stale[method](stale.ticket);
    assert.equal(stale.state().initialBooleanRun, null);
    assert.equal(stale.state().booleanBusy, false);
    assert.deepEqual(stale.calls, ['/backtest/run.json']);
    const ordinary = mounted({ command: 'range-all' });
    await ordinary[method](ordinary.ticket);
    assert.equal(ordinary.state().initialBooleanRun, null);
    assert.equal(ordinary.state().sweepMode, 'ordinary');
    assert.ok(ordinary.calls.includes('ordinary outcome'));
  }
});

test('a second allowance for the same search advances the actual results handoff revision', () => {
  const app = mounted(null), identity = 'ab'.repeat(32);
  app.handoffBooleanSearch(identity);
  assert.equal(app.state().launchedSearchIdentity, identity);
  assert.equal(app.state().launchedSearchRevision, 1);
  app.handoffBooleanSearch(identity);
  assert.equal(app.state().launchedSearchIdentity, identity);
  assert.equal(app.state().launchedSearchRevision, 2);
});

test('a recovered valid status clears the old startup warning; malformed or stale status cannot clear it', async () => {
  for (const method of ['adoptRunning', 'pollSweep']) {
    const prior = '/backtest/run.json answered 429';
    for (const phase of ['idle', 'running', 'done', 'failed']) {
      const app = mounted(null, { adoptWhy: prior, outcome: { phase, run: null, why: '' } });
      await app[method](app.ticket);
      assert.equal(app.state().adoptWhy, '', `${method} recovered ${phase}`);
    }
    const unknown = mounted(null, { adoptWhy: prior, outcome: { phase: 'unknown', run: null, why: 'unreadable' } });
    await unknown[method](unknown.ticket);
    assert.equal(unknown.state().adoptWhy, prior);
    assert.equal(unknown.state().sweep.phase, 'unknown');
    const stale = mounted(null, { current: false, adoptWhy: prior });
    await stale[method](stale.ticket);
    assert.equal(stale.state().adoptWhy, prior);
  }
});

test('the real status seam requires explicit no-job evidence and keeps malformed HTTP successes unknown', async () => {
  for (const method of ['adoptRunning', 'pollSweep']) {
    for (const body of [undefined, null, [], {}, false, 0, '', { running: undefined },
      { running: false }, { running: 0 }, { running: '' }, { running: [] }, { running: {} }]) {
      const app = mounted(null, { body, adoptWhy: 'previous 429 uncertainty' });
      await app[method](app.ticket);
      assert.equal(app.state().sweep.phase, 'unknown', `${method}: ${JSON.stringify(body)}`);
      assert.ok(app.calls.includes('schedule ordinary observer'));
      assert.equal(app.state().initialBooleanRun, null);
      assert.equal(app.state().booleanBusy, false);
    }
    const idle = mounted(null, { body: { running: null }, adoptWhy: 'previous 429 uncertainty' });
    await idle[method](idle.ticket);
    assert.equal(idle.state().sweep.phase, 'idle');
    assert.equal(idle.state().adoptWhy, '');
    assert.ok(!idle.calls.includes('schedule ordinary observer'));
    const active = mounted(null, { body: { running: { in_flight: true } }, adoptWhy: 'previous 429 uncertainty' });
    await active[method](active.ticket);
    assert.equal(active.state().sweep.phase, 'running');
    assert.equal(active.state().adoptWhy, '');
    assert.ok(active.calls.includes('schedule ordinary observer'));
  }
});

test('the actual index launch readiness refuses older servers without the new policy and retains independent server/input blockers', () => {
  const derived = variable(launchAst, 'serverWhy').init.arguments[0];
  const why = new Function('config', 'symbols', `return (${launchSource.slice(derived.start, derived.end)});`);
  for (const symbols of [['NIFTY'], ['BANKNIFTY'], ['NSE-NIFTY'], ['NSE-BANKNIFTY'], ['RELIANCE', 'NIFTY']]) {
    assert.match(why({ phase: 'ready', body: { ready: true } }, symbols), /required index day\/week rule/);
    assert.equal(why({ phase: 'ready', body: { ready: true, index_consistency_policy: {} } }, symbols), '');
  }
  assert.equal(why({ phase: 'ready', body: { ready: true } }, ['RELIANCE']), '');
  assert.match(why({ phase: 'ready', body: { ready: false, refusal: 'Input preparation unavailable', index_consistency_policy: {} } }, ['NIFTY']), /Input preparation unavailable/);
  assert.match(launchSource, /disabled=\{[^}]*serverWhy/);
});
