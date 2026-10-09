import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { parse } from 'svelte/compiler';
import { sweepAdmission, sweepLaunchStop, sweepSubmission } from '../src/lib/sweep-admission.js';

const free = () => ({ schema_version: 1, scope: 'cooperating-store-writers', available: true, why: 'The store lease is free; POST rechecks it.' });
const history = { phase: 'unknown', run: { where: 'cli', status: 'unknown' }, why: 'Old bounded log has no sweep marker.' };

test('a free current lease permits a new request while unknown history stays unknown', () => {
  const before = structuredClone(history);
  const admission = sweepAdmission({ running: history.run, admission: free() });
  assert.equal(sweepLaunchStop(history, admission, false), '');
  assert.deepEqual(history, before);
  assert.equal(sweepLaunchStop(history, { ...admission, available: false, why: 'Busy' }, false), 'Busy');
  for (const phase of ['starting', 'running']) {
    assert.ok(sweepLaunchStop({ ...history, phase }, admission, false));
  }
  assert.ok(sweepLaunchStop({ ...history, run: { where: 'browser' } }, admission, false));
  assert.ok(sweepLaunchStop(history, admission, true), 'an ambiguous POST must stay blocked even when the lease reads free');
});

test('missing, malformed and noncurrent lease authorities cannot enable a launch', () => {
  for (const admission of [undefined, null, [], {}, true, 1, { ...free(), available: 1 },
    { ...free(), available: 'true' }, { ...free(), schema_version: 2 }, { ...free(), scope: 'all-processes' },
    { ...free(), why: '' }, { ...free(), why: null }]) {
    const value = sweepAdmission({ admission });
    assert.equal(value.available, false);
    assert.ok(sweepLaunchStop(history, value, false));
  }
  for (const body of [undefined, null, [], true, 0, '']) assert.equal(sweepAdmission(body).available, false);
});

test('only exact accepted attempt tokens or explicit refusals settle POST uncertainty', () => {
  for (const attempt of ['1', '9007199254740993', '18446744073709551615']) {
    assert.deepEqual(sweepSubmission(202, { accepted: true, refusal: null, attempt }),
      { phase: 'running', attempt, why: '', confirmed: true });
  }
  for (const status of [400, 409, 503]) {
    assert.deepEqual(sweepSubmission(status, { accepted: false, refusal: 'No command was dispatched.' }),
      { phase: 'failed', attempt: '', why: 'No command was dispatched.', confirmed: true });
  }
  for (const [status, body] of [
    [200, { accepted: true, refusal: null, attempt: '1' }], [202, null], [503, {}],
    [202, { accepted: false, refusal: 'contradictory acceptance status' }],
    [409, { accepted: false, refusal: 'may have started', attempt: '1' }],
    ...[undefined, 1, '0', '01', '-1', '1.0', '18446744073709551616'].map(attempt =>
      [202, { accepted: true, refusal: null, attempt }]),
    [202, { accepted: true, refusal: null, attempt: '1', attempt_key: '2' }],
    [202, { accepted: true, refusal: null, attempt: '1', started: false }]
  ]) {
    assert.equal(sweepSubmission(Number(status), body).confirmed, false, JSON.stringify([status, body]));
    assert.equal(sweepSubmission(Number(status), body).phase, 'unknown');
  }
});

test('an audit start refused before dispatch settles the launch as failed (log-1, D-2501)', () => {
  const body = () => ({ schema_version: 1, refusal: 'invocation journal busy',
    code: 'invocation_audit_unavailable', handler_completed: false,
    why: 'The handler was not dispatched because its required audit start was unavailable.' });
  assert.deepEqual(sweepSubmission(503, body()),
    { phase: 'failed', attempt: '', why: 'invocation journal busy', confirmed: true });
  /** @type {[number, (b:any)=>void][]} */ const damage = [
    [503, b => { b.handler_completed = true; }], [503, b => { b.handler_completed = 'false'; }],
    [503, b => { delete b.handler_completed; }], [503, b => { b.code = 'invocation_audit_read_unavailable'; }],
    [503, b => { b.schema_version = 2; }], [503, b => { b.refusal = '   '; }], [503, b => { b.refusal = 7; }],
    [503, b => { b.refusal = 'x'.repeat(4097); }], [503, b => { b.attempt = '1'; }],
    [503, b => { b.attempt_key = '1'; }], [503, b => { b.started = true; }],
    [429, () => {}], [500, () => {}], [202, () => {}]
  ];
  for (const [status, mutate] of damage) {
    const damaged = body(); mutate(damaged);
    const outcome = sweepSubmission(status, damaged);
    assert.equal(outcome.confirmed, false, JSON.stringify([status, damaged]));
    assert.equal(outcome.phase, 'unknown');
  }
  assert.equal(sweepSubmission(503, { ...body(), refusal: 'x'.repeat(4096) }).confirmed, true, 'the bound is inclusive');
});

test('page keeps research selection usable and separates full-width settings from the Run button', () => {
  const page = readFileSync(new URL('../src/routes/backtest/+page.svelte', import.meta.url), 'utf8');
  const tree = parse(page);
  const mode = tree.instance?.content.body.flatMap((/** @type {any} */ node) => node.declarations ?? [])
    .find((/** @type {any} */ node) => node.id?.name === 'sweepMode');
  assert.ok(mode); assert.equal(mode.init.arguments[0].value, 'boolean');
  // Advanced mode switches retain the same in-flight lock now that the normal
  // form launches the full search without a mode dropdown.
  const advanced = /<div class="advanced-run-tools"[\s\S]*?<\/div>/.exec(page)?.[0];
  assert.ok(advanced);
  assert.equal((advanced.match(/disabled=\{inFlight\}/g) ?? []).length, 1);
  assert.equal((advanced.match(/disabled=\{inFlight \|\| indexWorkflow\}/g) ?? []).length, 2);
  for (const mode of ['boolean', 'ordinary', 'receipt']) {
    assert.ok(advanced.includes(`sweepMode = '${mode}'`));
    assert.ok(advanced.includes(`aria-pressed={sweepMode === '${mode}'}`));
  }
  assert.match(page, /<details class="research-mode-guide" open=\{sweepMode !== 'boolean'\}>/);
  assert.match(page, /\.eng\s*\{\s*grid-column: 1 \/ -1;/);
  assert.match(page, /id="sweep-scope-summary"/);
  assert.match(page, /<BooleanLaunch[^>]*active=\{sweepMode === 'boolean' && !indexWorkflow\}[\s\S]*?rungs=\{\[\.\.\.pickedRungs\]\}/);
  assert.match(page, /Run is unavailable:<\/b> \{launchStop\}/);
  assert.match(page, /Descend is unavailable:<\/b> \{launchStop\}/);
  assert.match(page, /\/backtest\/run\.json\?attempt=\$\{submittedAttempt\}/);
  assert.equal((page.match(/if \(launchStop\) return;/g) ?? []).length, 2);
  // Both launch POSTs read their reply once through `commandReply` (F5, D-3222).
  assert.equal((page.match(/applySweepSubmission\('\/backtest\/(run|descend)', response\.status, body, reason\)/g) ?? []).length, 2);
});

test('a reloaded child finishing refreshes admission without re-adopting or rerunning its old command', async () => {
  const page = readFileSync(new URL('../src/routes/backtest/+page.svelte', import.meta.url), 'utf8');
  const ast = parse(page);
  const declarations = ['observedRunning', 'refreshCurrentAdmission', 'setResearchBusy'].map(name => {
    const node = ast.instance?.content.body.find((/** @type {any} */ row) => row.type === 'FunctionDeclaration' && row.id?.name === name);
    assert.ok(node); return page.slice(node.start, node.end);
  }).join('\n');
  for (const child of ['boolean', 'receipt', 'index-stop']) {
    /** @type {any[]} */ const calls = [];
    const create = new Function('sweepAdmission', 'calls', 'body', `
      let indexStopBusy = true, booleanBusy = true, receiptBusy = true, launchAdmission = {available:false,why:'Earlier active snapshot'};
      const statusRequests = {schedule:(work,delay)=>calls.push({work,delay})};
      const ask_ = async (url,options) => { calls.push({url,options}); return {ok:true,json:async()=>body}; };
      ${declarations}
      return {setResearchBusy, refreshCurrentAdmission, state:()=>launchAdmission};
    `);
    const app = create(sweepAdmission, calls, {
      running: { command: 'boolean-qualified-search-stored', in_flight: false, report: 'retained completed attempt' }, admission: free()
    });
    app.setResearchBusy(child, false);
    assert.equal(calls.length, 1); assert.equal(calls[0].delay, 0);
    await calls[0].work({signal:new AbortController().signal,current:()=>true});
    assert.equal(app.state().available, true);
    assert.equal(calls.length, 2, 'only the one refresh GET; no re-adoption or loop');
    assert.equal(calls[1].url, '/backtest/run.json');
    assert.equal(calls[1].options.method, undefined);
    app.setResearchBusy(child, false);
    assert.equal(calls.length, 2, 'repeated idle notification schedules no duplicate read');
  }
});

// F5 (OBSV-24, D-3222): an unconfirmed `/backtest/run` or `/backtest/descend`
// answer dropped the body. The audit layer's refusal (these are audited
// routes) and a plain-text request-bounds refusal are named.
test('an unconfirmed sweep submission names the route, status and reason (F5)', async () => {
  const audit = { schema_version: 1, refusal: 'bounded request audit capacity is full', code: 'invocation_audit_unavailable',
    handler_completed: false, why: 'The handler was not dispatched because its required audit start was unavailable.' };
  const unknown = sweepSubmission(429, audit, undefined, '/backtest/descend');
  assert.equal(unknown.phase, 'unknown'); assert.equal(unknown.confirmed, false);
  assert.equal(unknown.why, `The launch response did not confirm acceptance or refusal (/backtest/descend answered HTTP 429: ${audit.refusal} ${audit.why}). It may have started; do not submit it again.`);
  assert.equal(sweepSubmission(431, null, 'REFUSED — the request headers are 70000 bytes.').why,
    'The launch response did not confirm acceptance or refusal (/backtest/run answered HTTP 431: REFUSED — the request headers are 70000 bytes.). It may have started; do not submit it again.');
  assert.equal(sweepSubmission(202, null).why,
    'The launch response did not confirm acceptance or refusal (/backtest/run answered HTTP 202 and named no reason). It may have started; do not submit it again.');
  const page = readFileSync(new URL('../src/routes/backtest/+page.svelte', import.meta.url), 'utf8');
  assert.equal((page.match(/const \{ body, reason \} = await commandReply\(response\);/g) ?? []).length, 2, 'both launch POSTs read their reply once');
  assert.match(page, /applySweepSubmission\('\/backtest\/run', response\.status, body, reason\)/);
  assert.match(page, /applySweepSubmission\('\/backtest\/descend', response\.status, body, reason\)/);
  const ingest = readFileSync(new URL('../src/routes/ingest/+page.svelte', import.meta.url), 'utf8');
  assert.match(ingest, /const \{ body: answer, reason \} = await commandReply\(r\);/);
  assert.match(ingest, /answer\?\.why \?\?\s*\(r\.ok\s*\? `The run was refused and gave no reason, which is itself the fault: HTTP \$\{r\.status\}\.`\s*: refusalSentence\('\/pull\/run', r\.status, reason\)\)/);
});
