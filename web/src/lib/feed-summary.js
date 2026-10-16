import { fetchAdmitted } from './admitted-fetch.js';
import { censusLag } from './store-census.js';
import { pooled } from './pooled.js';

/**
 * The states in which a census LOADED and its counters are the census's own
 * (D-3327 item 2). `capped` (near or at its capacity, or an append refused)
 * and `behind` (import days it refused and has not recounted) are a census
 * that loaded and cannot be called complete: its counts are real and are
 * read, and the state rides beside them so no page calls them the whole
 * store. Before WP-56 only `held` was read, so the first `behind` answer
 * would have been surveyed as an unreadable feed.
 */
export const LOADED_STATES = ['held', 'capped', 'behind'];

/**
 * The existing census headers describe the same manifest as the GET response.
 * HEAD keeps compatibility with the live API without transferring its full
 * inventory. Missing or damaged evidence is a refusal, never a zero count.
 * @param {string} wire
 * @param {Response} response
 * @returns {{bars: number, cells: number}}
 */
export function readFeedSummary(wire, response) {
  const { bars, cells } = readCensusSummary(wire, response);
  return { bars, cells };
}

/**
 * `readFeedSummary`, with every census header read (`censusLag`): the state,
 * generation, capacity, headroom and the import days the census refused.
 * @param {string} wire
 * @param {Response} response
 * @returns {{bars: number, cells: number, generation: number | null, lag: import('./store-census.js').CensusLag}}
 */
export function readCensusSummary(wire, response) {
  if (!response.ok) throw new Error(`Feed ${wire}: census HEAD answered HTTP ${response.status}`);
  const state = response.headers.get('x-brutex-census-state');
  const note = response.headers.get('x-brutex-census-note');
  const degraded = response.headers.get('x-brutex-census-degraded');
  if (degraded === null || degraded !== '') {
    throw new Error(`Feed ${wire}: census summary is ${degraded ? `degraded (${degraded})` : 'missing its integrity header'}`);
  }
  const lag = censusLag(response.headers);
  if (state === 'absent' && note?.startsWith(`${wire}: UNAVAILABLE`)) return { bars: 0, cells: 0, generation: null, lag };
  const count = note?.match(/^([^:]+): (0|[1-9]\d*) month\(s\), (0|[1-9]\d*) row\(s\), generation (0|[1-9]\d*)$/);
  if (state === null || !LOADED_STATES.includes(state) || !count || count[1] !== wire) {
    throw new Error(`Feed ${wire}: unavailable census summary (${note ?? state ?? 'missing headers'})`);
  }
  const [cells, bars, generation] = count.slice(2).map(Number);
  if (![cells, bars, generation].every(Number.isSafeInteger)) {
    throw new Error(`Feed ${wire}: census summary counters exceed exact browser integers`);
  }
  return { bars, cells, generation, lag };
}

/** Active reads only: simultaneous first-visit and picker surveys share work,
 * while a later refresh still revalidates the manifest's current headers.
 * @type {WeakMap<import('./ask.js').ask,Map<string,Promise<{bars:number,cells:number}>>>} */
const inFlight = new WeakMap();

/** @param {string} wire @param {import('./ask.js').ask} request */
export function readFeedHeader(wire, request) {
  let pending = inFlight.get(request);
  if (!pending) { pending = new Map(); inFlight.set(request, pending); }
  const held = pending.get(wire);
  if (held) return held;
  const reads = pending;
  const reading = (async () => {
    // A BUSY CENSUS IS ASKED AGAIN (DB-B51): a first visit's five-feed HEAD
    // survey met 429 while other tabs loaded, and one refusal threw.
    const response = await fetchAdmitted(`/store.json?feed=${encodeURIComponent(wire)}`,
      { signal: new AbortController().signal },
      (target, options) => request(target, { ...options, method: 'HEAD', cache: 'no-store', ms: 15_000 }));
    return readFeedSummary(wire, response);
  })().finally(() => {
    if (reads.get(wire) === reading) reads.delete(wire);
  });
  reads.set(wire, reading);
  return reading;
}

/**
 * @template {{wire: string, ready: boolean}} F
 * @param {F[]} list
 * @param {import('./ask.js').ask} request
 */
export function surveyFeedHeaders(list, request) {
  return pooled(list, 2, async (feed) => ({ ...feed, ...await readFeedHeader(feed.wire, request) }));
}
