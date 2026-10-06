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
 *   busy: boolean
 * }} Pilot
 */

/** @returns {Pilot} */
export function emptyPilot() {
  return { at: 0, inFlight: null, feeds: [], state: '', surveyed: null, blockedBy: null, error: null, busy: false };
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
    busy: false
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
