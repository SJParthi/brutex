import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  COMPARISON_STATUS,
  classifyRun,
  compareRuns,
  exactIntegerDelta,
  fillAssumptionGap,
  roundedScaledRatio,
  supportBasisPoints,
  supportRatioKey,
  validateLedgerPayload,
  validateRunForComputation
} from '../src/lib/comparison.js';

const complete = (overrides = {}) => ({
  index: 10,
  identity: 'a'.repeat(64),
  finished_micros: 1_700_000_000_000_000,
  feed: 'fixture-feed',
  underlying: 'NIFTY',
  sealed: true,
  halted: false,
  timeframe: '15min',
  whole_span: true,
  from_year: 2025,
  from_month: 1,
  to_year: 2025,
  to_month: 12,
  months_asked: 12,
  months_found: 12,
  bars: 4_000,
  min_hits: 80,
  combinations: 593_599,
  depth: 8,
  trades: 24,
  pessimistic: 1_000,
  optimistic: 1_400,
  worst_trade: -120,
  max_drawdown: 360,
  winner_mae: -90,
  winner_mfe: 180,
  all_mae: -120,
  exit_rungs: [0, 1, 2, 3, -1],
  mask_words: ['1', '0', '0', '0', '0', '0'],
  ...overrides
});

// The comparison receives this from `/backtest.json.signal_rungs`.  A short
// fixture set is enough to prove membership is exact rather than suffix-based;
// production never owns a second copy of `cli::EVERY_RUNG`.
const SIGNAL_RUNGS = Object.freeze(['1min', '15min', '60min']);
/** @param {any} run */
const statusOf = (run) => classifyRun(run, SIGNAL_RUNGS);

const ledgerRuns = () => [
  complete({ index: 1, identity: 'a'.repeat(64), finished_micros: 2 }),
  complete({ index: 0, identity: 'b'.repeat(64), finished_micros: 1 })
];

/** @param {Record<string, unknown>} [overrides] */
const ledger = (overrides = {}) => ({
  path: '/store/results/runs.bin',
  version: 3,
  writes_version: 3,
  appendable: true,
  commit_stamped: true,
  has_mask: true,
  total: 2,
  scanned: 2,
  hit_scan_cap: false,
  partial_tail: false,
  max_runs: 500,
  halted: 0,
  unsealed: 0,
  signal_rungs: SIGNAL_RUNGS,
  best_complete: 0,
  refusal: null,
  runs: ledgerRuns(),
  ...overrides
});

test('every exclusion has one explicit status and damaged data wins precedence', () => {
  const cases = [
    [complete({ sealed: false, halted: true }), COMPARISON_STATUS.INTEGRITY_FAILED],
    [complete({ halted: true, timeframe: '1day' }), COMPARISON_STATUS.HALTED],
    [complete({ timeframe: '1day', months_found: 11, whole_span: false }), COMPARISON_STATUS.NON_SIGNAL_TIMEFRAME],
    [complete({ months_found: 11, whole_span: false }), COMPARISON_STATUS.PARTIAL_SPAN],
    [
      complete({
        trades: 0,
        pessimistic: 0,
        optimistic: 0,
        worst_trade: 0,
        max_drawdown: 0,
        winner_mae: 0,
        winner_mfe: 0,
        all_mae: 0
      }),
      COMPARISON_STATUS.NO_TRADES
    ],
    [complete({ pessimistic: 1_500, optimistic: 1_400 }), COMPARISON_STATUS.CONTRADICTORY_FILL_TOTALS],
    [complete({ pessimistic: undefined }), COMPARISON_STATUS.MISSING_NUMERIC_RESULTS],
    [complete(), COMPARISON_STATUS.RANKABLE]
  ];

  for (const [run, key] of cases) {
    const status = statusOf(run);
    assert.equal(status.key, key);
    assert.ok(status.label.length > 0, `${key} needs a human label`);
    assert.ok(status.reason.length > 0, `${key} needs a visible reason`);
    assert.equal(status.eligible, key === COMPARISON_STATUS.RANKABLE);
  }
});

test('invalid counts and absent risk results cannot slip into ranking', () => {
  for (const bad of [
    { bars: 0 },
    { bars: NaN },
    { min_hits: -1 },
    { trades: 1.5 },
    { worst_trade: null },
    { max_drawdown: -1 },
    { max_drawdown: Infinity }
  ]) {
    assert.equal(
      statusOf(complete(bad)).key,
      COMPARISON_STATUS.MISSING_NUMERIC_RESULTS,
      JSON.stringify(bad)
    );
  }
});

test('every count printed by the comparison must be exact before any other status', () => {
  const unsafe = Number.MAX_SAFE_INTEGER + 1;
  for (const bad of [
    { index: unsafe },
    { from_year: unsafe },
    { from_month: 13 },
    { months_asked: unsafe },
    { months_found: unsafe },
    { months_found: 13 },
    { combinations: unsafe },
    { depth: unsafe },
    { trades: unsafe },
    { min_hits: 4_001 },
    { depth: unsafe, halted: true }
  ]) {
    assert.equal(
      statusOf(complete(bad)).key,
      COMPARISON_STATUS.MISSING_NUMERIC_RESULTS,
      JSON.stringify(bad)
    );
  }
});

test('one all-or-nothing admission door covers every full-report integer and mask word', () => {
  assert.deepEqual(validateRunForComputation(complete()), { ok: true, why: '' });

  const unsafe = Number.MAX_SAFE_INTEGER + 1;
  for (const bad of [
    { finished_micros: unsafe },
    { winner_mae: unsafe },
    { winner_mfe: unsafe },
    { all_mae: unsafe },
    { exit_rungs: [0, 1, unsafe, 3, -1] },
    { exit_rungs: [0, 1, 2, 3] },
    { mask_words: ['1', '0', '0', '0', '0', '18446744073709551616'] },
    { mask_words: ['01', '0', '0', '0', '0', '0'] },
    { exit_rungs: Object.assign(new Array(5), { 0: 0, 1: 1, 3: 3, 4: -1 }) },
    { mask_words: Object.assign(new Array(6), { 0: '1', 1: '0', 3: '0', 4: '0', 5: '0' }) },
    { identity: 'not-a-run-id' },
    { feed: '' },
    { underlying: '' },
    { timeframe: '' },
    { trades: -0 }
  ]) {
    const admission = validateRunForComputation(complete(bad));
    assert.equal(admission.ok, false, JSON.stringify(bad));
    assert.ok(admission.why.length > 0);
    assert.equal(
      statusOf(complete(bad)).key,
      COMPARISON_STATUS.MISSING_NUMERIC_RESULTS,
      JSON.stringify(bad)
    );
  }
});

test('integrity failure and unsafe derived fill arithmetic are metadata only', () => {
  const unsealed = validateRunForComputation(complete({ sealed: false }));
  assert.equal(unsealed.ok, false);
  assert.match(unsealed.why, /integrity seal/);

  const unsafeGap = validateRunForComputation(
    complete({ pessimistic: Number.MIN_SAFE_INTEGER, optimistic: Number.MAX_SAFE_INTEGER })
  );
  assert.equal(unsafeGap.ok, false);
  assert.match(unsafeGap.why, /optimistic minus pessimistic/);
});

test('semantic result contradictions never enter computation', () => {
  for (const bad of [
    { pessimistic: 1_500, optimistic: 1_400 },
    { trades: 0 },
    { worst_trade: 1 }
  ]) {
    assert.equal(validateRunForComputation(complete(bad)).ok, false, JSON.stringify(bad));
  }
  assert.equal(
    statusOf(complete({ pessimistic: 1_500, optimistic: 1_400 })).key,
    COMPARISON_STATUS.CONTRADICTORY_FILL_TOTALS
  );
});

test('narrow Rust wire domains and contradictory span metadata are refused', () => {
  for (const bad of [
    { from_year: 65_536 },
    { to_year: 65_536 },
    { months_asked: 4_294_967_296 },
    { months_found: 4_294_967_296 },
    { depth: 4_294_967_296 },
    { from_year: 2026, from_month: 2, to_year: 2026, to_month: 1, months_asked: 1 },
    { to_month: 2, months_asked: 12 },
    { months_found: 11, whole_span: true },
    { months_found: 12, whole_span: false }
  ]) {
    const admission = validateRunForComputation(complete(bad));
    assert.equal(admission.ok, false, JSON.stringify(bad));
  }
});

test('an unsafe row derives no secondary metrics even when their own endpoints are safe', () => {
  const [row] = compareRuns([
    complete({ finished_micros: Number.MAX_SAFE_INTEGER + 1 })
  ], SIGNAL_RUNGS);
  assert.equal(row.admitted, false);
  assert.equal(row.supportBp, null);
  assert.equal(row.fillGap, null);
  assert.equal(row.deltas, null);
});

test('integrity failure keeps precedence while unsafe bytes remain non-admitted', () => {
  const [row] = compareRuns([
    complete({ sealed: false, finished_micros: Number.MAX_SAFE_INTEGER + 1 })
  ], SIGNAL_RUNGS);
  assert.equal(row.admitted, false);
  assert.equal(row.status.key, COMPARISON_STATUS.INTEGRITY_FAILED);
  assert.equal(row.eligible, false);
  assert.equal(row.supportBp, null);
  assert.equal(row.fillGap, null);
});

test('comparison retains refused rows as metadata and marks them non-drillable', () => {
  const unsafe = complete({
    index: 99,
    finished_micros: Number.MAX_SAFE_INTEGER + 1,
    underlying: 'VISIBLE-METADATA'
  });
  const rows = compareRuns([complete(), unsafe], SIGNAL_RUNGS);
  const refused = rows.find((row) => row.underlying === 'VISIBLE-METADATA');

  assert.ok(refused, 'the refused row must remain visible');
  assert.equal(refused.admitted, false);
  assert.match(refused.admissionWhy, /finished_micros/);
  assert.equal(refused.eligible, false);
  assert.equal(refused.status.key, COMPARISON_STATUS.MISSING_NUMERIC_RESULTS);
});

test('support is basis points and the fill gap is the exact paisa interval', () => {
  assert.equal(supportBasisPoints(complete()), 200, '80 / 4,000 is 2.00%');
  assert.equal(supportBasisPoints(complete({ min_hits: 1, bars: 32 })), 313, '3.125% rounds to 313 bp');
  assert.equal(supportBasisPoints(complete({ bars: 0 })), null);
  assert.equal(supportBasisPoints(complete({ min_hits: undefined })), null);

  assert.equal(fillAssumptionGap(complete()), 400);
  assert.equal(fillAssumptionGap(complete({ pessimistic: -200, optimistic: 50 })), 250);
  assert.equal(fillAssumptionGap(complete({ pessimistic: 500, optimistic: 499 })), null);
  assert.equal(fillAssumptionGap(complete({ optimistic: undefined })), null);
});

test('comparison identity uses the exact reduced support ratio, never rounded basis points', () => {
  const one = complete({ min_hits: 1, bars: 20_000 });
  const two = complete({ min_hits: 2, bars: 20_000 });
  const equivalent = complete({ min_hits: 2, bars: 40_000 });
  assert.equal(supportBasisPoints(one), 1);
  assert.equal(supportBasisPoints(two), 1, 'the visible basis-point labels may round alike');
  assert.equal(supportRatioKey(one), '1/20000');
  assert.equal(supportRatioKey(two), '1/10000');
  assert.notEqual(supportRatioKey(one), supportRatioKey(two));
  assert.equal(supportRatioKey(one), supportRatioKey(equivalent));
});

test('one complete ledger envelope is admitted before any summary or row is published', () => {
  const admitted = validateLedgerPayload(ledger());
  assert.equal(admitted.ok, true, admitted.why);
  assert.equal(admitted.body?.runs.length, 2);

  const unsealed = validateLedgerPayload(
    ledger({
      total: 1,
      scanned: 1,
      unsealed: 1,
      best_complete: null,
      runs: [complete({ index: 0, identity: 'c'.repeat(64), sealed: false })]
    })
  );
  assert.equal(unsealed.ok, true, unsealed.why);

  const unavailable = validateLedgerPayload({
    path: '',
    total: 0,
    scanned: 0,
    hit_scan_cap: false,
    partial_tail: false,
    max_runs: 500,
    halted: 0,
    unsealed: 0,
    signal_rungs: SIGNAL_RUNGS,
    best_complete: null,
    refusal: 'the store root is unavailable',
    runs: []
  });
  assert.equal(unavailable.ok, true, unavailable.why);
});

test('ledger admission refuses unsafe, duplicated, misordered, or contradictory authority', () => {
  const malformed = [
    ledger({ total: Number.MAX_SAFE_INTEGER + 1 }),
    ledger({ scanned: 1 }),
    ledger({ total: 3 }),
    ledger({ hit_scan_cap: 1 }),
    ledger({ best_complete: 1 }),
    ledger({ halted: 1 }),
    ledger({ signal_rungs: ['1min', '01min'] }),
    ledger({ signal_rungs: ['15min', '1min'] }),
    ledger({ runs: [ledgerRuns()[1], ledgerRuns()[0]] }),
    ledger({
      runs: [ledgerRuns()[0], { ...ledgerRuns()[1], identity: ledgerRuns()[0].identity }]
    }),
    ledger({ runs: [ledgerRuns()[0], { ...ledgerRuns()[1], min_hits: 0 }] })
  ];
  for (const body of malformed) {
    const refused = validateLedgerPayload(body);
    assert.equal(refused.ok, false, JSON.stringify(body).slice(0, 180));
    assert.equal(refused.body, null);
    assert.match(refused.why, /malformed envelope/);
  }

  const capped = validateLedgerPayload(
    ledger({
      scanned: 1,
      hit_scan_cap: true,
      best_complete: 1,
      runs: [ledgerRuns()[0]]
    })
  );
  assert.equal(capped.ok, true, capped.why);
});

test('rankable rows lead by pessimistic result with stable ledger-index tie breaking', () => {
  const rows = compareRuns([
    complete({ index: 30, pessimistic: 800, optimistic: 900 }),
    complete({ index: 12, pessimistic: 1_100, optimistic: 1_500 }),
    complete({ index: 5, pessimistic: 1_100, optimistic: 1_200 }),
    complete({ index: 2, halted: true, pessimistic: 99_999, optimistic: 100_000 }),
    complete({ index: 1, timeframe: '1day' })
  ], SIGNAL_RUNGS);

  assert.deepEqual(rows.map((row) => row.index), [5, 12, 30, 2, 1]);
  assert.deepEqual(rows.map((row) => row.rank), [1, 2, 3, null, null]);
  assert.deepEqual(rows.map((row) => row.status.key), [
    COMPARISON_STATUS.RANKABLE,
    COMPARISON_STATUS.RANKABLE,
    COMPARISON_STATUS.RANKABLE,
    COMPARISON_STATUS.HALTED,
    COMPARISON_STATUS.NON_SIGNAL_TIMEFRAME
  ]);
  assert.equal(rows[0].reference, true);
  assert.equal(rows[1].reference, false);
  assert.equal(rows[3].reference, null);
  assert.equal(rows[3].deltas, null);
});

test('deltas are only against the best rankable reference', () => {
  const [leader, follower, excluded] = compareRuns([
    complete({
      index: 9,
      trades: 20,
      min_hits: 100,
      pessimistic: 2_000,
      optimistic: 2_500,
      max_drawdown: 300,
      worst_trade: -90
    }),
    complete({
      index: 10,
      trades: 26,
      min_hits: 80,
      pessimistic: 1_700,
      optimistic: 2_300,
      max_drawdown: 450,
      worst_trade: -140
    }),
    complete({ index: 11, trades: 0 })
  ], SIGNAL_RUNGS);

  assert.deepEqual(leader.deltas, {
    pessimistic: 0,
    optimistic: 0,
    trades: 0,
    supportBp: 0,
    fillGap: 0,
    maxDrawdown: 0,
    worstTrade: 0
  });
  assert.deepEqual(follower.deltas, {
    pessimistic: -300,
    optimistic: -200,
    trades: 6,
    supportBp: -50,
    fillGap: 100,
    maxDrawdown: 150,
    worstTrade: -50
  });
  assert.equal(excluded.rank, null);
  assert.equal(excluded.reference, null);
  assert.equal(excluded.deltas, null);
});

test('ineligible rows remain visible in their original order and keep honest null gaps', () => {
  const rows = compareRuns([
    complete({ index: 1, halted: true }),
    complete({ index: 2, pessimistic: 4_000, optimistic: 3_000 }),
    complete({ index: 3, optimistic: undefined }),
    complete({ index: 4, pessimistic: 10, optimistic: 20 })
  ], SIGNAL_RUNGS);

  assert.deepEqual(rows.map((row) => row.index), [4, 1, 2, 3]);
  assert.equal(rows[1].fillGap, 400, 'a halted row may still display its recorded gap');
  assert.equal(rows[2].fillGap, null, 'a contradictory interval is never fabricated');
  assert.equal(rows[3].fillGap, null, 'a missing endpoint is never coerced to zero');
});

test('a missing input is an empty comparison rather than an exception', () => {
  assert.deepEqual(compareRuns(undefined, SIGNAL_RUNGS), []);
  assert.deepEqual(compareRuns(null, SIGNAL_RUNGS), []);
});

test('malformed array members remain refused metadata instead of throwing', () => {
  const rows = compareRuns([null, [], 42, 'bad'], SIGNAL_RUNGS);
  assert.equal(rows.length, 4);
  assert.ok(rows.every((row) => row.admitted === false));
  assert.ok(rows.every((row) => row.eligible === false));
  assert.ok(rows.every((row) => row.status.key === COMPARISON_STATUS.MISSING_NUMERIC_RESULTS));
});

test('signal membership comes from the exact wire set, never a min suffix guess', () => {
  assert.equal(statusOf(complete({ timeframe: '15min' })).key, COMPARISON_STATUS.RANKABLE);
  for (const timeframe of ['0min', 'garbagemin', '1day']) {
    assert.equal(
      statusOf(complete({ timeframe })).key,
      COMPARISON_STATUS.NON_SIGNAL_TIMEFRAME,
      String(timeframe)
    );
  }
  assert.equal(
    statusOf(complete({ timeframe: '' })).key,
    COMPARISON_STATUS.MISSING_NUMERIC_RESULTS,
    'an empty label is malformed rather than a valid off-surface rung'
  );
  assert.equal(
    statusOf(complete({ timeframe: undefined })).key,
    COMPARISON_STATUS.MISSING_NUMERIC_RESULTS,
    'an absent label fails row admission before signal-surface classification'
  );
  assert.equal(
    classifyRun(complete(), undefined).key,
    COMPARISON_STATUS.SIGNAL_SURFACE_UNAVAILABLE,
    'an old or malformed response has no authority from which to infer a rung'
  );
});

test('an empty, duplicated, or malformed signal-rung envelope has no ranking authority', () => {
  for (const rungs of [[], ['15min', '15min'], ['15min', '']]) {
    assert.equal(
      classifyRun(complete(), rungs).key,
      COMPARISON_STATUS.SIGNAL_SURFACE_UNAVAILABLE,
      JSON.stringify(rungs)
    );
  }
});

test('unsafe money and fill gaps are excluded instead of rounded', () => {
  const unsafe = Number.MAX_SAFE_INTEGER + 1;
  for (const bad of [
    { pessimistic: unsafe, optimistic: unsafe },
    { optimistic: unsafe },
    { worst_trade: unsafe },
    { max_drawdown: unsafe }
  ]) {
    assert.equal(
      statusOf(complete(bad)).key,
      COMPARISON_STATUS.MISSING_NUMERIC_RESULTS,
      JSON.stringify(bad)
    );
  }

  const widerThanSafe = complete({
    pessimistic: -Number.MAX_SAFE_INTEGER,
    optimistic: Number.MAX_SAFE_INTEGER
  });
  assert.equal(fillAssumptionGap(widerThanSafe), null);
  assert.equal(statusOf(widerThanSafe).key, COMPARISON_STATUS.MISSING_NUMERIC_RESULTS);
});

test('a delta wider than the exact integer range cannot enter ranking', () => {
  const rows = compareRuns([
    complete({
      index: 1,
      pessimistic: Number.MAX_SAFE_INTEGER,
      optimistic: Number.MAX_SAFE_INTEGER
    }),
    complete({
      index: 2,
      pessimistic: -Number.MAX_SAFE_INTEGER,
      optimistic: -Number.MAX_SAFE_INTEGER
    })
  ], SIGNAL_RUNGS);

  assert.equal(rows[0].index, 1);
  assert.equal(rows[0].rank, 1);
  assert.equal(rows[1].index, 2);
  assert.equal(rows[1].rank, null);
  assert.equal(rows[1].eligible, false);
  assert.equal(rows[1].status.key, COMPARISON_STATUS.MISSING_NUMERIC_RESULTS);
  assert.equal(rows[1].deltas, null);
});

test('derived integer arithmetic refuses results outside the exact range', () => {
  assert.equal(exactIntegerDelta(40, 17), 23);
  assert.equal(
    exactIntegerDelta(Number.MAX_SAFE_INTEGER, -Number.MAX_SAFE_INTEGER),
    null
  );
  assert.equal(roundedScaledRatio(1, 4, 10_000), 2_500);
  assert.equal(
    roundedScaledRatio(Number.MAX_SAFE_INTEGER, 3, 1),
    3_002_399_751_580_330,
    'the exact integer quotient is not rounded through a binary float'
  );
  assert.equal(roundedScaledRatio(-3, 2, 1), -1, 'ties round toward positive infinity');
  assert.equal(roundedScaledRatio(-8, 3, 1), -3, 'negative values past a half round down');
  assert.equal(supportBasisPoints(complete({ min_hits: 3, bars: 20_000 })), 2);
  assert.equal(
    roundedScaledRatio(Number.MAX_SAFE_INTEGER, 1, 10_000),
    null,
    'safe endpoints do not make their scaled result exact'
  );
  assert.equal(roundedScaledRatio(1, 0, 10_000), null);
  assert.equal(roundedScaledRatio(1.5, 2, 10_000), null);
});
