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
import * as refusal from '../src/lib/refusal.js';

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
  const create = new Function('hooks', ...Object.keys(refusal), `
    let showProblems=false, problems=[], phase='idle', pressing=false, receipt=null, receipts=[], outcomes=[],
      outcomeIndex=new Map(), samples=[], netError=null, pollError=null, aborted=false, passSummary=null,
      runState=null, askedKeys=new Set(), sent=null, baseline=null, live=null, startedAt=0, lastGrowthAt=0,
      finishedAt=0, seenRead=0, controller=null, stopAsked=false, stopWarning='', releases=0;
    let releaseWatch=null;
    const ticked=[{key:'K'}], allBodies=[{dir:'1min',label:'1 min',body:'b',vendor:'v'}], store={reads:0};
    const watchStore=()=>()=>{releases++;};
    const snapshot=()=>hooks.snapshot(), request=(url,o)=>hooks.request(url,o);
    const watchRun=async()=>{hooks.watched++;}, resumeRun=async()=>{hooks.resumed++;};
    ${functions(['start', 'runPull', 'stopWatching'])}
    return {start,runPull,stopWatching,state:()=>({phase,pressing,netError,pollError,aborted,stopAsked,stopWarning,releases}),
      running:()=>{phase='running';}};
  `);
  const hooks = { snapshots: /** @type {any[]} */ ([]), posts: 0, watched: 0, resumed: 0,
    snapshot() { const d = deferred(); hooks.snapshots.push(d); return d.promise; },
    /** @type {(url:string,o:any)=>Promise<Response>} */
    request: async (url) => { if (url === '/pull/run') hooks.posts++; return Response.json({ started: true }, { status: 202 }); } };
  return { app: create(hooks, ...Object.values(refusal)), hooks };
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

test('a Stop the server took in memory but could not persist says so, and stays taken', async () => {
  // OBSV-09, D-3208. `POST /pull/run/stop` answers 503 with `stopping:true,
  // stop_persisted:false` AFTER setting the in-memory flag. The page threw on
  // `!r.ok`, re-enabled Stop and said the stop "could not be delivered", while
  // the run was winding down -- and the server's own warning, that the STOP
  // will not survive a restart, never reached the operator.
  const { app, hooks } = ingest();
  hooks.request = async () => Response.json(
    { stopping: true, stop_persisted: false, error: 'Recovery STOP was not durably recorded: EIO' },
    { status: 503 }
  );
  app.running();
  await app.stopWatching();
  assert.equal(app.state().stopAsked, true, 'the stop was taken');
  assert.equal(app.state().aborted, true);
  // Its own state: the next poll's `pollError = null` cannot wipe it.
  assert.match(app.state().stopWarning, /not durably recorded: EIO/);
  assert.equal(app.state().pollError, null);
  assert.match(source, /\{#if stopWarning\}[\s\S]*?role="alert"[\s\S]*?\{stopWarning\}/);

  // A failure whose body does not say the stop was taken is still undelivered.
  for (const reply of [
    () => new Response('gateway down', { status: 502 }),
    () => Response.json({ stopping: false, error: 'x' }, { status: 503 }),
    () => Response.json({ stopping: true, stop_persisted: true }, { status: 500 })
  ]) {
    const again = ingest();
    again.hooks.request = async () => reply();
    again.app.running();
    await again.app.stopWatching();
    assert.equal(again.app.state().stopAsked, false);
    assert.equal(again.app.state().aborted, false);
    assert.match(String(again.app.state().pollError), /could not be delivered/);
    assert.equal(again.app.state().stopWarning, '');
  }
});

test('a refused Stop names the route, the status and the server reason (OBSV-26)', async () => {
  // OBSV-26, D-3224. Every POST on this server passes origin admission and the
  // form-field check before its handler, and both refuse in text/plain:
  // "REFUSED -- ... Nothing was read or run." A refused stop, and any JSON
  // refusal other than the unpersisted-stop shape, was reported as "the server
  // answered HTTP 403" -- the reason, which says why the stop did not land and
  // what to change, was read and thrown away.
  const plain = 'REFUSED - a write must come from this server\'s own page. Nothing was read or run.\n';
  for (const [reply, reason] of /** @type {[() => Response, RegExp][]} */ ([
    [() => new Response(plain, { status: 403, headers: { 'content-type': 'text/plain; charset=utf-8' } }),
      /\/pull\/run\/stop answered HTTP 403: REFUSED - a write must come from this server's own page\. Nothing was read or run\./],
    [() => Response.json({ stopping: false, error: 'the run slot is held by another process' }, { status: 503 }),
      /\/pull\/run\/stop answered HTTP 503: the run slot is held by another process/],
    [() => new Response('', { status: 502 }), /\/pull\/run\/stop answered HTTP 502 and named no reason/]
  ])) {
    const { app, hooks } = ingest();
    hooks.request = async () => reply();
    app.running();
    await app.stopWatching();
    assert.equal(app.state().stopAsked, false, 'a refused stop was not taken');
    assert.equal(app.state().aborted, false);
    assert.equal(app.state().stopWarning, '');
    assert.match(String(app.state().pollError), /^Stop could not be delivered/);
    assert.match(String(app.state().pollError), reason);
  }
});
