// THE CENSUS SEARCH, DRIVEN.
//
// `$lib/find.js` claims one `Map.get` per keystroke against an index built over
// DISTINCT SYMBOLS rather than over census rows. That is CLAUDE.md §3 rule 4
// restated for the browser, the same claim `prefix.test.js` drives for the
// typeahead, and the same reason this half of the logic lives in a module with
// no runes in it: the page that uses it imports `$lib/ask.js`, node cannot
// resolve that alias, and a claim that cannot be driven is a claim nobody is
// checking.
//
// WHAT IS CLAIMED HERE IS NARROWER THAN "CONSTANT", and deliberately so.
// Fetching the bucket is O(1). Materialising the matched rows is O(matched),
// which is not constant and cannot be: returning m rows costs m. The tests
// below pin the part that IS constant and name the part that is not, rather
// than reporting a flattering ratio taken on a query that happens to match
// nothing.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { build, probe, symbolCount, MAX_PREFIX } from '../src/lib/find.js';
import { MAX_PREFIX as PREFIX_BOUND } from '../src/lib/prefix.js';

/** @typedef {{sym: string, seg: string, tf: string}} Row */

/**
 * Census rows as `/ingest` derives them: one row per (symbol × segment × rung),
 * which is why the row count and the symbol count are different numbers.
 *
 * @param {string[]} symbols
 * @param {number} [segs]
 * @param {number} [rungs]
 * @returns {Row[]}
 */
function census(symbols, segs = 1, rungs = 1) {
  /** @type {Row[]} */
  const rows = [];
  for (const sym of symbols) {
    for (let g = 0; g < segs; g += 1) {
      for (let t = 0; t < rungs; t += 1) rows.push({ sym, seg: `S${g}`, tf: `T${t}` });
    }
  }
  return rows;
}

/** @param {Row} r */
const sym = (r) => r.sym;

/**
 * `n` distinct symbols, each long enough to carry several infixes.
 *
 * @param {number} n
 * @returns {string[]}
 */
function universe(n) {
  const A = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ';
  /** @type {string[]} */
  const out = [];
  for (let i = 0; i < n; i += 1) {
    out.push(`${A[i % 26]}${A[(i / 26) % 26 | 0]}${A[(i / 676) % 26 | 0]}X${i}`);
  }
  return out;
}

/* ── what it finds ────────────────────────────────────────────────────── */

test('an empty query is every row, and the same array it was handed', () => {
  const rows = census(['NIFTY', 'BANKNIFTY']);
  const ix = build(rows, sym);
  // BY REFERENCE, not a copy: an unfiltered table must cost nothing at all.
  assert.equal(probe(ix, rows, ''), rows);
  assert.equal(probe(ix, rows, '   '), rows);
  assert.equal(probe(ix, rows, null), rows);
  assert.equal(probe(ix, rows, undefined), rows);
});

test('a symbol is found by a fragment in its MIDDLE, which is the whole point', () => {
  // The four names an operator means by "show me the banks". Not one of them
  // begins with BANK, so `prefix.js` answers this question with BANKNIFTY only.
  const rows = census(['AXISBANK', 'HDFCBANK', 'ICICIBANK', 'KOTAKBANK', 'BANKNIFTY', 'TCS']);
  const ix = build(rows, sym);
  const hit = probe(ix, rows, 'BANK').map(sym).sort();
  assert.deepEqual(hit, ['AXISBANK', 'BANKNIFTY', 'HDFCBANK', 'ICICIBANK', 'KOTAKBANK']);
  assert.equal(probe(ix, rows, 'TCS').length, 1);
});

test('a prefix and a suffix are both just infixes', () => {
  const rows = census(['RELIANCE']);
  const ix = build(rows, sym);
  for (const q of ['R', 'RE', 'RELI', 'RELIANCE', 'ANCE', 'CE', 'E', 'LIAN']) {
    assert.equal(probe(ix, rows, q).length, 1, `${q} should match RELIANCE`);
  }
  assert.deepEqual(probe(ix, rows, 'RELIANCEX'), [], 'a fragment nothing carries is empty');
  assert.deepEqual(probe(ix, rows, 'ZZ'), []);
});

test('a hyphen inside a symbol is a character, not a separator', () => {
  // BAJAJ-AUTO is a real NSE trading symbol and it is the case a token index
  // gets wrong: splitting on non-alphanumerics makes `BAJAJ-AU` unmatchable.
  const rows = census(['BAJAJ-AUTO', 'BAJFINANCE']);
  const ix = build(rows, sym);
  assert.equal(probe(ix, rows, 'BAJAJ-AUTO').length, 1);
  assert.equal(probe(ix, rows, 'BAJAJ-AU').length, 1);
  assert.equal(probe(ix, rows, 'J-A').length, 1);
  assert.equal(probe(ix, rows, 'BAJ').length, 2, 'both names carry BAJ');
});

test('case is folded on BOTH sides, not just the query', () => {
  // `prefix.js` upper-cases the query and indexes the symbol raw, so it can
  // only match an already-upper-case master. This folds the index too.
  const rows = census(['Nifty', 'axisbank']);
  const ix = build(rows, sym);
  for (const q of ['NIFTY', 'nifty', ' NiFtY ', 'IFT']) {
    assert.equal(probe(ix, rows, q).length, 1, `${JSON.stringify(q)} should match Nifty`);
  }
  assert.equal(probe(ix, rows, 'BANK').length, 1, 'a lower-case symbol is still findable');
});

test('every row carrying a matched symbol comes back, not just the first', () => {
  // A name is drawn once per segment per rung. Finding the name must find all
  // of its rows or the census would silently hide rungs.
  const rows = census(['TCS', 'INFY'], 3, 6);
  const ix = build(rows, sym);
  assert.equal(rows.length, 36);
  assert.equal(probe(ix, rows, 'TCS').length, 18);
  assert.equal(symbolCount(ix), 2, 'the index is over symbols, not over rows');
});

test('the result is grouped by symbol, which is what makes it one probe', () => {
  const rows = census(['AAA', 'BAA'], 1, 2);
  const ix = build(rows, sym);
  // Interleaved in `rows`; grouped on the way out. The module documents this
  // and every caller in the tree sorts before drawing.
  assert.deepEqual(
    probe(ix, rows, 'AA').map(sym),
    ['AAA', 'AAA', 'BAA', 'BAA']
  );
});

test('a row whose symbol is not a string is skipped, not indexed under undefined', () => {
  const rows = /** @type {Row[]} */ (
    /** @type {unknown} */ ([{ sym: 'NIFTY' }, { sym: null }, { sym: 42 }, { sym: '' }, {}])
  );
  const ix = build(rows, sym);
  assert.equal(probe(ix, rows, 'N').length, 1);
  assert.equal(symbolCount(ix), 1, 'the empty string is not a symbol either');
  assert.equal(probe(ix, rows, '').length, 5, 'the empty query still returns every row');
});

test('build tolerates no rows at all rather than throwing at a caller', () => {
  const ix = build([], sym);
  assert.equal(symbolCount(ix), 0);
  assert.deepEqual(probe(ix, [], 'X'), []);
  assert.deepEqual(probe(ix, [], ''), []);
});

test('the bound is the one prefix.js already publishes, not a second copy', () => {
  // `twofrontends.test.js` pins prefix.js against typeahead.js for exactly this
  // reason. A third depth nobody can see from any of the three is the defect.
  assert.equal(MAX_PREFIX, PREFIX_BOUND);
  assert.equal(MAX_PREFIX, 4);
});

test('a symbol shorter than the bound is indexed to its own length', () => {
  const rows = census(['AB']);
  const ix = build(rows, sym);
  assert.equal(probe(ix, rows, 'A').length, 1);
  assert.equal(probe(ix, rows, 'B').length, 1);
  assert.equal(probe(ix, rows, 'AB').length, 1);
  assert.equal(probe(ix, rows, 'ABC').length, 0);
});

/* ── what it costs ────────────────────────────────────────────────────── */

/**
 * Nanoseconds for one call, best of several runs to shed noise. The same
 * shape `prefix.test.js` uses, so the two numbers are comparable.
 *
 * @param {number} reps
 * @param {() => unknown} op
 * @returns {number}
 */
function cost(reps, op) {
  let best = Infinity;
  for (let run = 0; run < 5; run += 1) {
    op();
    const t0 = process.hrtime.bigint();
    for (let i = 0; i < reps; i += 1) op();
    const ns = Number(process.hrtime.bigint() - t0) / reps;
    if (ns < best) best = ns;
  }
  return best;
}

test('a miss costs the same at 750 symbols and at 75,000', () => {
  // THE PURE PROBE. No rows are materialised, so what is measured is the
  // `Map.get` alone — the part of this module that is genuinely O(1).
  // The ceiling is 3.0×, the figure `prefix.test.js` and
  // `crates/*/benches/ratio.rs` both use, and for the same reason: below that a
  // wall-clock ratio on a shared machine is noise.
  const small = census(universe(750));
  const large = census(universe(75_000));
  const ixS = build(small, sym);
  const ixL = build(large, sym);
  assert.equal(probe(ixS, small, 'QQQQ').length, 0);
  assert.equal(probe(ixL, large, 'QQQQ').length, 0);

  const ratio =
    cost(20_000, () => probe(ixL, large, 'QQQQ')) / cost(20_000, () => probe(ixS, small, 'QQQQ'));
  assert.ok(
    ratio < 3,
    `a missing fragment cost ${ratio.toFixed(2)}× more against a 100× catalogue — ` +
      `it is one Map.get and must not scale with the census`
  );
});

test('the census can grow without the keystroke growing, at a fixed symbol count', () => {
  // 750 names at one rung against the same 750 names at six rungs and three
  // segments: 750 rows against 13,500. The index is built over SYMBOLS, so the
  // probe that finds nothing must not notice.
  const thin = census(universe(750));
  const fat = census(universe(750), 3, 6);
  assert.equal(fat.length, 13_500);
  const ixT = build(thin, sym);
  const ixF = build(fat, sym);
  assert.equal(symbolCount(ixT), symbolCount(ixF), 'same names, eighteen times the rows');

  const ratio = cost(20_000, () => probe(ixF, fat, 'QQQQ')) / cost(20_000, () => probe(ixT, thin, 'QQQQ'));
  assert.ok(
    ratio < 3,
    `eighteen times the rows made the probe ${ratio.toFixed(2)}× dearer — the index ` +
      `is over distinct symbols and must not track the row count`
  );
});

test('the one non-constant path is named, and it is bounded by the bucket', () => {
  // Past MAX_PREFIX the probe filters the bucket it fetched. This pins that the
  // cost tracks the BUCKET and not the catalogue: two indexes of the same size,
  // one with every symbol crowded into a single gram bucket and one with them
  // spread, must not cost the same.
  /** @type {string[]} */
  const crowded = [];
  /** @type {string[]} */
  const spread = [];
  for (let i = 0; i < 20_000; i += 1) {
    crowded.push(`ABCD${i}`);
    spread.push(`${String.fromCharCode(65 + (i % 26))}${String.fromCharCode(65 + ((i / 26) % 26 | 0))}CD${i}`);
  }
  const rc = census(crowded);
  const rs = census(spread);
  const ixC = build(rc, sym);
  const ixS = build(rs, sym);

  const crowdedCost = cost(2_000, () => probe(ixC, rc, 'ABCDZ'));
  const spreadCost = cost(2_000, () => probe(ixS, rs, 'ABCDZ'));
  assert.equal(probe(ixC, rc, 'ABCDZ').length, 0);
  assert.ok(
    crowdedCost > spreadCost,
    `the long-query filter should track the bucket it filters (crowded ${crowdedCost.toFixed(0)}ns ` +
      `vs spread ${spreadCost.toFixed(0)}ns) — that is the documented non-constant path`
  );
});

test('materialising matched rows is linear in the MATCHES, and that is inherent', () => {
  // The honest half of the claim. Returning m rows costs m; no index changes
  // that. What this pins is that it costs m and NOT the census — the same
  // symbols at eighteen times the rows return eighteen times the rows for
  // eighteen-ish times the cost, not more.
  const names = universe(750);
  const thin = census(names);
  const fat = census(names, 3, 6);
  const ixT = build(thin, sym);
  const ixF = build(fat, sym);
  const hitT = probe(ixT, thin, 'X').length;
  const hitF = probe(ixF, fat, 'X').length;
  assert.ok(hitT > 0 && hitF === hitT * 18, `expected ${hitT * 18} rows, got ${hitF}`);

  const ratio = cost(2_000, () => probe(ixF, fat, 'X')) / cost(2_000, () => probe(ixT, thin, 'X'));
  assert.ok(
    ratio < 18 * 3,
    `returning 18× the rows cost ${ratio.toFixed(1)}× — linear in the matches is ` +
      `expected and unavoidable; superlinear is not`
  );
});
