/**
 * A fail-closed fold over the live sweep events returned by `/logs.json`.
 *
 * The status endpoint owns the opaque attempt number. Time is deliberately not
 * an identity: two attempts can start in the same clock tick, and a clock can
 * move backwards. Every event admitted here has to repeat that attempt in both
 * the telemetry record and its fields, then repeat the complete run context.
 *
 * `/logs.json` is newest first. The fold reverses only after validating that
 * order, so a late `rung sweeping` record cannot regress a rung that already
 * reached `done` on the wire.
 */

export type LivePhase = 'loading' | 'pricing' | 'priced' | 'done';

export interface LiveRung {
  key: string;
  rung: string;
  bars: number;
  minHits: number;
  supportPpm: number;
  done: boolean;
  why: string;
  recorded?: boolean;
  validating: boolean;
  phase: LivePhase;
  candidates: number;
  priced: number;
}

export interface ActiveSweepAttempt {
  attempt?: number;
  attempt_key?: string;
  kind: string;
  feed: string;
  underlying: string;
  from_year: number;
  from_month: number;
  to_year: number;
  to_month: number;
}

export type LiveProgress =
  | { phase: 'ready'; attempt: string; rungs: LiveRung[]; why: '' }
  | { phase: 'failed'; attempt: string | null; rungs: []; why: string };

type JsonObject = Record<string, unknown>;

interface LogRecord {
  seq: number;
  run: string;
  ts: number;
  level: string;
  target: string;
  message: string;
  cut: boolean;
  dropped_fields: number;
  fields: JsonObject;
}

interface RunContext {
  attempt: string;
  kind: string;
  feed: string;
  underlying: string;
  from_year: number;
  from_month: number;
  to_year: number;
  to_month: number;
}

const TARGET = 'cli.audit';
const START = 'sweep attempt started';
const SWEEPING = 'rung sweeping';
const GRID_ENTERED = 'exit grid entered';
// THE EVENT THE ENGINE EMITS MOST AND THE PAGE READ LEAST. `note_grid_progress`
// fires ten times per rung across the phase `cli`'s own comment calls "where a
// multi-hour sweep spends nearly all of its time" -- 87.6% of a measured
// 48-minute run. This fold discarded every one of them, so `priced` was written
// only at GRID_FINISHED and sat at 0 for that whole phase: the exact defect
// `note_grid_progress` was added to fix, still invisible one layer up.
const GRID_PROGRESS = 'exit grid progress';
const GRID_FINISHED = 'exit grid finished';
const RUNG_FINISHED = 'rung finished';
// VALIDATION BOUNDARIES, so the phase that emitted NOTHING is no longer the
// phase the page cannot see. Between the exit grid finishing and a row landing
// sit walk-forward, PBO and a bootstrap fixed at sixteen thousand trade
// re-walks; measured on the operator's own log, five of eight rungs that
// finished the grid never reached a committed result and the log could not say
// where they went. D-0504 gave those stages a boundary; this admits it.
const VALIDATION_ENTERED = 'validation stage entered';
const VALIDATION_FINISHED = 'validation stage finished';
const LIVE_MESSAGES = new Set([
  START,
  SWEEPING,
  GRID_ENTERED,
  GRID_PROGRESS,
  GRID_FINISHED,
  RUNG_FINISHED,
  VALIDATION_ENTERED,
  VALIDATION_FINISHED
]);

const object = (value: unknown): value is JsonObject =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

const safeInteger = (value: unknown): value is number =>
  typeof value === 'number' && Number.isSafeInteger(value);

const natural = (value: unknown): value is number => safeInteger(value) && value >= 0;

const positive = (value: unknown): value is number => safeInteger(value) && value > 0;

const U64_MAX = (1n << 64n) - 1n;

/** Aliases come from typed Rust u64 values before JSON number rounding. A
 * present alias must validate even when a usable legacy number also exists.
 * Unsafe numeric fields cannot establish identity; safe numeric fields still
 * have to agree, so adding an alias cannot hide a contradictory legacy ID.
 */
function exactToken(
  value: JsonObject,
  numericField: string,
  aliasField: string,
  allowZero = false
): string | null {
  const legacy = value[numericField];
  if (!Object.hasOwn(value, aliasField)) {
    return natural(legacy) && (allowZero || legacy > 0) ? String(legacy) : null;
  }
  const alias = value[aliasField];
  if (
    typeof alias !== 'string' || alias.length > 20 || !/^(0|[1-9]\d*)$/.test(alias) ||
    BigInt(alias) > U64_MAX || (!allowZero && alias === '0')
  ) return null;
  if (legacy !== undefined && (
    typeof legacy !== 'number' || !Number.isInteger(legacy) || legacy < 0 ||
    legacy > Number(U64_MAX) || (!allowZero && legacy === 0) ||
    (Number.isSafeInteger(legacy) && String(legacy) !== alias)
  )) return null;
  return alias;
}

/** The same identity guard is used by the page before it builds its log query.
 * Safe legacy numeric attempts are normalized to strings; no unsafe numeric
 * token is converted into an apparently exact key.
 */
export function liveAttemptKey(run: unknown): string | null {
  return object(run) ? exactToken(run, 'attempt', 'attempt_key') : null;
}

const nonempty = (value: unknown): value is string =>
  typeof value === 'string' && value.trim().length > 0;

const month = (value: unknown): value is number =>
  safeInteger(value) && value >= 1 && value <= 12;

function refused(why: string, attempt: string | null = null): LiveProgress {
  return { phase: 'failed', attempt, rungs: [], why };
}

function contextOf(run: unknown): { ok: true; value: RunContext } | { ok: false; why: string } {
  if (!object(run)) return { ok: false, why: 'The active sweep attempt is not an object.' };
  const attempt = liveAttemptKey(run);
  if (attempt === null) {
    return { ok: false, why: 'The active sweep has no consistent exact attempt token.' };
  }
  const kind = run.kind;
  const feed = run.feed;
  const underlying = run.underlying;
  if (!nonempty(kind)) return { ok: false, why: 'The active sweep has no valid kind.' };
  if (!nonempty(feed)) return { ok: false, why: 'The active sweep has no valid feed.' };
  if (!nonempty(underlying)) {
    return { ok: false, why: 'The active sweep has no valid underlying.' };
  }
  if (!safeInteger(run.from_year) || run.from_year < 1 || run.from_year > 9999) {
    return { ok: false, why: 'The active sweep has an invalid from_year.' };
  }
  if (!month(run.from_month)) {
    return { ok: false, why: 'The active sweep has an invalid from_month.' };
  }
  if (!safeInteger(run.to_year) || run.to_year < 1 || run.to_year > 9999) {
    return { ok: false, why: 'The active sweep has an invalid to_year.' };
  }
  if (!month(run.to_month)) {
    return { ok: false, why: 'The active sweep has an invalid to_month.' };
  }
  const from = run.from_year * 12 + run.from_month;
  const to = run.to_year * 12 + run.to_month;
  if (to < from) {
    return { ok: false, why: 'The active sweep span ends before it starts.' };
  }
  return {
    ok: true,
    value: {
      attempt,
      kind,
      feed,
      underlying,
      from_year: run.from_year,
      from_month: run.from_month,
      to_year: run.to_year,
      to_month: run.to_month
    }
  };
}

function recordsOf(payload: unknown): { ok: true; records: LogRecord[] } | { ok: false; why: string } {
  if (!object(payload)) return { ok: false, why: 'The event feed is not an object.' };
  if (!Array.isArray(payload.records)) {
    return { ok: false, why: 'The event feed has no records array.' };
  }
  if (!natural(payload.malformed) || payload.malformed !== 0) {
    return { ok: false, why: 'The event feed reports malformed records.' };
  }
  if (payload.partial_tail !== false) {
    return { ok: false, why: 'The event feed ended with a partial record.' };
  }
  if (payload.hit_scan_cap !== false) {
    return { ok: false, why: 'The event feed hit its scan cap.' };
  }
  if (typeof payload.reached_oldest !== 'boolean') {
    return { ok: false, why: 'The event feed omitted its oldest-record honesty flag.' };
  }
  if (!positive(payload.limit) || payload.records.length > payload.limit) {
    return { ok: false, why: 'The event feed has an invalid or exceeded record limit.' };
  }
  if (!Array.isArray(payload.errors) || payload.errors.some((why) => typeof why !== 'string')) {
    return { ok: false, why: 'The event feed has an invalid errors list.' };
  }
  if (payload.errors.length !== 0) {
    return { ok: false, why: 'The event feed reports a file read error.' };
  }
  if (payload.missing !== null && (!natural(payload.missing) || payload.missing !== 0)) {
    return { ok: false, why: 'The event feed reports missing events.' };
  }

  const records: LogRecord[] = [];
  for (let index = 0; index < payload.records.length; index += 1) {
    const candidate = payload.records[index];
    if (!object(candidate)) {
      return { ok: false, why: `Event ${index} is not an object.` };
    }
    const run = exactToken(candidate, 'run', 'run_key', true);
    if (
      !positive(candidate.seq) ||
      run === null ||
      !natural(candidate.ts) ||
      typeof candidate.level !== 'string' ||
      typeof candidate.target !== 'string' ||
      typeof candidate.message !== 'string' ||
      typeof candidate.cut !== 'boolean' ||
      !natural(candidate.dropped_fields) ||
      !object(candidate.fields)
    ) {
      return { ok: false, why: `Event ${index} has an invalid record shape.` };
    }
    if (candidate.cut) {
      return { ok: false, why: `Event ${index} was cut by the telemetry ceiling.` };
    }
    if (candidate.dropped_fields !== 0) {
      return { ok: false, why: `Event ${index} dropped telemetry fields.` };
    }
    records.push({ ...candidate, run } as unknown as LogRecord);
  }

  for (let index = 1; index < records.length; index += 1) {
    const newer = records[index - 1];
    const older = records[index];
    if (newer.ts < older.ts || (newer.ts === older.ts && newer.seq < older.seq)) {
      return { ok: false, why: 'The event feed is not newest first.' };
    }
  }
  return { ok: true, records };
}

function sameContext(fields: JsonObject, context: RunContext, marker: boolean): string | null {
  const expected: ReadonlyArray<keyof RunContext> = marker
    ? ['kind', 'feed', 'underlying', 'from_year', 'from_month', 'to_year', 'to_month']
    : ['feed', 'underlying', 'from_year', 'from_month', 'to_year', 'to_month'];
  for (const field of expected) {
    if (fields[field] !== context[field]) return field;
  }
  return null;
}

function rungOf(fields: JsonObject): string | null {
  return nonempty(fields.rung) ? fields.rung : null;
}

function requireCount(fields: JsonObject, name: string): number | null {
  return natural(fields[name]) ? fields[name] : null;
}

/**
 * Reduce one event-feed response for exactly one status-endpoint attempt.
 *
 * Foreign attempts are ignored, which makes an unfiltered/interleaved feed
 * safe. A live event that claims the active attempt in either token location
 * is not foreign: both tokens and every run-context field must then agree or
 * the complete response is refused.
 */
export function reduceLiveProgress(activeRun: unknown, payload: unknown): LiveProgress {
  const admittedContext = contextOf(activeRun);
  if (!admittedContext.ok) return refused(admittedContext.why);
  const context = admittedContext.value;
  const admittedRecords = recordsOf(payload);
  if (!admittedRecords.ok) return refused(admittedRecords.why, context.attempt);

  const matching: LogRecord[] = [];
  for (const record of admittedRecords.records) {
    if (record.target !== TARGET || !LIVE_MESSAGES.has(record.message)) continue;
    const fieldAttempt = exactToken(record.fields, 'attempt', 'attempt_key');
    if (fieldAttempt === null) {
      return refused(`A ${record.message} event has no consistent exact field attempt token.`, context.attempt);
    }
    const claimsCurrent = record.run === context.attempt || fieldAttempt === context.attempt;
    if (!claimsCurrent) continue;
    if (record.run !== context.attempt || fieldAttempt !== context.attempt) {
      return refused(
        `A ${record.message} event carries only one of the active attempt's two tokens.`,
        context.attempt
      );
    }
    const mismatch = sameContext(record.fields, context, record.message === START);
    if (mismatch !== null) {
      return refused(
        `A ${record.message} event disagrees with the active run's ${mismatch}.`,
        context.attempt
      );
    }
    matching.push(record);
  }

  const markers = matching.filter((record) => record.message === START);
  if (markers.length !== 1) {
    return refused(
      markers.length === 0
        ? 'The event feed does not contain this attempt\'s start marker.'
        : 'The event feed contains more than one start marker for this attempt.',
      context.attempt
    );
  }

  const chronological = [...matching].reverse();
  if (chronological[0] !== markers[0]) {
    return refused('An event for this attempt predates its start marker.', context.attempt);
  }

  const activeByRung = new Map<string, LiveRung>();
  const instances: LiveRung[] = [];
  const activity = new Map<string, number>();
  for (let index = 1; index < chronological.length; index += 1) {
    const record = chronological[index];
    const rung = rungOf(record.fields);
    if (rung === null) {
      return refused(`A ${record.message} event has no valid rung.`, context.attempt);
    }
    const held = activeByRung.get(rung);

    if (record.message === SWEEPING) {
      if (held !== undefined && !held.done) {
        return refused(`Rung ${rung} restarted before its prior step finished.`, context.attempt);
      }
      const bars = requireCount(record.fields, 'bars');
      const minHits = requireCount(record.fields, 'min_hits');
      const supportPpm = requireCount(record.fields, 'support_ppm');
      if (
        bars === null ||
        minHits === null ||
        supportPpm === null ||
        bars === 0 ||
        minHits === 0
      ) {
        return refused(`Rung ${rung} has invalid bars, min_hits, or support_ppm.`, context.attempt);
      }
      const started: LiveRung = {
        key: `${rung}:${record.seq}`,
        rung,
        bars,
        minHits,
        supportPpm,
        done: false,
        why: '',
        validating: false,
        phase: 'loading',
        candidates: 0,
        priced: 0
      };
      instances.push(started);
      activeByRung.set(rung, started);
      activity.set(started.key, index);
      continue;
    }

    if (held === undefined) {
      return refused(`${record.message} arrived before rung ${rung} started.`, context.attempt);
    }
    if (held.done) {
      return refused(`${record.message} arrived after rung ${rung} finished.`, context.attempt);
    }

    if (record.message === GRID_ENTERED) {
      if (held.phase !== 'loading') {
        return refused(`Rung ${rung} entered the exit grid out of order.`, context.attempt);
      }
      // This is deliberately NOT compared with `held.bars`. The sweeping
      // count is the signal rung (for example 100 x 60-minute bars), while the
      // grid is priced on the separately loaded 1-minute execution series
      // (for example 6,000 bars). Equating them rejects every coarse rung.
      const executionBars = requireCount(record.fields, 'execution_bars');
      const candidates = requireCount(record.fields, 'candidates');
      const validate = record.fields.validate;
      if (
        executionBars === null ||
        executionBars === 0 ||
        candidates === null ||
        candidates === 0 ||
        (validate !== 0 && validate !== 1)
      ) {
        return refused(`Rung ${rung} entered the exit grid with inconsistent counts.`, context.attempt);
      }
      held.phase = 'pricing';
      held.candidates = candidates;
      held.validating = validate === 1;
    } else if (record.message === GRID_PROGRESS) {
      if (held.phase !== 'pricing') {
        return refused(`Rung ${rung} reported exit-grid progress out of order.`, context.attempt);
      }
      const candidates = requireCount(record.fields, 'candidates');
      const priced = requireCount(record.fields, 'priced');
      // THE SAME STRICTNESS THE TWO NEIGHBOURS APPLY, plus monotonicity: a
      // decile that went BACKWARDS would mean two rungs' records had been
      // folded into one held state, and a progress bar that can retreat is a
      // progress bar nobody can read. `priced === held.priced` is allowed --
      // a decile boundary can repeat a count when nothing was admitted between
      // them -- but a fall is a refusal.
      if (
        candidates === null ||
        candidates !== held.candidates ||
        priced === null ||
        priced > candidates ||
        priced < held.priced
      ) {
        return refused(`Rung ${rung} reported inconsistent exit-grid progress.`, context.attempt);
      }
      held.priced = priced;
    } else if (record.message === VALIDATION_ENTERED || record.message === VALIDATION_FINISHED) {
      // NO STATE MACHINE HERE ON PURPOSE. These bracket three stages that run
      // AFTER pricing and before the row lands, and the fold's phases already
      // describe the sweep rather than the validation. Admitting them keeps the
      // records in the window `reduceLiveProgress` reads and lets the page show
      // that a run is alive during the stretch that used to be silent, without
      // inventing a phase the engine does not report.
      if (held.phase !== 'priced' && held.phase !== 'pricing') {
        return refused(`Rung ${rung} reported a validation stage out of order.`, context.attempt);
      }
    } else if (record.message === GRID_FINISHED) {
      if (held.phase !== 'pricing') {
        return refused(`Rung ${rung} finished the exit grid out of order.`, context.attempt);
      }
      const candidates = requireCount(record.fields, 'candidates');
      const priced = requireCount(record.fields, 'priced');
      if (
        candidates === null ||
        candidates !== held.candidates ||
        priced === null ||
        priced > candidates
      ) {
        return refused(`Rung ${rung} finished the exit grid with inconsistent counts.`, context.attempt);
      }
      held.phase = 'priced';
      held.priced = priced;
    } else if (record.message === RUNG_FINISHED) {
      const bars = requireCount(record.fields, 'bars');
      const minHits = requireCount(record.fields, 'min_hits');
      const recorded = record.fields.recorded;
      const why = record.fields.why;
      if (
        bars === null ||
        bars !== held.bars ||
        minHits === null ||
        minHits !== held.minHits ||
        (recorded !== 0 && recorded !== 1) ||
        typeof why !== 'string'
      ) {
        return refused(`Rung ${rung} finished with an invalid outcome.`, context.attempt);
      }
      if (recorded === 1 && (held.phase !== 'priced' || why !== '')) {
        return refused(`Rung ${rung} claims a recorded result without a completed grid.`, context.attempt);
      }
      if (recorded === 0 && why.trim().length === 0) {
        return refused(`Rung ${rung} refused without a reason.`, context.attempt);
      }
      held.done = true;
      held.phase = 'done';
      held.recorded = recorded === 1;
      held.why = why;
    }
    activity.set(held.key, index);
  }

  const rungs = instances.sort((left, right) => {
    const recent = (activity.get(right.key) ?? -1) - (activity.get(left.key) ?? -1);
    return recent || left.rung.localeCompare(right.rung);
  });
  return { phase: 'ready', attempt: context.attempt, rungs, why: '' };
}
