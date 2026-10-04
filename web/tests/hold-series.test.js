// THE PERFORMANCE TAB'S HOLDING CHARTS — CE-70 and CE-73 (D-2730, D-2731).
//
// Drives `$lib/hold-series.js`, the module `routes/backtest/+page.svelte`
// draws from; the last test pins that the page still draws from it.

import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import assert from 'node:assert/strict';

import { holdPeriods, periodKey, returnHistogram } from '../src/lib/hold-series.js';

const DAY_S = 86_400;
// 2023-01-02 09:15 IST is 03:45 UTC.
const START = Date.UTC(2023, 0, 2, 3, 45) / 1000;

/** `n` weekday 1day bars, closes rising by `step` each, opens equal to the previous close. */
function weekdayBars(/** @type {number} */ n, /** @type {number} */ step) {
  const bars = [];
  let t = START;
  let c = 100_000;
  while (bars.length < n) {
    const dow = new Date((t + 19_800) * 1000).getUTCDay();
    if (dow !== 0 && dow !== 6) {
      bars.push({ t, o: c, c: c + step });
      c += step;
    }
    t += DAY_S;
  }
  return bars;
}

test('a daily bucket on a 1day series is that day\'s close-to-close move, not zero', () => {
  const bars = weekdayBars(10, 100);
  const periods = holdPeriods(bars, 'daily');
  assert.equal(periods.length, 10);
  for (const p of periods) assert.equal(p.v, 100, `${p.label} must carry the day's +100 paisa`);
});

test('the same day of two different years is two buckets, keyed with the year', () => {
  const bars = weekdayBars(1000, 100);
  const periods = holdPeriods(bars, 'daily');
  assert.equal(periods.length, 1000, 'one bucket per trading day across ~4 years');
  assert.equal(new Set(periods.map((p) => p.label)).size, 1000);
  assert.ok(periods.every((p) => /^\d{4}-\d{2}-\d{2}$/.test(p.label)), 'YYYY-MM-DD labels');
  assert.equal(Math.max(...periods.map((p) => p.v)), 100, 'no year-over-year value drawn as one day');
  const weeks = holdPeriods(bars, 'weekly');
  assert.equal(new Set(weeks.map((p) => p.label)).size, weeks.length);
  assert.ok(weeks.length > 52 * 3, 'weekly buckets are not folded across years either');
});

test('the gap between buckets is counted, so the buckets sum to the holding P&L', () => {
  // 60min bars, 7 per session, +10 per bar, +500 overnight gap.
  const bars = [];
  let c = 50_000;
  for (let day = 0; day < 5; day += 1) {
    if (day > 0) c += 500;
    for (let k = 0; k < 7; k += 1) {
      const t = START + day * DAY_S + k * 3600;
      bars.push({ t, o: c, c: c + 10 });
      c += 10;
    }
  }
  const periods = holdPeriods(bars, 'daily');
  assert.equal(periods.length, 5);
  assert.equal(periods[0].v, 70, 'the first bucket is measured from its first open');
  for (const p of periods.slice(1)) assert.equal(p.v, 570, `${p.label}: 7 x 10 plus the 500 gap`);
  const sum = periods.reduce((a, p) => a + p.v, 0);
  assert.equal(sum, bars[bars.length - 1].c - bars[0].o);
});

test('the weekly key is the IST Monday, with its year, across a year boundary', () => {
  // Thu 2026-12-31 and Fri 2027-01-01 in IST share the week of Mon 2026-12-28.
  const thu = Date.UTC(2026, 11, 31, 4, 0) / 1000;
  assert.equal(periodKey(thu, 'weekly'), '2026-12-28');
  assert.equal(periodKey(thu + DAY_S, 'weekly'), '2026-12-28');
  // 2026-12-31 20:00 UTC is already 2027-01-01 in IST.
  assert.equal(periodKey(Date.UTC(2026, 11, 31, 20, 0) / 1000, 'daily'), '2027-01-01');
});

test('a mirror-image gain and loss land in mirror-image bins (half away from zero)', () => {
  // A base of 32 paisa makes a one-paisa move exactly 312.5 bp.
  const h = returnHistogram([
    { o: 32, c: 33 },
    { o: 32, c: 31 }
  ]);
  assert.ok(h);
  assert.equal(h.hi, 313);
  assert.equal(h.lo, -313, 'Math.round would have made this -312');
  assert.equal(h.avgLoss, -313);
  assert.equal(h.avgGain, 313);
});

test('a sub-half-bp loss is a loser, never a -0 counted as flat', () => {
  // -1 paisa on 20,000 is exactly -0.5 bp. `Math.round(-0.5)` is `-0`, which
  // `r < 0` does not count; half away from zero makes it -1, a loser.
  const h = returnHistogram([
    { o: 20_000, c: 19_999 },
    { o: 20_000, c: 20_001 }
  ]);
  assert.ok(h);
  assert.equal(h.losers, 1, '-0.5 bp rounds away from zero to -1');
  assert.equal(h.winners, 1);
  assert.equal(h.flat, 0);
  assert.ok(!Object.is(h.lo, -0));
});

test('a bar whose scaled move overflows is left out and counted', () => {
  const h = returnHistogram([
    { o: 1, c: Number.MAX_SAFE_INTEGER },
    { o: 100, c: 101 },
    { o: 100, c: 99 }
  ]);
  assert.ok(h);
  assert.equal(h.overflowed, 1);
  assert.equal(h.total, 2);
});

test('the backtest page draws both charts from this module', () => {
  const page = readFileSync(new URL('../src/routes/backtest/+page.svelte', import.meta.url), 'utf8');
  assert.match(page, /from '\$lib\/hold-series\.js'/);
  assert.match(page, /const holdPeriods = \$derived\.by\(\(\) => holdPeriodsOf\(series\.bars, periodScale\)\)/);
  assert.match(page, /const returnHistogram = \$derived\(returnHistogramOf\(series\.bars\)\)/);
  assert.match(page, /h\.overflowed > 0/, 'a left-out bar is said on the chart');
  assert.doesNotMatch(page, /Math\.round\(\(\(b\.c - b\.o\) \/ b\.o\) \* 10_000\)/);
});
