import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createCensusLoader, STORE_CENSUS_MS } from '../src/lib/store-census.js';

function deferred() {
  /** @type {(value: Response) => void} */
  let resolve = () => { throw new Error('Deferred response was not initialized'); };
  /** @type {Promise<Response>} */
  const promise = new Promise((done) => { resolve = done; });
  return { promise, resolve };
}

test('concurrent selected-feed consumers share one fetch, one parse, and the same array', async () => {
  let reads = 0;
  let parses = 0;
  const body = [{ instrument: 'example', rows: 100 }];
  const pending = deferred();
  const loader = createCensusLoader(async (url, options = {}) => {
    reads += 1;
    assert.equal(url, '/store.json?feed=example%20feed&encoding=census-tuples-v1');
    assert.equal(options.ms, 30_000);
    // A FIRST READ holds no validator of this page's own, so the browser's
    // cache may revalidate its stored copy (`no-cache` always asks the
    // server): an unchanged census then crosses the wire as a 304 (WP-56).
    assert.equal(options.cache, 'no-cache');
    await pending.promise;
    const result = Response.json(body, { headers: { 'x-source': 'actual' } });
    result.json = async () => { parses += 1; return body; };
    return result;
  });
  const first = loader.load('example feed', 0);
  const second = loader.load('example feed', 0);
  assert.equal(first, second);
  pending.resolve(Response.json([]));
  const answer = await first;
  assert.equal(answer.body, body);
  assert.equal(answer.headers.get('x-source'), 'actual');
  assert.equal(await loader.load('example feed', 0), answer);
  assert.equal(reads, 1);
  assert.equal(parses, 1);
  assert.equal(STORE_CENSUS_MS, 30_000);
});

test('a refresh makes a new read and late older answers cannot replace it', async () => {
  const pending = [deferred(), deferred()];
  let reads = 0;
  const loader = createCensusLoader(() => pending[reads++].promise);
  const before = loader.load('one', 0);
  const after = loader.load('one', 1);
  pending[1].resolve(Response.json([{ revision: 1 }]));
  const fresh = await after;
  pending[0].resolve(Response.json([{ revision: 0 }]));
  await before;
  assert.equal(await loader.load('one', 1), fresh);
  assert.deepEqual(fresh.body, [{ revision: 1 }]);
  assert.equal(reads, 2);
});

test('survey requests coalesce selected feed but do not evict it or retain every feed body', async () => {
  const reads = [];
  const loader = createCensusLoader(async (url) => {
    reads.push(url);
    return Response.json([{ url }]);
  });
  const survey = loader.load('selected', 0, { retain: false });
  const selected = loader.load('selected', 0);
  assert.equal(survey, selected);
  const answer = await selected;
  await loader.load('other', 0, { retain: false });
  await loader.load('other', 0, { retain: false });
  assert.equal(await loader.load('selected', 0), answer);
  assert.equal(reads.length, 3, 'unselected survey bodies are released rather than retained');
});

test('changing selected feed releases the previous completed cache entry', async () => {
  let reads = 0;
  const loader = createCensusLoader(async () => { reads += 1; return Response.json([]); });
  await loader.load('one', 0);
  await loader.load('two', 0);
  await loader.load('one', 0);
  assert.equal(reads, 3);
});

test('HTTP refusal retains status and headers and is never cached as an empty store', async () => {
  let reads = 0;
  const loader = createCensusLoader(async () => ++reads === 1
    ? new Response('unavailable', { status: 503, headers: { 'x-reason': 'locked' } })
    : Response.json([]));
  const failed = await loader.load('one', 0);
  assert.equal(failed.ok, false);
  assert.equal(failed.status, 503);
  assert.equal(failed.body, null);
  assert.equal(failed.headers.get('x-reason'), 'locked');
  assert.deepEqual((await loader.load('one', 0)).body, []);
  assert.equal(reads, 2);
});

test('malformed bodies and transport failures refuse visibly and release the retry key', async () => {
  for (const failure of [() => Response.json({}), () => new Response('{bad'), () => { throw new Error('connection lost'); }]) {
    let reads = 0;
    const loader = createCensusLoader(async () => ++reads === 1 ? failure() : Response.json([]));
    await assert.rejects(loader.load('one', 0), /not a JSON array|JSON|connection lost/);
    assert.deepEqual((await loader.load('one', 0)).body, []);
    assert.equal(reads, 2);
  }
});

test('the complete-body deadline aborts a stalled JSON body and allows retry', async () => {
  let reads = 0;
  /** @type {AbortSignal | null | undefined} */
  let signal;
  const loader = createCensusLoader(async (_url, options = {}) => {
    signal = options.signal;
    if (++reads > 1) return Response.json([]);
    const result = Response.json([]);
    result.json = () => new Promise(() => {});
    return result;
  }, 20);
  await assert.rejects(loader.load('one', 0), /No complete answer.*within 0.02 s/);
  assert.ok(signal);
  assert.equal(signal.aborted, true);
  assert.deepEqual((await loader.load('one', 0)).body, []);
});

test('invalid read keys refuse before any fetch', async () => {
  let reads = 0;
  const loader = createCensusLoader(async () => { reads += 1; return Response.json([]); });
  /** @type {[string, number][]} */
  const invalid = [['', 0], ['one', -1], ['one', 1.5], ['one', Infinity]];
  for (const [feed, generation] of invalid) {
    await assert.rejects(loader.load(feed, generation), /valid refresh generation/);
  }
  assert.equal(reads, 0);
});

test('validated refresh uses an ETag, no second parse, and the same immutable rows', async () => {
  let calls = 0, parses = 0;
  const rows = [{ stored: 'unchanged' }];
  const loader = createCensusLoader(async (_url, options = {}) => {
    calls++;
    if (calls > 1) {
      assert.equal(new Headers(options.headers).get('if-none-match'), '"exact-census"');
      return new Response(null, { status: 304, headers: { etag: '"exact-census"', 'x-brutex-census-note': 'same manifest' } });
    }
    assert.equal(new Headers(options.headers).get('if-none-match'), null);
    const response = Response.json(rows, { headers: { etag: '"exact-census"' } });
    response.json = async () => { parses++; return rows; };
    return response;
  });
  const original = await loader.load('first', 0);
  const refreshed = await loader.load('first', 1);
  assert.equal(refreshed.status, 304);
  assert.equal(refreshed.ok, true);
  assert.equal(refreshed.body, original.body);
  assert.equal(refreshed.headers.get('x-brutex-census-note'), 'same manifest');
  assert.equal(parses, 1);
});

test('unsolicited or mismatched unchanged responses refuse and never invent an empty inventory', async () => {
  const unsolicited = createCensusLoader(async () => new Response(null, { status: 304 }));
  await assert.rejects(unsolicited.load('first', 0), /unrecognised/);
  let calls = 0;
  const mismatch = createCensusLoader(async () => ++calls === 1
    ? Response.json([], { headers: { etag: '"one"' } })
    : new Response(null, { status: 304, headers: { etag: '"two"' } }));
  await mismatch.load('first', 0);
  await assert.rejects(mismatch.load('first', 1), /unrecognised/);
});

test('census backpressure retry preserves the exact validator and reports its bounded retry',async()=>{
  let calls=0;
  /** @type {{attempt:number,nextAttempt:number,delayMs:number,status:number}[]} */
  const busy=[];
  const loader=createCensusLoader(async(_url,options)=>{
    calls++;
    if(calls===1)return Response.json([{stored:'same'}],{headers:{etag:'"same"'}});
    assert.equal(new Headers(options?.headers).get('if-none-match'),'"same"');
    return calls===2 ? new Response(null,{status:429,headers:{'retry-after':'0'}})
      : new Response(null,{status:304,headers:{etag:'"same"'}});
  },STORE_CENSUS_MS,info=>busy.push(info));
  const first=await loader.load('example',0);
  const after=await loader.load('example',1);
  assert.equal(after.body,first.body);assert.equal(calls,3);
  assert.deepEqual(busy,[{attempt:1,nextAttempt:2,delayMs:0,status:429}]);
});

/* ======================================================================
   WP-56 — THE CENSUS HEADERS, THE MOVED CHECK, THE BANNER'S WORDS AND THE
   FACETS CLIENT (D-3327; DB-022, DB-060, DB-G03, DB-G11, DB-G60, DB-G78,
   DB-B56, DB-B67). Every header value below is invented for the fixture.
   ====================================================================== */
import { censusLag, partGenerations, sinceFor, censusMoved, censusLines, movedLines,
  createFacetsLoader, mergeFacets, facetsUrl, facetKey } from '../src/lib/store-census.js';

/** @param {Record<string, string>} extra */
const lagOf = (extra = {}) => censusLag(new Headers({
  'x-brutex-census-state': 'held',
  'x-brutex-census-note': 'example: 7 month(s), 70 row(s), generation 60',
  'x-brutex-census-degraded': '',
  'x-brutex-census-generation': '60',
  'x-brutex-census-capacity': '1000',
  'x-brutex-census-headroom': '900',
  'x-brutex-census-epoch': 'INDEX=000001,CASH=000001,FNO=000002',
  'x-brutex-census-part-generations': 'INDEX=10,CASH=20,FNO=30',
  'x-brutex-census-reason': '',
  'x-brutex-census-behind-days': '0',
  'x-brutex-census-behind-from': '',
  ...extra
}));

test('every census header is read once, and an unreadable one is a named fault, never a zero', () => {
  const lag = lagOf();
  assert.equal(lag.state, 'held');
  assert.equal(lag.generation, 60);
  assert.equal(lag.capacity, 1000);
  assert.equal(lag.headroom, 900);
  assert.deepEqual(lag.parts, { INDEX: 10, CASH: 20, FNO: 30 });
  assert.equal(lag.behindDays, 0);
  assert.equal(lag.behindFrom, null);
  assert.equal(lag.reason, null);
  assert.deepEqual(lag.faults, []);
  const bad = lagOf({ 'x-brutex-census-headroom': '-1', 'x-brutex-census-state': 'sleepy',
    'x-brutex-census-behind-from': 'yesterday', 'x-brutex-census-part-generations': 'INDEX=1,INDEX=2' });
  assert.equal(bad.headroom, null);
  assert.equal(bad.behindFrom, null);
  assert.equal(bad.parts, null);
  assert.equal(bad.faults.length, 4, bad.faults.join(' | '));
  // An API older than D-3327 names the generation only in its note.
  const old = censusLag(new Headers({ 'x-brutex-census-state': 'held', 'x-brutex-census-note': 'example: 1 month(s), 2 row(s), generation 3' }));
  assert.equal(old.generation, 3);
  assert.equal(old.parts, null);
  assert.equal(old.capacity, null, 'a header this build does not send is null, not zero');
});

test('part generations parse whole or not at all', () => {
  /** @type {string[]} */
  const faults = [];
  assert.deepEqual(partGenerations('INDEX=2097116,CASH=2114847,FNO=2841866', faults), { INDEX: 2097116, CASH: 2114847, FNO: 2841866 });
  assert.equal(partGenerations('', faults), null);
  assert.equal(partGenerations(null, faults), null);
  assert.deepEqual(faults, []);
  for (const bad of ['FNO=', 'FNO=01', 'fno=1', 'FNO=1,', 'FNO=99999999999999999999']) {
    const seen = /** @type {string[]} */ ([]);
    assert.equal(partGenerations(bad, seen), null, bad);
    assert.equal(seen.length, 1, bad);
  }
});

test('a facets since is the part generation of the segment asked, never the vendor-wide sum (DB-B56)', () => {
  const lag = lagOf();
  assert.equal(sinceFor(lag, 'FNO'), 30);
  assert.equal(sinceFor(lag, 'INDEX'), 10);
  assert.equal(sinceFor(lag, 'BSE'), null, 'a segment the census names no part for has no since');
  const single = lagOf({ 'x-brutex-census-part-generations': '' });
  assert.equal(sinceFor(single, 'FNO'), 60, 'a single file answers in its one generation');
  assert.equal(sinceFor(null, 'FNO'), null);
});

test('the store moved only when both generations are known and differ, and says which parts moved (DB-022)', () => {
  const read = { feed: 'example', lag: lagOf() };
  const head = (/** @type {Record<string, string>} */ extra, feed = 'example') => ({ feed, at: 5, lag: lagOf(extra), error: null });
  assert.equal(censusMoved(read, head({})), false);
  assert.deepEqual(censusMoved(read, head({ 'x-brutex-census-generation': '64', 'x-brutex-census-part-generations': 'INDEX=10,CASH=20,FNO=34' })),
    { from: 60, to: 64, segments: ['FNO'] });
  assert.equal(censusMoved(read, head({ 'x-brutex-census-generation': '64' }, 'other')), null, 'a HEAD of another feed says nothing');
  assert.equal(censusMoved(read, { feed: 'example', at: 5, lag: null, error: 'HTTP 503' }), null, 'a failed HEAD is not "unchanged"');
  assert.equal(censusMoved(null, head({})), null);
  assert.equal(censusMoved({ feed: 'example', lag: censusLag(new Headers()) }, head({})), null, 'no generation read, nothing to compare');
});

test('the banner says when the snapshot was read, and that it is a snapshot', () => {
  const at = Date.UTC(2026, 9, 10, 14, 30, 5);
  const lines = censusLines({ feed: 'example', at, lag: lagOf(), worker: null, head: null });
  assert.equal(lines.length, 1);
  assert.equal(lines[0].tone, 'info');
  assert.equal(lines[0].text, 'Census read at 10 Oct 2026, 20:00:05 IST, generation 60 (INDEX 10 · CASH 20 · FNO 30): every list, total and offset on this page is that snapshot.');
  const checked = censusLines({ feed: 'example', at, lag: lagOf(), worker: null, head: { feed: 'example', at: at + 5000, lag: lagOf(), error: null } });
  assert.match(checked[0].text, /Checked again at 10 Oct 2026, 20:00:10 IST: unchanged\.$/);
  const failed = censusLines({ feed: 'example', at, lag: lagOf(), worker: 'this browser runs no Web Worker, so this page decoded the census on its own thread',
    head: { feed: 'example', at: at + 5000, lag: null, error: 'HEAD /store.json?feed=example answered HTTP 503' } });
  assert.deepEqual(failed.map((l) => l.key), ['read', 'head', 'worker']);
  assert.equal(failed[1].text, 'Whether the store moved since could not be checked at 10 Oct 2026, 20:00:10 IST: HEAD /store.json?feed=example answered HTTP 503.');
  assert.equal(failed[2].text, 'This browser runs no Web Worker, so this page decoded the census on its own thread.');
});

test('a census behind its import or capped is a warning with its days, its room and the server sentence (DB-G03, DB-G11, DB-G60, DB-G78)', () => {
  const behind = censusLines({ feed: 'example', at: 0, worker: null, head: null, lag: lagOf({
    'x-brutex-census-state': 'behind', 'x-brutex-census-behind-days': '32', 'x-brutex-census-behind-from': '2019-11-14',
    'x-brutex-census-reason': 'behind: the import journal closed 32 day(s) census-refused from 2019-11-14' }) });
  assert.equal(behind[1].tone, 'warn');
  assert.equal(behind[1].text, 'The example census is behind its import: 32 import day(s) from 14 Nov 2019 are on disk and not in this census, so their bars are not listed here. Server: behind: the import journal closed 32 day(s) census-refused from 2019-11-14');
  const capped = censusLines({ feed: 'example', at: 0, worker: null, head: null, lag: lagOf({
    'x-brutex-census-state': 'capped', 'x-brutex-census-capacity': '2097152', 'x-brutex-census-headroom': '297',
    'x-brutex-census-reason': 'capped: 297 entries left' }) });
  assert.equal(capped[1].text, 'The example census is capped: 297 of 20,97,152 entries left in its fullest part, so what an import adds next may not be listed. Server: capped: 297 entries left');
  const unknown = censusLines({ feed: 'example', at: 0, worker: null, head: null, lag: censusLag(new Headers()) });
  assert.equal(unknown[1].text, 'This API sends no census state, so whether the census is complete is not known.');
  const faulty = censusLines({ feed: 'example', at: 0, worker: null, head: null, lag: lagOf({ 'x-brutex-census-capacity': 'lots' }) });
  assert.equal(faulty[1].key, 'faults');
  assert.match(faulty[1].text, /x-brutex-census-capacity is "lots", not a whole number/);
});

test('the moved lines name the generations, the snapshot and what moved for the instrument on screen', () => {
  const at = Date.UTC(2026, 9, 10, 14, 30, 5);
  const head = { feed: 'example', at: at + 15000, lag: null, error: null };
  const moved = { from: 60, to: 64, segments: ['FNO'] };
  const base = movedLines(moved, head, at, null, null);
  assert.deepEqual(base.map((l) => l.text), ['The store moved after this page read it: generation 60 → 64 (FNO changed), seen at 10 Oct 2026, 20:00:20 IST. This page still shows the snapshot read at 10 Oct 2026, 20:00:05 IST.']);
  const query = { feed: 'example', segment: 'FNO', underlying: 'SAMPLE' };
  const changed = movedLines(moved, head, at, 'SAMPLE', [{ ok: true, status: 200, query, reason: null,
    body: { underlyings: [{ symbol: 'SAMPLE', last_ts: Date.UTC(2020, 8, 15, 9, 59, 59) * 1000 }], months: [{}, {}], expiries: [{}], strikes: [], futures: [] } }]);
  assert.equal(changed[1].text, 'SAMPLE FNO: 2 month(s) and 1 expiry(ies) changed since this snapshot; its newest bar is now 15 Sep 2020, 15:29:59 IST.');
  const none = movedLines(moved, head, at, 'SAMPLE', [{ ok: true, status: 200, query, reason: null,
    body: { underlyings: [], months: [], expiries: [], strikes: [], futures: [] } }]);
  assert.equal(none[1].text, 'Nothing changed for SAMPLE (FNO): what moved is other instruments.');
  const gone = movedLines(moved, head, at, 'SAMPLE', [{ ok: false, status: 410, query, reason: 'since_too_old', body: null },
    { ok: false, status: 409, query: { ...query, segment: 'INDEX' }, reason: 'since_in_future', body: null }]);
  assert.deepEqual(gone.slice(1).map((l) => l.text), [
    'SAMPLE FNO: the server no longer holds the changes since this snapshot; Refresh reads it whole.',
    "SAMPLE INDEX: the server's census for it stands below this snapshot's generation (read whole since); Refresh reads it whole."
  ]);
});

test('the census loader publishes every header and when this browser received the answer', async () => {
  const loader = createCensusLoader(async () => Response.json([], { headers: { 'x-brutex-census-state': 'behind', 'x-brutex-census-generation': '9' } }),
    STORE_CENSUS_MS, () => {}, async () => [{ decodedBy: 'the reader passed in' }]);
  const before = Date.now();
  const answer = await loader.load('example', 0);
  assert.deepEqual(answer.body, [{ decodedBy: 'the reader passed in' }], 'the body reader is the one the caller passed (the worker)');
  assert.equal(answer.lag?.state, 'behind');
  assert.equal(answer.lag?.generation, 9);
  assert.ok((answer.readAt ?? 0) >= before);
});

/* ---- the facets client ---------------------------------------------- */
const facetsBody = (/** @type {any} */ over = {}) => ({ schema: 1, feed: 'example', segment: 'FNO', underlying: 'SAMPLE', expiry: null,
  generation: 30, since: null, state: 'held', degraded: null,
  underlyings: [{ exchange: 'NSE', symbol: 'SAMPLE', months: 2, expiries: 1, rows: { '1s': 5 }, first_ts: 1, last_ts: 2 }],
  months: [{ month: '2020-01', rows: { '1s': 2 } }, { month: '2020-02', rows: { '1s': 3 } }],
  expiries: [{ expiry: '2020-02-27', contracts: 2, rows: { '1s': 5 } }], strikes: [], futures: [], ...over });

test('facets: one cold read per selection, a refresh asks only since the held generation and merges it', async () => {
  /** @type {string[]} */
  const urls = [];
  const answers = [
    Response.json(facetsBody(), { headers: { 'x-brutex-census-part-generation': '30' } }),
    Response.json(facetsBody({ generation: 34, since: 30, months: [{ month: '2020-02', rows: { '1s': 7 } }, { month: '2020-03', rows: { '1s': 1 } }],
      expiries: [], underlyings: [{ exchange: 'NSE', symbol: 'SAMPLE', months: 3, expiries: 1, rows: { '1s': 10 }, first_ts: 1, last_ts: 9 }] }),
    { headers: { 'x-brutex-census-part-generation': '34' } })
  ];
  const loader = createFacetsLoader(async (url) => { urls.push(url); return /** @type {Response} */ (answers.shift()); });
  const query = { feed: 'example', segment: 'FNO', underlying: 'SAMPLE' };
  const [one, two] = await Promise.all([loader.load(query), loader.load(query)]);
  assert.equal(one, two);
  assert.equal(urls.length, 1, 'a shared cold read');
  const fresh = await loader.refresh(query);
  assert.deepEqual(urls, ['/census/facets.json?feed=example&segment=FNO&underlying=SAMPLE',
    '/census/facets.json?feed=example&segment=FNO&underlying=SAMPLE&since=30']);
  assert.ok(fresh.ok);
  assert.deepEqual(fresh.delta, { underlyings: 1, months: 2, expiries: 0, strikes: 0, futures: 0, since: 30, generation: 34 });
  assert.equal(fresh.body.generation, 34);
  assert.deepEqual(fresh.body.months.map((/** @type {any} */ m) => [m.month, m.rows['1s']]), [['2020-01', 2], ['2020-02', 7], ['2020-03', 1]]);
  assert.equal(fresh.body.expiries.length, 1, 'an unchanged expiry is kept');
  assert.equal(loader.peek(query), fresh);
});

test('facets: a refused delta (410 too old, 409 in the future) is read whole again and said so, never patched', async () => {
  for (const status of [410, 409]) {
    let call = 0;
    const loader = createFacetsLoader(async () => {
      call += 1;
      if (call === 2) return Response.json({ error: status === 410 ? 'since_too_old' : 'since_in_future' }, { status });
      return Response.json(facetsBody({ generation: call === 1 ? 30 : 12 }));
    });
    const query = { feed: 'example', segment: 'FNO', underlying: 'SAMPLE' };
    await loader.load(query);
    const again = await loader.refresh(query);
    assert.equal(call, 3, `${status}: one delta, then one whole read`);
    assert.ok(again.ok);
    assert.equal(again.reloaded, true);
    assert.equal(again.body.generation, 12);
  }
});

test('facets: a header and body that spell the generation differently are refused (DB-B56)', async () => {
  const loader = createFacetsLoader(async () => Response.json(facetsBody(), { headers: { 'x-brutex-census-part-generation': '7053829' } }));
  const answer = await loader.load({ feed: 'example', segment: 'FNO', underlying: 'SAMPLE' });
  assert.equal(answer.ok, false);
  assert.equal(answer.reason, '/census/facets.json: its header names part generation 7053829 and its body 30');
});

test('facets: changesSince answers the delta alone and keeps nothing', async () => {
  /** @type {string[]} */
  const urls = [];
  const loader = createFacetsLoader(async (url) => { urls.push(url);
    return Response.json(facetsBody({ generation: 34, since: 30, months: [], expiries: [], underlyings: [] })); });
  const query = { feed: 'example', segment: 'FNO', underlying: 'SAMPLE' };
  const delta = await loader.changesSince(query, 30);
  assert.deepEqual(urls, ['/census/facets.json?feed=example&segment=FNO&underlying=SAMPLE&since=30']);
  assert.deepEqual(delta.delta, { underlyings: 0, months: 0, expiries: 0, strikes: 0, futures: 0, since: 30, generation: 34 });
  assert.equal(loader.peek(query), null);
  const wrong = await createFacetsLoader(async () => Response.json(facetsBody({ since: 29 }))).changesSince(query, 30);
  assert.equal(wrong.ok, false, 'a delta since another generation is not this one');
});

test('facets: a merged answer equals the cold answer at the delta generation, strike sides included', () => {
  const strike = (/** @type {number} */ s, /** @type {any} */ ce, /** @type {any} */ pe) => ({ strike: s, ce, pe });
  const base = facetsBody({ expiry: '2020-02-27', strikes: [strike(100, { rows: { '1s': 1 } }, null), strike(200, { rows: { '1s': 2 } }, { rows: { '1s': 2 } })] });
  // The delta carries only the side that changed; the other is null, "unchanged".
  const delta = facetsBody({ expiry: '2020-02-27', generation: 31, since: 30, underlyings: [], months: [], expiries: [],
    strikes: [strike(100, null, { rows: { '1s': 4 } }), strike(150, { rows: { '1s': 3 } }, null)] });
  const cold = facetsBody({ expiry: '2020-02-27', generation: 31,
    strikes: [strike(100, { rows: { '1s': 1 } }, { rows: { '1s': 4 } }), strike(150, { rows: { '1s': 3 } }, null), strike(200, { rows: { '1s': 2 } }, { rows: { '1s': 2 } })] });
  assert.deepEqual(mergeFacets(base, delta), cold);
});

test('facets URLs and keys spell a query one way', () => {
  assert.equal(facetsUrl({ feed: 'g d', segment: 'FNO', underlying: 'M&M', exchange: 'NSE', expiry: '2020-01-30' }, 5),
    '/census/facets.json?feed=g%20d&segment=FNO&underlying=M%26M&exchange=NSE&expiry=2020-01-30&since=5');
  assert.equal(facetKey({ feed: 'a', segment: 'FNO' }), facetKey({ feed: 'a', segment: 'FNO', underlying: null, exchange: undefined }));
  assert.notEqual(facetKey({ feed: 'a', segment: 'FNO' }), facetKey({ feed: 'a', segment: 'INDEX' }));
});

/* ======================================================================
   WP-56 — A REFRESH ASKS ONLY FOR WHAT CHANGED (D-3327 item 3; DB-022,
   DB-060, DB-G59, DB-G70). A fixture API holding an invented census, which
   answers a whole body, a `since` delta, and each refusal the real one does.
   ====================================================================== */
import { deltaBase } from '../src/lib/store-census.js';
import { CENSUS_ENCODING, censusOrder, censusMeasureOf } from '../src/lib/store-census-wire.js';

/** `2019-03 NSE-INDEX-NIFTY 1min` and the bars it holds. @typedef {[string, number]} Line */

/** The tuples wire of `lines`, in the order given. @param {Line[]} lines @param {Record<string, unknown>} [more] */
function wire(lines, more = {}) {
  /** @type {any} */
  const value = { schema: 1, encoding: CENSUS_ENCODING, feed: 'example', instruments: [], months: [], timeframes: [], reasons: ['previous_not_stored'], rows: [], ...more };
  const at = (/** @type {string[]} */ list, /** @type {string} */ word) => (list.includes(word) ? list.indexOf(word) : list.push(word) - 1);
  for (const [text, bars] of lines) {
    const [month, instrument, timeframe] = text.split(' ');
    value.rows.push([at(value.instruments, instrument), at(value.months, month), at(value.timeframes, timeframe), bars, 1, 2, null, 0, null, 0]);
  }
  return value;
}

/**
 * A fixture API over one census: `lines` in the server's order, a generation
 * that rises by one per `commit`, and the changes each commit made. `since`
 * is answered as `/store.json` answers it.
 * @param {Line[]} lines
 * @param {{parts?: boolean, old?: boolean}} [shape] `parts`: a partitioned census; `old`: an API that reads no `since`
 */
function fixtureApi(lines, shape = {}) {
  const before = censusOrder();
  const row = (/** @type {Line} */ [text]) => { const [month, instrument, timeframe] = text.split(' '); return { month, instrument, timeframe }; };
  let held = [...lines].sort((a, b) => before(row(a), row(b)));
  let generation = 40;
  let floor = 40;
  let epoch = 'INDEX=000001,CASH=none,FNO=000002';
  /** @type {{generation: number, keys: string[]}[]} */
  const commits = [];
  /** @type {{url: string, options: any}[]} */
  const asked = [];
  /** @type {null | ((url: string) => Response | null)} */
  let override = null;
  const spell = (/** @type {number} */ g) => (shape.parts ? `INDEX=${g},CASH=0,FNO=${g}` : String(g));
  const headers = () => ({ 'x-brutex-census-state': 'held', 'x-brutex-census-degraded': '',
    'x-brutex-census-note': `example: ${held.length} month(s), ${held.reduce((a, [, n]) => a + n, 0)} row(s), generation ${generation}`,
    'x-brutex-census-generation': String(shape.parts ? 2 * generation : generation),
    ...(shape.old ? {} : { 'x-brutex-census-part-generations': shape.parts ? spell(generation) : '', 'x-brutex-census-epoch': shape.parts ? epoch : '' }) });
  return {
    asked,
    get generation() { return generation; },
    /** The whole census now, as row names. */
    names: () => held.map(([text]) => text),
    bars: () => held.reduce((a, [, n]) => a + n, 0),
    /** @param {Line[]} changes */
    commit(changes) {
      generation += 1;
      for (const change of changes) {
        const i = held.findIndex(([text]) => text === change[0]);
        if (i >= 0) held[i] = change; else held.push(change);
      }
      held.sort((a, b) => before(row(a), row(b)));
      commits.push({ generation, keys: changes.map(([text]) => text) });
    },
    /** The census was read whole again (a restart): no appends are held. */
    restart() { floor = generation; },
    newEpoch() { epoch = 'INDEX=000001,CASH=none,FNO=000003'; },
    /** @param {(url: string) => Response | null} with */
    answerWith(with_) { override = with_; },
    /** @type {typeof import('../src/lib/ask.js').ask} */
    request: async (url, options = {}) => {
      asked.push({ url: String(url), options });
      const forced = override?.(String(url));
      if (forced) return forced;
      const query = new URL(String(url), 'http://fixture').searchParams;
      const since = shape.old ? null : query.get('since');
      if (since === null) return Response.json(wire(held), { headers: { ...headers(), etag: `"g${generation}"` } });
      const said = { ...headers(), 'x-brutex-census-since': since };
      if (shape.parts && query.get('epoch') !== epoch) return Response.json({ error: 'epoch_changed' }, { status: 409, headers: said });
      const g = Number(shape.parts ? /FNO=(\d+)/.exec(since)?.[1] : since);
      if (g > generation) return Response.json({ error: 'since_in_future' }, { status: 409, headers: said });
      if (g < floor) return Response.json({ error: 'since_too_old' }, { status: 410, headers: said });
      const keys = new Set(commits.filter((c) => c.generation > g).flatMap((c) => c.keys));
      return Response.json(wire(held.filter(([text]) => keys.has(text)), { since, held: { months: held.length, rows: held.reduce((a, [, n]) => a + n, 0) } }), { headers: said });
    }
  };
}

/** @type {Line[]} */
const CENSUS = [['2019-03 NSE-INDEX-NIFTY 1min', 10], ['2019-03 NSE-CASH-RELIANCE 1s', 20],
  ['2019-03 NSE-FNO-NIFTY-2019-03-28-1150000-CE 1s', 30], ['2019-02 NSE-INDEX-NIFTY 1min', 40]];
const namesOf = (/** @type {any[] | null} */ rows) => (rows ?? []).map((r) => `${r.month} ${r.instrument} ${r.timeframe}`);

for (const parts of [false, true]) {
  test(`a refresh reads only the rows changed since the generation held and merges them (${parts ? 'a partitioned census' : 'a single file'})`, async () => {
    const api = fixtureApi(CENSUS, { parts });
    const loader = createCensusLoader(api.request);
    const first = await loader.load('example', 0);
    assert.deepEqual(first.how, { kind: 'whole', since: null, rows: 0, added: 0, replaced: 0, why: null });
    assert.equal(first.body?.length, 4);

    // NOTHING COMMITTED: a delta with no row; the rows held are kept, the SAME array.
    const same = await loader.load('example', 1);
    assert.equal(same.how?.kind, 'unchanged');
    assert.equal(same.body, first.body);
    assert.equal(same.headers.get('etag'), '"g40"', 'the validator still names these bytes');
    assert.equal(api.asked.length, 2);
    const since = parts ? 'INDEX=40,CASH=0,FNO=40' : '40';
    assert.equal(api.asked[1].url, `/store.json?feed=example&encoding=census-tuples-v1&since=${encodeURIComponent(since)}` +
      (parts ? `&epoch=${encodeURIComponent('INDEX=000001,CASH=none,FNO=000002')}` : ''));
    assert.equal(new Headers(api.asked[1].options.headers).get('if-none-match'), null, 'a delta is not a conditional read');

    // ONE COMMIT: a month grew, a new month and a new contract arrived.
    api.commit([['2019-03 NSE-INDEX-NIFTY 1min', 15], ['2019-04 NSE-INDEX-NIFTY 1min', 5], ['2019-03 NSE-FNO-NIFTY-2019-03-28-1160000-PE 1s', 7]]);
    const fresh = await loader.load('example', 2);
    assert.deepEqual(fresh.how, { kind: 'delta', since, rows: 3, added: 2, replaced: 1, why: null });
    assert.deepEqual(namesOf(fresh.body), api.names(), 'the census a whole read answers, position for position');
    assert.equal(fresh.body?.reduce((a, r) => a + r.rows, 0), api.bars());
    assert.equal(fresh.lag?.generation, parts ? 82 : 41);
    assert.equal(fresh.headers.get('etag'), null, 'merged rows have no validator: none names their bytes');
    assert.equal(fresh.body?.[2], first.body?.[1], 'a row nothing changed is the same object');
    assert.deepEqual(censusMeasureOf(/** @type {any[]} */ (fresh.body)), { order: null, bars: api.bars() });

    // AND AGAIN, FROM THE MERGED CENSUS: the next delta is asked since ITS generation.
    api.commit([['2019-03 NSE-CASH-RELIANCE 1s', 21]]);
    const next = await loader.load('example', 3);
    assert.equal(next.how?.kind, 'delta');
    assert.match(api.asked[3].url, parts ? /since=INDEX%3D41%2CCASH%3D0%2CFNO%3D41/ : /since=41$/);
    assert.deepEqual(namesOf(next.body), api.names());
    assert.equal(api.asked.length, 4, 'four requests in all: one whole census, three deltas');
  });
}

test('a refused delta reads the census whole and says why: too old, in the future, another epoch', async () => {
  /** @type {[string, (api: ReturnType<typeof fixtureApi>) => void, RegExp][]} */
  const cases = [
    ['too old', (api) => { api.commit([['2019-04 NSE-INDEX-NIFTY 1min', 5]]); api.restart(); }, /^the server refused a delta since INDEX=40,CASH=0,FNO=40 \(HTTP 410: since_too_old\)$/],
    ['another epoch', (api) => { api.commit([['2019-04 NSE-INDEX-NIFTY 1min', 5]]); api.newEpoch(); }, /^the server refused a delta since .* \(HTTP 409: epoch_changed\)$/]
  ];
  for (const [name, change, why] of cases) {
    const api = fixtureApi(CENSUS, { parts: true });
    const loader = createCensusLoader(api.request);
    const first = await loader.load('example', 0);
    change(api);
    const again = await loader.load('example', 1);
    assert.equal(again.how?.kind, 'whole', name);
    assert.match(again.how?.why ?? '', why, name);
    assert.deepEqual(namesOf(again.body), api.names(), name);
    assert.notEqual(again.body, first.body);
    assert.equal(api.asked.length, 3, `${name}: the whole census, the refused delta, the whole census again`);
    assert.equal(new Headers(api.asked[2].options.headers).get('if-none-match'), '"g40"', 'the whole read is still conditional');
  }
});

test('an API older than deltas is read whole, conditionally, and a refresh says so', async () => {
  const api = fixtureApi(CENSUS, { old: true });
  const loader = createCensusLoader(api.request);
  await loader.load('example', 0);
  api.commit([['2019-04 NSE-INDEX-NIFTY 1min', 5]]);
  const again = await loader.load('example', 1);
  assert.equal(again.how?.kind, 'whole');
  assert.equal(again.how?.why, 'this API answers no deltas (it names no part generations)');
  assert.equal(api.asked.length, 2, 'no delta was asked of an API that names no part generations');
  assert.equal(new Headers(api.asked[1].options.headers).get('if-none-match'), '"g40"');
  assert.deepEqual(deltaBase(new Headers({ 'x-brutex-census-part-generations': '', 'x-brutex-census-generation': '7' })), { since: '7', epoch: '' });
  assert.deepEqual(deltaBase(new Headers({ 'x-brutex-census-part-generations': 'FNO=7', 'x-brutex-census-epoch': 'FNO=000001' })), { since: 'FNO=7', epoch: 'FNO=000001' });
  assert.match(/** @type {any} */ (deltaBase(new Headers({ 'x-brutex-census-part-generations': 'FNO=7' }))).why, /no epoch/);
  assert.match(/** @type {any} */ (deltaBase(new Headers({ 'x-brutex-census-part-generations': '' }))).why, /no generation/);
});

test('an API that answers the whole census to a delta request is read as the whole census', async () => {
  const api = fixtureApi(CENSUS);
  const loader = createCensusLoader(api.request);
  await loader.load('example', 0);
  api.commit([['2019-04 NSE-INDEX-NIFTY 1min', 5]]);
  // The whole body, with no x-brutex-census-since: what a build between the two answers.
  api.answerWith((url) => (url.includes('since=') ? Response.json(wire(/** @type {Line[]} */ (api.names().map((text) => [text, 1]))), {
    headers: { 'x-brutex-census-state': 'held', 'x-brutex-census-degraded': '', 'x-brutex-census-generation': '41', 'x-brutex-census-part-generations': '' } }) : null));
  const again = await loader.load('example', 1);
  assert.equal(again.how?.kind, 'whole');
  assert.equal(again.how?.why, 'this API answered the whole census to a request for a delta');
  assert.equal(again.body?.length, 5);
  assert.equal(api.asked.length, 2, 'the answer in hand is used; it is not asked for twice');
});

test('a delta that does not add up, is out of order, or answers another since is never merged', async () => {
  /** @type {[string, (api: ReturnType<typeof fixtureApi>) => Response, RegExp][]} */
  const cases = [
    ['the counters disagree', (api) => Response.json(wire([['2019-04 NSE-INDEX-NIFTY 1min', 5]], { since: '40', held: { months: 9, rows: api.bars() } }), { headers: { 'x-brutex-census-since': '40', 'x-brutex-census-degraded': '' } }),
      /^the census after the delta since 40 holds 5 row\(s\) and 105 bar\(s\) where the server holds 9 and 105$/],
    ['nothing changed but the counters moved', (api) => Response.json(wire([], { since: '40', held: { months: 4, rows: 1 + api.bars() } }), { headers: { 'x-brutex-census-since': '40', 'x-brutex-census-degraded': '' } }),
      /^the census after the delta since 40 holds 4 row\(s\) and 100 bar\(s\) where the server holds 4 and 106$/],
    ['another since', () => Response.json(wire([], { since: '39', held: { months: 4, rows: 100 } }), { headers: { 'x-brutex-census-since': '39', 'x-brutex-census-degraded': '' } }),
      /^the answer to a delta since 40 names since "39"/],
    ['no counters', () => Response.json(wire([], { since: '40' }), { headers: { 'x-brutex-census-since': '40', 'x-brutex-census-degraded': '' } }),
      /no whole counters to check a merge against$/],
    ['rows out of order', () => Response.json(wire([['2019-02 NSE-INDEX-NIFTY 1min', 41], ['2019-04 NSE-INDEX-NIFTY 1min', 5]], { since: '40', held: { months: 5, rows: 106 } }), { headers: { 'x-brutex-census-since': '40', 'x-brutex-census-degraded': '' } }),
      /^the delta since 40 could not be merged \(the delta's row 2 .* does not follow its row 1/],
    ['not the tuples wire', () => Response.json({ rows: 'soon' }, { headers: { 'x-brutex-census-since': '40', 'x-brutex-census-degraded': '' } }),
      /^the delta since 40 could not be read \(The body of \/store\.json is not/]
  ];
  for (const [name, answer, why] of cases) {
    const api = fixtureApi(CENSUS);
    const loader = createCensusLoader(api.request);
    await loader.load('example', 0);
    api.commit([['2019-04 NSE-INDEX-NIFTY 1min', 5]]);
    api.answerWith((url) => (url.includes('since=') ? answer(api) : null));
    const again = await loader.load('example', 1);
    assert.equal(again.how?.kind, 'whole', name);
    assert.match(again.how?.why ?? '', why, name);
    assert.deepEqual(namesOf(again.body), api.names(), `${name}: the census is the server's, read whole`);
  }
});

test('a census that is not in the order a delta is merged by is never patched: its refresh is a whole read', async () => {
  // The whole body arrives with two rows swapped: an order this page does not know.
  const api = fixtureApi(CENSUS);
  const swapped = /** @type {Line[]} */ ([CENSUS[1], CENSUS[0], CENSUS[2], CENSUS[3]]);
  api.answerWith((url) => (url.includes('since=') ? null : Response.json(wire(swapped), {
    headers: { 'x-brutex-census-state': 'held', 'x-brutex-census-degraded': '', 'x-brutex-census-generation': '40', 'x-brutex-census-part-generations': '', etag: '"swapped"' } })));
  const loader = createCensusLoader(api.request);
  const first = await loader.load('example', 0);
  assert.deepEqual(namesOf(first.body), swapped.map(([text]) => text), 'drawn as it was sent');
  const again = await loader.load('example', 1);
  assert.equal(again.how?.kind, 'whole');
  assert.match(again.how?.why ?? '', /^the census held is not in the order a delta is merged by \(row 2 \(2019-03 NSE-INDEX-NIFTY 1min\) does not follow row 1 \(2019-03 NSE-CASH-RELIANCE 1s\)/);
  assert.ok(api.asked.every(({ url }) => !url.includes('since=')), 'no delta is asked for a census that cannot be merged into');
});

test('a busy or failing delta is a failed read with its reason, never a silent whole census; a degraded one is refused', async () => {
  const api = fixtureApi(CENSUS);
  const loader = createCensusLoader(api.request);
  await loader.load('example', 0);
  api.answerWith((url) => (url.includes('since=') ? Response.json({ error: 'census read unavailable: Saturated' }, { status: 503, headers: { 'x-brutex-census-since': '40' } }) : null));
  const busy = await loader.load('example', 1);
  assert.equal(busy.ok, false);
  assert.equal(busy.status, 503);
  assert.equal(busy.reason, 'census read unavailable: Saturated');
  assert.equal(api.asked.length, 2, 'one whole census, one refused delta, and no second 33 MB on top of a busy server');
  api.answerWith((url) => (url.includes('since=') ? Response.json(wire([], { since: '40', held: { months: 4, rows: 100 } }), {
    headers: { 'x-brutex-census-since': '40', 'x-brutex-census-degraded': 'recovered from an older generation' } }) : null));
  const degraded = await loader.load('example', 2);
  assert.equal(degraded.ok, false);
  assert.equal(degraded.degraded, 'recovered from an older generation');
  assert.match(degraded.reason ?? '', /refused rather than drawn/);
});
