// THE O(1) CLAIM FOR THE BROWSER'S PRICE LINE, ASSERTED AS A NUMBER.
//
// `docs/07-o1-architecture.md` states how a layer is proven: "A layer is not
// built because the code looks right. It is built when a test asserts the bound
// as a NUMBER." It also records what happens without one — layer 12 was `✓`
// with the words "a fixed row cap with paging, never O(universe)" while the
// fold behind the cap measured 3.569 ms at 2,787 instruments and 124.916 ms at
// 50,000, rendering exactly 200 rows both times. "The row cap is what hid it."
//
// The browser half of that layer had only a sentence and a hand measurement.
// This is the number.
//
// IT COUNTS READS RATHER THAN TIMING ANYTHING. A wall-clock assertion on a host
// running three `cli` test binaries at 430 % CPU — which is where every browser
// figure in that document was taken — measures the machine, not the algorithm.
// `sampleSeries` returns its own read count so the bound can be asserted
// exactly, on any machine, at any load.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { sampleSeries, sparkPath } from '../src/lib/spark.js';

/** @param {number} n @param {(i: number) => number} f */
const series = (n, f) => Array.from({ length: n }, (_, i) => ({ c: f(i) }));
/** @param {any} r */
const close = (r) => (typeof r?.c === 'number' ? r.c : null);

test('THE BOUND: reads are constant while the series grows 10,000-fold', () => {
  const TAKE = 240;
  // 100 is BELOW the sample count on purpose — the clamp is part of the bound,
  // and a function that read 240 times into a 100-element array would be
  // reading out of range 140 times rather than being fast.
  for (const n of [100, 1_000, 10_000, 100_000, 1_000_000]) {
    const s = sampleSeries(series(n, (i) => 1000 + (i % 97)), close, TAKE);
    assert.ok(s, `n=${n} produced no samples`);
    assert.equal(s.reads, Math.min(n, TAKE), `n=${n} read more than it promised`);
    assert.equal(s.of, n, 'the input length is reported for the caller to print');
  }
});

test('the drawn line runs earliest to latest even when the rows do not', () => {
  // A grid sorted newest-first hands its rows over in that order. Drawn as
  // given, a rising series reads as a falling one — every value right and the
  // picture wrong, which is the hardest kind of wrong to see.
  const rising = series(50, (i) => 100 + i);
  const asGiven = sampleSeries(rising, close, 10);
  const reversed = sampleSeries([...rising].reverse(), close, 10, true);
  assert.ok(asGiven && reversed);
  assert.equal(asGiven.up, true);
  assert.equal(reversed.up, true, 'newestFirst must not invert the direction');

  /* THE TWO DRAW THE SAME SHAPE, NOT THE SAME NUMBERS, and the first version of
     this test asserted the numbers and failed — correctly.
     Sampling at a stride of 5 takes indices 0,5,…,45. Ascending, that is
     100…145. Reversed, index 0 is the LAST element, so it is 149…104, and
     flipping it gives 104…149. Neither set is wrong: they are different
     samples of the same series, four apart, because the stride does not land
     on the same elements from the other end. Demanding equality would be
     demanding that sampling be symmetric, which index-sampling is not — and
     the fix for a failing assertion is not to make the code reverse-scan to
     satisfy it, which is the O(n) walk this whole module exists to avoid. What
     must hold is the DIRECTION and the MONOTONICITY. */
  for (const s of [asGiven, reversed]) {
    for (let i = 1; i < s.pts.length; i++) {
      assert.ok(s.pts[i].v > s.pts[i - 1].v, 'a rising series draws rising, from either end');
    }
  }
});

test('x is assigned after the order is final, not before', () => {
  // Assigning x during the read and then reversing leaves every point carrying
  // the x of its mirror: the values are right, the positions are the series
  // backwards. This pins the first and last x rather than trusting the order.
  const s = sampleSeries(series(20, (i) => i), close, 5, true);
  assert.ok(s);
  assert.equal(s.pts[0].x, 0);
  assert.equal(s.pts[s.pts.length - 1].x, 1);
  for (let i = 1; i < s.pts.length; i++) {
    assert.ok(s.pts[i].x > s.pts[i - 1].x, 'x must increase across the drawn width');
  }
});

test('a row carrying no number is skipped, never plotted as zero', () => {
  // A missing close is not a close of nothing. Plotted at the axis it draws a
  // crash that never happened, and it would drag `lo` to 0 with it.
  const rows = [{ c: 500 }, { c: null }, { c: 520 }, { c: undefined }, { c: 510 }];
  const s = sampleSeries(rows, close, 5);
  assert.ok(s);
  assert.equal(s.pts.length, 3, 'only the rows carrying a number are drawn');
  assert.equal(s.lo, 500);
  assert.equal(s.hi, 520);
  assert.equal(s.reads, 5, 'a skipped row still cost a read, and the count says so');
});

test('a flat series is flat, not a division by zero', () => {
  const s = sampleSeries(series(40, () => 2414245), close, 12);
  assert.ok(s);
  assert.equal(s.flat, true);
  assert.equal(s.lo, s.hi);
  const { line } = sparkPath(s, 1000, 100);
  assert.ok(!/NaN|Infinity/.test(line), 'a flat series must not emit NaN into the path');
  // Down the middle: every y is 50 in a 100-tall box.
  const ys = [...line.matchAll(/[ML][\d.]+ ([\d.]+)/g)].map((m) => Number(m[1]));
  assert.ok(
    ys.every((y) => y === 50),
    'a series with no spread is drawn on the centre line'
  );
});

test('fewer than two points is null, because one point is not a line', () => {
  assert.equal(sampleSeries([], close, 240), null);
  assert.equal(sampleSeries([{ c: 1 }], close, 240), null);
  assert.equal(sampleSeries(series(10, (i) => i), close, 1), null, 'take of 1 is not a line');
  assert.equal(
    sampleSeries([{ c: null }, { c: null }, { c: null }], close, 3),
    null,
    'three rows and no numbers is still no line'
  );
});

test('the path stays inside its box at both extremes', () => {
  const s = sampleSeries(series(200, (i) => (i % 2 ? 0 : 1_000_000)), close, 60);
  assert.ok(s);
  const { line, area } = sparkPath(s, 1000, 100, 4);
  const ys = [...line.matchAll(/[ML][\d.]+ ([\d.]+)/g)].map((m) => Number(m[1]));
  assert.ok(Math.min(...ys) >= 4, 'the padding keeps the high off the top edge');
  assert.ok(Math.max(...ys) <= 96, 'the padding keeps the low off the bottom edge');
  assert.ok(area.endsWith('Z'), 'the fill closes');
  assert.ok(area.startsWith(line), 'the fill is the line plus a floor, never a second shape');
});

test('an absurd take does not read past the end', () => {
  // `Fit` can ask for a page of 250 and a window can hold 12. The clamp is the
  // difference between a short read and 238 undefined lookups.
  const s = sampleSeries(series(12, (i) => i), close, 100_000);
  assert.ok(s);
  assert.equal(s.reads, 12);
  assert.equal(s.pts.length, 12);
});
