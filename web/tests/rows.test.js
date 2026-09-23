// THE ONE CHECK BETWEEN `/store.json` AND EVERY NUMBER TWO PAGES PRINT.
//
// `rowFault` decides whether a census row can be read at all. Its own doc says
// why it exists — "a promise is not a proof" — and nothing was proving the
// prover: it lived in `store.svelte.js`, 864 lines importing `$lib/ask.js`, an
// alias node cannot resolve, so it was unreachable from `node --test`.
//
// The contract it carries is not "is this row valid". It is: a row that fails
// is KEPT, with the reason, and never dropped and never coerced. Dropping makes
// an unreadable month indistinguishable from a month that does not exist;
// coercing to zero prints a measurement nobody took. So every assertion here is
// about the REASON as much as the verdict.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { rowFault, isWholeNumber, MONTH_KEY } from '../src/lib/rows.js';

/** A row that passes, to vary one field at a time from. */
const ok = () => ({ instrument: 'NSE-NIFTY', month: '2026-08', timeframe: '1min', rows: 375 });

/**
 * The fault this row carries, asserting there IS one.
 *
 * `rowFault` returns `string | null` — correctly, since null is the whole point
 * — so a bare `assert.match(rowFault(row), …)` is a type error at every call
 * that expects a fault. This narrows once instead of at nine call sites.
 *
 * @param {any} row
 * @returns {string}
 */
function faultOf(row) {
  const why = rowFault(row);
  assert.ok(why !== null, 'expected this row to carry a fault');
  return why;
}

test('a readable row has no fault', () => {
  assert.equal(rowFault(ok()), null);
});

test('ZERO ROWS IS A READABLE ROW, because zero means zero', () => {
  // CLAUDE.md §7 draws this line for the open-interest sentinel and it holds
  // here: a month that stored nothing is a measurement, not an absence. If this
  // ever returns a fault, `/db` and `/ingest` start reporting a counted zero as
  // unknown, which is the opposite of the mistake the rest of the page guards.
  assert.equal(rowFault({ ...ok(), rows: 0 }), null);
});

test('a missing or empty instrument is named, with its value', () => {
  for (const bad of [undefined, null, '', 42, {}]) {
    const why = rowFault({ ...ok(), instrument: bad });
    assert.ok(why, `${JSON.stringify(bad)} should fault`);
    assert.match(why, /`instrument`/, 'the reason names the field');
    assert.match(why, /not an instrument key/);
  }
});

test('a month that is not YYYY-MM is named, with its value', () => {
  for (const bad of ['2026', '2026-8', '26-08', '2026-08-01', '', null, 202608]) {
    const why = rowFault({ ...ok(), month: bad });
    assert.ok(why, `${JSON.stringify(bad)} should fault`);
    assert.match(why, /`month`/);
    assert.match(why, /YYYY-MM/);
  }
  // and the shapes that DO pass
  for (const good of ['2026-08', '1999-01', '2026-12']) {
    assert.equal(rowFault({ ...ok(), month: good }), null, `${good} should pass`);
  }
});

test('an empty timeframe is named', () => {
  for (const bad of ['', null, undefined, 60]) {
    const why = rowFault({ ...ok(), timeframe: bad });
    assert.ok(why);
    assert.match(why, /`timeframe`/);
  }
});

test('a row count that is not a whole non-negative number is named', () => {
  // Each of these is a DIFFERENT way for a count to be unknown, and all four
  // must fault rather than round, truncate or clamp.
  for (const bad of [-1, 3.5, NaN, Infinity, '375', null, undefined]) {
    const why = rowFault({ ...ok(), rows: bad });
    assert.ok(why, `${JSON.stringify(bad)} should fault`);
    assert.match(why, /`rows`/);
    assert.match(why, /whole number of records/);
  }
});

test('the fault names the value, not just the field', () => {
  // "malformed row" sends a reader back to the wire to find out which of four
  // fields it meant. The reason has to carry the offending value.
  assert.match(faultOf({ ...ok(), rows: -7 }), /-7/);
  assert.match(faultOf({ ...ok(), month: 'Aug-2026' }), /"Aug-2026"/);
});

test('a null or undefined row faults on the first field rather than throwing', () => {
  for (const nothing of [null, undefined, {}]) {
    const why = rowFault(nothing);
    assert.ok(why, 'no row is a fault, not an exception');
    assert.match(why, /`instrument`/);
  }
});

test('the fields are checked in a fixed order, so one row yields one reason', () => {
  // A row broken four ways reports the FIRST fault. That is what makes the
  // reason stable enough to show an operator: the same broken row does not
  // describe itself differently on two renders.
  const broken = { instrument: '', month: 'x', timeframe: '', rows: -1 };
  assert.match(faultOf(broken), /`instrument`/);
  assert.equal(rowFault(broken), rowFault({ ...broken }), 'and it is deterministic');
});

/* ── the pieces it is built from ──────────────────────────────────────── */

test('isWholeNumber is Number.isInteger and nothing else', () => {
  for (const yes of [0, -3, 375, 2 ** 31]) assert.equal(isWholeNumber(yes), true);
  for (const no of [0.5, NaN, Infinity, '1', null, undefined, [], {}])
    assert.equal(isWholeNumber(no), false, `${JSON.stringify(no)}`);
});

test('MONTH_KEY is anchored at both ends', () => {
  // Unanchored, `2026-08-01` and `x2026-08` would both pass and a day key would
  // be read as a month.
  assert.ok(MONTH_KEY.test('2026-08'));
  assert.ok(!MONTH_KEY.test('2026-08-01'));
  assert.ok(!MONTH_KEY.test('x2026-08'));
  assert.ok(!MONTH_KEY.test('2026-8'));
});
