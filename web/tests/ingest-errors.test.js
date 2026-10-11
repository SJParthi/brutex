// conc18-1 and conc18-2: the /ingest Pull press and the run's errors.
// This file holds conc18-2; tests/ingest-press.test.js holds conc18-1.
//
// conc18-1: `phase` turns `running` only after the pre-run census, so a guard
// on `phase` alone let a second press through the snapshot window: two census
// reads, then either a false pre-run error or two `POST /pull/run`, the 409
// loser of which set `done` over a live run and hid Stop.
// conc18-2: `netError` and `pollError` were written by every handler and
// rendered nowhere, so a refused or failed run looked like nothing happened.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { parse } from 'svelte/compiler';
import { isAskTimeout, stopSaid, waitedOut } from '../src/lib/ingest-run.js';

const source = readFileSync(new URL('../src/routes/ingest/+page.svelte', import.meta.url), 'utf8');
const ast = /** @type {any} */ (parse(source, { modern: true }));

/** @param {string[]} names */
function functions(names) {
  return names.map((name) => {
    const found = ast.instance.content.body.find((/** @type {any} */ n) => n.type === 'FunctionDeclaration' && n.id?.name === name);
    assert.ok(found, `ingest: actual ${name} function is required`);
    return source.slice(found.start, found.end);
  }).join('\n');
}
function deferred() {
  /** @type {(v?:any)=>void} */ let resolve = () => {};
  /** @type {(why:unknown)=>void} */ let reject = () => {};
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
const flush = () => new Promise((done) => setImmediate(done));

/** The actual `start`, `runPull` and `stopWatching`, over stubs. */
function ingest() {
  const create = new Function('hooks', 'lib', `
    const {stopSaid,isAskTimeout,waitedOut}=lib, PULL_WAIT_MS=120000;
    let adopted=null, stopNote='', armed=0;
    const untilPullClears=(signal)=>hooks.untilPullClears(signal);
    const readReceipt=()=>({good:true}), RECEIPT_HEADER='x-receipt', classifyBuild=()=>{}, readPilot=()=>{};
    let showProblems=false, problems=[], phase='idle', pressing=false, receipt=null, receipts=[], outcomes=[],
      outcomeIndex=new Map(), samples=[], netError=null, pollError=null, aborted=false, passSummary=null,
      runState=null, askedKeys=new Set(), sent=null, baseline=null, live=null, startedAt=0, lastGrowthAt=0,
      finishedAt=0, seenRead=0, controller=null, stopAsked=false, releases=0;
    let releaseWatch=null;
    const ticked=[{key:'K'}], allBodies=[{dir:'1min',label:'1 min',body:'b',vendor:'v'}], store={reads:0};
    const watchStore=()=>{armed++;return ()=>{releases++;};};
    const snapshot=()=>hooks.snapshot(), request=(url,o)=>hooks.request(url,o);
    const watchRun=async()=>{hooks.watched++;}, resumeRun=async()=>{hooks.resumed++;};
    ${functions(['start', 'runPull', 'stopWatching'])}
    return {start,runPull,stopWatching,
      state:()=>({phase,pressing,netError,pollError,aborted,stopAsked,releases,armed,stopNote,adopted}),
      running:()=>{phase='running';}, arm:()=>(controller=new AbortController())};
  `);
  const hooks = { snapshots: /** @type {any[]} */ ([]), posts: 0, watched: 0, resumed: 0,
    /** @type {(signal: AbortSignal) => Promise<void>} */
    untilPullClears: async () => {},
    snapshot() { const d = deferred(); hooks.snapshots.push(d); return d.promise; },
    /** @type {(url:string,o:any)=>Promise<Response>} */
    request: async (url) => { if (url === '/pull/run') hooks.posts++; return Response.json({ started: true }, { status: 202 }); } };
  return { app: create(hooks, { stopSaid, isAskTimeout, waitedOut }), hooks };
}

test('a Stop the server never took is not left recorded as an aborted run', async () => {
  for (const reply of [() => { throw new Error('socket closed'); }, () => Response.json({ stopping: false })]) {
    const { app, hooks } = ingest();
    hooks.request = async () => reply();
    app.running();
    await app.stopWatching();
    assert.equal(app.state().stopAsked, false);
    assert.equal(app.state().aborted, false);
    assert.ok(app.state().pollError);
  }
});

test('WP-61 (DB-B28, DB-B30): Stop says what the server stopped, and leaves a single hand pull\'s request open', async () => {
  // A per-row pull holds no /pull/run slot. The page printed "Nothing was
  // stopped: the server reports no run in progress" while it ran, and would
  // have aborted the one request that carries its receipt.
  const hand = ingest();
  hand.hooks.request = async () => Response.json({ stopping: true, what: 'hand_pull', when: 'the single hand pull in flight stops at its next instrument' });
  hand.app.running();
  const open = hand.app.arm();
  await hand.app.stopWatching();
  assert.equal(hand.app.state().stopAsked, true);
  assert.equal(hand.app.state().pollError, null);
  assert.match(hand.app.state().stopNote, /Stop was taken: the single hand pull in flight stops at its next instrument/);
  assert.equal(open.signal.aborted, false, 'the pull\'s own request stays open for its receipt');
  // A run's request returned long ago; aborting it is harmless and unchanged.
  const run = ingest();
  run.hooks.request = async () => Response.json({ stopping: true, what: 'run', when: 'the run\'s leg in flight stops at its next instrument' });
  run.app.running();
  const sent = run.app.arm();
  await run.app.stopWatching();
  assert.match(run.app.state().stopNote, /next instrument/);
  assert.doesNotMatch(run.app.state().stopNote, /leg boundary/);
  assert.equal(sent.signal.aborted, true);
  // Nothing in flight: the server's own sentence, and the press is not kept.
  const none = ingest();
  none.hooks.request = async () => Response.json({ stopping: false, what: 'nothing', when: 'no hand run or hand pull is in flight' });
  none.app.running();
  await none.app.stopWatching();
  assert.equal(none.app.state().stopAsked, false);
  assert.match(none.app.state().pollError, /Nothing was stopped: no hand run or hand pull is in flight/);
  assert.equal(none.app.state().stopNote, '');
});

test('WP-61 (DB-B31): a pull that outlives the page\'s wait is not "cancelled", and nothing is measured until it clears', async () => {
  const { app, hooks } = ingest();
  hooks.request = async () => { throw new Error('The request timed out and was cancelled'); };
  const clear = deferred();
  let waits = 0;
  hooks.untilPullClears = async () => { waits += 1; await clear.promise; };
  const pull = app.runPull([{ dir: '1min', label: 'Spot · 1 minute', body: 'b', vendor: 'v' }], new Set(['K']));
  await flush();
  hooks.snapshots[0].resolve({}); await flush(); await flush();
  assert.equal(waits, 1, 'the page waits for the cell in flight to clear');
  assert.equal(app.state().phase, 'running', 'the run is still shown as running, with its Stop');
  assert.match(app.state().netError, /stopped waiting for Spot · 1 minute after 120 s/);
  assert.match(app.state().netError, /The pull was NOT cancelled/);
  assert.equal(hooks.snapshots.length, 1, 'no after-reading is taken mid-pull');
  clear.resolve(); await flush(); await flush();
  assert.equal(hooks.snapshots.length, 2, 'the after-reading is taken once the pull has cleared');
  hooks.snapshots[1].resolve({}); await pull;
  assert.equal(app.state().phase, 'done');
  assert.equal(app.state().armed, 0, 'and no store clock was armed for it (DB-B53)');
  // A refused connection is still what it was: named, not waited on.
  const refused = ingest();
  refused.hooks.request = async () => { throw new Error('Failed to fetch'); };
  let waited = 0;
  refused.hooks.untilPullClears = async () => { waited += 1; };
  const failed = refused.app.runPull([{ dir: '1min', label: 'Spot · 1 minute', body: 'b', vendor: 'v' }], new Set(['K']));
  await flush(); refused.hooks.snapshots[0].resolve({}); await flush(); await flush();
  refused.hooks.snapshots[1]?.resolve({}); await failed;
  assert.equal(waited, 0);
  assert.match(refused.app.state().netError, /Failed to fetch/);
});

/** @param {any} node @param {(n:any, up:any[])=>void} visit @param {any[]} up */
function walk(node, visit, up = []) {
  if (!node || typeof node !== 'object') return;
  if (Array.isArray(node)) { for (const n of node) walk(n, visit, up); return; }
  if (typeof node.type === 'string') visit(node, up);
  const next = typeof node.type === 'string' ? [...up, node] : up;
  for (const [k, v] of Object.entries(node)) if (k !== 'metadata') walk(v, visit, next);
}

test('netError and pollError are rendered as alerts in the run card, whatever the phase', () => {
  for (const name of ['netError', 'pollError']) {
    /** @type {any[]} */ const hits = [];
    walk(ast.fragment, (n, up) => {
      if (n.type === 'IfBlock' && n.test?.type === 'Identifier' && n.test.name === name) hits.push({ n, up });
    });
    assert.ok(hits.length > 0, `{#if ${name}} must be in the markup`);
    const { n, up } = hits[0];
    const body = source.slice(n.start, n.end);
    assert.match(body, /role="alert"/);
    assert.match(body, new RegExp(`\\{${name}\\}`));
    const gated = up.filter((/** @type {any} */ a) => a.type === 'IfBlock' && /phase/.test(source.slice(a.test.start, a.test.end)));
    assert.deepEqual(gated, [], `${name} must not be hidden behind a phase test`);
  }
});
