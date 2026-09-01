import assert from 'node:assert/strict';
import test from 'node:test';

// @ts-expect-error Node 24 strips this module's erasable TypeScript at runtime;
// the app imports it through SvelteKit's resolver, which also admits `.ts`.
import { reduceLiveProgress } from '../src/lib/live-progress.ts';

const RUN = Object.freeze({
  attempt: 701,
  kind: 'sweep',
  feed: 'zerodha',
  underlying: 'NIFTY',
  from_year: 2024,
  from_month: 1,
  to_year: 2024,
  to_month: 12
});

const CONTEXT = Object.freeze({
  attempt: RUN.attempt,
  feed: RUN.feed,
  underlying: RUN.underlying,
  from_year: RUN.from_year,
  from_month: RUN.from_month,
  to_year: RUN.to_year,
  to_month: RUN.to_month
});

/**
 * @param {string} message
 * @param {Record<string, any>} fields
 * @param {Record<string, any>} [overrides]
 * @returns {Record<string, any>}
 */
function event(message, fields, overrides = {}) {
  return {
    seq: 1,
    run: RUN.attempt,
    ts: 1,
    level: 'info',
    target: 'cli.audit',
    message,
    cut: false,
    dropped_fields: 0,
    fields: { ...CONTEXT, ...fields },
    ...overrides
  };
}

/**
 * @param {number} [attempt]
 * @param {Record<string, any>} [overrides]
 * @returns {Record<string, any>}
 */
function marker(attempt = RUN.attempt, overrides = {}) {
  return event(
    'sweep attempt started',
    { ...CONTEXT, attempt, kind: RUN.kind },
    { run: attempt, ...overrides }
  );
}

/**
 * Assign increasing sequence/time to chronological input, then put it on the
 * wire in the newest-first order `/logs.json` promises.
 *
 * @param {Array<Record<string, any>>} chronological
 * @returns {Array<Record<string, any>>}
 */
function newestFirst(chronological) {
  return chronological
    .map((record, index) => ({ ...record, seq: index + 1, ts: 10_000 + index }))
    .reverse();
}

/**
 * @param {Array<Record<string, any>>} records
 * @param {Record<string, any>} [overrides]
 * @returns {Record<string, any>}
 */
function envelope(records, overrides = {}) {
  return {
    records,
    served_dir: '/tmp/logs',
    cli_dir: '/tmp/store/logs/cli',
    bytes_read: 100,
    files_read: 1,
    malformed: 0,
    partial_tail: false,
    hit_scan_cap: false,
    reached_oldest: false,
    scan_cap_bytes: 4_194_304,
    limit: Math.max(1, records.length),
    missing: null,
    errors: [],
    sink: null,
    ...overrides
  };
}

/** @param {string} rung @param {number} [bars] @param {number} [minHits] @param {number} [supportPpm] */
function sweep(rung, bars = 10_000, minHits = 500, supportPpm = 50_000) {
  return event('rung sweeping', { rung, bars, min_hits: minHits, support_ppm: supportPpm });
}

/** @param {string} rung @param {number} [executionBars] @param {number} [candidates] @param {number} [validate] */
function entered(rung, executionBars = 10_000, candidates = 25, validate = 1) {
  return event('exit grid entered', {
    rung,
    execution_bars: executionBars,
    candidates,
    cap: 25,
    validate
  });
}

/** @param {string} rung @param {number} [candidates] @param {number} [count] */
function priced(rung, candidates = 25, count = 20) {
  return event('exit grid finished', {
    rung,
    candidates,
    priced: count,
    grid_trades: 200
  });
}

/**
 * @param {string} rung
 * @param {number} [recorded]
 * @param {string} [why]
 * @param {number} [bars]
 * @param {number} [minHits]
 */
function finished(rung, recorded = 1, why = '', bars = 10_000, minHits = 500) {
  return event('rung finished', { rung, bars, min_hits: minHits, recorded, why });
}

test('newest-first, interleaved rungs fold oldest-first without phase regression', () => {
  const records = newestFirst([
    marker(),
    sweep('1min'),
    sweep('5min', 2_000, 100),
    entered('1min'),
    entered('5min', 2_000, 12),
    priced('1min'),
    finished('1min'),
    priced('5min', 12, 9)
  ]);
  const answer = reduceLiveProgress(RUN, envelope(records));

  assert.equal(answer.phase, 'ready');
  assert.equal(answer.attempt, RUN.attempt);
  assert.deepEqual(
    answer.rungs.map(({ rung, phase, done, candidates, priced: count }) => ({
      rung,
      phase,
      done,
      candidates,
      priced: count
    })),
    [
      { rung: '5min', phase: 'priced', done: false, candidates: 12, priced: 9 },
      { rung: '1min', phase: 'done', done: true, candidates: 25, priced: 20 }
    ],
    'rows are newest-activity first, but each row was folded chronologically'
  );
  assert.equal(answer.rungs[1].recorded, true);
});

test('an identical rerun is isolated by its opaque attempt, not by its context or time', () => {
  const oldAttempt = RUN.attempt - 1;
  const old = [
    marker(oldAttempt),
    sweep('60min'),
    entered('60min'),
    priced('60min'),
    finished('60min')
  ].map((record) => ({
    ...record,
    run: oldAttempt,
    fields: { ...record.fields, attempt: oldAttempt }
  }));
  const current = [marker(), sweep('1min')];
  const records = newestFirst([
    old[0],
    current[0],
    old[1],
    current[1],
    old[2],
    old[3],
    old[4]
  ]);

  const answer = reduceLiveProgress(RUN, envelope(records));
  assert.equal(answer.phase, 'ready');
  assert.deepEqual(answer.rungs.map((row) => row.rung), ['1min']);
  assert.equal(answer.rungs[0].phase, 'loading');
});

test('a coarse signal rung prices on its distinct one-minute execution series', () => {
  const records = newestFirst([
    marker(),
    sweep('60min', 100, 5),
    entered('60min', 6_000, 25),
    priced('60min', 25, 20),
    finished('60min', 1, '', 100, 5)
  ]);
  const answer = reduceLiveProgress(RUN, envelope(records));

  assert.equal(answer.phase, 'ready');
  assert.equal(answer.rungs[0].rung, '60min');
  assert.equal(answer.rungs[0].bars, 100, 'the visible count remains the signal-rung count');
  assert.equal(answer.rungs[0].phase, 'done');
  assert.equal(answer.rungs[0].recorded, true);
});

test('one descent rung can finish search steps and then open an exact validation step', () => {
  const records = newestFirst([
    marker(),
    sweep('15min', 2_000, 200, 100_000),
    entered('15min', 30_000, 25, 0),
    priced('15min'),
    finished('15min', 1, '', 2_000, 200),
    sweep('15min', 2_000, 100, 50_000),
    entered('15min', 30_000, 25, 0),
    priced('15min'),
    finished('15min', 1, '', 2_000, 100),
    sweep('15min', 2_000, 100, 50_000),
    entered('15min', 30_000, 25, 1)
  ]);
  const answer = reduceLiveProgress(RUN, envelope(records));

  assert.equal(answer.phase, 'ready');
  assert.equal(answer.rungs.length, 3);
  assert.equal(new Set(answer.rungs.map((row) => row.key)).size, 3);
  assert.equal(answer.rungs[0].phase, 'pricing');
  assert.equal(answer.rungs[0].supportPpm, 50_000);
  assert.equal(answer.rungs[0].validating, true);
  assert.deepEqual(answer.rungs.slice(1).map((row) => row.done), [true, true]);
});

test('unfiltered interleaved foreign runs and unrelated current audit events are ignored', () => {
  const foreign = event(
    'rung sweeping',
    {
      ...CONTEXT,
      attempt: 999,
      feed: 'another-feed',
      rung: '3min',
      bars: 800,
      min_hits: 40
    },
    { run: 999 }
  );
  const unrelated = event('ladder level', { rung: '1min' }, { run: RUN.attempt });
  const records = newestFirst([marker(), foreign, unrelated, sweep('1min')]);

  const answer = reduceLiveProgress(RUN, envelope(records));
  assert.equal(answer.phase, 'ready');
  assert.deepEqual(answer.rungs.map((row) => row.rung), ['1min']);
});

test('either current token makes a split-token record a refusal, never a foreign record', () => {
  for (const broken of [
    { run: RUN.attempt, fieldAttempt: 999 },
    { run: 999, fieldAttempt: RUN.attempt }
  ]) {
    const record = sweep('1min');
    record.run = broken.run;
    record.fields.attempt = broken.fieldAttempt;
    const answer = reduceLiveProgress(RUN, envelope(newestFirst([marker(), record])));
    assert.equal(answer.phase, 'failed');
    assert.match(answer.why, /two tokens/);
  }
});

test('every current event repeats the exact active-run context', () => {
  /** @type {Array<[string, unknown]>} */
  const mismatches = [
    ['feed', 'other'],
    ['underlying', 'BANKNIFTY'],
    ['from_year', 2023],
    ['from_month', 2],
    ['to_year', 2025],
    ['to_month', 11]
  ];
  for (const [field, wrong] of mismatches) {
    const record = sweep('1min');
    record.fields[field] = wrong;
    const answer = reduceLiveProgress(RUN, envelope(newestFirst([marker(), record])));
    assert.equal(answer.phase, 'failed', field);
    assert.match(answer.why, new RegExp(field));
  }
  const wrongKind = marker();
  wrongKind.fields.kind = 'descent';
  const answer = reduceLiveProgress(RUN, envelope(newestFirst([wrongKind])));
  assert.equal(answer.phase, 'failed');
  assert.match(answer.why, /kind/);
});

test('exactly one start marker brackets the admitted attempt', () => {
  const missing = reduceLiveProgress(RUN, envelope(newestFirst([sweep('1min')])));
  assert.equal(missing.phase, 'failed');
  assert.match(missing.why, /start marker/);

  const duplicate = reduceLiveProgress(RUN, envelope(newestFirst([marker(), marker()])));
  assert.equal(duplicate.phase, 'failed');
  assert.match(duplicate.why, /more than one start marker/);

  const before = reduceLiveProgress(RUN, envelope(newestFirst([sweep('1min'), marker()])));
  assert.equal(before.phase, 'failed');
  assert.match(before.why, /predates/);
});

test('every envelope truncation and corruption signal fails closed', () => {
  const records = newestFirst([marker()]);
  /** @type {Array<[string, Record<string, unknown>]>} */
  const corruptions = [
    ['malformed', { malformed: 1 }],
    ['partial', { partial_tail: true }],
    ['scan cap', { hit_scan_cap: true }],
    ['read error', { errors: ['permission denied'] }],
    ['missing', { missing: 2 }],
    ['absent malformed count', { malformed: undefined }],
    ['absent partial flag', { partial_tail: undefined }],
    ['absent cap flag', { hit_scan_cap: undefined }],
    ['absent oldest flag', { reached_oldest: undefined }],
    ['absent errors', { errors: undefined }],
    ['bad missing value', { missing: '0' }],
    ['bad limit', { limit: 0 }]
  ];
  for (const [name, overrides] of corruptions) {
    const answer = reduceLiveProgress(RUN, envelope(records, overrides));
    assert.equal(answer.phase, 'failed', name);
    assert.deepEqual(answer.rungs, [], name);
  }
});

test('a full page at its limit is valid when the exact marker is present and flags are clean', () => {
  const records = newestFirst([marker(), sweep('1min')]);
  const answer = reduceLiveProgress(
    RUN,
    envelope(records, { limit: records.length, reached_oldest: false })
  );
  assert.equal(answer.phase, 'ready');
  assert.equal(answer.rungs.length, 1);
});

test('cut, dropped, malformed-shape, and wrong-order records are refused', () => {
  const base = newestFirst([marker(), sweep('1min')]);
  /** @type {Array<[string, Array<Record<string, any>>]>} */
  const corruptions = [
    ['cut', [{ ...base[0], cut: true }, base[1]]],
    ['dropped', [{ ...base[0], dropped_fields: 1 }, base[1]]],
    ['bad seq', [{ ...base[0], seq: 0 }, base[1]]],
    ['bad fields', [{ ...base[0], fields: [] }, base[1]]],
    ['wrong order', [...base].reverse()]
  ];
  for (const [name, records] of corruptions) {
    const answer = reduceLiveProgress(RUN, envelope(records));
    assert.equal(answer.phase, 'failed', name);
  }
});

test('the phase machine rejects missing, repeated, regressed, and inconsistent steps', () => {
  const cases = [
    [marker(), entered('1min')],
    [marker(), sweep('1min'), sweep('1min')],
    [marker(), sweep('1min'), priced('1min')],
    [marker(), sweep('1min'), entered('1min'), entered('1min')],
    [marker(), sweep('1min'), entered('1min'), priced('1min'), entered('1min')],
    [marker(), sweep('1min'), entered('1min', 0)],
    [marker(), sweep('1min'), entered('1min'), priced('1min', 24)],
    [marker(), sweep('1min'), entered('1min'), priced('1min', 25, 26)],
    [marker(), sweep('1min'), finished('1min')],
    [marker(), sweep('1min'), finished('1min', 0, '')],
    [marker(), sweep('1min'), entered('1min'), priced('1min'), finished('1min'), priced('1min')]
  ];
  for (const records of cases) {
    const answer = reduceLiveProgress(RUN, envelope(newestFirst(records)));
    assert.equal(answer.phase, 'failed', records.map((record) => record.message).join(' -> '));
  }
});

test('a rung can refuse before or during pricing, but it must say why', () => {
  for (const records of [
    [marker(), sweep('1min'), finished('1min', 0, 'the ladder refused')],
    [
      marker(),
      sweep('1min'),
      entered('1min'),
      finished('1min', 0, 'the grid could not finish')
    ]
  ]) {
    const answer = reduceLiveProgress(RUN, envelope(newestFirst(records)));
    assert.equal(answer.phase, 'ready');
    assert.equal(answer.rungs[0].phase, 'done');
    assert.equal(answer.rungs[0].recorded, false);
    assert.match(answer.rungs[0].why, /refused|could not finish/);
  }
});

test('invalid active attempts cannot borrow otherwise valid event evidence', () => {
  const payload = envelope(newestFirst([marker()]));
  for (const run of [
    null,
    { ...RUN, attempt: 0 },
    { ...RUN, attempt: Number.MAX_SAFE_INTEGER + 1 },
    { ...RUN, feed: '' },
    { ...RUN, from_month: 0 },
    { ...RUN, to_month: 13 },
    { ...RUN, from_year: 2025, to_year: 2024 }
  ]) {
    const answer = reduceLiveProgress(run, payload);
    assert.equal(answer.phase, 'failed');
    assert.equal(answer.attempt, null);
    assert.deepEqual(answer.rungs, []);
  }
});
