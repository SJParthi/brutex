// conc18-4: the live "Best combinations so far" panel had no token and no run
// binding. A slower /live.json reply could overwrite a newer one, a reply could
// land for a sweep that had been replaced, and a new sweep opened on the
// previous one's rows while the verdict said "for this run".

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { parse } from 'svelte/compiler';

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
  const create = new Function('ask_', `
    let liveSeq=0, live=null, liveTopSeq=0, sweep={run:{kind:'grid', token:'A'}};
    let liveTop={phase:'idle',rows:[],trials:0,barMilli:0,idleSecs:0,stale:false,why:'',identity:''};
    const liveRunKey=(run)=>JSON.stringify([run?.kind, run?.token]);
    ${functions(['fetchLiveTop', 'invalidateLive'])}
    return {fetchLiveTop, invalidateLive, top:()=>liveTop, switchRun:(token)=>{sweep={run:{kind:'grid', token}};}};
  `);
  const app = create(() => { const reply = deferred(); replies.push(reply); return reply.promise; });
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
