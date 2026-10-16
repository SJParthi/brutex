// WP-56 (D-3327; DB-022, DB-G84, DB-B56, DB-B67): the REAL store module,
// compiled from its rune source exactly as `store-startup.test.js` does — the
// sliced fold, the census headers a read publishes, the generation watch and
// the survey of offered feeds only. Every census below is invented.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { compileModule } from 'svelte/compiler';

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

/** @param {string} instrument @param {string} month @param {string} timeframe @param {number} rows */
const row = (instrument, month, timeframe, rows) => ({ instrument, month, timeframe, rows, first_ts: 1, last_ts: 2,
  chg_bps: null, chg_why: 'previous_not_stored', prev_chg_bps: null, prev_chg_why: 'previous_not_stored' });

const settle = async () => { for (let i = 0; i < 20; i++) await new Promise((done) => setImmediate(done)); };

test('the sliced fold builds every index the one-pass fold built, in the same order', async () => {
  const module = await storeModule();
  /** @type {any[]} */
  const rows = [];
  // Enough rows that the fold runs in several slices.
  for (let i = 0; i < 3000; i++) rows.push(row(`NSE-CASH-S${i % 300}`, `2020-${String(12 - (i % 12)).padStart(2, '0')}`, i % 2 ? '1day' : '1min', i));
  rows.push({ instrument: 'NSE-CASH-BAD', month: 'not a month', timeframe: '1min', rows: 1 });
  const folded = await module.foldCensus(rows, undefined);
  assert.equal(folded.rows, rows);
  assert.equal(folded.cells, 3000);
  assert.equal(folded.bars, (2999 * 3000) / 2);
  assert.equal(folded.bad.length, 1);
  assert.equal(folded.badByInstrument.get('NSE-CASH-BAD').length, 1);
  assert.equal(folded.byInstrument.size, 300);
  for (const list of folded.byInstrument.values()) {
    for (let i = 1; i < list.length; i++) {
      const a = list[i - 1], b = list[i];
      assert.ok(a.month < b.month || (a.month === b.month && module.rungSeconds(a.timeframe) <= module.rungSeconds(b.timeframe)),
        'ascending by (month, rung)');
    }
  }
  assert.equal(folded.byCell.get('NSE-CASH-S7|1day|2020-06'), rows.find((r) => r.instrument === 'NSE-CASH-S7' && r.month === '2020-06' && r.timeframe === '1day')?.rows);
  let cells = 0;
  for (const month of folded.byMonth.values()) cells += month.cells;
  assert.equal(cells, 3000);
  const controller = new AbortController();
  controller.abort(new Error('a newer read replaced this one'));
  await assert.rejects(module.foldCensus(rows, controller.signal), /newer read/);
});

test('a read publishes the census headers and says the page decoded it when no worker could', async (t) => {
  const originalFetch = globalThis.fetch;
  t.after(() => { globalThis.fetch = originalFetch; });
  globalThis.fetch = async () => Response.json([row('NSE-INDEX-SAMPLE', '2020-01', '1s', 5)], { headers: {
    'x-brutex-census-state': 'behind', 'x-brutex-census-generation': '60', 'x-brutex-census-part-generations': 'INDEX=10,CASH=20,FNO=30',
    'x-brutex-census-behind-days': '2', 'x-brutex-census-behind-from': '2020-01-02', 'x-brutex-census-degraded': '' } });
  const module = await storeModule();
  const before = Date.now();
  module.syncStore('example');
  await module.readStoreCensus('example');
  await settle();
  assert.equal(module.store.state, 'ready');
  assert.equal(module.store.lag.state, 'behind');
  assert.equal(module.store.lag.generation, 60);
  assert.deepEqual(module.store.lag.parts, { INDEX: 10, CASH: 20, FNO: 30 });
  assert.equal(module.store.lag.behindFrom, '2020-01-02');
  assert.ok(module.store.at >= before);
  assert.equal(module.store.worker, 'this browser runs no Web Worker, so this page decoded the census on its own thread');
  module.syncStore(null);
  assert.equal(module.store.lag, null, 'no feed, no census headers');
});

test('the generation watch HEADs the census, keeps the answer, and stops when released', async (t) => {
  const originalFetch = globalThis.fetch;
  t.after(() => { globalThis.fetch = originalFetch; });
  /** @type {string[]} */
  const asked = [];
  let generation = 60;
  globalThis.fetch = async (url, options) => {
    asked.push(`${options?.method ?? 'GET'} ${url}`);
    return new Response(null, { headers: { 'x-brutex-census-state': 'held', 'x-brutex-census-generation': String(generation) } });
  };
  const module = await storeModule();
  await module.checkCensusHead('example');
  assert.equal(module.store.head.feed, 'example');
  assert.equal(module.store.head.lag.generation, 60);
  assert.equal(module.store.head.error, null);
  const release = module.watchCensus('example', 5);
  generation = 64;
  await new Promise((done) => setTimeout(done, 40));
  release();
  release();
  const count = asked.length;
  assert.ok(count >= 3, `the watch asked ${count - 1} time(s) in 40 ms at a 5 ms period`);
  assert.ok(asked.every((line) => line === 'HEAD /store.json?feed=example'), asked.join(' | '));
  assert.equal(module.store.head.lag.generation, 64);
  await new Promise((done) => setTimeout(done, 25));
  assert.equal(asked.length, count, 'a released watch asks nothing more');
  assert.equal(typeof module.watchCensus(null), 'function', 'no feed, nothing to watch');
  globalThis.fetch = async () => new Response(null, { status: 503 });
  await module.checkCensusHead('example');
  assert.equal(module.store.head.lag, null);
  assert.equal(module.store.head.error, 'HEAD /store.json?feed=example answered HTTP 503');
});

test('the survey asks only the feeds /feeds.json offers (D-3338; DB-G84)', async (t) => {
  const originalFetch = globalThis.fetch;
  t.after(() => { globalThis.fetch = originalFetch; });
  /** @type {string[]} */
  const asked = [];
  globalThis.fetch = async (url) => {
    asked.push(String(url));
    const feed = new URL(String(url), 'http://fixture').searchParams.get('feed');
    return new Response(null, { headers: { 'x-brutex-census-state': 'held', 'x-brutex-census-degraded': '',
      'x-brutex-census-note': `${feed}: 1 month(s), 2 row(s), generation 3` } });
  };
  const module = await storeModule();
  const answer = await module.surveyStores(/** @type {any} */ ([{ wire: 'zerodha', ready: true }, { wire: 'gdfl', ready: true, offered: true },
    { wire: 'dhan', ready: true, offered: false }, { wire: 'groww', ready: true, offered: false }]));
  assert.deepEqual(asked.map((url) => new URL(url, 'http://fixture').searchParams.get('feed')).sort(), ['gdfl', 'zerodha']);
  assert.deepEqual([...answer.byFeed.keys()].sort(), ['gdfl', 'zerodha']);
});
