// THE ROUNDING DIVERGENCE BETWEEN THE BROWSER AND THE ENGINE.
//
// `node --test web/tests/` — the runner is node's own, so this costs no
// dependency and no toolchain. It drives the same module
// `routes/db/+page.svelte` imports; nothing here is a copy of the page's
// arithmetic.
//
// The first three cases are the engine's own test, transcribed:
// `crates/api/src/server.rs::basis_points_round_half_away_from_zero_in_both_directions`.
// A base of 32 paisa makes a one-paisa move 10,000/32 = 312.5 basis points
// exactly, so nothing here depends on a rounding accident. `Math.round` — what
// the page used to call — returns 313 and -312, and fails the second case
// alone.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { basisPoints, bpsText, dirOf } from '../src/lib/bps.js';

/**
 * `basisPoints` that is known to have computed.
 *
 * It returns `number | null` — correctly, since `null` is the overflow verdict
 * — so every assertion expecting a number is a type error without this. Narrows
 * once instead of at each call.
 *
 * @param {number} base
 * @param {number} later
 * @returns {number}
 */
function bpsOf(base, later) {
  const bps = basisPoints(base, later);
  assert.ok(bps !== null, `basisPoints(${base}, ${later}) unexpectedly overflowed`);
  return bps;
}

test('a half rounds away from zero in BOTH directions', () => {
  assert.equal(basisPoints(32, 33), 313, '312.5 bp exactly, rounded away from zero');
  assert.equal(basisPoints(32, 31), -313, '-312.5 bp exactly, rounded away from zero — NOT -312');
  assert.equal(basisPoints(32, 34), 625, 'and a move that divides exactly is not nudged');
});

test('a gain and its mirror-image loss have equal magnitude', () => {
  // The consequence the engine's comment names: this is what a sorted column
  // exposes. `Math.round` breaks it at every exact half.
  for (const base of [3, 7, 32, 101, 1024, 65_536]) {
    for (const move of [1, 2, 5, 13]) {
      // Stated as a sum rather than a negation: when the rounded result is
      // zero, `-0` and `0` are different values under strict equality, and
      // that difference is this test's artefact rather than the page's.
      assert.equal(
        bpsOf(base, base + move) + bpsOf(base, base - move),
        0,
        `base ${base}, move ${move}`
      );
    }
  }
});

test('a real loss is never rendered as flat', () => {
  // `Math.round(-0.5)` is `-0`, which `bpsText` prints as an unsigned `0.00%`
  // and `dirOf` colours `flat`. Half a basis point down must stay negative.
  const bps = basisPoints(20_000, 19_999);
  assert.equal(bps, -1, 'exactly -0.5 bp rounds away from zero, not to -0');
  assert.ok(!Object.is(bps, -0), 'and is not negative zero');
});

test('zero move is a real zero, not an unknown', () => {
  assert.equal(basisPoints(500, 500), 0);
  assert.ok(Object.is(basisPoints(500, 500), 0), 'positive zero, so `dirOf` reads flat');
});

test('a scale that leaves the safe integers is refused, not rounded', () => {
  // The engine returns `Unknown::Overflow` rather than wrapping. Past 2^53 a
  // JavaScript number stops representing consecutive integers, so the browser
  // refuses at the same step for the f64 reason.
  assert.equal(basisPoints(1, Number.MAX_SAFE_INTEGER), null);
  assert.equal(basisPoints(2, -Number.MAX_SAFE_INTEGER), null);
  // And the boundary itself still computes rather than being refused early.
  assert.equal(basisPoints(10_000, 10_001), 1, 'an ordinary move is unaffected');
});

/* ── how a ratio reads, and what colour it earns ──────────────────────── */

test('a ratio reads as a signed percentage, built from the integer', () => {
  assert.equal(bpsText(125), '+1.25%');
  assert.equal(bpsText(-125), '-1.25%');
  assert.equal(bpsText(10_000), '+100.00%');
  assert.equal(bpsText(-10_000), '-100.00%');
});

test('the fraction is padded, so a column of them aligns', () => {
  assert.equal(bpsText(5), '+0.05%');
  assert.equal(bpsText(-5), '-0.05%');
  assert.equal(bpsText(105), '+1.05%');
  assert.equal(bpsText(150), '+1.50%');
});

test('zero carries no sign, because it has no direction', () => {
  assert.equal(bpsText(0), '0.00%');
  assert.equal(dirOf(0), 'flat');
});

test('`none` is its own word and never `flat`', () => {
  // A cell nobody can compute and a month that did not move are different
  // facts. One is an absence with a reason beside it; the other is a
  // measurement. They must not share a colour.
  assert.equal(dirOf(null), 'none');
  assert.equal(dirOf(undefined), 'none');
  assert.notEqual(dirOf(null), dirOf(0));
});

test('direction is the sign and nothing else', () => {
  assert.equal(dirOf(1), 'up');
  assert.equal(dirOf(10_000), 'up');
  assert.equal(dirOf(-1), 'down');
  assert.equal(dirOf(-10_000), 'down');
});

test('THE -0 TRAP: the formatter cannot see it, so the producer must not make it', () => {
  // `Math.abs(-0)` is `0` and `-0 < 0` is false, so a negative that lost its
  // magnitude renders as an unsigned `0.00%` and colours `flat`. That is
  // exactly what shipped: `/db` rounded with `Math.round`, `Math.round(-0.5)`
  // is `-0`, and a real loss printed as a month that did not move.
  assert.equal(bpsText(-0), '0.00%', 'the formatter genuinely cannot tell');
  assert.equal(dirOf(-0), 'flat');

  // THE FIX IS THE COUPLING, NOT THE FORMATTER. `basisPoints` rounds away from
  // zero in both directions, so a negative can never arrive with zero
  // magnitude. This walks every base and move that could round to zero and
  // requires that none of them yields `-0`.
  for (let base = 1; base <= 400; base += 1) {
    for (const move of [-1, 1]) {
      const bps = bpsOf(base, base + move);
      assert.ok(
        !Object.is(bps, -0),
        `basisPoints(${base}, ${base + move}) produced -0, which renders as flat`
      );
    }
  }
});

test('a loss too small to round still reads as a loss', () => {
  // The end-to-end statement of the bug: a one-paisa fall on a large base is
  // the smallest loss this page can show, and it must not be flat.
  const bps = bpsOf(20_000, 19_999);
  assert.equal(bpsText(bps), '-0.01%');
  assert.equal(dirOf(bps), 'down');
});
