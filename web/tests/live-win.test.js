// p14num-1: THE LIVE PANEL'S "WON %" WAS THE SHARE OF UP MOVES.
//
// `edge_wins` is `Edge::wins`, strictly positive forward moves on either side.
// A sell's positive moves are its losers, so the ideal short (`wins == 0`)
// printed "0% won" under "sell". The panel now states a win rate for a long row
// only, and names why it has none for a sell.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { liveWinShare } from '../src/lib/live-win.js';

test('a sell row is never given its up-move share as a win rate', () => {
  // outcome.rs `the_perfect_short_and_the_perfect_long_are_worth_the_same`.
  const perfectShort = liveWinShare({ direction: 'short', n: 50, edge_wins: 0 });
  assert.equal(perfectShort.known, false);
  assert.ok(!perfectShort.known && /sell wins on a down move/.test(perfectShort.why));
  // outcome.rs "excellent short": 10 of 100 moves up is not "10% won".
  assert.equal(liveWinShare({ direction: 'short', n: 100, edge_wins: 10 }).known, false);
  assert.equal(liveWinShare({ direction: 'sideways', n: 4, edge_wins: 1 }).known, false);
  assert.equal(liveWinShare(undefined).known, false);
  assert.deepEqual(liveWinShare({ direction: 'long', n: 100, edge_wins: 90 }), { known: true, wins: 90, n: 100, share: 0.9 });
  assert.deepEqual(liveWinShare({ direction: 'long', n: 0, edge_wins: 0 }), { known: true, wins: 0, n: 0, share: 0 });
});

test('the wire field is the side-blind up-move count, and the page reads it only through liveWinShare', () => {
  const live = readFileSync(new URL('../../crates/api/src/livejson.rs', import.meta.url), 'utf8');
  assert.match(live, /"edge_wins":\{\}/);
  const frontier = readFileSync(new URL('../../crates/cli/src/frontier.rs', import.meta.url), 'utf8');
  assert.match(frontier, /wins: scored\.edge\.wins,/);
  const page = readFileSync(new URL('../src/routes/backtest/+page.svelte', import.meta.url), 'utf8');
  assert.match(page, /\{@const leadWin = liveWinShare\(lead\)\}/);
  assert.match(page, /\{@const win = liveWinShare\(row\)\}/);
  assert.doesNotMatch(page, /edge_wins\s*\//, 'no template divides the up-move count into a win rate');
  assert.doesNotMatch(page, /Math\.round\(\(wins \/ n\) \* 100\)/, 'the row fact is not the up-move share');
});
