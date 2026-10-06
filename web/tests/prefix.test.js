// THE FRONT END'S ONE O(1) CLAIM, DRIVEN.
//
// `index.svelte.js` opens with it: "every keystroke is ONE Map probe on a
// 1..4-character prefix bucket — not a scan of 800, and not a request." That is
// this product's rule 4 restated for the browser, and nothing measured it.
//
// It could not be measured, either. The module holding the logic imports
// `$lib/ask.js`, which node cannot resolve, so the claim was unreachable from a
// test runner even in principle. `$lib/prefix.js` is that logic extracted, for
// the same reason `fold.js` and `completeness.js` were.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { build, probe, MAX_PREFIX } from '../src/lib/prefix.js';

/**
 * A catalogue of `n` distinct symbols, shaped like the master's rows.
 *
 * @param {number} n
 * @returns {Array<{symbol: string, i: number}>}
 */
function universe(n) {
  const A = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ';
  /** @type {Array<{symbol: string, i: number}>} */
  const rows = [];
  for (let i = 0; i < n; i += 1) {
    const s =
      A[i % 26] + A[(i >> 5) % 26] + A[(i >> 10) % 26] + A[(i >> 15) % 26] + String(i);
    rows.push({ symbol: s, i });
  }
  return rows;
}

/* ── correctness ──────────────────────────────────────────────────────── */

test('an empty query is the whole universe, not none of it', () => {
  const rows = universe(50);
  assert.equal(probe(build(rows), rows, '').length, 50);
  assert.equal(probe(build(rows), rows, '   ').length, 50);
  assert.equal(probe(build(rows), rows, null).length, 50);
});

test('one to four characters is one bucket', () => {
  const rows = [{ symbol: 'NIFTY' }, { symbol: 'NIFTYNXT50' }, { symbol: 'BANKNIFTY' }];
  const ix = build(rows);
  assert.equal(probe(ix, rows, 'N').length, 2);
  assert.equal(probe(ix, rows, 'NI').length, 2);
  assert.equal(probe(ix, rows, 'NIFT').length, 2);
  assert.equal(probe(ix, rows, 'B').length, 1);
  assert.deepEqual(probe(ix, rows, 'Z'), [], 'a prefix nothing carries is empty, not everything');
});

test('beyond four characters it filters the four-character bucket', () => {
  const rows = [{ symbol: 'NIFTY' }, { symbol: 'NIFTYNXT50' }, { symbol: 'NIFTZ' }];
  const ix = build(rows);
  assert.equal(probe(ix, rows, 'NIFTY').length, 2);
  assert.equal(probe(ix, rows, 'NIFTYN').length, 1);
  assert.equal(probe(ix, rows, 'NIFTYNXT50').length, 1);
  assert.equal(probe(ix, rows, 'NIFTYZ').length, 0);
});

test('the query is case-folded and trimmed, because the operator types either', () => {
  const rows = [{ symbol: 'NIFTY' }];
  const ix = build(rows);
  for (const typed of ['nifty', ' NIFTY ', 'NiFtY', '  nifty']) {
    assert.equal(probe(ix, rows, typed).length, 1, `${JSON.stringify(typed)} should match`);
  }
});

test('a row whose symbol is not a string is skipped, not indexed under undefined', () => {
  // A master that served one is a store problem. Burying it in a bucket nobody
  // probes hides it; skipping leaves it visible in `rows` and out of the index.
  const rows = [{ symbol: 'NIFTY' }, { symbol: null }, { symbol: 42 }, {}];
  const ix = build(rows);
  assert.equal(probe(ix, rows, 'N').length, 1);
  assert.equal(probe(ix, rows, '').length, 4, 'the empty query still returns every row');
});

test('a symbol shorter than the prefix bound is still indexed to its own length', () => {
  const rows = [{ symbol: 'AB' }];
  const ix = build(rows);
  assert.equal(probe(ix, rows, 'A').length, 1);
  assert.equal(probe(ix, rows, 'AB').length, 1);
  assert.equal(probe(ix, rows, 'ABC').length, 0);
  assert.equal(MAX_PREFIX, 4);
});

/* ── the claim itself ─────────────────────────────────────────────────── */

/**
 * Nanoseconds for one call, taken as the best of several runs to shed noise.
 *
 * @param {number} reps
 * @param {() => unknown} op
 * @returns {number}
 */
function cost(reps, op) {
  let best = Infinity;
  for (let run = 0; run < 5; run += 1) {
    op(); // warm
    const t0 = process.hrtime.bigint();
    for (let i = 0; i < reps; i += 1) op();
    const ns = Number(process.hrtime.bigint() - t0) / reps;
    if (ns < best) best = ns;
  }
  return best;
}

test('a keystroke costs the same at 800 instruments and at 80,000', () => {
  // 800 is the bound `index.svelte.js` names; 100× it is the headroom. The
  // ceiling is 3.0×, the same one `crates/*/benches/ratio.rs` uses, and for the
  // same reason: below that a wall-clock ratio on a shared machine is noise.
  //
  // THE PREFIX IS `A`, WHOSE BUCKET GROWS WITH THE CATALOGUE. This probed
  // `ABCD`, which neither universe holds, so both timings measured a `Map`
  // miss and a probe that copied or scanned its bucket passed (P2-02-02,
  // D-1766). Here the bucket is about 31 rows against about
  // 3,077, so a copy or a scan is a hundredfold and cannot hide under 3.0×.
  const small = universe(800);
  const large = universe(80_000);
  const ixS = build(small);
  const ixL = build(large);
  assert.ok((ixL.get('A') ?? []).length > 50 * (ixS.get('A') ?? []).length);

  // The deterministic half: the bucket is returned BY REFERENCE, never copied.
  assert.equal(probe(ixL, large, 'A'), ixL.get('A'));
  assert.equal(probe(ixS, small, 'a'), ixS.get('A'));

  /** @param {Map<string, Array<any>>} ix @param {Array<any>} rows */
  const at = (ix, rows) => cost(20_000, () => probe(ix, rows, 'A'));
  const ratio = at(ixL, large) / at(ixS, small);

  assert.ok(
    ratio < 3,
    `probe cost grew ${ratio.toFixed(2)}× for a 100× catalogue — it is a Map probe ` +
      `returning a bucket by reference and must not scale with the catalogue`
  );
});

test('the one place cost is NOT constant is named, and it is bounded by a bucket', () => {
  // Beyond four characters `probe` filters the four-character bucket. The
  // module says so. This pins that the cost tracks the BUCKET and not the
  // catalogue: a hundredfold catalogue whose `ABCD` bucket stays the same size
  // must not make the filter a hundred times dearer. It used to assert only
  // that a fat bucket cost more than a missing one, which a full scan of the
  // catalogue also satisfied (P2-02-02, D-1766).
  /** @param {number} others */
  const catalogue = (others) => {
    /** @type {Array<{symbol: string}>} */
    const rows = [];
    for (let i = 0; i < 400; i += 1) rows.push({ symbol: `ABCD${i}` });
    for (let i = 0; i < others; i += 1) rows.push({ symbol: `ZY${i}` });
    return rows;
  };
  const small = catalogue(0);
  const large = catalogue(40_000);
  const ixS = build(small);
  const ixL = build(large);
  assert.equal(probe(ixL, large, 'ABCD1').length, 111); // ABCD1, ABCD10..19, ABCD100..199
  assert.deepEqual(probe(ixL, large, 'ABCD1'), probe(ixS, small, 'ABCD1'));

  const ratio =
    cost(2_000, () => probe(ixL, large, 'ABCD1')) / cost(2_000, () => probe(ixS, small, 'ABCD1'));
  assert.ok(
    ratio < 3,
    `the long-query filter grew ${ratio.toFixed(2)}× for a 100× catalogue with the ` +
      `same bucket — it must filter the four-character bucket, not the catalogue`
  );
});
