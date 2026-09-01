/**
 * ONE HONEST COMPARISON TABLE.
 *
 * A ledger row is never dropped because it cannot be ranked.  `classifyRun`
 * gives every row one plain status and `compareRuns` puts the rankable rows
 * first while leaving every excluded row visible afterwards.
 *
 * The ordering deliberately follows the engine's selection rule: the adverse
 * fill total (`pessimistic`) is the headline, descending, and the ledger's
 * stable `index` breaks a tie.  The flattering fill total never decides rank.
 */

export const COMPARISON_STATUS = Object.freeze({
  INTEGRITY_FAILED: 'integrity_failed',
  HALTED: 'halted',
  SIGNAL_SURFACE_UNAVAILABLE: 'signal_surface_unavailable',
  NON_SIGNAL_TIMEFRAME: 'non_signal_timeframe',
  PARTIAL_SPAN: 'partial_span',
  NO_TRADES: 'no_trades',
  CONTRADICTORY_FILL_TOTALS: 'contradictory_fill_totals',
  MISSING_NUMERIC_RESULTS: 'missing_numeric_results',
  RANKABLE: 'rankable'
});

const STATUS = Object.freeze({
  [COMPARISON_STATUS.INTEGRITY_FAILED]: Object.freeze({
    key: COMPARISON_STATUS.INTEGRITY_FAILED,
    label: 'Integrity failed',
    eligible: false,
    reason: 'The run failed its integrity seal, so its numbers cannot be trusted.'
  }),
  [COMPARISON_STATUS.HALTED]: Object.freeze({
    key: COMPARISON_STATUS.HALTED,
    label: 'Stopped early',
    eligible: false,
    reason: 'The search stopped before the ladder reached extinction.'
  }),
  [COMPARISON_STATUS.SIGNAL_SURFACE_UNAVAILABLE]: Object.freeze({
    key: COMPARISON_STATUS.SIGNAL_SURFACE_UNAVAILABLE,
    label: 'Signal rungs unavailable',
    eligible: false,
    reason: 'The server did not publish the exact signal-rung set, so this row cannot be classified safely.'
  }),
  [COMPARISON_STATUS.NON_SIGNAL_TIMEFRAME]: Object.freeze({
    key: COMPARISON_STATUS.NON_SIGNAL_TIMEFRAME,
    label: 'Reference timeframe',
    eligible: false,
    reason: 'This timeframe supplies reference data; it is not a swept signal rung.'
  }),
  [COMPARISON_STATUS.PARTIAL_SPAN]: Object.freeze({
    key: COMPARISON_STATUS.PARTIAL_SPAN,
    label: 'Partial history',
    eligible: false,
    reason: 'The run did not cover every month it asked for.'
  }),
  [COMPARISON_STATUS.NO_TRADES]: Object.freeze({
    key: COMPARISON_STATUS.NO_TRADES,
    label: 'No trades',
    eligible: false,
    reason: 'The selected combination opened no position, so there is no trading result to rank.'
  }),
  [COMPARISON_STATUS.CONTRADICTORY_FILL_TOTALS]: Object.freeze({
    key: COMPARISON_STATUS.CONTRADICTORY_FILL_TOTALS,
    label: 'Fill totals conflict',
    eligible: false,
    reason: 'The adverse-fill total is greater than the optimistic-fill total.'
  }),
  [COMPARISON_STATUS.MISSING_NUMERIC_RESULTS]: Object.freeze({
    key: COMPARISON_STATUS.MISSING_NUMERIC_RESULTS,
    label: 'Results unavailable',
    eligible: false,
    reason: 'One or more numbers needed for an honest comparison are missing, invalid, or not exactly representable by this browser.'
  }),
  [COMPARISON_STATUS.RANKABLE]: Object.freeze({
    key: COMPARISON_STATUS.RANKABLE,
    label: 'Finished result',
    eligible: true,
    reason: 'Finished, sealed when a seal is present, and measured on a signal timeframe.'
  })
});

const RESULT_FIELDS = [
  'pessimistic',
  'optimistic',
  'worst_trade',
  'max_drawdown'
];

// Every integer the plain-language comparison prints or uses to identify its
// setting.  A row is never allowed to say “results unavailable” while quietly
// formatting an unsafe depth, month count, bar count, or enumeration count
// beside that refusal.
const DISPLAY_INTEGER_FIELDS = Object.freeze([
  'index',
  'from_year',
  'from_month',
  'to_year',
  'to_month',
  'months_asked',
  'months_found',
  'bars',
  'min_hits',
  'combinations',
  'depth',
  'trades'
]);

// Every numeric field in one `/backtest.json` run, including the values that
// only the full report consumes.  The browser has already parsed these as
// IEEE-754 Numbers; once one is outside the safe-integer range its original
// i64/u64 value cannot be recovered.  Admission must therefore happen before
// any subtraction, ratio, date conversion, ranking, chart scale, or drill-down
// state can see the row.
const RUN_INTEGER_FIELDS = Object.freeze([
  'index',
  'finished_micros',
  'from_year',
  'from_month',
  'to_year',
  'to_month',
  'months_asked',
  'months_found',
  'bars',
  'min_hits',
  'combinations',
  'depth',
  'trades',
  'pessimistic',
  'optimistic',
  'worst_trade',
  'max_drawdown',
  'winner_mae',
  'winner_mfe',
  'all_mae'
]);

const RUN_NON_NEGATIVE_FIELDS = Object.freeze([
  'index',
  'finished_micros',
  'from_year',
  'from_month',
  'to_year',
  'to_month',
  'months_asked',
  'months_found',
  'bars',
  'min_hits',
  'combinations',
  'depth',
  'trades',
  'max_drawdown'
]);

const U64_DECIMAL = /^(?:0|[1-9][0-9]{0,19})$/;
const U64_MAX = 18_446_744_073_709_551_615n;
const U16_MAX = 65_535;
const U32_MAX = 4_294_967_295;

/** @param {unknown} value */
const canonicalU64 = (value) => {
  if (typeof value !== 'string' || !U64_DECIMAL.test(value)) return false;
  try {
    return BigInt(value) <= U64_MAX;
  } catch {
    return false;
  }
};

/**
 * The one admission door between a parsed ledger row and every computation.
 *
 * A refused row remains displayable as escaped string metadata, but it must
 * never enter the answer, rung comparison, sortable ledger, or full report.
 * This is intentionally all-or-nothing: validating only the headline fields
 * still leaves a rounded MAE, timestamp, or exit rung available deeper in the
 * report.
 *
 * @param {unknown} candidate
 * @param {boolean} requireSeal
 * @returns {{ok: true, why: ''} | {ok: false, why: string}}
 */
function validateRun(candidate, requireSeal) {
  if (candidate === null || typeof candidate !== 'object' || Array.isArray(candidate)) {
    return { ok: false, why: 'The ledger row is not an object.' };
  }
  const run = /** @type {Record<string, any>} */ (candidate);
  for (const field of RUN_INTEGER_FIELDS) {
    if (!Number.isSafeInteger(run[field]) || Object.is(run[field], -0)) {
      return {
        ok: false,
        why: `${field} is absent or is not an exactly representable integer.`
      };
    }
  }
  for (const field of RUN_NON_NEGATIVE_FIELDS) {
    if (run[field] < 0) {
      return { ok: false, why: `${field} is negative even though it is a count or timestamp.` };
    }
  }
  // These fields are narrower than JavaScript's exact-integer range on the
  // wire.  Checking only `Number.isSafeInteger` admits values the Rust record
  // could never have contained and lets later calendar arithmetic multiply a
  // supposedly valid year beyond the exact range.
  if (run.from_year > U16_MAX || run.to_year > U16_MAX) {
    return { ok: false, why: 'A span year is outside the unsigned 16-bit wire domain.' };
  }
  for (const field of ['months_asked', 'months_found', 'depth']) {
    if (run[field] > U32_MAX) {
      return { ok: false, why: `${field} is outside the unsigned 32-bit wire domain.` };
    }
  }
  const fromOrdinal = run.from_year * 12 + (run.from_month - 1);
  const toOrdinal = run.to_year * 12 + (run.to_month - 1);
  const askedBySpan = toOrdinal - fromOrdinal + 1;
  if (
    run.from_month < 1 ||
    run.from_month > 12 ||
    run.to_month < 1 ||
    run.to_month > 12 ||
    run.months_asked < 1 ||
    fromOrdinal > toOrdinal ||
    run.months_asked !== askedBySpan ||
    run.months_found > run.months_asked ||
    run.whole_span !== (run.months_found === run.months_asked) ||
    run.bars < 1 ||
    run.min_hits < 1 ||
    run.min_hits > run.bars
  ) {
    return { ok: false, why: 'The run contains contradictory span, bar, or support counts.' };
  }
  if (
    typeof run.identity !== 'string' ||
    !/^[0-9a-f]{64}$/.test(run.identity) ||
    typeof run.feed !== 'string' ||
    run.feed.length === 0 ||
    typeof run.underlying !== 'string' ||
    run.underlying.length === 0 ||
    typeof run.timeframe !== 'string' ||
    run.timeframe.length === 0 ||
    typeof run.whole_span !== 'boolean' ||
    typeof run.halted !== 'boolean' ||
    typeof run.sealed !== 'boolean'
  ) {
    return { ok: false, why: 'The run identity, labels, or boolean state is malformed.' };
  }
  if (requireSeal && !run.sealed) {
    return {
      ok: false,
      why: 'The run failed its integrity seal and is retained as metadata only.'
    };
  }
  const fillGap = run.optimistic - run.pessimistic;
  if (!Number.isSafeInteger(fillGap)) {
    return {
      ok: false,
      why: 'optimistic minus pessimistic is outside the browser exact-integer range.'
    };
  }
  if (run.pessimistic > run.optimistic) {
    return { ok: false, why: 'The adverse-fill total exceeds the optimistic-fill total.' };
  }
  if (
    (run.trades === 0 &&
      [
        run.pessimistic,
        run.optimistic,
        run.worst_trade,
        run.max_drawdown,
        run.winner_mae,
        run.winner_mfe,
        run.all_mae
      ].some((value) => value !== 0)) ||
    (run.trades > 0 && run.worst_trade > 0)
  ) {
    return {
      ok: false,
      why: 'The trade count contradicts the result, risk, or excursion fields.'
    };
  }
  if (
    !Array.isArray(run.exit_rungs) ||
    run.exit_rungs.length !== 5 ||
    Array.from(run.exit_rungs).some(
      (rung) => !Number.isSafeInteger(rung) || rung < -1 || rung > 32_767
    )
  ) {
    return { ok: false, why: 'The five exit-rung indices are absent or not exact integers.' };
  }
  if (
    !Array.isArray(run.mask_words) ||
    run.mask_words.length !== 6 ||
    Array.from(run.mask_words).some((word) => !canonicalU64(word))
  ) {
    return { ok: false, why: 'The condition mask is not six canonical unsigned 64-bit words.' };
  }
  return { ok: true, why: '' };
}

/**
 * The computation door additionally requires the row's integrity seal.
 * Ledger-envelope admission uses the same complete schema check but retains a
 * structurally valid unsealed row as visible, non-computable metadata.
 *
 * @param {unknown} candidate
 * @returns {{ok: true, why: ''} | {ok: false, why: string}}
 */
export function validateRunForComputation(candidate) {
  return validateRun(candidate, true);
}

/**
 * @param {string} why
 * @returns {{ok: false, body: null, why: string}}
 */
const invalidLedger = (why) => ({
  ok: false,
  body: null,
  why: `/backtest.json returned a malformed envelope: ${why}`
});

/**
 * Admit the engine-owned signal-rung vocabulary without keeping a browser
 * copy. Values are canonical positive-minute words, unique, and increasing in
 * the order the server publishes them.
 *
 * @param {unknown} candidate
 * @returns {string[] | null}
 */
const canonicalSignalRungs = (candidate) => {
  if (!Array.isArray(candidate) || candidate.length === 0) return null;
  let previous = 0;
  const seen = new Set();
  for (const value of candidate) {
    if (typeof value !== 'string' || seen.has(value)) return null;
    const match = /^([1-9][0-9]*)min$/.exec(value);
    if (!match) return null;
    const minutes = Number(match[1]);
    if (!Number.isSafeInteger(minutes) || minutes <= previous) return null;
    previous = minutes;
    seen.add(value);
  }
  return candidate;
};

/**
 * Validate one complete `/backtest.json` response before any row or summary is
 * published. The API derives indexes from append positions and returns the
 * newest contiguous prefix, so order, uniqueness, counts, cap state, and the
 * advertised best row are all independently reconstructable here.
 *
 * A configuration-level 503 deliberately omits file-version fields. It is
 * admitted only in its exact empty/refused shape so the server's reason remains
 * visible without granting the body any ranking authority.
 *
 * @param {unknown} candidate
 * @returns {{ok: true, body: Record<string, any>, why: ''} | {ok: false, body: null, why: string}}
 */
export function validateLedgerPayload(candidate) {
  if (candidate === null || typeof candidate !== 'object' || Array.isArray(candidate)) {
    return invalidLedger('the response is not an object.');
  }
  const body = /** @type {Record<string, any>} */ (candidate);
  if (!Array.isArray(body.runs)) return invalidLedger('`runs` is not an array.');
  if (typeof body.path !== 'string') return invalidLedger('`path` is not a string.');
  if (body.refusal !== null && (typeof body.refusal !== 'string' || body.refusal.length === 0)) {
    return invalidLedger('`refusal` is neither null nor a non-empty string.');
  }
  const rungs = canonicalSignalRungs(body.signal_rungs);
  if (rungs === null) {
    return invalidLedger('`signal_rungs` is not one canonical, unique, increasing minute-rung list.');
  }

  const countFields = ['total', 'scanned', 'max_runs', 'halted', 'unsealed'];
  for (const field of countFields) {
    if (!Number.isSafeInteger(body[field]) || body[field] < 0 || Object.is(body[field], -0)) {
      return invalidLedger(`\`${field}\` is not an exact non-negative integer.`);
    }
  }
  for (const field of ['hit_scan_cap', 'partial_tail']) {
    if (typeof body[field] !== 'boolean') return invalidLedger(`\`${field}\` is not boolean.`);
  }
  if (
    body.best_complete !== null &&
    (!Number.isSafeInteger(body.best_complete) || body.best_complete < 0)
  ) {
    return invalidLedger('`best_complete` is neither null nor an exact non-negative index.');
  }

  const normalFields = [
    'version',
    'writes_version',
    'appendable',
    'commit_stamped',
    'has_mask'
  ];
  const present = normalFields.filter((field) => body[field] !== undefined).length;
  if (present === 0) {
    if (
      typeof body.refusal !== 'string' ||
      body.path !== '' ||
      body.total !== 0 ||
      body.scanned !== 0 ||
      body.max_runs < 1 ||
      body.halted !== 0 ||
      body.unsealed !== 0 ||
      body.hit_scan_cap ||
      body.partial_tail ||
      body.best_complete !== null ||
      body.runs.length !== 0
    ) {
      return invalidLedger('the versionless configuration-refusal shape is contradictory.');
    }
    return { ok: true, body, why: '' };
  }
  if (present !== normalFields.length) {
    return invalidLedger('the normal file-version fields are only partially present.');
  }
  for (const field of ['version', 'writes_version']) {
    if (
      !Number.isSafeInteger(body[field]) ||
      body[field] < 0 ||
      body[field] > U32_MAX ||
      Object.is(body[field], -0)
    ) {
      return invalidLedger(`\`${field}\` is outside the exact unsigned 32-bit domain.`);
    }
  }
  if (body.writes_version === 0) return invalidLedger('`writes_version` cannot be zero.');
  for (const field of ['appendable', 'commit_stamped', 'has_mask']) {
    if (typeof body[field] !== 'boolean') return invalidLedger(`\`${field}\` is not boolean.`);
  }
  if (body.path.length === 0) return invalidLedger('a file-backed response has an empty `path`.');
  if (body.max_runs < 1 || body.scanned > body.max_runs) {
    return invalidLedger('`max_runs` is zero or smaller than `scanned`.');
  }
  if (body.scanned !== body.runs.length || body.total < body.scanned) {
    return invalidLedger('`runs`, `scanned`, and `total` do not reconcile.');
  }
  if (body.hit_scan_cap && body.total <= body.scanned) {
    return invalidLedger('`hit_scan_cap` is true without an unread older record.');
  }
  if (body.refusal === null && body.hit_scan_cap !== (body.total > body.scanned)) {
    return invalidLedger('the successful read cap does not match `total > scanned`.');
  }
  if (body.appendable !== (body.version === body.writes_version && body.refusal === null)) {
    return invalidLedger('`appendable` contradicts the read/write versions or refusal state.');
  }
  if (body.has_mask !== (body.version >= body.writes_version && body.refusal === null)) {
    return invalidLedger('`has_mask` contradicts the ledger version or refusal state.');
  }

  const identities = new Set();
  const indexes = new Set();
  let halted = 0;
  let unsealed = 0;
  /** @type {Record<string, any> | null} */
  let best = null;
  for (let at = 0; at < body.runs.length; at += 1) {
    const run = body.runs[at];
    const admitted = validateRun(run, false);
    if (!admitted.ok) return invalidLedger(`runs[${at}] is invalid: ${admitted.why}`);
    const expectedIndex = body.total - 1 - at;
    if (run.index !== expectedIndex) {
      return invalidLedger(`runs[${at}].index is not the canonical newest-first index ${expectedIndex}.`);
    }
    if (identities.has(run.identity) || indexes.has(run.index)) {
      return invalidLedger('run identities and indexes must both be unique.');
    }
    identities.add(run.identity);
    indexes.add(run.index);
    if (run.halted) halted += 1;
    if (!run.sealed) unsealed += 1;
    if (
      run.sealed &&
      !run.halted &&
      (best === null ||
        run.pessimistic > best.pessimistic ||
        (run.pessimistic === best.pessimistic && run.index < best.index))
    ) {
      best = run;
    }
  }
  if (body.halted !== halted || body.unsealed !== unsealed) {
    return invalidLedger('the halted or unsealed summary does not match the admitted rows.');
  }
  const bestIndex = best?.index ?? null;
  if (body.best_complete !== bestIndex) {
    return invalidLedger('`best_complete` does not identify the best sealed, non-halted returned row.');
  }
  return { ok: true, body, why: '' };
}

/** @param {unknown} value */
const wholeNonNegative = (value) =>
  Number.isSafeInteger(value) && /** @type {number} */ (value) >= 0;

/**
 * The exact engine-owned rung set carried by `/backtest.json`.
 *
 * A suffix rule is not equivalent: `0min` and `garbagemin` both end in `min`
 * and neither is a rung `cli::EVERY_RUNG` can sweep.  Malformed or absent wire
 * data therefore becomes no authority rather than a hand-kept browser default.
 *
 * @param {unknown} signalRungs
 * @returns {Set<string> | null}
 */
const rungSetOf = (signalRungs) => {
  const values = signalRungs instanceof Set
    ? [...signalRungs]
    : Array.isArray(signalRungs)
      ? signalRungs
      : null;
  const admitted = canonicalSignalRungs(values);
  return admitted === null ? null : new Set(admitted);
};

/**
 * An integer subtraction the browser can represent exactly.
 *
 * Two individually safe signed integers can be farther apart than
 * `Number.MAX_SAFE_INTEGER`; returning that rounded difference would make the
 * table's delta disagree with the two exact endpoints it displays.
 *
 * @param {unknown} value
 * @param {unknown} reference
 * @returns {number | null}
 */
export const exactIntegerDelta = (value, reference) => {
  if (
    typeof value !== 'number' ||
    typeof reference !== 'number' ||
    !Number.isSafeInteger(value) ||
    !Number.isSafeInteger(reference)
  ) return null;
  const delta = value - reference;
  return Number.isSafeInteger(delta) ? delta : null;
};

/**
 * A rounded integer ratio whose endpoints and result are exactly representable.
 *
 * Dividing before scaling avoids overflowing an otherwise small ratio, but a
 * malicious denominator can still make the scaled result exceed the exact
 * integer range.  Every page-level basis-point calculation uses this door.
 *
 * @param {unknown} numerator
 * @param {unknown} denominator
 * @param {unknown} scale
 * @returns {number | null}
 */
export const roundedScaledRatio = (numerator, denominator, scale) => {
  if (
    !Number.isSafeInteger(numerator) ||
    !Number.isSafeInteger(denominator) ||
    /** @type {number} */ (denominator) <= 0 ||
    !Number.isSafeInteger(scale) ||
    /** @type {number} */ (scale) < 0
  ) return null;
  const product = BigInt(/** @type {number} */ (numerator)) * BigInt(/** @type {number} */ (scale));
  const divisor = BigInt(/** @type {number} */ (denominator));
  let rounded = product / divisor;
  const remainder = product % divisor;
  if (product >= 0n) {
    if (remainder * 2n >= divisor) rounded += 1n;
  } else if (-remainder * 2n > divisor) {
    // `Math.round(-1.5) === -1`: exact halves move toward +Infinity.
    rounded -= 1n;
  }
  const limit = BigInt(Number.MAX_SAFE_INTEGER);
  return rounded < -limit || rounded > limit ? null : Number(rounded);
};

/**
 * Support as integer basis points of the bars that were swept.
 *
 * Positive quantities make `Math.round`'s half-to-+Infinity rule identical to
 * half-away-from-zero.  Division happens before the scale so a large, valid
 * count is not made unsafe merely to express a ratio.
 *
 * @param {any} run
 * @returns {number | null}
 */
export function supportBasisPoints(run) {
  if (
    !Number.isSafeInteger(run?.bars) ||
    run.bars <= 0 ||
    !wholeNonNegative(run?.min_hits)
  ) {
    return null;
  }
  return roundedScaledRatio(run.min_hits, run.bars, 10_000);
}

/**
 * The exact canonical support ratio used as comparison identity.
 *
 * Basis points are a display unit and deliberately round. They cannot identify
 * a search: `1 / 20_000` and `2 / 20_000` both display as one basis point but
 * enumerate different frequent frontiers. Reducing the two exact integers with
 * `BigInt` keeps equivalent fractions together without merging distinct ones.
 *
 * @param {any} run
 * @returns {string | null}
 */
export function supportRatioKey(run) {
  if (
    !Number.isSafeInteger(run?.bars) ||
    run.bars <= 0 ||
    !Number.isSafeInteger(run?.min_hits) ||
    run.min_hits < 1 ||
    run.min_hits > run.bars
  ) {
    return null;
  }
  let left = BigInt(run.min_hits);
  let right = BigInt(run.bars);
  const numerator = left;
  const denominator = right;
  while (right !== 0n) {
    const remainder = left % right;
    left = right;
    right = remainder;
  }
  return `${numerator / left}/${denominator / left}`;
}

/**
 * Paisa attributable to the fill assumption, or `null` when the two totals do
 * not form the promised pessimistic <= optimistic interval.
 *
 * @param {any} run
 * @returns {number | null}
 */
export function fillAssumptionGap(run) {
  const worst = run?.pessimistic;
  const best = run?.optimistic;
  if (!Number.isSafeInteger(worst) || !Number.isSafeInteger(best) || worst > best) return null;
  const gap = best - worst;
  return Number.isSafeInteger(gap) ? gap : null;
}

/**
 * Give a ledger row one deterministic, human-readable verdict.
 *
 * The order is intentional.  A damaged run is first and a halted run second:
 * later facts may still be printed, but they must not disguise the stronger
 * reason the row cannot enter the ranking.
 *
 * @param {any} run
 * @param {unknown} signalRungs the engine-owned rung list from the wire
 * @returns {{key: string, label: string, eligible: boolean, reason: string}}
 */
export function classifyRun(run, signalRungs) {
  return classifyAgainst(run, rungSetOf(signalRungs));
}

/**
 * Classify against a set normalized once by the caller.
 *
 * @param {any} run
 * @param {Set<string> | null} signalRungs
 * @returns {{key: string, label: string, eligible: boolean, reason: string}}
 */
function classifyAgainst(run, signalRungs) {
  if (run?.sealed === false) return STATUS[COMPARISON_STATUS.INTEGRITY_FAILED];
  if (
    Number.isSafeInteger(run?.pessimistic) &&
    Number.isSafeInteger(run?.optimistic) &&
    run.pessimistic > run.optimistic
  ) {
    return STATUS[COMPARISON_STATUS.CONTRADICTORY_FILL_TOTALS];
  }
  if (!validateRunForComputation(run).ok) {
    return STATUS[COMPARISON_STATUS.MISSING_NUMERIC_RESULTS];
  }
  const displayedIntegersAreUsable = DISPLAY_INTEGER_FIELDS.every((field) =>
    wholeNonNegative(run?.[field])
  );
  const calendarFieldsAreUsable =
    run?.from_month >= 1 &&
    run?.from_month <= 12 &&
    run?.to_month >= 1 &&
    run?.to_month <= 12 &&
    run?.months_asked > 0 &&
    run?.months_found <= run?.months_asked;
  const supportCountsAreUsable = run?.bars > 0 && run?.min_hits <= run?.bars;
  if (!displayedIntegersAreUsable || !calendarFieldsAreUsable || !supportCountsAreUsable) {
    return STATUS[COMPARISON_STATUS.MISSING_NUMERIC_RESULTS];
  }
  if (run?.halted === true) return STATUS[COMPARISON_STATUS.HALTED];
  if (signalRungs === null) {
    return STATUS[COMPARISON_STATUS.SIGNAL_SURFACE_UNAVAILABLE];
  }
  if (!signalRungs.has(String(run?.timeframe ?? ''))) {
    return STATUS[COMPARISON_STATUS.NON_SIGNAL_TIMEFRAME];
  }
  if (
    run?.whole_span === false ||
    (wholeNonNegative(run?.months_found) &&
      wholeNonNegative(run?.months_asked) &&
      run.months_found < run.months_asked)
  ) {
    return STATUS[COMPARISON_STATUS.PARTIAL_SPAN];
  }
  if (run?.trades === 0) return STATUS[COMPARISON_STATUS.NO_TRADES];
  const countResultsAreUsable = run.trades > 0;
  const scalarResultsAreUsable =
    RESULT_FIELDS.every((field) => Number.isSafeInteger(run?.[field])) &&
    run.max_drawdown >= 0 &&
    supportBasisPoints(run) !== null &&
    fillAssumptionGap(run) !== null;
  if (!countResultsAreUsable || !scalarResultsAreUsable) {
    return STATUS[COMPARISON_STATUS.MISSING_NUMERIC_RESULTS];
  }
  return STATUS[COMPARISON_STATUS.RANKABLE];
}

/** @param {any} row */
const tieIndex = (row) =>
  Number.isSafeInteger(row.index) ? row.index : row.sourceIndex;

/**
 * Order rankable runs and decorate every row for a plain-language table.
 *
 * `deltas` are `this row - reference row`, so the reference is all zeroes and
 * a negative pessimistic delta says exactly how far a run trails the leader.
 * Ineligible rows receive neither a rank nor a reference/delta claim.
 *
 * @param {unknown} runs
 * @param {unknown} signalRungs the engine-owned rung list from the wire
 * @returns {any[]}
 */
export function compareRuns(runs, signalRungs) {
  const signalRungSet = rungSetOf(signalRungs);
  const rows = (Array.isArray(runs) ? runs : []).map((run, sourceIndex) => {
    const admission = validateRunForComputation(run);
    const status = classifyAgainst(run, signalRungSet);
    return {
      ...run,
      sourceIndex,
      admitted: admission.ok,
      admissionWhy: admission.why,
      status,
      eligible: status.eligible,
      supportBp: admission.ok ? supportBasisPoints(run) : null,
      fillGap: admission.ok ? fillAssumptionGap(run) : null,
      rank: null,
      reference: null,
      deltas: null
    };
  });

  const candidates = rows
    .filter((row) => row.eligible)
    .sort((a, b) => {
      if (a.pessimistic !== b.pessimistic) return a.pessimistic < b.pessimistic ? 1 : -1;
      if (tieIndex(a) !== tieIndex(b)) return tieIndex(a) < tieIndex(b) ? -1 : 1;
      return a.sourceIndex - b.sourceIndex;
    });
  const reference = candidates[0] ?? null;

  for (const row of candidates) {
    // A candidate exists only when the same filter supplied the reference.
    // The guard states that relation for both humans and static checkers.
    if (reference === null) break;
    const deltas = {
      pessimistic: exactIntegerDelta(row.pessimistic, reference.pessimistic),
      optimistic: exactIntegerDelta(row.optimistic, reference.optimistic),
      trades: exactIntegerDelta(row.trades, reference.trades),
      supportBp: exactIntegerDelta(row.supportBp, reference.supportBp),
      fillGap: exactIntegerDelta(row.fillGap, reference.fillGap),
      maxDrawdown: exactIntegerDelta(row.max_drawdown, reference.max_drawdown),
      worstTrade: exactIntegerDelta(row.worst_trade, reference.worst_trade)
    };
    if (Object.values(deltas).some((delta) => delta === null)) {
      row.status = STATUS[COMPARISON_STATUS.MISSING_NUMERIC_RESULTS];
      row.eligible = false;
      continue;
    }
    row.deltas = deltas;
  }

  const rankable = candidates.filter((row) => row.eligible);
  for (let i = 0; i < rankable.length; i += 1) {
    rankable[i].rank = i + 1;
    rankable[i].reference = i === 0;
  }
  const ineligible = rows.filter((row) => !row.eligible);
  return [...rankable, ...ineligible];
}
