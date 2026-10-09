// conc18-4: the live "Best combinations so far" panel had no token and no run
// binding. A slower /live.json reply could overwrite a newer one, a reply could
// land for a sweep that had been replaced, and a new sweep opened on the
// previous one's rows while the verdict said "for this run".

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { parse } from 'svelte/compiler';
import { refusalFrom } from '../src/lib/refusal.js';

const source = readFileSync(new URL('../src/routes/backtest/+page.svelte', import.meta.url), 'utf8');
const ast = /** @type {any} */ (parse(source));
/** @param {string[]} names */
function functions(names) {
  return names.map((name) => {
    const found = ast.instance.content.body.find((/** @type {any} */ n) => n.type === 'FunctionDeclaration' && n.id?.name === name);
    assert.ok(found, `backtest: actual ${name} function is required`);
    return source.slice(found.start, found.end);
  }).join('\n');
}
function deferred() {
  /** @type {(v?:any)=>void} */ let resolve = () => {};
  const promise = new Promise((yes) => { resolve = yes; });
  return { promise, resolve };
}
const flush = () => new Promise((done) => setImmediate(done));
const heap = (/** @type {string} */ identity, /** @type {number} */ trials) =>
  Response.json({ runs: [{ identity, trials, bar_milli: 3000, idle_secs: 1, stale: false, rows: [{ rank: 1, direction: 'long', n: 1 }] }] });

function page() {
  /** @type {ReturnType<typeof deferred>[]} */ const replies = [];
  const create = new Function('ask_', 'refusalFrom', `
    let liveSeq=0, live=null, liveTopSeq=0, sweep={run:{kind:'grid', token:'A'}};
    let liveTop={phase:'idle',rows:[],trials:0,barMilli:0,idleSecs:0,stale:false,why:'',identity:''};
    const liveRunKey=(run)=>JSON.stringify([run?.kind, run?.token]);
    ${functions(['fetchLiveTop', 'invalidateLive'])}
    return {fetchLiveTop, invalidateLive, top:()=>liveTop, switchRun:(token)=>{sweep={run:{kind:'grid', token}};}};
  `);
  const app = create(() => { const reply = deferred(); replies.push(reply); return reply.promise; }, refusalFrom);
  return { app, replies };
}

test('an older /live.json reply landing last does not overwrite the newer one', async () => {
  const { app, replies } = page();
  const older = app.fetchLiveTop(); const newer = app.fetchLiveTop();
  replies[1].resolve(heap('newer', 20)); await newer;
  replies[0].resolve(heap('older', 10)); await older; await flush();
  assert.equal(app.top().trials, 20);
  assert.equal(app.top().identity, 'newer', 'the heap shown names its run identity');
});

test('a reply for a sweep that has been replaced is dropped, and the replacement opens empty', async () => {
  const { app, replies } = page();
  await (async () => { const first = app.fetchLiveTop(); replies[0].resolve(heap('A-heap', 5)); await first; })();
  assert.equal(app.top().phase, 'ready');
  const late = app.fetchLiveTop();
  app.switchRun('B'); app.invalidateLive(true);
  assert.equal(app.top().phase, 'idle', 'a new sweep does not open on the previous rows');
  replies[1].resolve(heap('A-heap', 6)); await late; await flush();
  assert.equal(app.top().phase, 'idle');
  assert.equal(app.top().rows.length, 0);
});

test('the verdict no longer claims the heap is this run', () => {
  assert.doesNotMatch(source, /bar of \{bar\.toFixed\(2\)\} for this run/);
  assert.match(source, /liveTop\.identity\.slice\(0, 12\)/);
});

// W4 (OBSV-15, D-3214): `/live.json` refuses with `{"runs":[],"listed":false,
// "refusal":…}` (`crates/api/src/livejson.rs` `unavailable`); the panel printed
// the status and a guess ("The heap is still being written to the store").
test('a refused /live.json read names the server refusal and guesses nothing (W4)', async () => {
  const { app, replies } = page();
  const read = app.fetchLiveTop();
  const refusal = 'live snapshot capacity is full; no blocking task was queued. Retry after another detail read finishes';
  replies[0].resolve(Response.json({ runs: [], listed: false, refusal }, { status: 429 }));
  await read; await flush();
  assert.equal(app.top().phase, 'failed');
  assert.match(app.top().why, new RegExp(`^/live\\.json answered HTTP 429: ${refusal.replace(/[.]/g, '\\.')}`));
  assert.doesNotMatch(app.top().why, /still being written/);
  const late = page();
  const stale = late.app.fetchLiveTop();
  late.app.switchRun('B');
  late.replies[0].resolve(Response.json({ runs: [], listed: false, refusal }, { status: 503 }));
  await stale; await flush();
  assert.equal(late.app.top().phase, 'idle', 'a refusal for a replaced sweep is dropped');
  const slow = page(), text = deferred();
  const reading = slow.app.fetchLiveTop();
  slow.replies[0].resolve({ ok: false, status: 503, text: () => text.promise });
  await flush();
  slow.app.switchRun('C');
  text.resolve(JSON.stringify({ runs: [], listed: false, refusal }));
  await reading; await flush();
  assert.equal(slow.app.top().phase, 'idle', 'a sweep replaced while the refusal body was read is not overwritten');
});

// F3 (OBSV-20, D-3219): `/live.json` sends `idle_secs` and `stale` as null when
// the heap file cannot be dated (`crates/api/src/livejson.rs`). The page sorted
// on `Number(r.idle_secs)`, and `Number(null)` is 0 -- finite -- so an undated
// heap sorted FIRST, despite the comment saying it sorts last; it was then
// rendered "updated 0s ago" with no "not moving" pill (`Number(null) || 0`,
// `stale === true`). Unknown age was shown as the freshest possible age.
test('an undated heap sorts after a dated one and keeps its unknown age and staleness (F3)', async () => {
  const run = (/** @type {string} */ identity, /** @type {any} */ idle, /** @type {any} */ stale) =>
    ({ identity, trials: 7, bar_milli: 3000, idle_secs: idle, stale, rows: [{ rank: 1, direction: 'long', n: 1 }] });
  const { app, replies } = page();
  let read = app.fetchLiveTop();
  replies[0].resolve(Response.json({ runs: [run('undated', null, null), run('dated', 400, false)] }));
  await read; await flush();
  assert.equal(app.top().identity, 'dated', 'a heap with no age never outranks one whose age was measured');
  assert.equal(app.top().idleSecs, 400);
  read = app.fetchLiveTop();
  replies[1].resolve(Response.json({ runs: [run('undated', null, null)] }));
  await read; await flush();
  assert.equal(app.top().identity, 'undated', 'an undated heap with rows is still shown');
  assert.equal(app.top().idleSecs, null, 'null on the wire is not 0 seconds');
  assert.equal(app.top().stale, null, 'null on the wire is not "moving"');
  for (const [idle, kept] of [[0, 0], [59, 59], [-1, null], ['5', null], [1.5, 1.5], [Infinity, null]]) {
    read = app.fetchLiveTop();
    replies.at(-1)?.resolve(Response.json({ runs: [run('one', idle, false)] }));
    await read; await flush();
    assert.equal(app.top().idleSecs, kept, `idle_secs ${String(idle)}`);
  }
});

test('the freshness line says an undated heap is undated, never "0s ago" (F3)', () => {
  const create = new Function(`${functions(['liveFreshness'])}; return liveFreshness;`);
  const freshness = create();
  assert.deepEqual(freshness({ idleSecs: null, stale: null }),
    { text: 'age unknown: /live.json could not date this heap', warn: 'undated' });
  assert.deepEqual(freshness({ idleSecs: 0, stale: false }), { text: 'updated 0s ago', warn: null });
  assert.deepEqual(freshness({ idleSecs: 59, stale: false }), { text: 'updated 59s ago', warn: null });
  assert.deepEqual(freshness({ idleSecs: 60, stale: false }), { text: 'updated 1m ago', warn: null });
  assert.deepEqual(freshness({ idleSecs: 120, stale: false }), { text: 'updated 2m ago', warn: null });
  assert.deepEqual(freshness({ idleSecs: 121, stale: false }), { text: 'updated 2m ago', warn: 'not moving' });
  assert.deepEqual(freshness({ idleSecs: 5, stale: true }), { text: 'updated 5s ago', warn: 'not moving' });
  assert.deepEqual(freshness({ idleSecs: 5, stale: null }), { text: 'updated 5s ago', warn: null });
  assert.match(source, /\{@const freshness = liveFreshness\(liveTop\)\}/);
  assert.doesNotMatch(source, /liveTop\.idleSecs < 60/, 'the panel renders through the helper');
});
