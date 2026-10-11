// conc18-1 and conc18-2: the /ingest Pull press and the run's errors.
// This file holds conc18-1; tests/ingest-errors.test.js holds conc18-2.
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

test('a second Pull press during the pre-run census is refused, not run', async () => {
  const { app, hooks } = ingest();
  const first = app.start(); const second = app.start();
  assert.equal(hooks.snapshots.length, 1, 'the second press must not start its own census');
  assert.equal(app.state().pressing, true);
  hooks.snapshots[0].resolve({}); await first; await second;
  assert.equal(hooks.posts, 1, 'exactly one POST /pull/run');
  assert.equal(app.state().phase, 'running');
  assert.equal(app.state().pressing, false);
  assert.equal(app.state().netError, null);
});

test('a failed pre-run census releases the latch and names the failure', async () => {
  const { app, hooks } = ingest();
  const press = app.start();
  hooks.snapshots[0].reject(new Error('census down')); await press;
  assert.equal(app.state().pressing, false);
  assert.match(app.state().netError, /could not be read before starting.*census down/);
  const again = app.start(); assert.equal(hooks.snapshots.length, 2, 'a later press is offered again');
  hooks.snapshots[1].reject(new Error('x')); await again;
});

test('a row Pull during the main press is refused, not a second run', async () => {
  const { app, hooks } = ingest();
  const press = app.start();
  const row = app.runPull([{ dir: '1min', label: '1 min', body: 'b', vendor: 'v' }], new Set(['K']));
  const row2 = app.runPull([{ dir: '1min', label: '1 min', body: 'b', vendor: 'v' }], new Set(['K']));
  assert.equal(hooks.snapshots.length, 1);
  hooks.snapshots[0].reject(new Error('stop here')); await press; await row; await row2;
});

test('a 409 from /pull/run picks up the run already in flight instead of ending the card', async () => {
  const { app, hooks } = ingest();
  hooks.request = async (url) => {
    if (url === '/pull/run') hooks.posts++;
    return Response.json({ started: false, why: 'A run is already in flight on this server.' }, { status: 409 });
  };
  const press = app.start(); await flush();
  hooks.snapshots[0].resolve({}); await press;
  assert.equal(hooks.resumed, 1, 'the in-flight run is resumed and watched');
  assert.equal(hooks.watched, 0);
  // WP-61 (DB-B53): a press arms NO store clock any more. It re-read the whole
  // census (16 to 33 MB) every five seconds for as long as the run lasted.
  assert.equal(app.state().armed, 0, 'no store clock is armed by a press');
  assert.equal(app.state().releases, 0);
  assert.match(app.state().netError, /already in flight/);
});

test('the Pull button and the row buttons are disabled while the press is held', () => {
  const submit = /<button class="btn primary" type="submit" disabled=\{([^}]*)\}/.exec(source);
  assert.match(submit?.[1] ?? '', /pressing/);
  // The row button also waits on the row's unsettled spans (DB-B38) and on a
  // pull this tab did not start (DB-B29).
  assert.match(source, /disabled=\{phase === 'running' \|\| pressing \|\| problems\.length > 0 \|\| spans\.length === 0 \|\| busy\?\.blocksPull === true\}/);
});
