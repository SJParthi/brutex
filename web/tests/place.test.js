// WHICH MASTER A RUN'S BAR PATH IS RESOLVED FROM.
//
// The defect, in its own shape: `/backtest` loads ONE instrument catalogue, for
// `feeds.active`, and builds its bar request with `run.feed`. Those are the
// same value until "Show every feed" is switched on — and then an operator can
// open a run belonging to another vendor, whose exchange and segment were just
// read out of the active vendor's master and whose request goes to its own.
//
// Nothing drove this rule before: it was three lines inside
// `routes/backtest/+page.svelte`, and `node --test` cannot import a `.svelte`
// file. `$lib/pick.js` was carved out of `feeds.svelte.js` for the same reason
// and records it in its own header. This is that carve, and this is the driver.
//
// It is asserted WITHOUT A SWEEP on purpose. The runtime path needs a populated
// results ledger, which needs a real brute-force run; the rule does not, and a
// rule that can only be checked by producing data is a rule nobody checks.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { placeIn } from '../src/lib/place.js';

/** Dhan's master, as `/instruments.json` sends it. */
const DHAN = [
  { symbol: 'NIFTY', exchange: 'NSE', segment: 'INDEX' },
  { symbol: 'RELIANCE', exchange: 'NSE', segment: 'CASH' }
];

test('THE REGRESSION: a run from another feed is refused, not answered', () => {
  // The catalogue on hand is Dhan's; the run is Zerodha's. Before the fix this
  // returned NSE/INDEX — Dhan's answer, used to build a Zerodha request.
  assert.equal(placeIn(DHAN, 'dhan', 'NIFTY', 'zerodha'), null);
});

test('the same feed still resolves', () => {
  assert.deepEqual(placeIn(DHAN, 'dhan', 'NIFTY', 'dhan'), {
    exchange: 'NSE',
    segment: 'INDEX'
  });
});

test('no second feed in play is not a mismatch', () => {
  // `forFeed` omitted means the caller has nothing to compare against, which
  // is not the same as comparing and disagreeing.
  assert.deepEqual(placeIn(DHAN, 'dhan', 'RELIANCE'), { exchange: 'NSE', segment: 'CASH' });
});

test('a symbol the master does not list is null, feed match or not', () => {
  assert.equal(placeIn(DHAN, 'dhan', 'BANKNIFTY', 'dhan'), null);
});

test('an unloaded catalogue refuses rather than throwing', () => {
  assert.equal(placeIn([], null, 'NIFTY', 'dhan'), null);
  assert.equal(placeIn(undefined, null, 'NIFTY', 'dhan'), null);
});

test('a half-known row is not a place', () => {
  // `{exchange: 'NSE', segment: undefined}` would build `.../NSE/undefined/X`
  // and ask the bar route for it — a request made of a value nobody supplied.
  assert.equal(placeIn([{ symbol: 'X', exchange: 'NSE' }], 'dhan', 'X', 'dhan'), null);
  assert.equal(placeIn([{ symbol: 'X', segment: 'CASH' }], 'dhan', 'X', 'dhan'), null);
});

test('the empty string is not a feed, so it cannot mismatch', () => {
  // `activeFeed` is `feeds.active ?? ''` on that page, and an empty `forFeed`
  // must read as "none supplied" rather than as a feed that disagrees with
  // every other one.
  assert.deepEqual(placeIn(DHAN, 'dhan', 'NIFTY', ''), { exchange: 'NSE', segment: 'INDEX' });
});
