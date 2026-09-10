import assert from 'node:assert/strict';
import test from 'node:test';

// @ts-expect-error Node 24 strips this module's erasable TypeScript at runtime;
// the app imports it through SvelteKit's resolver, which also admits `.ts`.
import { liveAttemptKey, reduceLiveProgress } from '../src/lib/live-progress.ts';

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

/** @param {string} rung @param {number} [candidates] @param {number} [count] */
function progress(rung, candidates = 25, count = 10) {
  return event('exit grid progress', { rung, candidates, priced: count });
}

/** @param {string} rung @param {string} [stage] */
function validating(rung, stage = 'bootstrap') {
  return event('validation stage entered', { rung, stage });
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

test('exit-grid progress advances priced during the phase that used to report zero', () => {
  // THE PHASE THIS COVERS IS 87.6% OF A MEASURED 48-MINUTE RUN. `priced` was
  // written only at `exit grid finished`, so for nearly the whole of a
  // multi-hour sweep the page showed a rung pricing 0 of N — the exact defect
  // `note_grid_progress` was added to fix, still invisible one layer up because
  // this fold discarded every one of its records.
  const records = newestFirst([
    marker(),
    sweep('1min'),
    entered('1min'),
    progress('1min', 25, 10),
    progress('1min', 25, 18)
  ]);
  const answer = reduceLiveProgress(RUN, envelope(records));
  assert.equal(answer.phase, 'ready');
  assert.equal(answer.rungs[0].phase, 'pricing', 'progress must not end the pricing phase');
  assert.equal(answer.rungs[0].priced, 18, 'the newest decile is the one shown');
  assert.equal(answer.rungs[0].candidates, 25);
  assert.equal(answer.rungs[0].done, false);
});

test('a validation boundary keeps a finished rung alive without inventing a phase', () => {
  // Between `exit grid finished` and a row landing sit walk-forward, PBO and a
  // bootstrap fixed at sixteen thousand trade re-walks. MEASURED on the
  // operator's log: eight rungs reached `exit grid finished` and three reached
  // a committed result. The boundary says the run is alive in that stretch; it
  // does not claim a phase the engine does not report.
  const records = newestFirst([
    marker(),
    sweep('1min'),
    entered('1min'),
    priced('1min'),
    validating('1min', 'walk-forward')
  ]);
  const answer = reduceLiveProgress(RUN, envelope(records));
  assert.equal(answer.phase, 'ready');
  assert.equal(answer.rungs[0].phase, 'priced', 'validation does not move the sweep phase');
  assert.equal(answer.rungs[0].priced, 20);
});

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
  assert.equal(answer.attempt, String(RUN.attempt));
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
    [marker(), sweep('1min'), entered('1min'), priced('1min'), finished('1min'), priced('1min')],
    // PROGRESS BEFORE THE GRID IS ENTERED has no candidate count to agree with.
    [marker(), sweep('1min'), progress('1min')],
    // A DECILE THAT WENT BACKWARDS. A progress bar that can retreat is one
    // nobody can read, and a fall here means two rungs' records were folded
    // into one held state.
    [marker(), sweep('1min'), entered('1min'), progress('1min', 25, 18), progress('1min', 25, 10)],
    // PROGRESS PAST THE CANDIDATE COUNT, and progress naming a different one.
    [marker(), sweep('1min'), entered('1min'), progress('1min', 25, 26)],
    [marker(), sweep('1min'), entered('1min'), progress('1min', 24, 10)],
    // A VALIDATION BOUNDARY BEFORE THE GRID WAS EVER ENTERED.
    [marker(), sweep('1min'), validating('1min')]
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

const U64_BASE = 1n << 63n, U64_MAX = (1n << 64n) - 1n;

/** Simulate JSON's rounded legacy numeric field beside the server's exact alias.
 * These are generated protocol fixtures, not recorded executions.
 * @param {string} key @returns {Record<string, any>} */
function durableRun(key) {
  return { ...RUN, attempt: Number(key), attempt_key: key };
}

/** @param {Record<string, any>} row @param {string} key @returns {Record<string, any>} */
function durableEvent(row, key) {
  return { ...row, run: Number(key), run_key: key,
    fields: { ...row.fields, attempt: Number(key), attempt_key: key } };
}

test('exact durable attempts above 2^53 and through u64 max keep live pricing visible', () => {
  for (const key of [String((1n << 53n) + 1n), String(U64_BASE + 1n), String(U64_MAX)]) {
    const run = durableRun(key);
    const records = newestFirst([marker(), sweep('1min'), entered('1min'), progress('1min', 25, 18)])
      .map(row => durableEvent(row, key));
    const answer = reduceLiveProgress(run, envelope(records));
    assert.equal(liveAttemptKey(run), key);
    assert.equal(answer.phase, 'ready'); assert.equal(answer.attempt, key);
    assert.equal(answer.rungs[0].phase, 'pricing'); assert.equal(answer.rungs[0].priced, 18);
    assert.equal(answer.rungs[0].done, false);
  }
});

test('safe legacy IDs and matching new aliases normalize to the same exact string', () => {
  for (const attempt of [1, RUN.attempt, Number.MAX_SAFE_INTEGER]) {
    const run = { ...RUN, attempt };
    const records = newestFirst([marker(attempt), { ...sweep('1min'), run: attempt,
      fields: { ...sweep('1min').fields, attempt } }]);
    const legacy = reduceLiveProgress(run, envelope(records));
    const aliased = reduceLiveProgress({ ...run, attempt_key: String(attempt) },
      envelope(records.map(row => durableEvent(row, String(attempt)))));
    assert.equal(liveAttemptKey(run), String(attempt));
    assert.deepEqual(legacy, aliased);
    assert.equal(legacy.phase, 'ready'); assert.equal(legacy.attempt, String(attempt));
  }
});

test('alias-only records remain exact when legacy numeric fields are omitted', () => {
  const key = String(U64_BASE + 1n), run = durableRun(key);
  delete run.attempt;
  const records = newestFirst([marker(), sweep('1min')]).map(row => {
    const exact = durableEvent(row, key); delete exact.run; delete exact.fields.attempt;
    return exact;
  });
  const answer = reduceLiveProgress(run, envelope(records));
  assert.equal(answer.phase, 'ready'); assert.equal(answer.attempt, key);
  assert.equal(answer.rungs[0].rung, '1min');
});

test('present malformed or overflowing aliases cannot fall back to otherwise safe legacy tokens', () => {
  const invalid = [undefined, null, '', '0', '-1', '+701', '0701', '701.0', '7.01e2', ' 701', '701 ',
    String(U64_MAX + 1n), '9'.repeat(1000), 701, true, {}, [], 701n];
  for (const alias of invalid) {
    const active = { ...RUN, attempt_key: alias };
    assert.equal(liveAttemptKey(active), null);
    const noActive = reduceLiveProgress(active, envelope(newestFirst([marker()])));
    assert.equal(noActive.phase, 'failed'); assert.equal(noActive.attempt, null);
    for (const at of ['run', 'field']) {
      const row = sweep('1min');
      if (at === 'run') row.run_key = alias; else row.fields.attempt_key = alias;
      const answer = reduceLiveProgress(RUN, envelope(newestFirst([marker(), row])));
      assert.equal(answer.phase, 'failed', `${at}: ${String(alias)}`);
      assert.deepEqual(answer.rungs, []);
    }
  }
});

test('safe numeric aliases that contradict exact keys are refused in all three identity locations', () => {
  const active = { ...RUN, attempt_key: String(RUN.attempt + 1) };
  assert.equal(liveAttemptKey(active), null);
  for (const at of ['run', 'field']) {
    const row = sweep('1min');
    if (at === 'run') row.run_key = String(RUN.attempt + 1);
    else row.fields.attempt_key = String(RUN.attempt + 1);
    const answer = reduceLiveProgress(RUN, envelope(newestFirst([marker(), row])));
    assert.equal(answer.phase, 'failed'); assert.deepEqual(answer.rungs, []);
  }
  const key = String(U64_BASE + 1n), run = durableRun(key);
  const row = durableEvent(sweep('1min'), key); row.fields.attempt = RUN.attempt;
  assert.equal(reduceLiveProgress(run, envelope(newestFirst([durableEvent(marker(), key), row]))).phase, 'failed');
});

test('unsafe numeric IDs without exact aliases never become apparently precise attempts', () => {
  const key = String(U64_BASE + 1n), run = durableRun(key), markerRow = durableEvent(marker(), key);
  const numericOnly = { ...run }; delete numericOnly.attempt_key;
  assert.equal(liveAttemptKey(numericOnly), null);
  assert.equal(reduceLiveProgress(numericOnly, envelope(newestFirst([markerRow]))).phase, 'failed');
  for (const at of ['run', 'field']) {
    const row = durableEvent(sweep('1min'), key);
    if (at === 'run') delete row.run_key; else delete row.fields.attempt_key;
    const answer = reduceLiveProgress(run, envelope(newestFirst([markerRow, row])));
    assert.equal(answer.phase, 'failed'); assert.deepEqual(answer.rungs, []);
  }
});

test('an exact alias does not excuse malformed or impossible legacy number fields', () => {
  const key = String(U64_BASE + 1n);
  for (const numeric of [null, '9223372036854775809', -1, 0, 1.5, NaN, Infinity, 1e100, {}, U64_BASE + 1n]) {
    assert.equal(liveAttemptKey({ ...durableRun(key), attempt: numeric }), null);
    for (const at of ['run', 'field']) {
      const row = durableEvent(sweep('1min'), key);
      if (at === 'run') row.run = numeric; else row.fields.attempt = numeric;
      const answer = reduceLiveProgress(durableRun(key),
        envelope(newestFirst([durableEvent(marker(), key), row])));
      assert.equal(answer.phase, 'failed'); assert.deepEqual(answer.rungs, []);
    }
  }
});

test('adjacent durable attempts with identical rounded Numbers never borrow each other’s rungs', () => {
  const current = String(U64_BASE + 1n), foreign = String(U64_BASE + 2n);
  assert.equal(Number(current), Number(foreign), 'the fixture reproduces the unsafe-number collision');
  const records = newestFirst([
    durableEvent(marker(), foreign), durableEvent(marker(), current),
    durableEvent(sweep('60min'), foreign), durableEvent(sweep('1min'), current),
    durableEvent(entered('60min'), foreign), durableEvent(priced('60min'), foreign),
    durableEvent(finished('60min'), foreign)
  ]);
  const answer = reduceLiveProgress(durableRun(current), envelope(records));
  assert.equal(answer.phase, 'ready'); assert.equal(answer.attempt, current);
  assert.deepEqual(answer.rungs.map(row => row.rung), ['1min']);
  assert.equal(answer.rungs[0].phase, 'loading');
});

test('either exact current alias makes a split-token collision a refusal, not a foreign event', () => {
  const current = String(U64_BASE + 1n), foreign = String(U64_BASE + 2n);
  for (const at of ['run', 'field']) {
    const row = durableEvent(sweep('1min'), current);
    if (at === 'run') row.run_key = foreign; else row.fields.attempt_key = foreign;
    const answer = reduceLiveProgress(durableRun(current),
      envelope(newestFirst([durableEvent(marker(), current), row])));
    assert.equal(answer.phase, 'failed'); assert.match(answer.why, /two tokens/);
    assert.equal(answer.attempt, current); assert.deepEqual(answer.rungs, []);
  }
});

test('new exact IDs retain start-marker, context, truncation and missing-evidence refusals', () => {
  const key = String(U64_MAX), run = durableRun(key);
  const markerRow = durableEvent(marker(), key), row = durableEvent(sweep('1min'), key);
  for (const records of [[row], [markerRow, markerRow], [row, markerRow]]) {
    const answer = reduceLiveProgress(run, envelope(newestFirst(records)));
    assert.equal(answer.phase, 'failed'); assert.match(answer.why, /start marker/);
  }
  for (const overrides of [{ partial_tail: true }, { missing: 1 }, { hit_scan_cap: true },
    { errors: ['journal unavailable'] }, { malformed: 1 }]) {
    const answer = reduceLiveProgress(run, envelope(newestFirst([markerRow, row]), overrides));
    assert.equal(answer.phase, 'failed'); assert.deepEqual(answer.rungs, []);
  }
  const wrong = durableEvent(sweep('1min'), key); wrong.fields.underlying = 'BANKNIFTY';
  assert.match(reduceLiveProgress(run, envelope(newestFirst([markerRow, wrong]))).why, /underlying/);
  const startedOnly = reduceLiveProgress(run, envelope(newestFirst([markerRow])));
  assert.equal(startedOnly.phase, 'ready'); assert.deepEqual(startedOnly.rungs, [], 'a start alone invents no finished rung');
});

test('normalizing exact aliases leaves frozen wire input and its numeric compatibility fields unchanged', () => {
  /** @param {any} value */
  function freeze(value) {
    if (value && typeof value === 'object') { Object.values(value).forEach(freeze); Object.freeze(value); }
    return value;
  }
  const key = String(U64_MAX), run = durableRun(key);
  const body = envelope(newestFirst([marker(), sweep('1min')]).map(row => durableEvent(row, key)));
  const before = structuredClone({ run, body });
  assert.equal(reduceLiveProgress(freeze(run), freeze(body)).phase, 'ready');
  assert.deepEqual({ run, body }, before);
  assert.equal(typeof body.records[0].run, 'number');
  assert.equal(body.records[0].run_key, key);
});
