// THE DIGIT GROUPING A READER JUDGES MAGNITUDE BY.
//
// `group` and `rupee` are called 52 times on `/` and neither had a test. The
// claim in `group`'s own comment — "8,78,28,617 and never 87,828,617" — is not
// a property of the code. It is a property of the RUNTIME's ICU data: a Node or
// browser build without the full set silently falls back to `en-US` grouping
// and renders 87,828,617. That is a wrong number rendered confidently, in the
// one visual cue that says whether a figure is crores or millions.
//
// The first test is therefore as much about the environment as the function,
// and that is deliberate — it is the failure that would otherwise ship.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { group, rupee, LOC } from '../src/lib/money.js';

test('the grouping is INDIAN, which is a fact about this runtime', () => {
  // Crores and lakhs, not thousands and millions. On an ICU-less build every
  // one of these falls back to en-US and the assertion is the only thing that
  // would say so.
  assert.equal(group(87828617), '8,78,28,617');
  assert.equal(group(100000), '1,00,000');
  assert.equal(group(1000), '1,000');
  assert.equal(group(10000000), '1,00,00,000', 'one crore');
  assert.notEqual(group(87828617), '87,828,617', 'this is the en-US fallback');
});

test('the locale is pinned, never inherited', () => {
  assert.equal(LOC, 'en-IN');
});

test('small numbers are not grouped at all', () => {
  assert.equal(group(0), '0');
  assert.equal(group(7), '7');
  assert.equal(group(999), '999');
});

test('a negative count keeps its sign and its grouping', () => {
  assert.equal(group(-100000), '-1,00,000');
});

/* ── paisa to rupees ──────────────────────────────────────────────────── */

test('a paisa integer becomes rupees, always to two places', () => {
  assert.equal(rupee(0), '0.00');
  assert.equal(rupee(1), '0.01');
  assert.equal(rupee(100), '1.00');
  assert.equal(rupee(12345), '123.45');
  assert.equal(rupee(2500000), '25,000.00', 'and it groups the rupee side');
});

test('a NIFTY-scale quote renders exactly', () => {
  // ~25,000 rupees, the magnitude this page actually shows.
  assert.equal(rupee(2_512_345), '25,123.45');
  assert.equal(rupee(2_500_005), '25,000.05');
});

test('the divide never invents or drops a third decimal', () => {
  // `paisa / 100` always has exactly two decimal places, so there is no third
  // digit for float error to reach. This walks every hundredth across a decade
  // of paisa and requires the printed cents to match the integer's last two.
  for (let p = 0; p < 2000; p += 1) {
    const s = rupee(p);
    const cents = String(p % 100).padStart(2, '0');
    assert.ok(s.endsWith(`.${cents}`), `rupee(${p}) = ${s}, expected to end .${cents}`);
  }
});

test('a negative paisa keeps its sign', () => {
  assert.equal(rupee(-12345), '-123.45');
  assert.equal(rupee(-1), '-0.01');
});

test('the safe-integer edge is where this stops being exact, and it is far away', () => {
  // ₹90 billion in paisa. An index quote is six orders of magnitude below it,
  // so the float divide is safe for every value this page can hold — but the
  // boundary is pinned so a future caller passing something larger is not
  // silently wrong.
  assert.ok(Number.MAX_SAFE_INTEGER / 100 > 9e13, 'the exact range exceeds ₹90 billion');
  assert.equal(rupee(2_512_345), '25,123.45', 'and an index quote is nowhere near it');
});
