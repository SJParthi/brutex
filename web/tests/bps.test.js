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

import { basisPoints } from '../src/lib/bps.js';

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
        basisPoints(base, base + move) + basisPoints(base, base - move),
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
