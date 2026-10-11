// WP-61 (DB-041, DB-080): /markets reads a month file whole only while it is
// small enough to; past the bound it reads the newest bars, and says so.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { MONTH_READ_MAX, TAIL_PAGE, TAIL_PAGES, foldBars, tailCut, tailPlan } from '../src/lib/markets-tail.js';

test('a one-minute month is read whole; a one-second month is bounded, by size and not by name', () => {
  const minute = tailPlan([{ month: '2026-08', rows: 8_250 }, { month: '2026-09', rows: 8_625 }]);
  assert.deepEqual([minute.bounded, minute.from, minute.to, minute.pages, minute.biggest], [false, '2026-08', '2026-09', 0, 8_625]);
  const second = tailPlan([{ month: '2026-09', rows: 382_496 }]);
  assert.deepEqual([second.bounded, second.from, second.to, second.pages], [true, '2026-09', '2026-09', TAIL_PAGES]);
  assert.match(second.why, /3,82,496 bars/);
  assert.match(second.why, new RegExp(`newest\\s+${(TAIL_PAGE * TAIL_PAGES).toLocaleString('en-IN')} bars at most`));
  // The exact edge: the bound itself is still read whole.
  assert.equal(tailPlan([{ month: '2026-09', rows: MONTH_READ_MAX }]).bounded, false);
  assert.equal(tailPlan([{ month: '2026-09', rows: MONTH_READ_MAX + 1 }]).bounded, true);
});

test('empty, unordered, duplicate and malformed month lists', () => {
  assert.deepEqual(tailPlan([]), { bounded: false, from: '', to: '', pages: 0, biggest: 0, why: '' });
  assert.equal(tailPlan(/** @type {any} */ (null)).bounded, false);
  // ONE BIG MONTH AMONG SMALL ONES BOUNDS THE WHOLE READ, and the span is the
  // months' own whatever order they arrive in.
  const mixed = tailPlan([{ month: '2026-09', rows: 10 }, { month: '2026-07', rows: 400_000 }, { month: '2026-08', rows: 10 }, { month: '2026-08', rows: 10 }]);
  assert.deepEqual([mixed.bounded, mixed.from, mixed.to], [true, '2026-07', '2026-09']);
  assert.match(mixed.why, /2026-07 to 2026-09/);
  // A count that is absent, negative, fractional or text is not a size: it is
  // never read as a big month, and a row with no month is dropped.
  const bad = tailPlan(/** @type {any} */ ([
    { month: '2026-09', rows: -5 }, { month: '2026-09', rows: 1.5 }, { month: '2026-09', rows: '999999' },
    { month: '2026-09' }, { month: 'September', rows: 999_999 }, null
  ]));
  assert.deepEqual([bad.bounded, bad.biggest, bad.from, bad.to], [false, 0, '2026-09', '2026-09']);
});

test('the page reads through the plan and names what it drew', () => {
  const page = readFileSync(new URL('../src/routes/markets/+page.svelte', import.meta.url), 'utf8');
  assert.match(page, /tailPlan\(monthsAtBase\.filter\(/);
  assert.match(page, /if \(plan\.bounded\) \{\s*const got = await readTail\(row, plan, feed, rung, ticket\.signal\)/);
  assert.match(page, /readWindowPage\(q\.toString\(\), TAIL_PAGE, signal\)/);
  assert.match(page, /data-markets-tail=/);
});

// ---------------------------------------------------------------------------
// WP-61 repair: a bounded read is cut on a bucket edge of the drawn rung. The
// newest N bars end at an arbitrary second, and the fragment of the oldest
// bucket was folded and drawn as a whole candle. Every price is invented.
// ---------------------------------------------------------------------------
const IST = 19_800;
// 14:05:00 IST on an invented day: a 5m, 15m-off and 1h-off bucket edge.
const EDGE = Date.UTC(2031, 0, 6, 8, 35, 0) / 1000;
/** 900 one-second bars over three 5m buckets; the first 100 seconds trade far lower. */
const SERIES = Array.from({ length: 900 }, (_, i) => {
  const p = 700_000 + (i < 100 ? i - 5_000 : i);
  return { t: EDGE + i, o: p, h: p + 3, l: p - 3, c: p + 1, v: 2 };
});

test('a tail that starts mid-bucket does not draw the fragment as a candle', () => {
  const truth = foldBars(SERIES, 300, IST);
  assert.deepEqual(truth.map((b) => b.t), [EDGE, EDGE + 300, EDGE + 600]);
  assert.deepEqual([truth[0].o, truth[0].l, truth[0].v], [695_000, 694_997, 600]);
  // THE NEWEST 800 BARS: the read starts 100 seconds into the first candle.
  const tail = SERIES.slice(100);
  // The defect, stated: folding the fragment gives another open and another low.
  const fragment = foldBars(tail, 300, IST)[0];
  assert.deepEqual([fragment.t, fragment.o, fragment.l, fragment.v], [EDGE, 700_100, 700_097, 400]);
  const cut = tailCut(tail, 300, IST, true);
  assert.deepEqual(cut.cut, { bucket: EDGE, until: EDGE + 300, dropped: 200, first: EDGE + 100, last: EDGE + 299 });
  assert.equal(cut.bars.length, 600);
  // WHAT IS DRAWN IS EXACTLY THE WHOLE CANDLES OF THE FULL SERIES.
  assert.deepEqual(foldBars(cut.bars, 300, IST), truth.slice(1));
});

test('the cut is on IST bucket edges, per drawn rung', () => {
  const tail = SERIES.slice(100);
  // 15m buckets on IST break at 14:00 and 14:15: the whole 900-second series is
  // 14:05..14:20, so its first 15m candle (14:00) is a fragment by 600 bars
  // and its second (14:15) is whole from its first second.
  const m15 = tailCut(tail, 900, IST, true);
  assert.deepEqual(m15.cut, { bucket: EDGE - 300, until: EDGE + 600, dropped: 500, first: EDGE + 100, last: EDGE + 599 });
  assert.deepEqual(foldBars(m15.bars, 900, IST).map((b) => b.t), [EDGE + 600]);
  // A RUNG LONGER THAN THE WHOLE READ: nothing whole is left, and the cut says so.
  const day = tailCut(tail, 86_400, IST, true);
  assert.equal(day.bars.length, 0);
  assert.equal(day.cut?.dropped, 800);
  assert.deepEqual(foldBars(day.bars, 86_400, IST), []);
});

test('nothing is cut when the oldest bucket is whole, or when nothing older exists', () => {
  // The read starts on the candle's first second: nothing older can belong to it.
  const onEdge = tailCut(SERIES.slice(300), 300, IST, true);
  assert.equal(onEdge.cut, null);
  assert.equal(onEdge.bars.length, 600);
  // The series simply BEGINS mid-bucket (a listing, a first import): the whole
  // series was read, so its first candle is the series' own and is drawn.
  const whole = tailCut(SERIES.slice(100), 300, IST, false);
  assert.equal(whole.cut, null);
  assert.equal(foldBars(whole.bars, 300, IST)[0].o, 700_100);
});

test('empty, single, duplicate, out-of-order and garbage tails', () => {
  assert.deepEqual(tailCut([], 300, IST, true), { bars: [], cut: null });
  assert.deepEqual(tailCut(/** @type {any} */ (null), 300, IST, true), { bars: [], cut: null });
  // No rung, a zero rung, a negative rung: nothing is bucketed, so nothing is cut or folded.
  for (const seconds of [null, 0, -5, Number.NaN]) {
    assert.equal(tailCut(SERIES.slice(100), /** @type {any} */ (seconds), IST, true).cut, null);
    assert.deepEqual(foldBars(SERIES, /** @type {any} */ (seconds), IST), []);
  }
  // ONE BAR, mid-bucket, with older bars unread: it is a fragment.
  const one = tailCut([SERIES[150]], 300, IST, true);
  assert.deepEqual([one.bars.length, one.cut?.dropped, one.cut?.first, one.cut?.last], [0, 1, EDGE + 150, EDGE + 150]);
  // ONE BAR on the edge: a whole (one-bar) candle.
  assert.equal(tailCut([SERIES[300]], 300, IST, true).cut, null);
  // OUT OF ORDER: the oldest bar is found, not assumed to be first.
  const shuffled = [...SERIES.slice(100)].reverse();
  const cut = tailCut(shuffled, 300, IST, true);
  assert.deepEqual([cut.cut?.dropped, cut.cut?.first, cut.bars.length], [200, EDGE + 100, 600]);
  // A REPEATED OLDEST BAR (a page re-read while the store grew) is dropped with its bucket.
  const repeated = tailCut([SERIES[100], ...SERIES.slice(100)], 300, IST, true);
  assert.deepEqual([repeated.cut?.dropped, repeated.bars.length], [201, 600]);
  // A bar with no time is not the oldest bar and is never dropped as one.
  const junk = tailCut(/** @type {any} */ ([{ t: Number.NaN }, { t: 'x' }, ...SERIES.slice(300)]), 300, IST, true);
  assert.equal(junk.cut, null);
});

test('the page draws a bounded read through the cut and says what it left out', () => {
  const page = readFileSync(new URL('../src/routes/markets/+page.svelte', import.meta.url), 'utf8');
  assert.match(page, /tailCut\(rawBars, tfSeconds, IST_OFFSET, tailRead\.drawn < tailRead\.total\)/);
  assert.match(page, /foldBars\(tailDrawn\.bars, tfSeconds, IST_OFFSET\)/);
  assert.doesNotMatch(page, /aggregate\(rawBars/);
  assert.match(page, /data-markets-tail-cut=\{cut\.dropped\}/);
  // A read with no whole candle of the rung is a named reason, never an empty chart.
  assert.match(page, /tailDrawn\.cut && tailDrawn\.bars\.length === 0[\s\S]{0,120}kind: 'tailshort'/);
});
