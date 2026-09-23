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

import { group, rupee, LOC, exact, whole, oneDp } from '../src/lib/money.js';

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

test('integer quotient and remainder never invent or drop a paisa', () => {
  // This walks every hundredth across a decade and requires the printed cents
  // to match the integer's last two without floating-point division.
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

test('the complete safe-integer boundary retains its exact final paisa', () => {
  assert.equal(rupee(Number.MAX_SAFE_INTEGER), '9,00,71,99,25,47,409.91');
  assert.equal(rupee(Number.MIN_SAFE_INTEGER), '-9,00,71,99,25,47,409.91');
  assert.equal(rupee(Number.MAX_SAFE_INTEGER + 1), '—');
});

/* ── the em dash, which Intl does not supply ──────────────────────────── */

test('Intl renders a non-answer as though it were one, and these do not', () => {
  // This is the whole reason the three below exist. `/db` shipped the left-hand
  // column at 126 call sites while `/audit` guarded the same values: two pages,
  // one quantity, two different words for not knowing.
  assert.equal(new Intl.NumberFormat(LOC).format(NaN), 'NaN');
  assert.equal(new Intl.NumberFormat(LOC).format(Infinity), '∞');
  assert.equal(exact(NaN), '—');
  assert.equal(whole(NaN), '—');
  assert.equal(oneDp(NaN), '—');
});

test('every shape of "no number" is the same em dash', () => {
  for (const nothing of [NaN, Infinity, -Infinity]) {
    assert.equal(exact(nothing), '—', `exact(${nothing})`);
    assert.equal(whole(nothing), '—', `whole(${nothing})`);
    assert.equal(oneDp(nothing), '—', `oneDp(${nothing})`);
  }
});

test('ZERO IS A NUMBER AND IS NEVER THE DASH', () => {
  // The counterpart of the `?? 0` defect on /audit: a counted zero must not
  // become an unknown any more than an unknown may become a zero.
  assert.equal(exact(0), '0');
  assert.equal(whole(0), '0');
  assert.equal(oneDp(0), '0');
});

test('exact does NOT round, because a record count is already whole', () => {
  // Rounding here would hide a fractional value that should never have arrived
  // from the store at all.
  assert.equal(exact(1234), '1,234');
  assert.equal(exact(1234.7), '1,234.7');
});

test('whole rounds, because a fold or an average is not a count', () => {
  assert.equal(whole(1234.7), '1,235');
  assert.equal(whole(1234.2), '1,234');
  assert.equal(whole(-1234.7), '-1,235');
});

test('oneDp keeps a ratio that would lose its meaning as an integer', () => {
  assert.equal(oneDp(99.94), '99.9');
  assert.equal(oneDp(0.04), '0');
  assert.equal(oneDp(12345.67), '12,345.7', 'and it still groups');
});

test('all three group the Indian way, so no page disagrees with another', () => {
  assert.equal(exact(87828617), '8,78,28,617');
  assert.equal(whole(87828617.4), '8,78,28,617');
  assert.equal(oneDp(87828617), '8,78,28,617');
});
