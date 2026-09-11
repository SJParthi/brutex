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
    assert.equal(options.cache, 'no-store');
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
