// WHAT A STORE KEY IS MADE OF, DRIVEN.
//
// `/db` offered the store's raw keys as instruments. Measured against a live
// store: 27 keys, ONE underlying. The parser this drives is what lets the
// Instrument rung offer BANKNIFTY once instead of BANKNIFTY twenty-seven ways.
//
// The two cases worth pinning hardest are the two the old code got wrong: the
// contract TAIL has to come off before the underlying is taken, and the
// underlying may itself contain hyphens.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  parseKey,
  groupByUnderlying,
  segmentOf,
  strikeExact,
  FUTURE_TAIL,
  OPTION_TAIL
} from '../src/lib/instrument.js';

/**
 * The bucket for one underlying, asserted present.
 *
 * `Map.get` is `T | undefined` and every call below has just put the key in,
 * so this says "present" once rather than a cast at each site.
 *
 * @param {Map<string, ReturnType<typeof parseKey>[]>} m
 * @param {string} k
 * @returns {ReturnType<typeof parseKey>[]}
 */
function bucket(m, k) {
  const b = m.get(k);
  assert.ok(b, `expected a bucket for ${k}`);
  return b;
}

/* ── the three shapes ─────────────────────────────────────────────────── */

test('a spot key is exchange, segment and symbol', () => {
  const p = parseKey('NSE-INDEX-BANKNIFTY');
  assert.equal(p.exchange, 'NSE');
  assert.equal(p.segment, 'INDEX');
  assert.equal(p.underlying, 'BANKNIFTY');
  assert.equal(p.kind, 'spot');
  assert.equal(p.expiry, null);
  assert.equal(p.strike, null);
  assert.equal(p.side, null);
  assert.equal(p.why, null);
});

test('an option key gives up its expiry, strike and side', () => {
  const p = parseKey('NSE-FNO-BANKNIFTY-2026-07-28-4810000-PE');
  assert.equal(p.underlying, 'BANKNIFTY', 'THE POINT: not the whole key');
  assert.equal(p.segment, 'FNO');
  assert.equal(p.kind, 'option');
  assert.equal(p.expiry, '2026-07-28');
  assert.equal(p.strike, 4810000, 'paisa, as an integer');
  assert.equal(p.side, 'PE');
});

test('a future key gives up its expiry and no strike', () => {
  const p = parseKey('NSE-FNO-NIFTY-2026-07-28-FUT');
  assert.equal(p.underlying, 'NIFTY');
  assert.equal(p.kind, 'future');
  assert.equal(p.expiry, '2026-07-28');
  assert.equal(p.strike, null);
  assert.equal(p.side, null);
});

/* ── the two the old code got wrong ───────────────────────────────────── */

test('THE REGRESSION: the tail comes off BEFORE the underlying is taken', () => {
  // `parts.slice(2).join('-')` is right for spot and wrong for every contract.
  // It yields the whole tail as part of the name, which is how one instrument
  // came to be offered twenty-seven ways.
  const key = 'NSE-FNO-BANKNIFTY-2026-07-28-4810000-PE';
  assert.equal(key.split('-').slice(2).join('-'), 'BANKNIFTY-2026-07-28-4810000-PE', 'the old way');
  assert.equal(parseKey(key).underlying, 'BANKNIFTY', 'the right way');
});

test('an underlying may contain hyphens, and BAJAJ-AUTO is why', () => {
  assert.equal(parseKey('NSE-CASH-BAJAJ-AUTO').underlying, 'BAJAJ-AUTO');
  assert.equal(parseKey('NSE-FNO-BAJAJ-AUTO-2026-07-28-9000000-CE').underlying, 'BAJAJ-AUTO');
  assert.equal(parseKey('NSE-FNO-BAJAJ-AUTO-2026-07-28-FUT').underlying, 'BAJAJ-AUTO');
  // A third-token parser would answer BAJAJ to all three.
  assert.notEqual(parseKey('NSE-CASH-BAJAJ-AUTO').underlying, 'BAJAJ');
});

test('twenty-seven keys over one underlying group to one entry', () => {
  const keys = ['NSE-INDEX-BANKNIFTY'];
  for (let i = 0; i < 13; i += 1) {
    keys.push(`NSE-FNO-BANKNIFTY-2026-07-28-${4000000 + i * 10000}-CE`);
    keys.push(`NSE-FNO-BANKNIFTY-2026-07-28-${4000000 + i * 10000}-PE`);
  }
  assert.equal(keys.length, 27, 'the measured shape of the live store');
  const { byUnderlying, unreadable } = groupByUnderlying(keys);
  assert.equal(byUnderlying.size, 1, 'ONE instrument, not twenty-seven');
  assert.deepEqual([...byUnderlying.keys()], ['BANKNIFTY']);
  assert.equal(bucket(byUnderlying, 'BANKNIFTY').length, 27);
  assert.equal(unreadable.length, 0);
});

/* ── the segment rung ─────────────────────────────────────────────────── */

test('the rung comes from the TAIL, because FNO covers both contracts', () => {
  // The key's own segment token cannot separate a future from an option; only
  // the tail can, and that is the whole reason `kind` exists.
  assert.equal(segmentOf(parseKey('NSE-INDEX-BANKNIFTY')), 'spot');
  assert.equal(segmentOf(parseKey('NSE-CASH-TCS')), 'spot');
  assert.equal(segmentOf(parseKey('NSE-FNO-BANKNIFTY-2026-07-28-FUT')), 'futures');
  assert.equal(segmentOf(parseKey('NSE-FNO-BANKNIFTY-2026-07-28-4810000-CE')), 'options');
  assert.equal(segmentOf(parseKey('rubbish')), null);
});

test('one underlying can hold several rungs, and that is the cascade', () => {
  const { byUnderlying } = groupByUnderlying([
    'NSE-INDEX-BANKNIFTY',
    'NSE-FNO-BANKNIFTY-2026-07-28-4810000-CE',
    'NSE-FNO-BANKNIFTY-2026-07-28-FUT'
  ]);
  const rungs = new Set(bucket(byUnderlying, 'BANKNIFTY').map(segmentOf));
  assert.deepEqual([...rungs].sort(), ['futures', 'options', 'spot']);
});

/* ── a key that cannot be read is named, not dropped ──────────────────── */

test('an unreadable key is NAMED and kept, never silently dropped', () => {
  // Dropping is how a held instrument becomes invisible; filing it under an
  // invented symbol is worse.
  for (const bad of ['', '   ', 'BANKNIFTY', 'NSE-INDEX', 'NSE-INDEX-', null, undefined, 42]) {
    const p = parseKey(/** @type {any} */ (bad));
    assert.equal(p.underlying, null, `${JSON.stringify(bad)} should not parse`);
    assert.ok(p.why, 'and it should say why');
  }
});

test('groupByUnderlying keeps the unreadable ones to one side', () => {
  const { byUnderlying, unreadable } = groupByUnderlying([
    'NSE-INDEX-BANKNIFTY',
    'NSE-INDEX',
    'nonsense'
  ]);
  assert.equal(byUnderlying.size, 1);
  assert.equal(unreadable.length, 2);
  assert.ok(unreadable.every((p) => p.why));
});

test('parseKey trims, and tolerates no keys at all', () => {
  assert.equal(parseKey('  NSE-INDEX-BANKNIFTY  ').underlying, 'BANKNIFTY');
  const { byUnderlying, unreadable } = groupByUnderlying([]);
  assert.equal(byUnderlying.size, 0);
  assert.equal(unreadable.length, 0);
  assert.equal(groupByUnderlying(/** @type {any} */ (null)).byUnderlying.size, 0);
});

/* ── money stays an integer ───────────────────────────────────────────── */

test('a strike renders from paisa with integer arithmetic only', () => {
  assert.equal(strikeExact(4810000), '48100');
  assert.equal(strikeExact(0), '0');
  assert.equal(strikeExact(12345), '123.45');
  assert.equal(strikeExact(100), '1');
  assert.equal(strikeExact(-12345), '-123.45', 'the sign survives');
  assert.equal(strikeExact(null), '—');
  assert.equal(strikeExact(/** @type {any} */ (1.5)), '—', 'a fractional paisa is not a strike');
});

/* ── the patterns themselves ──────────────────────────────────────────── */

test('the tail patterns are anchored, so a symbol cannot be mistaken for one', () => {
  assert.ok(OPTION_TAIL.test('-2026-07-28-4810000-CE'));
  assert.ok(FUTURE_TAIL.test('-2026-07-28-FUT'));
  // An instrument that merely CONTAINS something tail-shaped, not at the end.
  assert.equal(parseKey('NSE-CASH-FUT').underlying, 'FUT', 'a symbol called FUT is a symbol');
  assert.equal(parseKey('NSE-CASH-FUT').kind, 'spot');
});
