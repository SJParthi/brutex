import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { compileModule } from 'svelte/compiler';

/** Compile the actual rune module; no replacement implementation is tested. */
async function storeModule() {
  const source = await readFile(new URL('../src/lib/store.svelte.js', import.meta.url), 'utf8');
  const { js } = compileModule(source, { filename: 'store.svelte.js', generate: 'client' });
  const code = js.code.replace(/from (["'])([^"']+)\1/g, (_match, quote, path) => {
    const resolved = path.startsWith('$lib/')
      ? new URL(`../src/lib/${path.slice(5)}`, import.meta.url).href : import.meta.resolve(path);
    return `from ${quote}${resolved}${quote}`;
  });
  return import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}#${Math.random()}`);
}

test('the real store fold and catalog read share one parsed response; clearing rejects the old flight', async (t) => {
  const originalFetch = globalThis.fetch;
  t.after(() => { globalThis.fetch = originalFetch; });
  /** @type {(response: Response) => void} */
  let finish = () => { throw new Error('fetch was not called'); };
  let reads = 0;
  globalThis.fetch = async () => {
    reads += 1;
    return new Promise((resolve) => { finish = resolve; });
  };
  const module = await storeModule();
  module.syncStore('first');
  const catalog = module.readStoreCensus('first');
  assert.equal(reads, 1);
  module.syncStore(null);
  finish(Response.json([]));
  await catalog;
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(module.store.state, 'none');
  assert.equal(module.store.feed, null);
  assert.equal(module.store.at, null);
  assert.equal(module.store.reads, 0, 'the cleared flight must not publish a new reading');
  module.syncStore('first');
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(module.store.state, 'ready');
  assert.equal(module.store.feed, 'first');
  assert.equal(reads, 1, 'reselecting the same generation reuses the parsed answer');

  module.store.generation += 1;
  module.syncStore('first');
  const refreshed = module.readStoreCensus('first');
  module.syncStore(null);
  module.syncStore('first');
  finish(Response.json([]));
  await refreshed;
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(module.store.reads, 2, 'clear/reselect must not fold both old and new subscribers');
  assert.equal(reads, 2, 'the reselected refresh also shares one physical response');
});

test('the real store keeps raw census identity and reuses its folded indexes after a verified 304', async (t) => {
  const originalFetch = globalThis.fetch;
  t.after(() => { globalThis.fetch = originalFetch; });
  const body = [{ feed:'first', instrument:'NSE-CASH-FIXTURE', timeframe:'1min', month:'2025-01', rows:2,
    first_ts:1, last_ts:2, chg_bps:0, chg_why:null, prev_chg_bps:null, prev_chg_why:'previous_not_stored' }];
  let calls = 0;
  globalThis.fetch = async (_url, options) => {
    if (++calls > 1) {
      assert.equal(new Headers(options?.headers).get('if-none-match'), '"snapshot"');
      return new Response(null, { status:304, headers:{etag:'"snapshot"'} });
    }
    const response = Response.json(body, {headers:{etag:'"snapshot"'}});
    response.json = async () => body;
    return response;
  };
  const module = await storeModule();
  module.syncStore('first');
  await module.readStoreCensus('first');
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(module.store.rows, body, 'no per-row Svelte proxies on immutable census data');
  assert.equal(module.store.rows[0], body[0]);
  const byCell = module.store.byCell, byInstrument = module.store.byInstrument;
  await module.refreshStore();
  assert.equal(module.store.rows, body);
  assert.equal(module.store.byCell, byCell, 'a valid unchanged response skips all census refolding');
  assert.equal(module.store.byInstrument, byInstrument);
  assert.equal(module.store.reads, 2);
  assert.equal(module.store.state, 'ready');
});

test('the real all-feed survey reads only header summaries and names each unreadable feed', async (t) => {
  const originalFetch = globalThis.fetch;
  t.after(() => { globalThis.fetch = originalFetch; });
  let calls = 0;
  globalThis.fetch = async (url, options) => {
    calls++;
    assert.equal(options?.method, 'HEAD');
    if (String(url).includes('broken')) return new Response(null,{status:503});
    const feed = String(url).includes('empty') ? 'empty' : 'held';
    return new Response(null,{headers:{'x-brutex-census-state':'held','x-brutex-census-degraded':'',
      'x-brutex-census-note':`${feed}: ${feed === 'held' ? '2' : '0'} month(s), ${feed === 'held' ? '9' : '0'} row(s), generation 3`}});
  };
  const module = await storeModule();
  const answer = await module.surveyStores(/** @type {any} */ ([{wire:'held',ready:true},{wire:'empty',ready:false},{wire:'broken',ready:true}]));
  assert.equal(calls, 3);
  assert.deepEqual(answer.byFeed.get('held'),{wire:'held',ready:true,bars:9,cells:2,any:true,error:null});
  assert.equal(answer.byFeed.get('empty').any, false);
  assert.match(answer.byFeed.get('broken').error, /HTTP 503/);
});
