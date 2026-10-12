import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { RUN_SEGMENTS, facetsUrl, rungMonths } from '../src/lib/run-rungs.js';

test('a run learns its rungs from its underlying facets, never the whole census (DB-B58)', () => {
  assert.equal(facetsUrl('gdfl', 'INDEX', 'NIFTY'), '/census/facets.json?feed=gdfl&segment=INDEX&underlying=NIFTY');
  assert.deepEqual(RUN_SEGMENTS, ['INDEX', 'CASH']);
  const index = { months: [
    { month: '2024-01', rows: { '1s': 400000, '1min': 7000, '1day': 22 } },
    { month: '2024-02', rows: { '1min': 6800, '1day': 0 } }
  ] };
  const cash = { months: [] };
  const got = rungMonths([index, cash, null, { months: 'nonsense' }]);
  assert.deepEqual([...got.entries()].sort(), [['1day', 1], ['1min', 2], ['1s', 1]]);
});

test('the backtest page reads facets for its rungs and no longer downloads the census', () => {
  const page = readFileSync(new URL('../src/routes/backtest/+page.svelte', import.meta.url), 'utf8');
  const start = page.indexOf('async function loadRungs(');
  assert.ok(start > 0, 'loadRungs exists');
  const body = page.slice(start, page.indexOf('\n  }\n', start));
  assert.match(body, /facetsUrl\(/);
  assert.doesNotMatch(body, /ask_\(`\/store\.json/, 'no census download');
});
