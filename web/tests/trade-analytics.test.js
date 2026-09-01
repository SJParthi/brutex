import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  TIME_GRAINS,
  equityMaxDrawdown,
  istHourLabel,
  periodLabel,
  streakSeries,
  timePatternRows,
  validateTradePayload,
  windowSeries
} from '../src/lib/trade-analytics.js';

/** @param {number} key @param {number} [trades] @param {number} [worstWins] @param {Record<string, number>} [overrides] */
const bucket = (key, trades = 4, worstWins = 3, overrides = {}) => ({
  key,
  trades,
  wins: worstWins,
  worst_wins: worstWins,
  best_paisa: 200,
  worst_paisa: 100,
  largest_win: 75,
  largest_loss: -25,
  ...overrides
});

const PERIOD_KEYS = Object.freeze({
  hour: 9,
  weekday: 0,
  day: 19_737,
  week: 2_820,
  month: 2024 * 12,
  quarter: 2024 * 4,
  half: 2024 * 2,
  year: 2024
});

const validTradePayload = () => {
  const trades = [
    {
      seq: 0,
      direction: 'short',
      signal_bar: 10,
      entry_bar: 11,
      exit_bar: 13,
      best: 10,
      worst: 5,
      bars_held: 2,
      entry_micros: 1_705_290_600_000_000,
      exit_micros: 1_705_290_720_000_000,
      adverse_ppm: 2_500,
      adverse_paisa: 25,
      favourable_ppm: 12_500,
      favourable_paisa: 125
    },
    {
      seq: 1,
      direction: 'short',
      signal_bar: 20,
      entry_bar: 21,
      exit_bar: 24,
      best: -1,
      worst: -2,
      bars_held: 3,
      entry_micros: 1_705_290_900_000_000,
      exit_micros: 1_705_291_080_000_000,
      adverse_ppm: 3_000,
      adverse_paisa: 30,
      favourable_ppm: 9_000,
      favourable_paisa: 90
    }
  ];
  const periods = Object.fromEntries(
    TIME_GRAINS.map(({ key }) => [
      key,
      [
        bucket(PERIOD_KEYS[key], 2, 1, {
          wins: 1,
          best_paisa: 9,
          worst_paisa: 3,
          largest_win: 5,
          largest_loss: -2
        })
      ]
    ])
  );
  return {
    identity: 'cd'.repeat(32),
    policy: 'chosen-grid-v1',
    direction: 'short',
    trades,
    periods,
    count: trades.length,
    refusal: null
  };
};

test('all eight Rust period grains remain available to the page', () => {
  assert.deepEqual(
    TIME_GRAINS.map((grain) => grain.key),
    ['hour', 'weekday', 'day', 'week', 'month', 'quarter', 'half', 'year']
  );
  const periods = Object.fromEntries(TIME_GRAINS.map(({ key }, index) => [key, [bucket(index)]]));
  const result = timePatternRows(periods);
  assert.ok(result);
  for (const { key } of TIME_GRAINS) assert.ok(result.grains[key].length > 0, key);
});

test('integer period keys receive exact, honest labels', () => {
  assert.equal(istHourLabel(4), '04:00–05:00');
  assert.equal(istHourLabel(23), '23:00–00:00');
  assert.equal(periodLabel('weekday', 0), 'Mon');
  assert.equal(periodLabel('day', 0), '1970-01-01');
  assert.equal(periodLabel('week', 0), 'Mon 1969-12-29');
  assert.equal(periodLabel('month', 2024 * 12 + 11), 'Dec 2024');
  assert.equal(periodLabel('quarter', 2025 * 4 + 2), 'Q3 2025');
  assert.equal(periodLabel('half', 2025 * 2 + 1), 'H2 2025');
  assert.equal(periodLabel('year', 2026), '2026');
});

test('hour, weekday and best-month folds retain empty slots and aggregate across years', () => {
  const result = timePatternRows({
    hour: [bucket(4, 10, 6)],
    weekday: [bucket(4, 10, 6)],
    month: [bucket(2024 * 12 + 7, 4, 3), bucket(2025 * 12 + 7, 6, 4)]
  });
  assert.ok(result);
  assert.equal(result.grains.hour.length, 24);
  assert.equal(result.grains.hour[4].label, '04:00–05:00');
  assert.equal(result.grains.hour[4].trades, 10);
  assert.equal(result.grains.hour[5].trades, 0);
  assert.equal(result.grains.weekday.length, 7);
  const friday = result.grains.weekday.find((/** @type {any} */ row) => row.label === 'Fri');
  assert.ok(friday);
  assert.equal(friday.trades, 10);
  assert.ok(result.bestCalendarMonth);
  assert.equal(result.bestCalendarMonth.label, 'Aug');
  assert.equal(result.bestCalendarMonth.trades, 10);
  assert.equal(result.total, 10);
  assert.equal(result.floor, 1);
});

test('streaks are ordered by seq and preserve every run for count and amount charts', () => {
  const result = streakSeries([
    { seq: 5, worst: 7 },
    { seq: 2, worst: 10 },
    { seq: 3, worst: 20 },
    { seq: 4, worst: -5 },
    { seq: 6, worst: -3 }
  ]);
  assert.deepEqual(result.ordered.map((row) => row.seq), [2, 3, 4, 5, 6]);
  assert.deepEqual(result.streaks, [
    { index: 1, kind: 'win', count: 2, amount: 30, startSeq: 2, endSeq: 3 },
    { index: 2, kind: 'loss', count: 1, amount: -5, startSeq: 4, endSeq: 4 },
    { index: 3, kind: 'win', count: 1, amount: 7, startSeq: 5, endSeq: 5 },
    { index: 4, kind: 'loss', count: 1, amount: -3, startSeq: 6, endSeq: 6 }
  ]);
  assert.equal(result.longestWin, 2);
  assert.equal(result.longestLoss, 1);
  assert.equal(result.avgWinStreak, 1.5);
  assert.equal(result.avgLossStreak, 1);
});

test('maximum drawdown includes the loss from zero before the first trade', () => {
  // Worst-case trade results [-100, +50] produce cumulative equity [-100, -50].
  // Seeding the peak from the first point falsely reported zero; capital began
  // at zero, so the authoritative drawdown is 100 paisa.
  assert.equal(equityMaxDrawdown([-100, -50]), 100);
  assert.equal(equityMaxDrawdown([100, 40, 150, 20]), 130);
  assert.equal(equityMaxDrawdown([]), null);
});

test('flat trades break wins and extend losing streaks exactly like the Rust grid', () => {
  const result = streakSeries([
    { seq: 1, worst: 10 },
    { seq: 2, worst: 0 },
    { seq: 3, worst: -2 },
    { seq: 4, worst: 5 }
  ]);
  assert.deepEqual(result.streaks.map(({ kind, count, amount }) => ({ kind, count, amount })), [
    { kind: 'win', count: 1, amount: 10 },
    { kind: 'loss', count: 2, amount: -2 },
    { kind: 'win', count: 1, amount: 5 }
  ]);
  assert.equal(result.longestLoss, 2);
});

test('empty and malformed inputs stay empty rather than inventing a run', () => {
  assert.deepEqual(streakSeries(undefined).streaks, []);
  assert.equal(timePatternRows(null), null);
  assert.equal(timePatternRows({}), null);
});

test('dense histories admit only a fixed window to the DOM and clamp every page edge', () => {
  const rows = Array.from({ length: 101 }, (unused, index) => index);
  assert.deepEqual(windowSeries(rows, 0, 24), {
    rows: rows.slice(0, 24),
    page: 0,
    pages: 5,
    start: 0,
    total: 101
  });
  assert.deepEqual(windowSeries(rows, 99, 24), {
    rows: rows.slice(96),
    page: 4,
    pages: 5,
    start: 96,
    total: 101
  });
  assert.equal(windowSeries(rows, -10, 24).page, 0);
  assert.equal(windowSeries(rows, Number.NaN, 24).page, 0);
  assert.equal(windowSeries(rows, 0, 0).rows.length, 1);
  assert.deepEqual(windowSeries(undefined, 100, 24), {
    rows: [],
    page: 0,
    pages: 1,
    start: 0,
    total: 0
  });
});

test('a complete safe trade payload admits every row and every Rust period grain', () => {
  const payload = validTradePayload();
  const result = validateTradePayload(payload);
  assert.equal(result.ok, true, result.why);
  assert.equal(result.empty, false);
  assert.equal(result.rows, payload.trades);
  assert.equal(result.periods, payload.periods);
});

test('UTC Sunday evening is categorized as Monday and the whole IST hour', () => {
  const payload = validTradePayload();
  // 2024-01-14 20:00 UTC == 2024-01-15 01:30 IST. Raw-UTC bucketing calls
  // this Sunday/hour 20; the NSE exchange calendar calls it Monday/hour 1.
  const sundayUtcMondayIst = 1_705_262_400_000_000;
  payload.trades[0].entry_micros = sundayUtcMondayIst;
  payload.trades[0].exit_micros = sundayUtcMondayIst + 120_000_000;
  payload.trades[1].entry_micros = sundayUtcMondayIst + 300_000_000;
  payload.trades[1].exit_micros = sundayUtcMondayIst + 480_000_000;
  payload.periods.hour[0].key = 1;

  const result = validateTradePayload(payload);
  assert.equal(result.ok, true, result.why);
  assert.equal(periodLabel('hour', payload.periods.hour[0].key), '01:00–02:00');
  assert.equal(periodLabel('weekday', payload.periods.weekday[0].key), 'Mon');
  assert.equal(periodLabel('week', payload.periods.week[0].key), 'Mon 2024-01-15');
});

test('legitimate empty trade states stay distinct from malformed partial analytics', () => {
  const absent = validateTradePayload({
    identity: 'cd'.repeat(32),
    policy: null,
    direction: null,
    trades: [],
    count: 0,
    refusal: 'the historical trade file is absent',
    periods: Object.fromEntries(TIME_GRAINS.map(({ key }) => [key, []]))
  });
  assert.equal(absent.ok, true, absent.why);
  assert.equal(absent.empty, true);
  assert.ok(absent.periods);
  assert.equal(absent.policy, null);
  assert.equal(absent.direction, null);

  const existing = validateTradePayload({
    identity: 'cd'.repeat(32),
    policy: 'chosen-grid-v1',
    direction: 'long',
    trades: [],
    count: 0,
    refusal: null,
    periods: Object.fromEntries(TIME_GRAINS.map(({ key }) => [key, []]))
  });
  assert.equal(existing.ok, true, existing.why);
  assert.equal(existing.empty, true);
  assert.ok(existing.periods);

  const partial = validateTradePayload({
    identity: 'cd'.repeat(32),
    policy: 'chosen-grid-v1',
    direction: 'long',
    trades: [],
    count: 0,
    refusal: null
  });
  assert.equal(partial.ok, false);
  assert.match(partial.why, /omitted the eight period/);

  const prefix = /** @type {any} */ (validTradePayload());
  prefix.refusal = 'row two failed its seal';
  assert.match(validateTradePayload(prefix).why, /partial or refused/);

  const extraGrain = /** @type {any} */ (validTradePayload());
  extraGrain.periods.decade = [];
  assert.match(validateTradePayload(extraGrain).why, /exactly the eight versioned grains/);
});

test('one unsafe or malformed trade integer refuses the whole payload', () => {
  /** @type {[string, number][]} */
  const cases = [
    ['seq', Number.MAX_SAFE_INTEGER + 1],
    ['signal_bar', -1],
    ['bars_held', 2.5],
    ['worst', Number.MIN_SAFE_INTEGER - 1],
    ['entry_micros', Number.POSITIVE_INFINITY],
    ['adverse_ppm', Number.MAX_SAFE_INTEGER + 1],
    ['adverse_paisa', -1],
    ['favourable_ppm', 2.5],
    ['favourable_paisa', Number.POSITIVE_INFINITY]
  ];
  for (const [field, value] of cases) {
    const payload = validTradePayload();
    /** @type {Record<string, unknown>} */ (
      /** @type {unknown} */ (payload.trades[0])
    )[field] = value;
    const result = validateTradePayload(payload);
    assert.equal(result.ok, false, field);
    assert.deepEqual(result.rows, [], `${field} must expose no plausible prefix`);
  }

  const mismatch = validTradePayload();
  mismatch.count = 1;
  assert.match(validateTradePayload(mismatch).why, /count says 1/);

  const wrongDuration = validTradePayload();
  wrongDuration.trades[0].bars_held = 1;
  assert.match(validateTradePayload(wrongDuration).why, /does not exactly equal/);
});

test('chosen-grid policy and direction are canonical and every row agrees with the receipt', () => {
  const policy = validTradePayload();
  policy.policy = 'unstopped-v2';
  assert.match(validateTradePayload(policy).why, /chosen-grid-v1/);

  const direction = validTradePayload();
  direction.direction = 'up';
  assert.match(validateTradePayload(direction).why, /direction `long` or `short`/);

  const row = validTradePayload();
  row.trades[1].direction = 'long';
  const refused = validateTradePayload(row);
  assert.equal(refused.ok, false);
  assert.match(refused.why, /trades\[1\]\.direction/);
  assert.deepEqual(refused.rows, []);
});

test('wire order, exact signal source, and one-position-at-a-time occupancy are mandatory', () => {
  const reversed = validTradePayload();
  reversed.trades.reverse();
  assert.match(validateTradePayload(reversed).why, /canonical wire sequence 0/);

  const lateSource = validTradePayload();
  lateSource.trades[0].signal_bar = 8;
  assert.match(validateTradePayload(lateSource).why, /fill-sourced signal or an exact next-bar/);

  const overlap = validTradePayload();
  overlap.trades[1].entry_bar = overlap.trades[0].exit_bar;
  overlap.trades[1].signal_bar = overlap.trades[1].entry_bar;
  overlap.trades[1].bars_held = overlap.trades[1].exit_bar - overlap.trades[1].entry_bar;
  assert.match(validateTradePayload(overlap).why, /prior one-position-at-a-time trade/);
});

test('chosen rows reconcile to the immutable committed ledger aggregate', () => {
  const payload = validTradePayload();
  /** @type {Record<string, any>} */
  const run = {
    identity: payload.identity,
    trades: 2,
    pessimistic: 3,
    optimistic: 9,
    worst_trade: -2,
    max_drawdown: 2
  };
  assert.equal(validateTradePayload(payload, run).ok, true);
  for (const field of ['trades', 'pessimistic', 'optimistic', 'worst_trade', 'max_drawdown']) {
    const wrong = { ...run, [field]: run[field] + 1 };
    const result = validateTradePayload(payload, wrong);
    assert.equal(result.ok, false, field);
    assert.match(result.why, /do not reconcile exactly/);
  }
  const wrongIdentity = { ...run, identity: 'ef'.repeat(32) };
  assert.match(validateTradePayload(payload, wrongIdentity).why, /answered for run/);
});

test('safe endpoints whose fold would leave the exact range are refused before analytics', () => {
  const payload = validTradePayload();
  payload.trades[0].best = Number.MAX_SAFE_INTEGER;
  payload.trades[0].worst = Number.MAX_SAFE_INTEGER;
  payload.trades[1].best = 1;
  payload.trades[1].worst = 1;
  const result = validateTradePayload(payload);
  assert.equal(result.ok, false);
  assert.match(result.why, /accumulation is not exactly representable/);
  assert.deepEqual(result.rows, []);
});

test('all eight period arrays and all integer bucket facts are mandatory and checked', () => {
  const missing = validTradePayload();
  delete missing.periods.quarter;
  assert.match(validateTradePayload(missing).why, /exactly the eight versioned grains/);

  const unsafe = validTradePayload();
  unsafe.periods.hour[0].largest_win = Number.MAX_SAFE_INTEGER + 1;
  const refused = validateTradePayload(unsafe);
  assert.equal(refused.ok, false);
  assert.match(refused.why, /periods\.hour\[0\]\.largest_win/);

  const partial = validTradePayload();
  partial.periods.year[0].trades = 1;
  assert.match(validateTradePayload(partial).why, /periods\.year\[0\]\.trades is 1, not 2/);

  const roundedMoney = validTradePayload();
  roundedMoney.periods.day[0].worst_paisa = 4;
  assert.match(validateTradePayload(roundedMoney).why, /periods\.day\[0\]\.worst_paisa is 4, not 3/);
});

test('period and slot sums cannot cross the safe boundary through individually safe buckets', () => {
  const payload = validTradePayload();
  payload.periods.month = [
    bucket(2024 * 12, 1, 1, {
      wins: 1,
      best_paisa: Number.MAX_SAFE_INTEGER,
      worst_paisa: Number.MAX_SAFE_INTEGER,
      largest_win: Number.MAX_SAFE_INTEGER,
      largest_loss: Number.MAX_SAFE_INTEGER
    }),
    bucket(2025 * 12, 1, 0, {
      wins: 0,
      best_paisa: 1,
      worst_paisa: 1,
      largest_win: 1,
      largest_loss: 1
    })
  ];
  const result = validateTradePayload(payload);
  assert.equal(result.ok, false);
  assert.match(result.why, /periods\.month carries 2 buckets, not the 1 implied/);
});

test('period totals cannot hide a trade moved into the wrong calendar bucket', () => {
  const payload = validTradePayload();
  payload.periods.year[0].key = 2050;
  const result = validateTradePayload(payload);
  assert.equal(result.ok, false);
  assert.match(result.why, /periods\.year\[0\]\.key is 2050, not 2024/);
  assert.deepEqual(result.rows, []);
});

test('the largest safe integer remains admissible when every fold stays safe', () => {
  const payload = validTradePayload();
  payload.trades = [
    {
      ...payload.trades[0],
      best: Number.MAX_SAFE_INTEGER,
      worst: Number.MAX_SAFE_INTEGER
    }
  ];
  payload.count = 1;
  payload.periods = Object.fromEntries(
    TIME_GRAINS.map(({ key }) => [
      key,
      [
        bucket(PERIOD_KEYS[key], 1, 1, {
          wins: 1,
          best_paisa: Number.MAX_SAFE_INTEGER,
          worst_paisa: Number.MAX_SAFE_INTEGER,
          largest_win: Number.MAX_SAFE_INTEGER,
          largest_loss: Number.MAX_SAFE_INTEGER
        })
      ]
    ])
  );
  const result = validateTradePayload(payload);
  assert.equal(result.ok, true, result.why);
});
