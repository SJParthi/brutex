// CE-77, D-1786: a degraded (recovered older-generation) census answered 200
// and every selected-feed reader drew it as the current store.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { censusFailure, createCensusLoader, degradedRefusal } from '../src/lib/store-census.js';

/** @param {string} path */
const source = (path) => readFileSync(new URL(path, import.meta.url), 'utf8');
const held = (extra = {}) => ({ 'x-brutex-census-state': 'held', 'x-brutex-census-note': 'dhan: 2 month(s), 9 row(s), generation 3', ...extra });

test('an empty or absent degraded header is not a refusal; a non-empty one is', () => {
  assert.equal(degradedRefusal(new Headers(held({ 'x-brutex-census-degraded': '' }))), null);
  assert.equal(degradedRefusal(new Headers(held())), null);
  const why = degradedRefusal(new Headers(held({ 'x-brutex-census-degraded': 'newest header slot failed its checksum' })));
  assert.match(why ?? '', /DEGRADED \(newest header slot failed its checksum\)/);
  assert.match(why ?? '', /older generation/);
});

test('a degraded 200 is refused by the loader, its rows are not parsed, and it is not cached', async () => {
  let reads = 0;
  let parses = 0;
  const loader = createCensusLoader(async () => {
    reads += 1;
    const response = Response.json([{ instrument: 'NSE-INDEX-NIFTY', month: '2026-01', timeframe: '1min', rows: 1 }], {
      headers: held({ 'x-brutex-census-degraded': 'recovered generation 2 of 3' })
    });
    response.json = async () => { parses += 1; return []; };
    return response;
  });
  const answer = await loader.load('dhan', 0);
  assert.equal(answer.ok, false);
  assert.equal(answer.status, 200);
  assert.equal(answer.body, null);
  assert.equal(answer.degraded, 'recovered generation 2 of 3');
  assert.match(censusFailure(answer), /^\/store\.json: the census is DEGRADED \(recovered generation 2 of 3\)/);
  assert.equal(parses, 0);
  await loader.load('dhan', 0);
  assert.equal(reads, 2, 'a refused answer is never served from the cache');
});

test('a 304 that turns a held census degraded is refused, not served from the earlier rows', async () => {
  let call = 0;
  const loader = createCensusLoader(async () => {
    call += 1;
    if (call === 1) {
      return Response.json([], { headers: held({ etag: '"g3"', 'x-brutex-census-degraded': '' }) });
    }
    return new Response(null, { status: 304, headers: { etag: '"g3"', 'x-brutex-census-degraded': 'recovered generation 2' } });
  });
  const first = await loader.load('dhan', 0);
  assert.equal(first.ok, true);
  const second = await loader.load('dhan', 1);
  assert.equal(second.ok, false);
  assert.equal(second.body, null);
  assert.match(censusFailure(second), /DEGRADED \(recovered generation 2\)/);
});

test('every page that folds the selected census shows the refusal', () => {
  // /db and /markets already render `store.error`; /terminal did not.
  assert.match(source('../src/routes/db/+page.svelte'), /store\.error/);
  assert.match(source('../src/routes/markets/+page.svelte'), /store\.error/);
  const terminal = source('../src/routes/terminal/+page.svelte');
  assert.match(terminal, /\{#if store\.state === 'error'\}\s*<p class="tcensus" role="alert">/);
  assert.match(terminal, /\{store\.error \?\? /);
  // The backtest form reads the census directly, and names the refusal.
  assert.match(source('../src/routes/backtest/+page.svelte'), /\$\{censusFailure\(response\)\}/);
});
