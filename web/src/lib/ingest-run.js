/**
 * WHAT /ingest SAYS ABOUT A RUN IT IS WATCHING — the run's own ask, the answer
 * to Stop, a request this page stopped waiting for, and the last run's outcome
 * (WP-61; DB-G21, DB-B28, DB-B30, DB-B31, DB-B32, DB-B37).
 *
 * Pure, so `web/tests/ingest-run.test.js` drives every branch under
 * `node --test`. The page only lays these out.
 *
 * WHY EACH EXISTS
 *
 *   adoptRun     A tab reloaded mid-run, or a second tab, measured the running
 *                pull against ITS OWN feed and selection and named the wrong
 *                feed ("Reading Global Datafeeds" over a Zerodha run), and its
 *                elapsed clock restarted at the reload. `/pull/run.json` now
 *                carries the run's own start (`startedAtMs`) and what each leg
 *                asked (`asked`); this reads them so the page shows and locks
 *                to the run's feed, rungs, universe and window (DB-B32).
 *   stopSaid     `POST /pull/run/stop` now says WHAT it stopped (`run`,
 *                `hand_pull`, `nothing`) and when it takes effect. The page
 *                printed "Nothing was stopped: the server reports no run in
 *                progress" during a per-row pull that was running (DB-B30),
 *                and promised "the next leg boundary" for a stop that now bites
 *                at the next instrument (DB-B28).
 *   waitedOut    The browser's 120 s ceiling told the operator a per-row pull
 *                "timed out and was cancelled" while the server kept pulling
 *                and writing (`detached_pull` runs to its end whoever
 *                listens). The page now says it stopped WAITING, and keeps
 *                watching the cell in flight until it clears (DB-B31).
 *   lastRun      The outcome of the last run was on no page: a run that
 *                refused 223,900 members is 1,120 pages of member failures
 *                after its own record. `/audit.json`'s `last_runs` names the
 *                newest run records in O(1), and this folds an `OPENED`
 *                record with its closing one into a sentence (DB-G21).
 */

import { stampLabel } from './dates.js';
import { spanText } from './ingest-status.js';

/** @param {unknown} v @returns {v is string} */
const text = (v) => typeof v === 'string' && v !== '';
/** @param {unknown} v @returns {string | null} */
const isoDay = (v) => (typeof v === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(v) ? v : null);

/**
 * THE RUN'S OWN ASK, read from `/pull/run.json` (DB-B32).
 *
 * `vendors`, `rungs`, the window and the target are what the legs themselves
 * name; a field the legs disagree on is `null` and `mixed` lists it, so the
 * page states "legs differ" rather than showing the first leg's value as the
 * run's. `known` is `false` for a document from an API older than this field
 * (no `asked`): the page then says it cannot name the run's ask, and falls
 * back to nothing — never to its own selection presented as the run's.
 *
 * @param {any} doc `/pull/run.json`
 * @returns {{
 *   known: boolean, startedAtMs: number | null, vendors: string[], rungs: string[],
 *   from: string | null, to: string | null, target: string | null,
 *   members: number | null, mixed: string[], legs: number
 * }}
 */
export function adoptRun(doc) {
  const asked = Array.isArray(doc?.asked) ? doc.asked.filter((/** @type {any} */ leg) => leg && typeof leg === 'object') : [];
  const startedAtMs =
    typeof doc?.startedAtMs === 'number' && Number.isSafeInteger(doc.startedAtMs) && doc.startedAtMs > 0
      ? doc.startedAtMs
      : null;
  if (asked.length === 0) {
    return { known: false, startedAtMs, vendors: [], rungs: [], from: null, to: null, target: null, members: null, mixed: [], legs: 0 };
  }
  /** @type {string[]} */
  const mixed = [];
  /** @template T @param {string} name @param {(leg: any) => T} of @returns {T | null} */
  const one = (name, of) => {
    const values = [...new Set(asked.map(of))];
    if (values.length === 1) return values[0];
    mixed.push(name);
    return null;
  };
  const vendors = [...new Set(asked.map((/** @type {any} */ leg) => leg.vendor).filter(text))];
  const rungs = [...new Set(asked.map((/** @type {any} */ leg) => leg.dir).filter(text))];
  const from = one('from', (leg) => isoDay(leg.from));
  const to = one('to', (leg) => isoDay(leg.to));
  const target = one('target', (leg) => (text(leg.target) ? leg.target : null));
  const members = one('members', (leg) => (Number.isSafeInteger(leg.members) && leg.members >= 0 ? leg.members : null));
  return { known: true, startedAtMs, vendors, rungs, from, to, target, members, mixed, legs: asked.length };
}

/**
 * The adopted run in one sentence, for the run card.
 *
 * @param {ReturnType<typeof adoptRun>} run
 * @param {(wire: string) => string} nameOf
 */
export function adoptedSentence(run, nameOf) {
  if (!run.known) {
    return 'This run was already going when this page loaded. /pull/run.json on this server does not say what it asked for (an API older than the field), so this page cannot name its feed, timeframes or window, and does not present its own selection as the run’s.';
  }
  const feeds = run.vendors.map(nameOf).join(', ') || 'a feed the run does not name';
  const window =
    run.from && run.to ? `${run.from} to ${run.to}` : `a window that differs between its legs${run.mixed.length ? ` (${run.mixed.join(', ')})` : ''}`;
  const who =
    run.members === null
      ? 'instruments that differ between its legs'
      : run.members === 0
        ? `every member of ${run.target ?? 'its target'}`
        : `${run.members} named instrument(s) of ${run.target ?? 'its target'}`;
  const started = run.startedAtMs === null ? 'at a time the server did not record' : `at ${stampLabel(run.startedAtMs)} IST`;
  return `This run was started ${started}, from this tab before a reload or from another tab. It asks ${feeds} for ${run.rungs.join(', ') || 'rungs it does not name'} over ${window}, ${who}, in ${run.legs} leg(s). The form above shows the run’s own selection and is locked to it until the run ends.`;
}

/**
 * WHAT `POST /pull/run/stop` SAID, as the sentence the page prints.
 *
 * `stopped` is whether anything was asked to stop. The words are the server's
 * own `when` where it sent one; an older API (no `what`) is read by its
 * `stopping` flag alone and says so.
 *
 * @param {any} said the response body
 * @returns {{ stopped: boolean, what: 'run' | 'hand_pull' | 'nothing' | 'unlabelled', text: string }}
 */
export function stopSaid(said) {
  const stopping = said?.stopping === true;
  const what = said?.what === 'run' || said?.what === 'hand_pull' || said?.what === 'nothing' ? said.what : 'unlabelled';
  const when = text(said?.when) ? said.when : null;
  if (stopping) {
    return {
      stopped: true,
      what,
      text:
        what === 'hand_pull'
          ? `Stop was taken: ${when ?? 'the single hand pull in flight stops at its next instrument; bars already landed stay'}.`
          : what === 'run'
            ? `Stop was taken: ${when ?? 'the run stops at its next instrument and starts no further leg or pass; bars already landed stay'}.`
            : 'Stop was taken. This server did not say what it stopped or when it takes effect (an API older than that answer): a run stops between legs there, which can be long.'
    };
  }
  return {
    stopped: false,
    what: what === 'unlabelled' ? 'unlabelled' : 'nothing',
    text: `Nothing was stopped: ${when ?? 'the server reports no run in progress. A single per-row pull is not stoppable on an API older than WP-61, and runs to its end on the server'}. The status below is the reading that matters.`
  };
}

/**
 * Whether an error is `$lib/ask.js`'s own ceiling (the request outlived `ms`),
 * as opposed to a refused connection or the operator's abort.
 * @param {unknown} error
 */
export function isAskTimeout(error) {
  return error instanceof Error && /timed out and was cancelled/.test(error.message);
}

/**
 * THE SENTENCE FOR A PULL THIS PAGE STOPPED WAITING FOR (DB-B31).
 *
 * The request was cancelled in the BROWSER; the pull was not. `POST /pull/spot`
 * is detached on the server and runs to its end whoever is listening, so the
 * truthful statement is that this page stopped waiting, and what it does next.
 *
 * @param {string} route
 * @param {number} ms the ceiling that passed
 * @param {string} label the leg's label
 */
export function waitedOut(route, ms, label) {
  return `This page stopped waiting for ${label} after ${Math.round(ms / 1000)} s, which is its own ceiling for one answer from ${route}. The pull was NOT cancelled: the server runs it to its end whoever is listening, and keeps writing what lands. This page is now watching /ingest/status.json until that pull clears, and measures the store only then; pressing Pull again before it does would be refused, because the feed's seat is held.`;
}

/**
 * THE LAST RUN, from `/audit.json`'s `last_runs` (newest first).
 *
 * A run that reached the vendor is TWO run records: `OPENED` when it began,
 * then its closing record; its member failures sit between them (after the
 * `OPENED` record). A request refused at the door is one record. So:
 *
 *   newest is `OPENED`           -> `open`: in flight, or its process stopped
 *                                   before it wrote a closing record.
 *   newest closes an `OPENED`    -> `closed`, with the opening's start and the
 *                                   member failures that followed it.
 *   newest stands alone          -> `closed`, a refusal or a legacy record.
 *
 * `failures` is the closing record's own count; `membersAfter` is how many
 * records follow the opening (or, for a legacy run, the closing) record up to
 * the next run. They are separate measurements and both are shown.
 *
 * @param {any} body an `/audit.json` body
 * @returns {{
 *   state: 'unread' | 'none' | 'open' | 'closed',
 *   tone: 'up' | 'warn' | 'down' | 'info',
 *   outcome: string | null, sentence: string,
 *   at: number | null, startedAt: number | null, tookMs: number | null,
 *   failures: number | null, membersAfter: number | null, barsStored: number | null,
 *   requests: number | null, window: string | null, source: string | null, note: string | null, ordinal: number | null,
 *   error: string | null
 * }}
 */
export function lastRun(body) {
  const blank = {
    outcome: null, at: null, startedAt: null, tookMs: null, failures: null, membersAfter: null,
    barsStored: null, requests: null, window: null, source: null, note: null, ordinal: null, error: null
  };
  if (!body || typeof body !== 'object') {
    return { ...blank, state: 'unread', tone: 'info', sentence: 'The pull journal has not been read yet.' };
  }
  if (text(body.last_runs_error)) {
    return { ...blank, state: 'unread', tone: 'down', error: body.last_runs_error, sentence: `The last run could not be read from the pull journal: ${body.last_runs_error}` };
  }
  if (!Array.isArray(body.last_runs)) {
    return {
      ...blank, state: 'unread', tone: 'warn',
      sentence: 'This server’s /audit.json does not list the newest runs (an API older than `last_runs`), so the last run’s outcome is not shown here. /audit pages through the journal.'
    };
  }
  const runs = body.last_runs.filter((/** @type {any} */ r) => r && r.record && !r.record.fault);
  if (runs.length === 0) {
    const scope = body.journal?.feed_scope;
    return {
      ...blank, state: 'none', tone: 'info',
      sentence: text(scope?.holds) && scope?.runs === 0 ? `No run of this feed is in the pull journal. ${scope.holds}` : 'No run of this feed is in the pull journal.'
    };
  }
  const newest = runs[0];
  const r = newest.record;
  /** @param {any} rec */
  const secs = (rec) => (typeof rec?.at === 'number' && Number.isFinite(rec.at) ? rec.at : null);
  const members = (/** @type {any} */ run) => (Number.isSafeInteger(run?.members_after) ? run.members_after : null);
  const common = {
    outcome: text(r.outcome) ? r.outcome : null,
    at: secs(r),
    window: text(r.window) ? r.window : null,
    source: text(r.source) ? r.source : null,
    note: text(r.note_whole) ? r.note_whole : text(r.note) ? r.note : null,
    ordinal: Number.isSafeInteger(r.ordinal) ? r.ordinal : null,
    barsStored: Number.isSafeInteger(r.bars_stored) ? r.bars_stored : null,
    failures: Number.isSafeInteger(r.failures) ? r.failures : null,
    requests: Number.isSafeInteger(r.members) ? r.members : null,
    error: null
  };
  const when = (/** @type {number | null} */ at) => (at === null ? 'an unrecorded time' : `${stampLabel(at * 1000)} IST`);
  if (r.outcome === 'OPENED') {
    const after = members(newest);
    return {
      ...common, state: 'open', tone: 'warn', startedAt: common.at, tookMs: null, membersAfter: after,
      sentence: `The newest run of this feed OPENED at ${when(common.at)}${common.window ? ` over ${common.window}` : ''} and has no closing record${after === null ? '' : `; ${after.toLocaleString('en-IN')} member failure(s) follow it so far`}. It is in flight right now (see the cell in flight above), or its process stopped before it ended: the journal alone cannot tell the two apart.`
    };
  }
  // A CLOSING RECORD'S OPENING IS THE RUN RECORD JUST BEFORE IT, when that one
  // is `OPENED` over the same window: the two were written by one run.
  const before = runs[1]?.record;
  const opening = before && before.outcome === 'OPENED' && before.window === r.window ? runs[1] : null;
  const startedAt = opening ? secs(opening.record) : null;
  const membersAfter = opening ? members(opening) : members(newest);
  const tookMs =
    startedAt !== null && common.at !== null && common.at >= startedAt
      ? (common.at - startedAt) * 1000
      : typeof r.took_micros === 'number' && r.took_micros > 0
        ? Math.round(r.took_micros / 1000)
        : null;
  const good = r.outcome === 'STORED';
  const mid = r.outcome === 'STORED NOTHING';
  const parts = [
    `The last run of this feed ended ${common.outcome ?? 'with an outcome this build cannot name'} at ${when(common.at)}`,
    startedAt !== null ? `having opened at ${when(startedAt)}` : null,
    tookMs !== null ? `(${spanText(tookMs)})` : null,
    common.window ? `over ${common.window}` : null
  ].filter(Boolean);
  const counts = [
    common.requests !== null && common.requests > 0 ? `${common.requests.toLocaleString('en-IN')} member request(s)` : null,
    common.barsStored !== null ? `${common.barsStored.toLocaleString('en-IN')} bar(s) stored` : null,
    common.failures !== null ? `${common.failures.toLocaleString('en-IN')} failure(s) by its own count` : null,
    membersAfter !== null ? `${membersAfter.toLocaleString('en-IN')} member-failure record(s) journalled after its ${opening ? 'opening' : 'own'} record` : null
  ].filter(Boolean);
  return {
    ...common, state: 'closed', tone: good ? 'up' : mid ? 'warn' : 'down', startedAt, tookMs, membersAfter,
    sentence: `${parts.join(' ')}: ${counts.join(', ') || 'no counts recorded'}.${common.note ? ` Its note: ${common.note}` : ''}`
  };
}
