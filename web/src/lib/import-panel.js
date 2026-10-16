/**
 * THE GDFL IMPORT, AS /ingest AND /audit DRAW IT (WP-61; DB-G07, DB-G22,
 * DB-A51).
 *
 * GDFL is imported by `cli gdfl-import`, journalled in `imports/gdfl.journal`,
 * and until WP-33a no route read it: the import's progress, its incomplete
 * days and the census-cap refusal reached the browser only as raw log lines,
 * /ingest said "Global Datafeeds has nothing to ingest yet" while an options
 * day landed every minute, and /audit's pulse read "still" because it watched
 * the census generation alone. `/imports.json?feed=gdfl` and the `imports`
 * block of `/audit.json?feed=gdfl` now serve the journal; this module folds
 * them into what a page draws, pure so `web/tests/import-panel.test.js` drives
 * it under `node --test`.
 *
 * WHAT IS MEASURED AND WHAT IS NOT. Counts, days, stages, reasons and the pace
 * percentiles are the server's (the importer's own `day imported` events). An
 * ETA is the days left times the measured p50 time per day: an extrapolation,
 * and every sentence that carries one says so. "Importing now" is a claim
 * about an event's age, never about a process the browser cannot see.
 */

import { stampLabel } from './dates.js';

/** How recent the importer's newest `day imported` event must be for the page to say an import is active: a stocks day takes ~3 s, an options day ~45-60 s (p50/p90 measured), so ten minutes without one is not a slow day. */
export const ACTIVE_WITHIN_MS = 10 * 60 * 1000;

/** The kinds the journal knows, in the order the importer runs them. */
export const KINDS = ['indices', 'stocks', 'options'];

/**
 * A duration in the largest two units, for a sentence.
 * @param {number} ms
 */
export function span(ms) {
  if (!Number.isFinite(ms) || ms < 0) return '—';
  // UNDER A SECOND IS SAID IN MILLISECONDS: an indices day imports in about
  // 80 ms, and "0s per day" read as no time at all.
  if (ms > 0 && ms < 1000) return `${Math.round(ms)}ms`;
  const s = Math.floor(ms / 1000);
  const d = Math.floor(s / 86400);
  const h = Math.floor((s % 86400) / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m ${s % 60}s`;
  return `${s}s`;
}

/**
 * One kind's line: where it stands and how fast it went.
 *
 * @param {any} kind one entry of `kinds`
 * @param {number} now ms
 */
export function kindLine(kind, now) {
  const pace = kind?.pace ?? null;
  const newest = Number.isFinite(pace?.newest_at_ms) ? pace.newest_at_ms : null;
  const age = newest === null ? null : Math.max(0, now - newest);
  return {
    kind: String(kind?.kind ?? ''),
    lastDay: typeof kind?.last_day === 'string' ? kind.last_day : null,
    lastVerb: typeof kind?.last_verb === 'string' ? kind.last_verb : null,
    lastLine: Number.isSafeInteger(kind?.last_line) ? kind.last_line : null,
    p50: Number.isFinite(pace?.p50_ms_per_day) ? pace.p50_ms_per_day : null,
    p90: Number.isFinite(pace?.p90_ms_per_day) ? pace.p90_ms_per_day : null,
    perHour: Number.isFinite(pace?.days_per_hour_at_p50) ? pace.days_per_hour_at_p50 : null,
    newestAt: newest,
    age,
    active: age !== null && age <= ACTIVE_WITHIN_MS
  };
}

/**
 * The extrapolated time left for `days` more days at the p50 pace, or `null`
 * when either is unknown. Always labelled as an extrapolation.
 *
 * @param {number | null} days
 * @param {number | null} p50 ms per day
 */
export function extrapolated(days, p50) {
  if (!Number.isSafeInteger(days) || days === null || days < 0 || p50 === null || !Number.isFinite(p50) || p50 <= 0) return null;
  const ms = days * p50;
  return {
    ms,
    text: `about ${span(ms)} left — EXTRAPOLATED: ${days.toLocaleString('en-IN')} day(s) times the measured p50 of ${span(p50)} per day, not a measurement of the days left`
  };
}

/**
 * The panel, from the `imports` summary (`/audit.json?feed=gdfl`'s block or
 * the same shape) and the census `store` block beside it.
 *
 * @param {any} summary
 * @param {any} store the `store` block of `/audit.json?feed=gdfl`, or null
 * @param {{ now: number, archiveTo?: string | null, optionsLeft?: number | null }} at
 *   `optionsLeft` is the archive days after the options' last day, measured
 *   off the indices' journal (`rows_in_range`), or null when not read.
 */
export function importPanel(summary, store, at) {
  if (!summary || typeof summary !== 'object') {
    return { state: 'unread', why: 'the import journal summary has not been read', kinds: [] };
  }
  if (typeof summary.error === 'string') {
    return { state: 'error', why: summary.error, kinds: [] };
  }
  const journal = summary.journal ?? {};
  if (journal.present === false) {
    return {
      state: 'absent',
      why: `no import journal at ${journal.path ?? 'a path the server did not state'}: nothing has been imported for this feed on this store`,
      kinds: []
    };
  }
  /** @type {ReturnType<typeof kindLine>[]} */
  const kinds = (Array.isArray(summary.kinds) ? summary.kinds : []).map((/** @type {any} */ k) => kindLine(k, at.now));
  const active = kinds.filter((k) => k.active);
  const newest = kinds.reduce(
    (best, k) => (k.newestAt !== null && (best === null || k.newestAt > best.newestAt) ? k : best),
    /** @type {ReturnType<typeof kindLine> | null} */ (null)
  );
  const options = kinds.find((k) => k.kind === 'options') ?? null;
  const counts = summary.counts ?? {};
  const notDone = ['incomplete', 'refused', 'open'].reduce(
    (n, k) => n + (Number.isSafeInteger(counts[k]) ? counts[k] : 0),
    0
  );
  const capacity = store?.capacity ?? null;
  const eta = options ? extrapolated(at.optionsLeft ?? null, options.p50) : null;
  return {
    state: active.length > 0 ? 'active' : 'quiet',
    why:
      active.length > 0
        ? `the importer filed a ${active.map((k) => k.kind).join(' and ')} day ${span(active[0].age ?? 0)} ago (its own 'day imported' event)`
        : newest
          ? `no 'day imported' event for ${span(newest.age ?? 0)}: the newest was ${newest.kind} at ${stampLabel(newest.newestAt ?? 0)} IST. Nothing is importing, or the importer has stopped; this page cannot see a process, only its events`
          : 'no import event is in the log the server reads',
    kinds,
    options,
    archiveTo: at.archiveTo ?? null,
    optionsLeft: at.optionsLeft ?? null,
    eta,
    counts,
    notDone,
    censusRefused: {
      count: Number.isSafeInteger(summary.census_refused?.count) ? summary.census_refused.count : 0,
      days: Array.isArray(summary.census_refused?.days) ? summary.census_refused.days : [],
      thenDone: Number.isSafeInteger(summary.census_refused_then_done)
        ? summary.census_refused_then_done
        : Number.isSafeInteger(summary.census_refused_then_done?.count)
          ? summary.census_refused_then_done.count
          : 0
    },
    newestNotDone: summary.newest_not_done ?? null,
    runs: Array.isArray(summary.runs) ? summary.runs : [],
    runsTotal: Number.isSafeInteger(summary.runs_total) ? summary.runs_total : 0,
    capacity: capacity && Number.isSafeInteger(capacity.limit)
      ? {
          limit: capacity.limit,
          committed: capacity.committed,
          free: capacity.free,
          full: capacity.full === true,
          segment: capacity.segment ?? null,
          headroom: Number.isSafeInteger(store?.headroom) ? store.headroom : capacity.free
        }
      : null,
    span: summary.span ?? null,
    notes: Array.isArray(summary.notes) ? summary.notes : []
  };
}

/**
 * One not-done day of `/imports.json`, as a row a reader can act on: the
 * day, the kind, the outcome, the stage and the reason, and the importer's
 * own words for it when the log has them.
 *
 * @param {any} row one of `/imports.json`'s `rows`
 */
export function dayRow(row) {
  const logged = Array.isArray(row?.logged) ? row.logged : [];
  const words = logged
    .map((/** @type {any} */ l) => (typeof l?.why === 'string' ? `${l.about ? `${l.about}: ` : ''}${l.why}` : null))
    .filter(Boolean);
  return {
    key: `${row?.kind}|${row?.day}`,
    kind: String(row?.kind ?? ''),
    day: String(row?.day ?? ''),
    outcome: String(row?.outcome ?? ''),
    stage: typeof row?.stage === 'string' ? row.stage : null,
    reason: typeof row?.reason === 'string' ? row.reason : null,
    files: Number.isSafeInteger(row?.files) ? row.files : null,
    failures: Number.isSafeInteger(row?.failures) ? row.failures : null,
    words: /** @type {string[]} */ (words),
    unsettled: row?.stage == null && row?.stage_from == null
  };
}

/**
 * How the census-refused days read in a sentence: the count, the first and
 * last, and how many of them were imported since (a recount or a resume).
 *
 * @param {{count: number, days: string[], thenDone: number}} refused
 */
export function censusRefusedSentence(refused) {
  if (refused.count === 0 && refused.thenDone === 0) return 'The census refused no import day.';
  const now = refused.count === 0
    ? 'No day is refused by the census now'
    : `${refused.count} day(s) are refused by the census now (${refused.days[0]} … ${refused.days[refused.days.length - 1]})`;
  return `${now}; ${refused.thenDone} day(s) the census refused earlier were imported since, by a recount or a resume.`;
}

/**
 * The `/audit` pulse's journal half (DB-G07): what moved between two answers
 * besides the census — the pull journal's records, the import journal's lines,
 * and the importer's newest event. A census held steady by a refusal while
 * the importer writes is "moving", never "still".
 *
 * @param {{ journal: number | null, importLines: number | null, importAt: number | null } | undefined} prev
 * @param {{ journal: number | null, importLines: number | null, importAt: number | null } | undefined} last
 */
export function journalStep(prev, last) {
  if (!prev || !last) return { moved: false, journal: 0, importLines: 0, importAt: null, shrank: '' };
  const known = (/** @type {number | null} */ a, /** @type {number | null} */ b) => Number.isSafeInteger(a) && Number.isSafeInteger(b);
  const delta = (/** @type {number | null} */ a, /** @type {number | null} */ b) =>
    known(a, b) ? Math.max(0, /** @type {number} */ (b) - /** @type {number} */ (a)) : 0;
  const journal = delta(prev.journal, last.journal);
  const importLines = delta(prev.importLines, last.importLines);
  const newer = last.importAt !== null && (prev.importAt === null || last.importAt > prev.importAt);
  // A JOURNAL THAT GOT SHORTER IS SAID, never clamped to "did not move": both
  // are append-only, so a smaller count is a truncated or replaced file.
  /** @type {string[]} */
  const fell = [];
  const say = (/** @type {string} */ what, /** @type {number | null} */ a, /** @type {number | null} */ b) => {
    if (known(a, b) && /** @type {number} */ (b) < /** @type {number} */ (a))
      fell.push(`${what} fell from ${Number(a).toLocaleString('en-IN')} to ${Number(b).toLocaleString('en-IN')}`);
  };
  say('the pull journal’s record count', prev.journal, last.journal);
  say('the import journal’s line count', prev.importLines, last.importLines);
  return {
    moved: journal > 0 || importLines > 0 || newer,
    journal,
    importLines,
    importAt: newer ? last.importAt : null,
    shrank: fell.join('; ')
  };
}

/**
 * The newest import event's stamp in an `/audit.json?feed=gdfl` body, or null.
 * @param {any} body
 */
export function newestImportAt(body) {
  const kinds = Array.isArray(body?.imports?.kinds) ? body.imports.kinds : [];
  let best = null;
  for (const k of kinds) {
    const at = k?.pace?.newest_at_ms;
    if (Number.isFinite(at) && (best === null || at > best)) best = at;
  }
  return best;
}

/**
 * The day after an ISO day, as ISO, or null for text that is not one.
 * @param {string | null | undefined} iso
 */
export function nextDay(iso) {
  if (typeof iso !== 'string' || !/^\d{4}-\d{2}-\d{2}$/.test(iso)) return null;
  const t = Date.parse(`${iso}T00:00:00Z`);
  if (!Number.isFinite(t)) return null;
  return new Date(t + 86_400_000).toISOString().slice(0, 10);
}

/**
 * The `/imports.json?feed=<wire>` route an imports summary names for its own
 * days, or null when it names none or names something else.
 * @param {any} summary
 * @returns {string | null}
 */
export function importRoute(summary) {
  const route = summary?.route;
  return typeof route === 'string' && /^\/imports\.json\?feed=[a-z0-9_-]+$/.test(route) ? route : null;
}

/**
 * The reads the panel needs beyond the summary: every not-done day with its
 * reason (`incomplete`, `refused`, `open`: one `/imports.json` page each) and
 * the archive days the options have still to reach, measured as the indices'
 * journalled days after the options' last day (`rows_in_range`).
 *
 * THE ROUTE IS THE SUMMARY'S OWN (`summary.route`, e.g.
 * `/imports.json?feed=<wire>`): this module names no feed, so a second
 * imported feed is read the same way. A summary that names no route is said,
 * never guessed.
 *
 * @param {(url: string, init?: any) => Promise<Response>} request
 * @param {any} summary the imports summary
 * @param {AbortSignal} [signal]
 * @returns {Promise<{ notDone: any[] | null, notDoneError: string | null, optionsLeft: number | null, more: number }>}
 */
export async function readImportExtras(request, summary, signal) {
  const route = importRoute(summary);
  if (route === null) {
    return {
      notDone: null,
      notDoneError: 'the imports summary names no /imports.json route, so its not-done days cannot be read (an API older than `imports.route`)',
      optionsLeft: null,
      more: 0
    };
  }
  /** @type {any[]} */
  const notDone = [];
  let more = 0;
  /** @type {string | null} */
  let notDoneError = null;
  for (const outcome of ['incomplete', 'refused', 'open']) {
    try {
      const r = await request(`${route}&outcome=${outcome}`, { signal });
      const body = await r.json();
      if (!r.ok) throw new Error(body?.error ?? `HTTP ${r.status}`);
      notDone.push(...(Array.isArray(body.rows) ? body.rows : []));
      const inRange = Number.isSafeInteger(body.rows_in_range) ? body.rows_in_range : 0;
      more += Math.max(0, inRange - (Array.isArray(body.rows) ? body.rows.length : 0));
    } catch (why) {
      if (signal?.aborted) throw why;
      notDoneError = `${route}&outcome=${outcome}: ${why instanceof Error ? why.message : String(why)}`;
      break;
    }
  }
  /** @type {number | null} */
  let optionsLeft = null;
  const options = Array.isArray(summary?.kinds) ? summary.kinds.find((/** @type {any} */ k) => k?.kind === 'options') : null;
  const after = nextDay(options?.last_day);
  const to = typeof summary?.span?.to === 'string' ? summary.span.to : null;
  if (after && to && after <= to) {
    try {
      const r = await request(`${route}&kind=indices&from=${after}&to=${to}`, { signal });
      const body = await r.json();
      if (r.ok && Number.isSafeInteger(body.rows_in_range)) optionsLeft = body.rows_in_range;
    } catch (why) {
      if (signal?.aborted) throw why;
      optionsLeft = null;
    }
  } else if (after && to) {
    optionsLeft = 0;
  }
  notDone.sort((a, b) => (a.day === b.day ? String(a.kind).localeCompare(String(b.kind)) : a.day < b.day ? 1 : -1));
  return { notDone: notDoneError ? null : notDone, notDoneError, optionsLeft, more };
}
