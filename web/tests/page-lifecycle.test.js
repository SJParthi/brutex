import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { parse } from 'svelte/compiler';
import { createPageRequests, watchVisible } from '../src/lib/page-requests.js';
import { pooled } from '../src/lib/pooled.js';

/** @param {string} route @param {string[]} names */
function functions(route, names) {
  const source = readFileSync(new URL(`../src/routes/${route}/+page.svelte`, import.meta.url), 'utf8');
  const ast = parse(source);
  const code = names.map((name) => {
    const found = ast.instance?.content.body.find((/** @type {any} */ node) => node.type === 'FunctionDeclaration' && node.id?.name === name);
    assert.ok(found, `${route}: actual ${name} function is required`);
    return source.slice(found.start, found.end);
  }).join('\n');
  return { source, code };
}
function deferred() {
  /** @type {(value?:any)=>void} */ let resolve = () => {};
  /** @type {(why:unknown)=>void} */ let reject = () => {};
  const promise = /** @type {Promise<any>} */ (new Promise((yes, no) => { resolve = yes; reject = no; }));
  return { promise, resolve, reject };
}
const flush = () => new Promise((done) => setImmediate(done));
function clock() {
  let id = 0;
  /** @type {Map<number,()=>void>} */ const timers = new Map();
  return { timers,
    /** @param {()=>void} work */ schedule(work) { timers.set(++id, work); return id; },
    /** @param {number} key */ cancel(key) { timers.delete(key); },
    tick() { const next = timers.entries().next().value; if (next) { timers.delete(next[0]); next[1](); } }
  };
}

test('the actual Autopilot reader cannot publish a response, error, or body after cancellation', async () => {
  const { source, code } = functions('autopilot', ['tick']);
  assert.match(source, /watchVisible\(tick, TICK_MS/);
  assert.doesNotMatch(source, /setInterval\(tick/);
  const create = new Function('ask', `${code}\nconst seen=[]; let ap=null;
    const readState=value=>({ok:true,value}), adopt=value=>seen.push(value), setLink=value=>seen.push(value);
    return {tick,seen};`);
  for (const kind of ['response', 'body', 'error']) {
    const delayed = deferred();
    let current = true;
    const response = Response.json({ state: 'paused' });
    if (kind === 'body') response.json = () => delayed.promise;
    const app = create(async () => kind === 'body' ? response : delayed.promise);
    const ticket = { signal: new AbortController().signal, current: () => current };
    const reading = app.tick(ticket);
    await flush(); current = false;
    if (kind === 'error') delayed.reject(new Error('late failure'));
    else delayed.resolve(kind === 'body' ? { state: 'paused' } : new Response(null, { status: 404 }));
    await reading;
    assert.deepEqual(app.seen, [], kind);
  }
  const good = create(async () => Response.json({ state: 'paused' }));
  await good.tick({ signal: new AbortController().signal, current: () => true });
  assert.equal(good.seen[0].state, 'paused');
  assert.equal(good.seen[1].kind, 'ok');
});

/** @param {(url:string, options?:RequestInit)=>Promise<Response>} request */
function auditPage(request) {
  const { code } = functions('audit', ['read', 'refreshCurrent', 'refresh', 'readOlder', 'loadOlder']);
  const timer = clock();
  const create = new Function('ask', 'createPageRequests', 'timer', `
    const feeds={active:'A'}, document={hidden:false}, load={state:'idle',error:null};
    const auditRequests=createPageRequests(timer), olderRequests=createPageRequests(timer);
    let payload=null, samples=[], rtt=[], base=null, older=[], pagesHeld=1, loadingOlder=false, live=true;
    const SAMPLES=32, MAX_PAGES=16, hot=false, HOT_MS=2000, COLD_MS=8000;
    ${code}
    return {refresh,loadOlder,feeds,load,timer,auditRequests,olderRequests,
      state:()=>({payload,older,pagesHeld,loadingOlder}),
      stop:()=>{auditRequests.dispose();olderRequests.dispose();},
      hide:()=>{document.hidden=true;auditRequests.cancel();olderRequests.cancel();}};
  `);
  return create(request, createPageRequests, timer);
}
/** @param {string} label */
const auditBody = (label) => ({ label, at: 10, store: { months: [], bars: 1, instrument_months: 1, generation: 1 }, runs: [label], journal: { pages: 3 } });

test('the actual Audit stream coalesces rapid refresh and A → B → A changes without accepting the older A', async () => {
  /** @type {{url:string,signal:AbortSignal|null|undefined,reply:ReturnType<typeof deferred>}[]} */ const requests = [];
  const app = auditPage((url, options = {}) => { const reply = deferred(); requests.push({ url, signal: options.signal, reply }); return reply.promise; });
  const first = app.refresh();
  for (let i = 0; i < 100; i++) void app.refresh();
  assert.equal(requests.length, 1);
  app.auditRequests.cancel(); app.feeds.active = 'B'; void app.refresh();
  app.auditRequests.cancel(); app.feeds.active = 'A'; void app.refresh();
  requests[0].reply.resolve(Response.json(auditBody('old')));
  await first; await flush();
  assert.equal(app.state().payload, null);
  assert.equal(requests[0].signal?.aborted, true);
  app.timer.tick();
  assert.equal(requests.length, 2);
  assert.ok(requests[1].url.includes('feed=A'));
  requests[1].reply.resolve(Response.json(auditBody('new')));
  await flush();
  assert.equal(app.state().payload.label, 'new');
  assert.equal(app.timer.timers.size, 1);
  app.hide(); assert.equal(app.timer.timers.size, 0); app.stop();
});

test('the actual Audit reader reports failure and recovers while retaining a single poll timer', async () => {
  let calls = 0;
  const app = auditPage(async () => ++calls === 1 ? new Response('store busy', { status: 503 }) : Response.json(auditBody('recovered')));
  await app.refresh();
  assert.equal(app.load.state, 'error');
  assert.match(app.load.error, /503.*store busy/);
  assert.equal(app.timer.timers.size, 1);
  app.timer.tick(); await flush();
  assert.equal(app.load.state, 'ok');
  assert.equal(app.load.error, null);
  assert.equal(app.state().payload.label, 'recovered');
  assert.equal(app.timer.timers.size, 1);
  app.stop();
});

test('the actual Audit older-page response cannot append after changing feed or leaving the page', async () => {
  for (const replacement of ['feed', 'unmount']) {
    const old = deferred();
    let calls = 0;
    const app = auditPage(async () => ++calls === 1 ? Response.json(auditBody('current')) : old.promise);
    await app.refresh();
    const reading = app.loadOlder();
    if (replacement === 'feed') { app.feeds.active = 'B'; app.olderRequests.cancel(); }
    else app.stop();
    old.resolve(Response.json(auditBody('stale older page')));
    await reading;
    assert.deepEqual(app.state().older, []);
    assert.equal(app.state().pagesHeld, 1);
    app.stop();
  }
});

test('replacing a chart request waits for its cancelled batch and never exceeds the shared worker limit', async () => {
  const { source } = functions('markets', ['readMonth']);
  assert.match(source, /pooled\(months, IN_FLIGHT/);
  assert.match(source, /chartRequests\.run/);
  assert.match(source, /return \(\) => chartRequests.cancel\(\)/);
  assert.doesNotMatch(source, /Promise.all\(months.map/);
  const timer = clock(), reads = createPageRequests(timer);
  let active = 0, peak = 0;
  /** @type {ReturnType<typeof deferred>[]} */ const replies = [];
  /** @type {string[]} */ const published = [];
  /** @param {string} name @returns {import('../src/lib/page-requests.js').Work} */
  const work = (name) => async (ticket) => {
    try {
      await pooled([0, 1, 2, 3], 2, async () => {
        active++; peak = Math.max(peak, active);
        const reply = deferred(); replies.push(reply); await reply.promise; active--; return name;
      }, ticket.signal);
      if (ticket.current()) published.push(name);
    } catch (why) { if (ticket.current()) throw why; }
  };
  const old = reads.run(work('old')); reads.cancel(); void reads.run(work('new'));
  assert.equal(replies.length, 2);
  replies[0].resolve(); replies[1].resolve(); await old;
  timer.tick(); assert.equal(replies.length, 4);
  replies[2].resolve(); replies[3].resolve(); await flush();
  replies[4].resolve(); replies[5].resolve(); await flush();
  assert.equal(peak, 2);
  assert.deepEqual(published, ['new']);
  reads.dispose();
});

test('the actual Mapping join never labels an older feed response as the selected feed', async () => {
  const { code, source } = functions('mapping', ['readJoin', 'fetchJoin']);
  assert.match(source, /onDestroy\(\(\) => \{ masterRequests.dispose\(\); joinRequests.dispose\(\); \}\)/);
  /** @type {ReturnType<typeof deferred>[]} */ const replies = [];
  const timer = clock();
  const create = new Function('ask', 'createPageRequests', 'timer', `
    const feeds={active:'A'}, joinRequests=createPageRequests(timer);
    let load={phase:'idle',body:null,why:''}; ${code}
    return {fetchJoin,feeds,joinRequests,state:()=>load};`);
  const app = create(async () => { const reply = deferred(); replies.push(reply); return reply.promise; }, createPageRequests, timer);
  const old = app.fetchJoin('A');
  app.feeds.active = 'B'; void app.fetchJoin('B');
  app.feeds.active = 'A'; void app.fetchJoin('A');
  replies[0].resolve(Response.json({ label: 'old A' })); await old;
  assert.equal(app.state().body, null);
  timer.tick(); assert.equal(replies.length, 2);
  replies[1].resolve(Response.json({ label: 'new A' })); await flush();
  assert.equal(app.state().body.label, 'new A');
  app.feeds.active = null; await app.fetchJoin(null);
  assert.equal(app.state().phase, 'failed');
  assert.equal(app.state().body, null);
  assert.match(app.state().why, /No feed is chosen/);
  assert.equal(replies.length, 2);
  app.joinRequests.dispose();
});

test('the actual Mapping master-status read cannot publish after unmount', async () => {
  const { code } = functions('mapping', ['readMastersCurrent']);
  const delayed = deferred(); let current = true;
  const create = new Function('ask', `${code}\nlet onDisk=[],restartNeeded=false; return {readMastersCurrent,state:()=>({onDisk,restartNeeded})};`);
  const app = create(async () => delayed.promise);
  const reading = app.readMastersCurrent({ signal: new AbortController().signal, current: () => current });
  current = false;
  delayed.resolve(Response.json({ masters: [{ state: 'old' }], restart_required: true }));
  await reading;
  assert.deepEqual(app.state(), { onDisk: [], restartNeeded: false });
});

test('the actual Whole audit binds every answer to its requested selection and preserves named current failures', async () => {
  const { code, source } = functions('gaps', ['runCurrent', 'run']);
  assert.match(source, /questionKey = \$derived\(JSON.stringify\(\[feeds.active, symbol, rung, from, to\]\)\)/);
  assert.match(source, /onDestroy\(\(\) => auditRequests.dispose\(\)\)/);
  const create = new Function('ask', 'createPageRequests', `
    const feeds={active:'A'}, parsed={exchange:'NSE',segment:'INDEX',underlying:'NIFTY'}, hasMinuteSource=true;
    const symbol='NSE-INDEX-NIFTY', rung='1min', from='2025-01', to='2025-02', contractTail=()=>'';
    let questionKey='original',result={phase:'idle',body:null,why:''};
    const auditRequests=createPageRequests(); ${code}
    return {run,runCurrent,auditRequests,state:()=>result,replace:()=>{questionKey='replacement';}};`);
  for (const at of ['response', 'body', 'error']) {
    const delayed = deferred();
    const response = Response.json({ summary: 'old range' });
    if (at === 'body') response.text = () => delayed.promise;
    const app = create(async () => at === 'body' ? response : delayed.promise, createPageRequests);
    const reading = app.run(); await flush(); app.replace();
    if (at === 'error') delayed.reject(new Error('old failure'));
    else delayed.resolve(at === 'body' ? JSON.stringify({ summary: 'old range' }) : response);
    await reading;
    assert.equal(app.state().body, null, at);
    assert.equal(app.state().phase, 'loading', 'the actual selection effect clears this state; old work cannot replace it');
    app.auditRequests.dispose();
  }
  for (const response of [new Response('missing', { status: 404 }), new Response('malformed')]) {
    const app = create(async () => response, createPageRequests);
    await app.run();
    assert.equal(app.state().phase, 'failed');
    assert.match(app.state().why, /404|did not parse as JSON/);
    app.auditRequests.dispose();
  }
  const good = create(async () => Response.json({ summary: 'current range' }), createPageRequests);
  await good.run(); assert.equal(good.state().body.summary, 'current range'); good.auditRequests.dispose();
});

/** @param {import('../src/lib/ask.js').ask} request */
function ingestPage(request) {
  const { source, code } = functions('ingest', ['stopWatchingRun', 'resumeRunCurrent', 'resumeRun', 'pollRunCurrent', 'watchRun']);
  assert.match(source, /viewAlive = false;\s*resumeRequests.dispose\(\);\s*stopWatchingRun\(\)/);
  const timer = clock();
  const create = new Function('request', 'createPageRequests', 'realWatchVisible', 'timer', `
    const resumeRequests=createPageRequests(timer), POLL_MS=2000;
    let viewAlive=true,stopRunWatch=null,runWatchPromise=null,finishRunWatch=null,wake=()=>{};
    const document={visibilityState:'visible',addEventListener:(_,fn)=>{wake=fn;},removeEventListener:()=>{wake=()=>{};}};
    const watchVisible=(work,delay,options)=>realWatchVisible(work,delay,{...options,...timer});
    let snapshots=0,releases=0,pollError=null,runState=null,phase='idle',startedAt=0,lastGrowthAt=0,finishedAt=0;
    let baseline=null,live=null,seenRead=0,sent=null,passSummary=null,netError=null,releaseWatch=()=>{releases++;};
    const store={reads:0}, snapshot=async()=>{snapshots++;return {};}, watchStore=()=>()=>{releases++;};
    ${code}
    return {watchRun,resumeRun,timer,state:()=>({pollError,runState,phase,snapshots,releases,passSummary}),
      hide:()=>{document.visibilityState='hidden';wake();}, show:()=>{document.visibilityState='visible';wake();},
      dispose:()=>{viewAlive=false;resumeRequests.dispose();stopWatchingRun();}};
  `);
  return create(request, createPageRequests, watchVisible, timer);
}
const runningIngest = () => ({ running: true, feeds: [{ legs: 2, legsDone: 1, doing: 'fixture progress' }] });

test('the actual Ingest observer is shared, pauses hidden, resumes once and cannot send mutations', async () => {
  /** @type {{reply:ReturnType<typeof deferred>,signal:AbortSignal|null|undefined}[]} */ const requests = [];
  const app = ingestPage((url, options = {}) => {
    assert.equal(url, '/pull/run.json'); assert.equal(options.method, undefined);
    const reply = deferred(); requests.push({ reply, signal: options.signal }); return reply.promise;
  });
  const watching = app.watchRun();
  assert.strictEqual(app.watchRun(), watching);
  assert.equal(requests.length, 1);
  app.hide(); assert.equal(requests[0].signal?.aborted, true);
  requests[0].reply.resolve(Response.json(runningIngest())); await flush();
  assert.equal(app.state().runState, null);
  assert.equal(app.timer.timers.size, 0);
  app.show(); assert.equal(requests.length, 2);
  requests[1].reply.resolve(Response.json(runningIngest())); await flush();
  assert.equal(app.state().runState.running, true);
  assert.equal(app.timer.timers.size, 1);
  app.dispose(); await watching;
  app.timer.tick(); assert.equal(requests.length, 2);
  assert.equal(app.timer.timers.size, 0);
  await app.watchRun(); assert.equal(requests.length, 2);
});

test('the actual Ingest observer preserves unknown status through failures and stops only after completion', async () => {
  let calls = 0;
  const app = ingestPage(async () => {
    calls++;
    if (calls === 1) return new Response(null, { status: 503 });
    if (calls === 2) return Response.json({ running: false, feeds: 'malformed' });
    return Response.json({ running: false, feeds: [], finished: 'saved completion' });
  });
  const watching = app.watchRun(); await flush();
  assert.match(app.state().pollError, /unknown.*503/);
  app.timer.tick(); await flush();
  assert.match(app.state().pollError, /valid running flag/);
  assert.equal(app.state().phase, 'idle');
  app.timer.tick(); await watching;
  assert.equal(app.state().phase, 'done');
  assert.equal(app.state().passSummary, 'saved completion');
  assert.equal(app.state().pollError, null);
  assert.equal(app.state().releases, 1);
  assert.equal(app.timer.timers.size, 0);
  app.dispose();
});

test('the actual Ingest resume read cannot start a detached status loop after unmount', async () => {
  const reply = deferred(); let calls = 0;
  const app = ingestPage(async () => { calls++; return reply.promise; });
  const reading = app.resumeRun(); app.dispose();
  reply.resolve(Response.json(runningIngest())); await reading;
  assert.equal(calls, 1);
  assert.equal(app.state().phase, 'idle');
  assert.equal(app.state().snapshots, 0);
  assert.equal(app.timer.timers.size, 0);
});

test('the actual Terminal quote batch stops queued work on navigation while existing shared reads finish', async () => {
  const source = readFileSync(new URL('../src/lib/terminal.svelte.js', import.meta.url), 'utf8');
  const wrapped = `<script>${source}</script>`, ast = parse(wrapped);
  const exported = ast.instance?.content.body.find((/** @type {any} */ node) =>
    node.type === 'ExportNamedDeclaration' && node.declaration?.type === 'FunctionDeclaration' && node.declaration.id?.name === 'quotesOn');
  assert.ok(exported?.type === 'ExportNamedDeclaration' && exported.declaration);
  const declaration = exported.declaration;
  const code = wrapped.slice(declaration.start, declaration.end);
  const page = readFileSync(new URL('../src/routes/terminal/+page.svelte', import.meta.url), 'utf8');
  assert.match(page, /at,\s*controller.signal/);
  assert.match(page, /live = false;\s*controller.abort\(\)/);
  /** @type {ReturnType<typeof deferred>[]} */ const replies = [];
  const quoteOn = async () => { const reply = deferred(); replies.push(reply); return reply.promise; };
  const quoteBatch = new Function('pooled', 'IN_FLIGHT', 'quoteOn', `${code}\nreturn quotesOn;`)(pooled, 6, quoteOn);
  const controller = new AbortController();
  const reading = quoteBatch('feed', Array.from({ length: 1000 }, (_, key) => ({ key: String(key), timeframe: '1min', month: '2025-01' })), '', '', controller.signal);
  const refused = assert.rejects(reading, /AbortError|aborted/);
  assert.equal(replies.length, 6);
  controller.abort();
  for (const reply of replies) reply.resolve({ price: null });
  await refused;
  assert.equal(replies.length, 6, 'the other 994 rows must never start a shared read after leaving');
});
