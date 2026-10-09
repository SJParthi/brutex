// THE PRESETS SPANNED 1.48 MONTHS PER "MONTH" (D-3514).
//
// The backtest chart's 1M/3M/6M/1Y presets multiplied 555 minutes by 21
// sessions. 555 is the minute of the 09:15 open past IST midnight, not the
// length of a session; a regular session is 375 one-minute bars
// (`pull::session::BARS_PER_REGULAR_SESSION`, 09:15 to 15:29). The bar counts
// below are the fold's own per-session counts, `pull/tests/anchor.rs`.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { barsPerMonth, SESSION_MINUTES } from '../src/lib/presets.js';

test('a session is 375 minutes, not the 555-minute open anchor', () => {
  assert.equal(SESSION_MINUTES, 375);
});

test('a month is 21 sessions of the bars one session folds into', () => {
  for (const [secs, perSession] of [
    [60, 375],
    [120, 188],
    [300, 75],
    [600, 38],
    [900, 25],
    [1800, 13],
    [3600, 7],
  ]) {
    assert.equal(barsPerMonth(secs), perSession * 21, `${secs}s`);
  }
  assert.equal(barsPerMonth(86_400), 21, 'a daily bar is one per session');
  assert.equal(barsPerMonth(7 * 86_400), 21, 'and wider rungs are capped at daily');
});

test('a rung with no length has no month', () => {
  for (const secs of [0, -60, NaN, undefined]) {
    assert.equal(barsPerMonth(secs), 0, String(secs));
  }
});
