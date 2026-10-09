import { ask } from './ask.js';
import { decodeCensus } from './store-census-wire.js';
import { fetchWithBusyRetry } from '../../saved-backtest/requests.js';
import { reasonOf } from './refusal.js';

export const STORE_CENSUS_MS = 30_000;

/**
 * `reason` is the server's own sentence for a refused read; `degraded` is the
 * `x-brutex-census-degraded` header when it is non-empty.
 * @typedef {{ok: boolean, status: number, body: any[] | null, headers: Headers,
 *   reason?: string | null, degraded?: string | null}} CensusAnswer
 */

export const CENSUS_DEGRADED_HEADER = 'x-brutex-census-degraded';

/**
 * A DEGRADED CENSUS IS NOT THE CURRENT STORE, AND IS REFUSED AS ONE (CE-77,
 * D-1786). The server answers 200 for a manifest whose newest generation
 * failed its checksum, with the generation it could still recover and the
 * reason in this header (`census.rs`, `VendorCensus::degraded`): a month a
 * later generation committed is not in those rows. Every selected-feed reader
 * folded them as the store with no mark, so /db, /markets, /terminal and the
 * backtest form showed committed months as never pulled. The HEAD survey
 * (`feed-summary.js`) already refused it; the GET path now does the same.
 * @param {Headers} headers
 * @returns {string | null} the refusal sentence, or null when not degraded
 */
export function degradedRefusal(headers) {
  const degraded = headers.get(CENSUS_DEGRADED_HEADER);
  if (degraded === null || degraded.trim() === '') return null;
  return `the census is DEGRADED (${degraded.trim()}): the server could only recover an older ` +
    'generation of the manifest, so months a later generation committed are missing from it. ' +
    'It is refused rather than drawn as the current store';
}

/**
 * Share the request AND the parsed body. Keep at most one completed selected
 * feed answer; explicit all-feed surveys share flights but retain only totals.
 * A new selection/generation prevents a late answer from replacing that cache.
 * Consumers must treat the shared rows as read-only.
 *
 * @param {typeof ask} [request]
 * @param {number} [ceilingMs]
 * @param {(info:{attempt:number,nextAttempt:number,delayMs:number,status:number})=>void} [onBusy]
 */
export function createCensusLoader(request = ask, ceilingMs = STORE_CENSUS_MS, onBusy = () => {}) {
  /** @type {Map<string, Promise<CensusAnswer>>} */
  const flights = new Map();
  /** @type {{key: string, feed: string, answer: CensusAnswer} | null} */
  let cached = null;
  /** @type {string | null} */
  let retainedKey = null;

  /** @param {string} feed @param {CensusAnswer | null} previous @param {string} key @returns {Promise<CensusAnswer>} */
  async function fetchBody(feed, previous, key) {
    const url = `/store.json?feed=${encodeURIComponent(feed)}&encoding=census-tuples-v1`;
    const controller = new AbortController();
    /** @type {ReturnType<typeof setTimeout> | undefined} */
    let timer;
    const deadline = new Promise((_, reject) => {
      timer = setTimeout(() => {
        reject(new Error(`No complete answer from ${url} within ${ceilingMs / 1000} s`));
        controller.abort();
      }, ceilingMs);
    });
    const response = (async () => {
      const etag = previous?.headers.get('etag');
      const result = await fetchWithBusyRetry(url, { signal: controller.signal,
        onBusy: info => { if (retainedKey === key) onBusy(info); } },
        (target, options) => request(target, { ...options, ms: ceilingMs,
          ...(etag ? { headers: { 'If-None-Match': etag } } : {}) }));
      if (result.status === 304) {
        if (!previous || !etag || (result.headers.has('etag') && result.headers.get('etag') !== etag)) {
          throw new Error('The store returned an unrecognised unchanged-response validator');
        }
        // A validated unchanged answer keeps the SAME immutable rows. This
        // avoids another parse, fold and complete database decoration.
        const headers = new Headers(previous.headers);
        result.headers.forEach((value, key) => headers.set(key, value));
        const stale = degradedRefusal(headers);
        if (stale) return { ok: false, status: 304, body: null, headers, reason: stale, degraded: headers.get(CENSUS_DEGRADED_HEADER) };
        return { ok: true, status: 304, body: previous.body, headers };
      }
      if (!result.ok) {
        // THE REASON IS READ, NOT DROPPED (CE-83, D-1789). A 429/503 names
        // itself in `{"error":...}`; an unreadable census answers 503 with an
        // empty array and puts its sentence in `x-brutex-census-note`.
        const reason = (await reasonOf(result)) ?? result.headers.get('x-brutex-census-note');
        return { ok: false, status: result.status, body: null, headers: result.headers, reason: reason || null };
      }
      const stale = degradedRefusal(result.headers);
      if (stale) {
        return { ok: false, status: result.status, body: null, headers: result.headers,
          reason: stale, degraded: result.headers.get(CENSUS_DEGRADED_HEADER) };
      }
      const body = await decodeCensus(await result.json(), feed, controller.signal);
      return { ok: true, status: result.status, body, headers: result.headers };
    })();
    try {
      return /** @type {CensusAnswer} */ (await Promise.race([response, deadline]));
    } finally {
      clearTimeout(timer);
    }
  }

  /**
   * @param {string} feed
   * @param {number} generation
   * @param {{retain?: boolean}} [options]
   * @returns {Promise<CensusAnswer>}
   */
  function load(feed, generation, { retain = true } = {}) {
    if (typeof feed !== 'string' || feed.length === 0 ||
        !Number.isSafeInteger(generation) || generation < 0) {
      return Promise.reject(new Error('A store census needs a feed and a valid refresh generation'));
    }
    const key = JSON.stringify([feed, generation]);
    if (retain) {
      retainedKey = key;
      if (cached?.feed !== feed) cached = null;
    }
    if (cached?.key === key) return Promise.resolve(cached.answer);
    let flight = flights.get(key);
    if (!flight) {
      flight = fetchBody(feed, cached?.feed === feed ? cached.answer : null, key).then((answer) => {
        if (answer.ok && retainedKey === key) cached = { key, feed, answer };
        return answer;
      }).finally(() => { flights.delete(key); });
      flights.set(key, flight);
    }
    return flight;
  }

  return { load };
}

/**
 * The sentence a refused census read is shown as: the status and the server's
 * own reason (or the degraded refusal), never the status alone. CE-83.
 * @param {CensusAnswer} answer
 * @returns {string}
 */
export function censusFailure(answer) {
  if (answer.degraded) return `/store.json: ${answer.reason}`;
  return answer.reason
    ? `/store.json answered HTTP ${answer.status}: ${answer.reason}`
    : `/store.json answered HTTP ${answer.status} and named no reason`;
}
