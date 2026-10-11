// What /ingest holds from `/ingest/status.json`, and what it must say about it.
//
// CE-79, D-1787. The page read `in_flight`, `waiting_on` and `state` and
// nothing else. `surveyed:false` with an empty `waiting_on` -- the api's
// explicit "nothing surveyed, not nothing outstanding" -- folded to `[]`,
// which reads as "no feed halted", so a halted feed's missing months were
// labelled short or never instead of fail. The 503 body's `error` (a poisoned
// status lock) was thrown away, `pilot.error` had no render site, and a failed
// read kept the previous `inFlight` and `feeds`, so the retry and fail verdicts
// went on being decided from an answer the server had stopped giving.
//
// This module is the one fold. A refused or failed read CLEARS the flight and
// the feed list (the previous answer is not current), names the reason, and
// marks the survey unknown. `blocked_by` is carried verbatim.
//
// WP-61 (DB-G20, DB-G21, DB-B29, DB-B39). The route now names the cell in
// flight with its feed, timeframe, window, driver, position and measured pace
// (WP-34, D-3341), and this module is where the page reads them:
//
//   * `flightOf` / `flightRow` / `flightMonth`: ONLY the row the cell names is
//     "in flight" -- the same feed, timeframe, spot segment and instrument --
//     never every row whose symbol matches, in every feed and rung.
//   * `flightPace`: the run's measured pace and an ETA that is always labelled
//     an extrapolation, in the server's own words for its basis.
//   * `stallVerdict`: "stalled" is judged against the measured time per
//     instrument, not a fixed 20 s that a healthy 45 s instrument always trips.
//   * `busyNotice`: an idle tab says a pull is going on this server, who
//     drives it and what stops it, instead of offering a Pull that is refused.

import { refusalSentence } from './refusal.js';

/**
 * @typedef {{
 *   at: number,
 *   inFlight: any,
 *   feeds: any[],
 *   state: string,
 *   surveyed: boolean | null,
 *   blockedBy: string | null,
 *   error: string | null,
 *   busy: boolean,
 *   seat: string | null,
 *   paused: boolean | null
 * }} Pilot
 */

/** @returns {Pilot} */
export function emptyPilot() {
  return { at: 0, inFlight: null, feeds: [], state: '', surveyed: null, blockedBy: null, error: null, busy: false, seat: null, paused: null };
}

/**
 * A read that did not produce an answer. Nothing from the previous answer
 * survives: the flight, the halts and the survey are all unknown now.
 * @param {string} why
 * @returns {Pilot}
 */
export function failedPilot(why) {
  return { ...emptyPilot(), error: why };
}

/**
 * Fold one `/ingest/status.json` response (status plus parsed body, or
 * `undefined` when the body was not JSON).
 * @param {number} status
 * @param {unknown} body
 * @param {number} now
 * @returns {Pilot}
 */
export function foldIngestStatus(status, body, now) {
  const j = /** @type {Record<string, any> | null} */ (
    body !== null && typeof body === 'object' && !Array.isArray(body) ? body : null
  );
  if (status < 200 || status > 299) {
    const reason = j && typeof j.error === 'string' && j.error.trim() !== '' ? j.error.trim() : null;
    return failedPilot(refusalSentence('/ingest/status.json', status, reason));
  }
  if (j === null) return failedPilot('/ingest/status.json answered with a body that is not a JSON object');
  if (typeof j.surveyed !== 'boolean') {
    return failedPilot('/ingest/status.json did not say whether a round has been surveyed (`surveyed` is missing), so an empty feed list cannot be read as "nothing halted"');
  }
  return {
    at: now,
    inFlight: j.in_flight ?? null,
    feeds: Array.isArray(j.waiting_on) ? j.waiting_on : [],
    state: String(j.state ?? ''),
    surveyed: j.surveyed,
    blockedBy: typeof j.blocked_by === 'string' && j.blocked_by.trim() !== '' ? j.blocked_by.trim() : null,
    error: null,
    busy: false,
    seat: typeof j.pull_seat === 'string' ? j.pull_seat : null,
    paused: typeof j.paused === 'boolean' ? j.paused : null
  };
}

/**
 * The notice to draw beside the census table, or null when the ladder answer
 * is current and surveyed. `bad` is a failed read; `warn` is an answer that
 * cannot say whether a feed halted.
 * @param {Pilot} pilot
 * @returns {{ tone: 'bad' | 'warn', head: string, text: string } | null}
 */
export function pilotNotice(pilot) {
  if (pilot.error) {
    return {
      tone: 'bad',
      head: 'The backfill ladder could not be read',
      text: `${pilot.error}. Whether a feed has halted is unknown, so no row below is marked failed from it and none is marked retrying.`
    };
  }
  if (pilot.surveyed === false) {
    return {
      tone: 'warn',
      head: 'Nothing surveyed yet — not nothing outstanding',
      text: `${pilot.blockedBy ?? 'The server did not say why.'} Until a round is surveyed, a halted feed cannot be shown on any row below.`
    };
  }
  return null;
}

/* ======================================================================
   THE CELL IN FLIGHT (WP-61; DB-G20, DB-G21, DB-B29, DB-B39)
   ====================================================================== */

/**
 * A duration in its largest two units, for a sentence: `2h 05m`, `3m 20s`,
 * `45s`. An unknown is an em dash, never a zero.
 * @param {unknown} ms
 */
export function spanText(ms) {
  if (typeof ms !== 'number' || !Number.isFinite(ms) || ms < 0) return '—';
  const s = Math.round(ms / 1000);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (h > 0) return `${h}h ${String(m).padStart(2, '0')}m`;
  if (m > 0) return `${m}m ${String(s % 60).padStart(2, '0')}s`;
  return `${s}s`;
}

/** @param {unknown} v @returns {number | null} */
const whole = (v) => (typeof v === 'number' && Number.isSafeInteger(v) && v >= 0 ? v : null);
/** @param {unknown} v @returns {string | null} */
const isoDay = (v) => (typeof v === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(v) ? v : null);

/**
 * THE CELL IN FLIGHT, normalised from `in_flight`, or `null` when the server
 * names none or names one this page cannot read (no instrument).
 *
 * `feed` and `timeframe` are empty strings on an API older than WP-34; a row
 * is then matched on neither, so such a cell marks NO row rather than every
 * row of its symbol (the DB-G20 defect), and `flightPace` says the cell is
 * unlabelled.
 *
 * @typedef {{
 *   instrument: string, feed: string, timeframe: string, driver: string,
 *   from: string | null, to: string | null, month: string,
 *   index: number | null, of: number | null,
 *   elapsedMs: number | null, runElapsedMs: number | null,
 *   perInstrumentMs: number | null, etaMs: number | null, etaBasis: string
 * }} Flight
 *
 * @param {unknown} cell `in_flight` as `/ingest/status.json` sent it
 * @returns {Flight | null}
 */
export function flightOf(cell) {
  if (cell === null || typeof cell !== 'object' || Array.isArray(cell)) return null;
  const c = /** @type {Record<string, any>} */ (cell);
  const instrument = typeof c.instrument === 'string' ? c.instrument.trim() : '';
  if (instrument === '') return null;
  const window = c.window !== null && typeof c.window === 'object' ? c.window : null;
  return {
    instrument,
    feed: typeof c.feed === 'string' ? c.feed : '',
    timeframe: typeof c.timeframe === 'string' ? c.timeframe : '',
    driver: typeof c.driver === 'string' ? c.driver : '',
    from: isoDay(window?.from),
    to: isoDay(window?.to),
    month: typeof c.month === 'string' ? c.month : '',
    index: whole(c.index),
    of: whole(c.of),
    elapsedMs: whole(c.elapsed_ms),
    runElapsedMs: whole(c.run_elapsed_ms),
    perInstrumentMs: whole(c.per_instrument_ms),
    etaMs: whole(c.eta_ms),
    etaBasis: typeof c.eta_basis === 'string' ? c.eta_basis : ''
  };
}

/** The bare symbol of an instrument spelling: `NSE-INDEX-NIFTY` -> `NIFTY`. */
const bare = (/** @type {string} */ text) => {
  const parts = String(text ?? '').split('-');
  return (parts.length > 2 ? parts.slice(2).join('-') : String(text ?? '')).toUpperCase();
};

/**
 * WHETHER THE CELL IN FLIGHT IS THIS ROW (DB-G20).
 *
 * All four must hold: the row's feed is the cell's, its timeframe is the
 * cell's, its segment is spot (only `POST /pull/spot`'s loop publishes a cell,
 * `server.rs` `cell_of`), and its instrument is the cell's. A row of the same
 * symbol under another feed, another rung or a derivative segment is NOT in
 * flight; it used to be marked "retrying" all the same.
 *
 * @param {Flight | null} flight
 * @param {{ feed: string | null | undefined, seg: string, tf: string, sym: string, key?: string, mkey?: string }} row
 */
export function flightRow(flight, row) {
  if (!flight || !row) return false;
  if (flight.feed === '' || flight.timeframe === '') return false;
  if (!row.feed || row.feed !== flight.feed) return false;
  if (row.seg !== 'spot') return false;
  if (row.tf !== flight.timeframe) return false;
  const want = bare(flight.instrument);
  return [row.sym, row.key, row.mkey].some((name) => typeof name === 'string' && name !== '' && bare(name) === want);
}

/**
 * Whether a month file lies inside the window the cell is fetching. A cell
 * with no window (an API older than WP-34) names its one `month`.
 *
 * @param {Flight | null} flight
 * @param {string} ym `YYYY-MM`
 */
export function flightMonth(flight, ym) {
  if (!flight || typeof ym !== 'string') return false;
  if (flight.from && flight.to) return ym >= flight.from.slice(0, 7) && ym <= flight.to.slice(0, 7);
  return flight.month === ym;
}

/**
 * THE RUN'S PACE, AS THE SERVER MEASURED IT, and an ETA that is never shown
 * without the word (DB-G21, `CLAUDE.md` §3 rule 6).
 *
 * `share` is the leg's own fraction: instruments FINISHED over instruments in
 * the leg (`index` is the one being fetched, so `index - 1` are done). `eta`
 * is the server's `eta_ms` -- the instruments left times the mean of those
 * done -- and its sentence carries the server's own `eta_basis`.
 *
 * @param {Flight | null} flight
 * @returns {{
 *   where: string, share: number | null, done: number | null, of: number | null,
 *   pace: string | null, eta: string | null, etaMs: number | null, labelled: boolean
 * } | null}
 */
export function flightPace(flight) {
  if (!flight) return null;
  const labelled = flight.feed !== '' && flight.timeframe !== '';
  const done = flight.index === null ? null : Math.max(0, flight.index - 1);
  const share = done !== null && flight.of !== null && flight.of > 0 ? Math.min(1, done / flight.of) : null;
  const where =
    flight.index !== null && flight.of !== null
      ? `${flight.instrument} — instrument ${flight.index} of ${flight.of}`
      : flight.instrument;
  const pace =
    flight.perInstrumentMs === null
      ? null
      : `${spanText(flight.perInstrumentMs)} per instrument, measured: the run's ${spanText(flight.runElapsedMs)} so far over the ${done} finished`;
  const eta =
    flight.etaMs === null
      ? null
      : `about ${spanText(flight.etaMs)} left for this leg — EXTRAPOLATED${flight.etaBasis ? ` (${flight.etaBasis})` : ', not measured'}`;
  return { where, share, done, of: flight.of, pace, eta, etaMs: flight.etaMs, labelled };
}

/** A run with no measured pace is "stalled" only after this long with no progress. */
export const STALL_FLOOR_MS = 120_000;
/** How many measured per-instrument times with no progress count as a stall. */
export const STALL_FACTOR = 3;

/**
 * WHETHER A RUN HAS STALLED, judged against its own measured cadence (DB-B39).
 *
 * The page said "stalled" after 20 s without census growth. A 1min stock leg
 * lands once per INSTRUMENT and one instrument took 44.7 s on average (journal
 * ordinal 223936: 9,300 s over 208), so a healthy pull was "stalled" most of
 * the time. Progress is now any change the server reports -- the instrument in
 * flight, its position, a leg finishing, rows landing -- and the threshold is
 * `STALL_FACTOR` times the measured time per instrument, never under
 * `STALL_FLOOR_MS`.
 *
 * @param {{ sinceProgressMs: number, perInstrumentMs: number | null }} at
 * @returns {{ stalled: boolean, afterMs: number, why: string }}
 */
export function stallVerdict(at) {
  const per = typeof at.perInstrumentMs === 'number' && at.perInstrumentMs > 0 ? at.perInstrumentMs : null;
  const afterMs = Math.max(STALL_FLOOR_MS, per === null ? 0 : per * STALL_FACTOR);
  const stalled = Number.isFinite(at.sinceProgressMs) && at.sinceProgressMs >= afterMs;
  const rule =
    per === null
      ? `no pace is measured yet, so the threshold is the floor of ${spanText(STALL_FLOOR_MS)}`
      : `${STALL_FACTOR} times the measured ${spanText(per)} per instrument, and never under ${spanText(STALL_FLOOR_MS)}`;
  return {
    stalled,
    afterMs,
    why: `Nothing the server reports has moved for ${spanText(at.sinceProgressMs)}: not the instrument in flight, not its position, not a leg, not a landed row. Called stalled after ${spanText(afterMs)} (${rule}). The request is still open; this is what a waiting rate governor looks like from outside, and also what a slow vendor looks like.`
  };
}

/**
 * What changes when a run makes progress, as one string: two readings that
 * differ are progress (DB-B39). Built from what the server reports only.
 *
 * @param {Flight | null} flight
 * @param {{ feeds?: Array<{ legsDone?: number }>, rowsNow?: number | null, passes?: number } | null | undefined} run
 */
export function progressKey(flight, run) {
  const legs = Array.isArray(run?.feeds) ? run.feeds.map((f) => f?.legsDone ?? 0).join(',') : '';
  return [flight?.feed ?? '', flight?.timeframe ?? '', flight?.instrument ?? '', flight?.index ?? '', legs, run?.rowsNow ?? '', run?.passes ?? ''].join('|');
}

/**
 * WHAT AN IDLE TAB MUST SAY ABOUT A PULL IT DID NOT START (DB-B29).
 *
 * An idle /ingest tab never asked whether a pull was going, so it offered Pull
 * and a per-row "Pull N" that the server then refused with a 409. This reads
 * the two documents an idle tab now polls slowly and says who is pulling and
 * what stops it. `blocksPull` is true when this feed's seat is the one in use,
 * so the page disables its own Pull with this sentence as the reason.
 *
 * @param {Pilot} pilot
 * @param {{ running?: boolean, asked?: Array<{ vendor?: string }>, feeds?: Array<{ vendor?: string }> } | null | undefined} run `/pull/run.json`
 * @param {string | null | undefined} feed the feed this page is on
 * @param {(wire: string) => string} [nameOf]
 * @returns {{ tone: 'info' | 'warn', head: string, text: string, blocksPull: boolean, driver: string } | null}
 */
export function busyNotice(pilot, run, feed, nameOf = (wire) => wire) {
  const flight = flightOf(pilot?.inFlight);
  const running = run?.running === true;
  const runFeeds = running
    ? [...new Set([...(run?.asked ?? []), ...(run?.feeds ?? [])].map((f) => f?.vendor).filter((v) => typeof v === 'string' && v !== ''))]
    : [];
  if (!flight && !running) return null;
  const pace = flightPace(flight);
  const cell = flight
    ? `${nameOf(flight.feed) || 'a feed this API does not name'} · ${flight.timeframe || 'timeframe not named'} · ${pace?.where ?? flight.instrument}`
    : 'between two instruments or legs';
  if (running) {
    const mine = Boolean(feed) && runFeeds.includes(/** @type {string} */ (feed));
    return {
      tone: 'info',
      head: 'A hand run is going on this server',
      text: `It was started from /ingest (this tab or another) over ${runFeeds.map(nameOf).join(', ') || 'feeds /pull/run.json does not name'}. Now: ${cell}. Stop on this page (POST /pull/run/stop) stops it at its next instrument; pausing the autopilot does not touch it.`,
      blocksPull: mine || runFeeds.length === 0,
      driver: 'run'
    };
  }
  const f = /** @type {Flight} */ (flight);
  const mine = Boolean(feed) && f.feed === feed;
  if (f.driver === 'hand') {
    return {
      tone: 'info',
      head: 'A single hand pull is in flight on this server',
      text: `${cell}. It was sent by a row's Pull (POST /pull/spot), from this tab or another, and it runs to its end on the server whoever is listening. Stop on this page stops it at its next instrument.${mine ? ' A second pull of this feed would be refused: its seat is held.' : ''}`,
      blocksPull: mine,
      driver: 'hand'
    };
  }
  return {
    tone: 'warn',
    head: `The ${f.driver || 'autopilot'} is pulling on this server`,
    text: `${cell}. ${mine ? "A hand pull of this feed would be refused while it holds the feed's seat: pause it at /autopilot first (it resumes from wherever the store reaches). " : ''}Stop on this page does not touch it.`,
    blocksPull: mine,
    driver: f.driver || 'autopilot'
  };
}
