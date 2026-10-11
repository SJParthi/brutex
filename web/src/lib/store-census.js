import { ask } from './ask.js';
import { decodeCensus, mergeCensus, measureCensus, censusMeasureOf, noteCensusMeasure, CENSUS_ENCODING } from './store-census-wire.js';
import { denomKey } from './completeness.js';
import { inSlices } from './census-worker.js';
import { fetchWithBusyRetry } from '../../saved-backtest/requests.js';
import { reasonOf } from './refusal.js';
import { dayLabel, secondLabel } from './dates.js';
import { group } from './money.js';

export const STORE_CENSUS_MS = 30_000;

/**
 * `reason` is the server's own sentence for a refused read; `degraded` is the
 * `x-brutex-census-degraded` header when it is non-empty. `lag` is every
 * census header read once (`censusLag`), and `readAt` the moment this
 * browser received the answer — the "read at" a page shows beside the rows.
 * `how` says how the rows came to be the current ones (`CensusHow`).
 * @typedef {{ok: boolean, status: number, body: any[] | null, headers: Headers,
 *   reason?: string | null, degraded?: string | null, lag?: CensusLag, readAt?: number,
 *   how?: CensusHow}} CensusAnswer
 */

/**
 * HOW A CENSUS ANSWER'S ROWS CAME TO BE THE CURRENT ONES (WP-56, D-3327 item
 * 3; DB-022, DB-060):
 *
 *   `whole`      the whole census was read. `why` is `null` for a first read,
 *                and otherwise the reason a refresh could not be a delta, in
 *                words: never a silent 33 MB.
 *   `delta`      only the rows changed since `since` were read (`rows` of
 *                them: `added` new, `replaced` in place) and merged into the
 *                census this page held.
 *   `unchanged`  the server said nothing changed (an empty delta, or a 304),
 *                and the rows are the ones this page already held.
 *
 *   `scoped`     only the census rows of the instrument on screen were read,
 *                from `/census/facets.json` (`readScope`): the first paint of
 *                /db (WP-56b). `rows` of them; the whole census was not read.
 *
 * @typedef {{kind: 'whole' | 'delta' | 'unchanged' | 'scoped', since: string | null, rows: number,
 *   added: number, replaced: number, why: string | null}} CensusHow
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

/* ======================================================================
   THE CENSUS'S OWN STATE, READ OFF ITS HEADERS (D-3327 item 2; DB-G03,
   DB-G11, DB-G60, DB-G78, DB-022)
   ----------------------------------------------------------------------
   `/store.json`, its HEAD and `/census/facets.json` all stamp the same
   headers. `held` was the only word a loaded census ever answered, and it
   answered it while the manifest refused every options day from 2019-11-14:
   the page drew a census four days behind its import as the whole store.
   WP-30 added `capped` and `behind`, the capacity, the headroom, the
   generation and the first refused import day; this reads them once, for
   every page, and keeps "the build does not send this" (null) apart from
   "nothing to say" (an empty header).
   ====================================================================== */
/**
 * @typedef {{
 *   state: string | null,
 *   note: string | null,
 *   degraded: string | null,
 *   generation: number | null,
 *   capacity: number | null,
 *   headroom: number | null,
 *   epoch: string | null,
 *   behindDays: number | null,
 *   behindFrom: string | null,
 *   reason: string | null,
 *   parts: Record<string, number> | null,
 *   faults: string[]
 * }} CensusLag
 */

/** The five words a loaded or unloaded census answers with (D-3327). */
export const CENSUS_STATES = ['held', 'capped', 'behind', 'absent', 'unreadable'];

/**
 * `x-brutex-census-part-generations`, `INDEX=2097116,CASH=2114847,FNO=2841866`,
 * as `{INDEX: 2097116, …}`; `null` when the header is absent or empty (an
 * older API, or a single-file census whose one generation is
 * `x-brutex-census-generation`). A malformed value is `null` and named in
 * `faults`, never a partial map a `since` could be read from (DB-B56, DB-B67).
 * @param {string | null} value
 * @param {string[]} faults
 * @returns {Record<string, number> | null}
 */
export function partGenerations(value, faults) {
  if (value === null || value.trim() === '') return null;
  /** @type {Record<string, number>} */
  const out = {};
  for (const pair of value.trim().split(',')) {
    const m = /^([A-Z]+)=(0|[1-9]\d*)$/.exec(pair);
    if (!m || !Number.isSafeInteger(Number(m[2])) || Object.hasOwn(out, m[1])) {
      faults.push(`x-brutex-census-part-generations is ${JSON.stringify(value)}, not SEGMENT=generation pairs`);
      return null;
    }
    out[m[1]] = Number(m[2]);
  }
  return out;
}

/**
 * THE GENERATION A FACETS `since` IS ASKED IN, for one segment (DB-B56).
 *
 * `/census/facets.json` answers in ONE part's generation; the vendor-wide
 * `x-brutex-census-generation` is the sum of the parts and is never a
 * `since` (the server refuses it `since_in_future`). A partitioned census
 * names each part's own; a single file's one generation is the vendor-wide
 * one. `null` when neither is known — the caller then has no delta to ask.
 * @param {CensusLag | null | undefined} lag
 * @param {string} segment `INDEX`, `CASH` or `FNO`
 * @returns {number | null}
 */
export function sinceFor(lag, segment) {
  if (!lag) return null;
  if (lag.parts) return Object.hasOwn(lag.parts, segment) ? lag.parts[segment] : null;
  return lag.generation;
}

/**
 * Every census header, read. A counter is a whole number or null; a header
 * that is present and unreadable is null AND named in `faults`, never a zero.
 * @param {Headers} headers
 * @returns {CensusLag}
 */
export function censusLag(headers) {
  /** @type {string[]} */
  const faults = [];
  /** @param {string} name */
  const text = (name) => {
    const value = headers.get(name);
    return value === null || value.trim() === '' ? null : value.trim();
  };
  /** @param {string} name */
  const count = (name) => {
    const value = text(name);
    if (value === null) return null;
    if (!/^(0|[1-9]\d*)$/.test(value) || !Number.isSafeInteger(Number(value))) {
      faults.push(`${name} is ${JSON.stringify(value)}, not a whole number`);
      return null;
    }
    return Number(value);
  };
  const state = text('x-brutex-census-state');
  if (state !== null && !CENSUS_STATES.includes(state)) faults.push(`x-brutex-census-state is ${JSON.stringify(state)}, a word this page does not know`);
  const behindFrom = text('x-brutex-census-behind-from');
  if (behindFrom !== null && !/^\d{4}-\d{2}-\d{2}$/.test(behindFrom)) faults.push(`x-brutex-census-behind-from is ${JSON.stringify(behindFrom)}, not a day`);
  const note = text('x-brutex-census-note');
  // AN API OLDER THAN D-3327 sends the generation only inside the note
  // (`gdfl: 385442 month(s), 3628697985 row(s), generation 7053829`).
  const noted = note?.match(/, generation (0|[1-9]\d*)$/);
  const generation = count('x-brutex-census-generation') ??
    (noted && Number.isSafeInteger(Number(noted[1])) ? Number(noted[1]) : null);
  return {
    state,
    note,
    degraded: text(CENSUS_DEGRADED_HEADER),
    generation,
    capacity: count('x-brutex-census-capacity'),
    headroom: count('x-brutex-census-headroom'),
    epoch: text('x-brutex-census-epoch'),
    behindDays: count('x-brutex-census-behind-days'),
    behindFrom: behindFrom !== null && /^\d{4}-\d{2}-\d{2}$/.test(behindFrom) ? behindFrom : null,
    reason: text('x-brutex-census-reason'),
    parts: partGenerations(headers.get('x-brutex-census-part-generations'), faults),
    faults
  };
}

/* ======================================================================
   HAS THE STORE MOVED SINCE THE PAGE READ IT? (DB-022)
   ----------------------------------------------------------------------
   /db read the census once per feed and never looked again: in twelve
   minutes of a running import GDFL went from 173,432 to 182,943 month rows
   and the page still drew the opening snapshot, with no "read at" and no
   word that anything had moved. A HEAD of `/store.json` is header-only and
   carries the same generation; comparing the two is the whole check, and
   the page then SAYS the store moved instead of silently re-downloading the
   census (33 MB on GDFL) every few seconds.
   ====================================================================== */
/**
 * One HEAD of the census, as the generation watch keeps it.
 * @typedef {{feed: string, at: number, lag: CensusLag | null, error: string | null}} HeadCheck
 */

/**
 * Whether the census moved after it was read.
 *
 * `null` when it cannot be said — no read, no HEAD, a HEAD for another feed,
 * a failed HEAD, or a generation either side did not send; never `false` for
 * a question nobody could answer. `false` when the generations are equal.
 * Otherwise what moved: the vendor-wide generations before and after, and
 * the segments whose own part generation rose (empty for a single-file
 * census, whose one generation is the vendor-wide one).
 *
 * @param {{feed: string | null, lag: CensusLag | null | undefined} | null} read
 * @param {HeadCheck | null} head
 * @returns {null | false | {from: number, to: number, segments: string[]}}
 */
export function censusMoved(read, head) {
  if (!read?.feed || !read.lag || !head || head.feed !== read.feed || head.error || !head.lag) return null;
  const from = read.lag.generation, to = head.lag.generation;
  if (from === null || to === null) return null;
  if (from === to) return false;
  const was = read.lag.parts ?? {}, now = head.lag.parts ?? {};
  const segments = Object.keys(now).filter((segment) => now[segment] !== was[segment]);
  return { from, to, segments };
}

/**
 * Read the body of a census answer into rows. The default parses on the
 * calling thread; `store.svelte.js` passes the census worker's.
 * @typedef {(response: Response, feed: string, signal: AbortSignal) => Promise<any[]>} CensusBodyReader
 */

/** @type {CensusBodyReader} */
export const readOnPage = async (response, feed, signal) => decodeCensus(await response.json(), feed, signal);

/* ======================================================================
   A REFRESH ASKS ONLY FOR WHAT CHANGED (WP-56, D-3327 item 3; DB-022,
   DB-060, DB-G59, DB-G70)
   ----------------------------------------------------------------------
   A page that held the census refreshed it by reading it again: 33 MB on
   GDFL for every commit an import made, every five seconds while `/ingest`
   watched one, because the validator changes with every commit. A refresh
   now names the generation the page holds (`/store.json?...&since=`) and is
   answered with the rows appended after it, on the same wire; they are
   merged into the census the page holds (`mergeCensus`), in its order.

   NEVER PATCHED ON TRUST. A merge is believed only when (1) the census held
   was checked to be in the order the merge places rows by, (2) the server
   answered the `since` that was asked, and (3) the merged census holds
   exactly the rows and the bars the server says the census holds now. Any
   other outcome (a refused `since`, an epoch that moved, an API that
   answers no deltas, a count that does not add up) reads the census WHOLE,
   and the answer says why in words (`how.why`), never silently.
   ====================================================================== */
/**
 * What a `since` is asked with, read off the headers of the census held:
 * each part's own generation and the epochs they were read from, or a
 * single file's one generation. `{why}` where no delta can be asked.
 * @param {Headers} headers
 * @returns {{since: string, epoch: string} | {why: string}}
 */
export function deltaBase(headers) {
  const parts = headers.get('x-brutex-census-part-generations');
  if (parts === null) return { why: 'this API answers no deltas (it names no part generations)' };
  const epoch = (headers.get('x-brutex-census-epoch') ?? '').trim();
  if (parts.trim() !== '') {
    return epoch === ''
      ? { why: 'the census held names its parts and no epoch, so the files its generations count are not known' }
      : { since: parts.trim(), epoch };
  }
  const generation = (headers.get('x-brutex-census-generation') ?? '').trim();
  return /^(0|[1-9]\d*)$/.test(generation)
    ? { since: generation, epoch: '' }
    : { why: 'the census held names no generation' };
}

/** `whole`, `delta` or `unchanged`, as `CensusHow` spells them. */
const how = (/** @type {CensusHow['kind']} */ kind, /** @type {Partial<CensusHow>} */ more = {}) =>
  ({ kind, since: null, rows: 0, added: 0, replaced: 0, why: null, ...more });

/**
 * Share the request AND the parsed body. Keep at most one completed selected
 * feed answer; explicit all-feed surveys share flights but retain only totals.
 * A new selection/generation prevents a late answer from replacing that cache.
 * Consumers must treat the shared rows as read-only.
 *
 * A first read is the whole census. A later read of the same feed asks only
 * for what changed since the census held, and reads it whole when that
 * cannot be done; `answer.how` says which, and why (see the block above).
 *
 * @param {typeof ask} [request]
 * @param {number} [ceilingMs]
 * @param {(info:{attempt:number,nextAttempt:number,delayMs:number,status:number})=>void} [onBusy]
 * @param {CensusBodyReader} [readBody] how a 200 body becomes rows
 */
export function createCensusLoader(request = ask, ceilingMs = STORE_CENSUS_MS, onBusy = () => {}, readBody = readOnPage) {
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
    const signal = controller.signal;
    /** @type {ReturnType<typeof setTimeout> | undefined} */
    let timer;
    const deadline = new Promise((_, reject) => {
      timer = setTimeout(() => {
        reject(new Error(`No complete answer from ${url} within ${ceilingMs / 1000} s`));
        controller.abort();
      }, ceilingMs);
    });
    /** @param {string} target @param {RequestInit} more */
    const send = (target, more) => fetchWithBusyRetry(target, { signal,
      onBusy: info => { if (retainedKey === key) onBusy(info); } },
      (to, options) => request(to, { ...options, ms: ceilingMs, ...more }));
    /** A refused read, with its headers read. @param {Response} result @param {string | null} reason @param {string | null} [degraded] */
    const refused = (result, reason, degraded) => ({ ok: false, status: result.status, body: null, headers: result.headers,
      reason, ...(degraded === undefined ? {} : { degraded }), lag: censusLag(result.headers), readAt: Date.now() });

    /**
     * THE WHOLE CENSUS, conditionally when this page holds a validated copy.
     * `why` is the reason a refresh was not a delta, or null for a first read.
     * @param {string | null} why @param {Response} [already] a whole body an API answered a delta request with
     * @returns {Promise<CensusAnswer>}
     */
    async function whole(why, already) {
      const etag = previous?.headers.get('etag');
      // A FIRST READ MAY BE REVALIDATED BY THE BROWSER'S OWN CACHE (`no-cache`
      // always asks the server): an unchanged census is then a 304 between
      // the browser and the server and no body crosses the wire again. With
      // a validator of this page's own the request stays `no-store`, so the
      // 304 below is this page's and the rows kept are the ones it holds.
      const result = already ?? await send(url, etag ? { headers: { 'If-None-Match': etag } } : { cache: 'no-cache' });
      if (result.status === 304) {
        if (!previous || !etag || (result.headers.has('etag') && result.headers.get('etag') !== etag)) {
          throw new Error('The store returned an unrecognised unchanged-response validator');
        }
        // A validated unchanged answer keeps the SAME immutable rows. This
        // avoids another parse, fold and complete database decoration.
        const headers = new Headers(previous.headers);
        result.headers.forEach((value, key) => headers.set(key, value));
        const stale = degradedRefusal(headers);
        if (stale) return { ok: false, status: 304, body: null, headers, reason: stale, degraded: headers.get(CENSUS_DEGRADED_HEADER), lag: censusLag(headers), readAt: Date.now() };
        return { ok: true, status: 304, body: previous.body, headers, lag: censusLag(headers), readAt: Date.now(), how: how('unchanged', { why }) };
      }
      if (!result.ok) {
        // THE REASON IS READ, NOT DROPPED (CE-83, D-1789). A 429/503 names
        // itself in `{"error":...}`; an unreadable census answers 503 with an
        // empty array and puts its sentence in `x-brutex-census-note`.
        const reason = (await reasonOf(result)) ?? result.headers.get('x-brutex-census-note');
        return refused(result, reason || null);
      }
      const stale = degradedRefusal(result.headers);
      if (stale) return refused(result, stale, result.headers.get(CENSUS_DEGRADED_HEADER));
      const body = await readBody(result, feed, signal);
      return { ok: true, status: result.status, body, headers: result.headers, lag: censusLag(result.headers), readAt: Date.now(), how: how('whole', { why }) };
    }

    /**
     * ONLY WHAT CHANGED since the census `held`, merged into it; or the
     * reason it could not be, for `whole` to carry.
     * @param {CensusAnswer & {body: any[]}} held
     * @param {{since: string, epoch: string}} base what the `since` is asked with (`deltaBase`)
     * @returns {Promise<{answer: CensusAnswer} | {why: string, whole?: Response}>}
     */
    async function delta(held, base) {
      const measure = censusMeasureOf(held.body) ?? await measureCensus(held.body, inSlices, signal);
      noteCensusMeasure(held.body, measure);
      if (measure.order !== null) return { why: `the census held is not in the order a delta is merged by (${measure.order})` };
      const { since, epoch } = base;
      // NEVER FROM A BROWSER CACHE (WP-56b): a delta is the answer to "what
      // changed after the generation I hold", asked once, and the server
      // sends no Cache-Control with it. `no-store` like the HEAD and the
      // facets requests, so that does not rest on what a browser chooses.
      const result = await send(`${url}&since=${encodeURIComponent(since)}${epoch ? `&epoch=${encodeURIComponent(epoch)}` : ''}`, { cache: 'no-store' });
      if (result.headers.get('x-brutex-census-since') === null) {
        // AN API OLDER THAN DELTAS reads no `since`: a 200 from it is the whole
        // census, and is read as one, where the worker can parse it.
        return result.ok
          ? { why: 'this API answered the whole census to a request for a delta', whole: result }
          : { answer: refused(result, ((await reasonOf(result)) ?? result.headers.get('x-brutex-census-note')) || null) };
      }
      if ([400, 409, 410].includes(result.status)) {
        return { why: `the server refused a delta since ${since} (HTTP ${result.status}: ${(await reasonOf(result)) ?? 'no reason named'})` };
      }
      if (!result.ok) return { answer: refused(result, ((await reasonOf(result)) ?? result.headers.get('x-brutex-census-note')) || null) };
      const stale = degradedRefusal(result.headers);
      if (stale) return { answer: refused(result, stale, result.headers.get(CENSUS_DEGRADED_HEADER)) };
      /** @type {any} */
      let wire;
      /** @type {any[]} */
      let rows;
      try {
        wire = await result.json();
        rows = await decodeCensus(wire, feed, signal);
      } catch (why) {
        signal.throwIfAborted();
        return { why: `the delta since ${since} could not be read (${String(/** @type {any} */ (why)?.message ?? why)})` };
      }
      const now = wire?.held;
      if (wire?.since !== since || !Number.isSafeInteger(now?.months) || !Number.isSafeInteger(now?.rows)) {
        return { why: `the answer to a delta since ${since} names since ${JSON.stringify(wire?.since ?? null)} and no whole counters to check a merge against` };
      }
      const adds = (/** @type {number} */ months, /** @type {number} */ bars) => months === now.months && bars === now.rows
        ? null
        : `the census after the delta since ${since} holds ${months} row(s) and ${bars} bar(s) where the server holds ${now.months} and ${now.rows}`;
      if (rows.length === 0) {
        const off = adds(held.body.length, measure.bars);
        if (off) return { why: off };
        const headers = new Headers(held.headers);
        result.headers.forEach((value, name) => headers.set(name, value));
        headers.delete('x-brutex-census-since');
        return { answer: { ok: true, status: 200, body: held.body, headers, lag: censusLag(headers), readAt: Date.now(),
          how: how('unchanged', { since }) } };
      }
      /** @type {Awaited<ReturnType<typeof mergeCensus>>} */
      let merged;
      try {
        merged = await mergeCensus(held.body, rows, inSlices, signal, measure.ranks ?? null);
      } catch (why) {
        signal.throwIfAborted();
        return { why: `the delta since ${since} could not be merged (${String(/** @type {any} */ (why)?.message ?? why)})` };
      }
      const off = adds(merged.rows.length, measure.bars + merged.bars);
      if (off) return { why: off };
      noteCensusMeasure(merged.rows, { order: null, bars: now.rows, ranks: measure.ranks ?? null });
      // THE MERGED ROWS HAVE NO VALIDATOR: an `etag` names a whole body's
      // bytes, and none was sent. The next refresh is a delta again.
      const headers = new Headers(result.headers);
      headers.delete('x-brutex-census-since');
      headers.delete('etag');
      return { answer: { ok: true, status: 200, body: merged.rows, headers, lag: censusLag(headers), readAt: Date.now(),
        how: how('delta', { since, rows: rows.length, added: merged.added, replaced: merged.replaced }) } };
    }

    const response = (async () => {
      if (!previous?.ok || !Array.isArray(previous.body)) return whole(null);
      // DECIDED BEFORE ANYTHING IS AWAITED: where no delta can be asked, the
      // whole read's request leaves in this same turn, as a first read's
      // does, so a caller that selects, clears and reselects in one turn
      // still shares one physical response (`store-startup.test.js`).
      const base = deltaBase(previous.headers);
      if ('why' in base) return whole(base.why);
      const tried = await delta(/** @type {CensusAnswer & {body: any[]}} */ (previous), base);
      if ('answer' in tried) return tried.answer;
      return whole(tried.why, tried.whole);
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

/* ======================================================================
   THE SELECTION'S FACETS — `/census/facets.json` (D-3327 item 3; DB-060,
   DB-G59, DB-G70)
   ----------------------------------------------------------------------
   The lists one selection needs (its underlyings, months, expiries, strikes
   and futures, each with rows per rung, first and last stamp, last close),
   answered by the server from map probes — 11,752 bytes for GDFL NIFTY spot
   against 33,349,105 for the whole census (shadow of the operator's store,
   10 Oct 2026). A refresh asks only for what changed after the generation
   the page holds (`?since=`), and merges it by key; a generation the server
   no longer holds the appends of (410 `since_too_old`) is loaded afresh,
   never patched partially.
   ====================================================================== */
export const FACETS_MS = 15_000;
/* THE CEILING OF ONE REQUEST OF THE FIRST PAINT'S SCOPE (`readScope`), its
   own and shorter (WP-56b, the second check's follow-up): these answers are
   a few kilobytes read off an index (p99 under 25 ms measured over n=200 on
   the real store), and a scope that does not answer falls back to the whole
   census, so every second waited here is a second before the first paint.
   Three seconds is more than a hundred times the measured p99; past it the
   scope is given up, the whole census is read, and the banner says why. */
export const SCOPE_MS = 3_000;
/** The longest one request of a scope waits before it is asked again after a 429. */
export const SCOPE_RETRY_MAX_MS = 1_500;

/**
 * @typedef {{feed: string, segment: string, underlying?: string | null,
 *   exchange?: string | null, expiry?: string | null}} FacetQuery
 * @typedef {{underlyings: number, months: number, expiries: number, strikes: number, futures: number}} FacetCounts
 * @typedef {{ok: boolean, status: number, key: string, query: FacetQuery, body: any | null,
 *   headers: Headers | null, lag: CensusLag | null, reason: string | null, readAt: number,
 *   delta: (FacetCounts & {since: number, generation: number}) | null, reloaded: boolean}} FacetAnswer
 */

/** The URL for a query, with `since` when a delta is asked. @param {FacetQuery} query @param {number | null} [since] */
export function facetsUrl(query, since = null) {
  // NO SEGMENT IS THE ROOT (WP-56b): the feed's segments, and nothing else.
  if (query.segment === undefined || query.segment === null) return `/census/facets.json?feed=${encodeURIComponent(query.feed)}`;
  const parts = [`feed=${encodeURIComponent(query.feed)}`, `segment=${encodeURIComponent(query.segment)}`];
  if (query.underlying) parts.push(`underlying=${encodeURIComponent(query.underlying)}`);
  if (query.exchange) parts.push(`exchange=${encodeURIComponent(query.exchange)}`);
  if (query.expiry) parts.push(`expiry=${encodeURIComponent(query.expiry)}`);
  if (since !== null) parts.push(`since=${since}`);
  return `/census/facets.json?${parts.join('&')}`;
}

/** One spelling of a query, for a cache key. @param {FacetQuery} query */
export const facetKey = (query) => JSON.stringify([query.feed, query.segment, query.underlying ?? null,
  query.exchange ?? null, query.expiry ?? null]);

/**
 * Whether a body is a facets answer for this query; the sentence if not.
 * @param {any} body @param {FacetQuery} query @returns {string | null}
 */
/**
 * Why a body is not the ROOT answer for `feed` (`/census/facets.json?feed=`:
 * the segments the feed holds, in the census's order, and that order), or
 * null.
 * @param {any} body @param {string} feed
 */
export function rootFault(body, feed) {
  if (!body || typeof body !== 'object' || body.schema !== 1) return 'it is not a schema-1 facets answer';
  if (body.feed !== feed || body.segment !== null) return `it answers ${body.feed}/${body.segment}, not the root of ${feed}`;
  if (!Array.isArray(body.segments)) return 'it lists no segments (an API older than the root answer)';
  const order = facetsOrder(body);
  if (order.why) return order.why;
  let at = -1;
  for (const row of body.segments) {
    const next = order.segments.indexOf(row?.segment);
    if (next < 0) return `its segment ${JSON.stringify(row?.segment ?? null)} is not in the order it states`;
    if (next <= at) return `its segments are not in the order it states (${row.segment})`;
    at = next;
    if (!Number.isSafeInteger(row.generation) || row.generation < 0) return `the generation of ${row.segment} is not a whole number`;
    if (!Number.isSafeInteger(row.underlyings) || row.underlyings < 1) return `${row.segment} is listed and counts no underlying`;
    if (!row.rows || typeof row.rows !== 'object') return `${row.segment} names no rows`;
  }
  return null;
}

export function facetsFault(body, query) {
  if (!body || typeof body !== 'object' || body.schema !== 1) return 'it is not a schema-1 facets answer';
  if (body.feed !== query.feed || body.segment !== query.segment) return `it answers ${body.feed}/${body.segment}, not ${query.feed}/${query.segment}`;
  if ((body.underlying ?? null) !== (query.underlying ?? null) || (body.expiry ?? null) !== (query.expiry ?? null)) {
    return 'it answers another underlying or expiry';
  }
  if (!Number.isSafeInteger(body.generation) || body.generation < 0) return 'its generation is not a whole number';
  for (const list of ['underlyings', 'months', 'expiries', 'strikes', 'futures']) {
    if (!Array.isArray(body[list])) return `its ${list} is not a list`;
  }
  return null;
}

/** The order the server writes `(Exchange, Symbol)` in: its enum order. */
/** @param {string} a @param {string} b */
const textOrder = (a, b) => (a < b ? -1 : a > b ? 1 : 0);
/** @param {any} a @param {any} b */
/**
 * The order of two underlyings, BY THE ORDER THE SERVER STATED (`order`, in
 * every facets answer since WP-56b): exchanges in its list's order, then the
 * symbol. This module keeps no exchange list of its own, so a new exchange
 * needs no change here. An exchange the answer does not name (an API older
 * than `order`, or a list that omits it) is placed after the named ones by
 * its text, and `mergeFacets` says so in `orderWhy`.
 * @param {string[]} exchanges
 */
const underlyingOrderOf = (exchanges) => {
  const rank = new Map(exchanges.map((word, at) => [word, at]));
  const of = (/** @type {string} */ word) => rank.get(word) ?? exchanges.length;
  return (/** @type {any} */ a, /** @type {any} */ b) =>
    (of(a.exchange) - of(b.exchange)) || textOrder(a.exchange, b.exchange) || textOrder(a.symbol, b.symbol);
};

/**
 * The exchanges and segments in the census's order, as a facets answer
 * states them, or the reason it states none this page can use.
 * @param {any} body
 * @returns {{exchanges: string[], segments: string[], why: string | null}}
 */
export function facetsOrder(body) {
  const words = (/** @type {any} */ list) => Array.isArray(list) && list.every((word) => typeof word === 'string' && word !== '') &&
    new Set(list).size === list.length;
  const order = body?.order;
  if (order && words(order.exchanges) && words(order.segments)) return { exchanges: order.exchanges, segments: order.segments, why: null };
  return { exchanges: [], segments: [], why: order === undefined
    ? 'this API states no order for exchanges and segments (it is older than the order statement)'
    : 'the order this API states is not two lists of distinct names' };
}

/**
 * Replace the base's rows by the delta's, by key, keep the server's order.
 * @param {any[]} base @param {any[]} delta @param {(row: any) => string} keyOf
 * @param {(a: any, b: any) => number} order @param {(was: any, now: any) => any} [combine]
 */
function mergeList(base, delta, keyOf, order, combine = (_, now) => now) {
  if (delta.length === 0) return base;
  const by = new Map(base.map((row) => [keyOf(row), row]));
  for (const row of delta) {
    const key = keyOf(row);
    by.set(key, by.has(key) ? combine(by.get(key), row) : row);
  }
  return [...by.values()].sort(order);
}

/**
 * A cold facets answer moved forward by a `since` delta: what a cold load at
 * the delta's generation answers (`store-census.test.js` holds the two
 * equal). Every changed row carries its CURRENT totals, so a row is replaced,
 * never added to. One exception, and it is the server's rule: a strike row in
 * a delta carries only the side that changed, and its other side is `null`
 * meaning "unchanged" (a cold answer's `null` means "not held"); the census
 * never loses a side, so the held side is kept.
 *
 * @param {any} base a valid cold or merged answer
 * @param {any} delta a valid delta answer for the same query
 */
export function mergeFacets(base, delta) {
  const order = facetsOrder(delta.order === undefined ? base : delta);
  const underlyingOrder = underlyingOrderOf(order.exchanges);
  const unnamed = [...new Set([...base.underlyings, ...delta.underlyings].map((row) => row.exchange))]
    .filter((word) => !order.exchanges.includes(word));
  const orderWhy = order.why ?? (unnamed.length ? `the order this API states does not name ${unnamed.join(', ')}` : null);
  return {
    ...base,
    ...(delta.order ?? base.order ? { order: delta.order ?? base.order } : {}),
    // SAID ONLY WHEN THERE IS SOMETHING TO SAY: underlyings on an exchange
    // the answer's order does not name were placed after the named ones.
    ...(orderWhy ? { orderWhy } : {}),
    generation: delta.generation,
    state: delta.state,
    degraded: delta.degraded,
    since: null,
    underlyings: mergeList(base.underlyings, delta.underlyings, (row) => `${row.exchange}|${row.symbol}`, underlyingOrder),
    months: mergeList(base.months, delta.months, (row) => row.month, (a, b) => textOrder(a.month, b.month)),
    expiries: mergeList(base.expiries, delta.expiries, (row) => row.expiry, (a, b) => textOrder(a.expiry, b.expiry)),
    strikes: mergeList(base.strikes, delta.strikes, (row) => String(row.strike), (a, b) => a.strike - b.strike,
      (was, now) => ({ ...was, ...now, ce: now.ce ?? was.ce, pe: now.pe ?? was.pe })),
    futures: mergeList(base.futures, delta.futures, (row) => row.contract, (a, b) => textOrder(a.contract, b.contract))
  };
}

/** How many rows of each list a delta changed. @param {any} delta @returns {FacetCounts} */
export const facetCounts = (delta) => ({ underlyings: delta.underlyings.length, months: delta.months.length,
  expiries: delta.expiries.length, strikes: delta.strikes.length, futures: delta.futures.length });

/**
 * Facets per selection: a cold load once, then deltas.
 *
 * `load(query)` answers from the kept answer for that query, or reads it
 * cold. `refresh(query)` asks the server only for what changed after the
 * kept answer's generation and merges it; a 410 reloads cold and says so
 * (`reloaded`). A failed refresh answers the failure and keeps the last good
 * answer for the next attempt, so a page can show both under their own names.
 * At most `keep` selections are kept.
 *
 * @param {typeof ask} [request]
 * @param {{keep?: number, ms?: number, now?: () => number}} [options]
 */
export function createFacetsLoader(request = ask, { keep = 8, ms = FACETS_MS, now = Date.now } = {}) {
  /** @type {Map<string, FacetAnswer>} */
  const kept = new Map();
  /** @type {Map<string, Promise<FacetAnswer>>} */
  const flights = new Map();

  /** @param {FacetQuery} query @param {string} key @param {number | null} since @returns {Promise<FacetAnswer>} */
  async function read(query, key, since) {
    const fail = (/** @type {number} */ status, /** @type {string} */ reason, /** @type {Headers | null} */ headers = null) =>
      ({ ok: false, status, key, query, body: null, headers, lag: headers ? censusLag(headers) : null,
        reason, readAt: now(), delta: null, reloaded: false });
    const response = await request(facetsUrl(query, since), { cache: 'no-store', ms });
    if (!response.ok) {
      // 410 `since_too_old` and 409 `since_in_future` (DB-B56) keep their
      // status, so a caller can reload afresh rather than show an error; any
      // other refusal is the server's own sentence.
      const reason = (await reasonOf(response)) ?? response.headers.get('x-brutex-census-note');
      return fail(response.status, reason || `/census/facets.json answered HTTP ${response.status} and named no reason`, response.headers);
    }
    const body = await response.json();
    const isRoot = query.segment === undefined || query.segment === null;
    const fault = isRoot ? rootFault(body, query.feed) : facetsFault(body, query);
    if (!fault && isRoot) {
      return { ok: true, status: response.status, key, query, body, headers: response.headers,
        lag: censusLag(response.headers), reason: null, readAt: now(), delta: null, reloaded: false };
    }
    if (fault) return fail(response.status, `/census/facets.json: ${fault}`, response.headers);
    if (since !== null && body.since !== since) return fail(response.status, `/census/facets.json answered a delta since ${body.since}, not since ${since}`, response.headers);
    // ONE SPELLING OF THE GENERATION (DB-B56): where the server names the
    // part that answered, it is the body's; a disagreement is refused rather
    // than a delta keyed on one and merged under the other.
    const part = response.headers.get('x-brutex-census-part-generation');
    if (part !== null && part !== '' && Number(part) !== body.generation) {
      return fail(response.status, `/census/facets.json: its header names part generation ${part} and its body ${body.generation}`, response.headers);
    }
    return { ok: true, status: response.status, key, query, body, headers: response.headers,
      lag: censusLag(response.headers), reason: null, readAt: now(), delta: null, reloaded: false };
  }

  /** @param {string} key @param {FacetAnswer} answer */
  function remember(key, answer) {
    kept.delete(key);
    kept.set(key, answer);
    while (kept.size > keep) kept.delete(/** @type {string} */ (kept.keys().next().value));
  }

  /** @param {string} key @param {() => Promise<FacetAnswer>} work */
  function shared(key, work) {
    let flight = flights.get(key);
    if (!flight) {
      flight = work().finally(() => flights.delete(key));
      flights.set(key, flight);
    }
    return flight;
  }

  /** @param {FacetQuery} query @returns {Promise<FacetAnswer>} */
  function load(query) {
    const key = facetKey(query);
    const held = kept.get(key);
    if (held) return Promise.resolve(held);
    return shared(key, async () => {
      const answer = await read(query, key, null);
      if (answer.ok) remember(key, answer);
      return answer;
    });
  }

  /** @param {FacetQuery} query @returns {Promise<FacetAnswer>} */
  function refresh(query) {
    const key = facetKey(query);
    const held = kept.get(key);
    if (!held) return load(query);
    return shared(`${key}#since`, async () => {
      const since = held.body.generation;
      const answer = await read(query, key, since);
      if (answer.status === 410 || answer.status === 409) {
        // NOT PATCHED: the server no longer holds what changed after `since`
        // (410), or stands at a generation below it, a census read whole in
        // a new epoch (409): either way the selection is read whole again and
        // the page is told, never patched from a delta it cannot trust.
        kept.delete(key);
        const fresh = await read(query, key, null);
        if (fresh.ok) remember(key, { ...fresh, reloaded: true });
        return fresh.ok ? { ...fresh, reloaded: true } : fresh;
      }
      if (!answer.ok) return answer;
      const merged = { ...answer, body: mergeFacets(held.body, answer.body),
        delta: { ...facetCounts(answer.body), since, generation: answer.body.generation } };
      remember(key, merged);
      return merged;
    });
  }

  /**
   * WHAT CHANGED FOR A SELECTION SINCE A GENERATION THE CALLER HOLDS — the
   * delta alone, nothing kept, nothing merged. /db asks it with the
   * generation its census snapshot was read at (`sinceFor`), so "the store
   * moved" can say what moved for the selection on screen at O(delta), with
   * no second download of the census (DB-022, DB-060).
   * @param {FacetQuery} query @param {number} since
   * @returns {Promise<FacetAnswer>}
   */
  async function changesSince(query, since) {
    const key = facetKey(query);
    const answer = await read(query, key, since);
    return answer.ok ? { ...answer, delta: { ...facetCounts(answer.body), since, generation: answer.body.generation } } : answer;
  }

  /** The kept answer for a query, if any. @param {FacetQuery} query */
  const peek = (query) => kept.get(facetKey(query)) ?? null;

  /**
   * THE ROOT OF A FEED (WP-56b): its segments in the census's order, each
   * with its part's generation, underlyings counted, rows per rung and span.
   * One request of a few hundred bytes, whatever the census holds; never
   * kept, because it is the answer to "what does the feed hold NOW".
   * @param {string} feed
   */
  const root = (feed) => shared(`root|${feed}`, () => read({ feed }, `root|${feed}`, null));

  return { load, refresh, changesSince, peek, root };
}

/* ======================================================================
   THE FIRST PAINT FROM FACETS (WP-56b; DB-060, DB-G59, DB-G70, DB-A31)
   ----------------------------------------------------------------------
   A cold /db read the whole census (33,349,105 B on GDFL, 16,060,547 B on
   Zerodha on 11 Oct 2026) before it drew one bar of one instrument. The
   server now answers ONE INSTRUMENT'S census rows, in the census's own wire,
   with the feed's denominators for the months they name
   (`/census/facets.json?...&rows=census-tuples-v1`, `facets.rs`,
   `rows_answer`). `readScope` reads the root (which segments the feed holds;
   no list of segments lives here) and then that instrument's rows in each
   segment, and hands back rows the page prepares exactly as it prepares the
   whole census.

   IT ANSWERS THE CENSUS'S ROWS OR IT SAYS WHY NOT. A segment whose rows
   pass the server's cap (an underlying's whole option chain) is asked again
   at the expiry on screen, and is otherwise named in `unread`, never left
   out in silence. Anything else that goes wrong (a refusal, answers read at
   two generations, counts that do not add up) is `{ok: false, reason}`, and
   the caller reads the census whole and says that it did and why.
   ====================================================================== */

/**
 * @typedef {{underlying: string, expiry?: string | null}} ScopeWanted
 * @typedef {{segment: string, expiry: string | null, months: number, rows: number, generation: number}} ScopeRead
 * @typedef {{segment: string, why: string}} ScopeUnread
 * @typedef {{underlying: string, expiry: string | null, read: ScopeRead[], unread: ScopeUnread[], months: number}} CensusScope
 * @typedef {{fullest: Map<string, number>, support: Map<string, number>}} Observed
 * @typedef {{ok: true, rows: any[], observed: Observed, lag: CensusLag, readAt: number, scope: CensusScope, retried: number}
 *   | {ok: false, reason: string}} ScopeAnswer
 */

/** The URL of one instrument's census rows. @param {FacetQuery} query */
export const rowsUrl = (query) => `${facetsUrl(query)}&rows=${CENSUS_ENCODING}`;

/**
 * Why a body is not the rows answer for `query`, or null.
 * @param {any} body @param {FacetQuery} query @returns {string | null}
 */
export function rowsFault(body, query) {
  if (!body || typeof body !== 'object' || body.schema !== 1) return 'it is not a schema-1 facets answer';
  if (body.feed !== query.feed || body.segment !== query.segment) return `it answers ${body.feed}/${body.segment}, not ${query.feed}/${query.segment}`;
  if ((body.underlying ?? null) !== (query.underlying ?? null) || (body.expiry ?? null) !== (query.expiry ?? null)) return 'it answers another underlying or expiry';
  if (!Number.isSafeInteger(body.generation) || body.generation < 0) return 'its generation is not a whole number';
  if (!body.held || !Number.isSafeInteger(body.held.months) || body.held.months < 0 || !Number.isSafeInteger(body.held.rows) || body.held.rows < 0) return 'it does not count the rows it holds';
  if (!body.census || typeof body.census !== 'object') return 'it carries no census rows (an API older than the rows answer)';
  if (!Array.isArray(body.denominators)) return 'it carries no denominators';
  for (const row of body.denominators) {
    if (!Array.isArray(row) || row.length !== 4 || typeof row[0] !== 'string' || typeof row[1] !== 'string' ||
        !Number.isSafeInteger(row[2]) || row[2] < 0 || !Number.isSafeInteger(row[3]) || row[3] < 1) return 'a denominator is not [month, rung, fullest, keys]';
  }
  return null;
}

/**
 * The census rows of ONE instrument, from `/census/facets.json`, with the
 * feed's denominators for them. See the block above.
 *
 * @param {typeof ask} request
 * @param {string} feed
 * @param {ScopeWanted} wanted
 * @param {{ms?: number, now?: () => number, signal?: AbortSignal}} [options]
 * @returns {Promise<ScopeAnswer>}
 */
export async function readScope(request, feed, wanted, { ms = SCOPE_MS, now = Date.now, signal = new AbortController().signal,
  pause = (/** @type {number} */ waitMs) => new Promise((done) => setTimeout(done, waitMs)) } = {}) {
  /* A SATURATED SERVER IS ASKED ONCE MORE, NOT ANSWERED WITH THE WHOLE
     CENSUS (WP-56b). The API admits a bounded number of census reads at a
     time and answers the rest 429 with `retry-after` (seen on the real
     store: 12 of 20 requests at once). Falling back to the 33 MB census on
     the first 429 would put the largest read there is on a server that has
     just said it is full. Each request of the scope is asked again once,
     after the wait the server named (capped at `SCOPE_RETRY_MAX_MS`); a
     second 429 gives the scope up and says so. */
  let retried = 0;
  /** @param {string} url */
  const get = async (url) => {
    const first = await request(url, { cache: 'no-store', ms });
    if (first.status !== 429 || signal.aborted) return first;
    const named = Number(first.headers.get('retry-after'));
    await pause(Math.min(SCOPE_RETRY_MAX_MS, Number.isFinite(named) && named > 0 ? named * 1000 : SCOPE_RETRY_MAX_MS));
    retried += 1;
    return request(url, { cache: 'no-store', ms });
  };
  /** @param {string} reason @returns {ScopeAnswer} */
  const fail = (reason) => ({ ok: false, reason });
  const underlying = wanted.underlying;
  const expiry = wanted.expiry ?? null;
  /** A facets refusal's code and sentence (`{"error", "why"}`). @param {Response} response */
  const refusalOf = async (response) => {
    /** @type {any} */
    let refusal = null;
    try { refusal = await response.json(); } catch { refusal = null; }
    return { code: typeof refusal?.error === 'string' ? refusal.error : null,
      why: typeof refusal?.why === 'string' && refusal.why !== '' ? refusal.why
        : typeof refusal?.error === 'string' ? refusal.error : `HTTP ${response.status} and no reason named` };
  };
  const rootResponse = await get(facetsUrl({ feed, segment: /** @type {any} */ (null) }));
  if (!rootResponse.ok) {
    return fail(`/census/facets.json?feed=${feed} answered HTTP ${rootResponse.status}: ${(await refusalOf(rootResponse)).why}`);
  }
  const root = await rootResponse.json();
  const rootWhy = rootFault(root, feed);
  if (rootWhy) return fail(`/census/facets.json?feed=${feed}: ${rootWhy}`);
  const lag = censusLag(rootResponse.headers);
  // A DEGRADED CENSUS IS NOT DRAWN FROM (CE-77): the whole read refuses it
  // in its own words, so the scope steps aside for it.
  if (lag.degraded || root.degraded) return fail(`the census is degraded (${lag.degraded ?? root.degraded})`);
  if (root.segments.length === 0) return fail(`${feed} holds no census row`);

  /** @param {string} segment @param {string | null} at */
  const rowsOf = async (segment, at) => {
    /** @type {FacetQuery} */
    const query = { feed, segment, underlying, ...(at ? { expiry: at } : {}) };
    const response = await get(rowsUrl(query));
    if (!response.ok) return { ok: false, ...(await refusalOf(response)), query, body: null, headers: response.headers };
    const body = await response.json();
    const fault = rowsFault(body, query);
    return fault
      ? { ok: false, code: null, why: fault, query, body: null, headers: response.headers }
      : { ok: true, code: null, why: '', query, body, headers: response.headers };
  };
  const answers = await Promise.all(root.segments.map(async (/** @type {any} */ row) => {
    let answer = await rowsOf(row.segment, null);
    // A CHAIN TOO LONG TO LIST WHOLE IS ASKED AT THE EXPIRY ON SCREEN.
    if (!answer.ok && answer.code === 'rows_too_many' && expiry) answer = await rowsOf(row.segment, expiry);
    return { row, answer };
  }));

  /** @type {ScopeRead[]} */
  const read = [];
  /** @type {ScopeUnread[]} */
  const unread = [];
  /** @type {any[]} */
  const rows = [];
  const fullest = new Map();
  const support = new Map();
  for (const { row, answer } of answers) {
    if (!answer.ok && answer.code === 'rows_too_many') {
      // THE CHAIN AT THE EXPIRY ON SCREEN COULD NOT BE LISTED: the scope
      // would be drawn without the selection it was read for, and a grid
      // drawn from it would say the store holds no such instrument-month
      // (the second check's blocking 2). No scope then: the whole census is
      // read in the same flight and the page stays on its reading state.
      if (expiry && answer.query.expiry) return fail(`${underlying} in ${row.segment} at ${expiry}: ${answer.why}`);
      unread.push({ segment: row.segment, why: answer.why });
      continue;
    }
    if (!answer.ok) return fail(`${underlying} in ${row.segment}: ${answer.why}`);
    const body = answer.body;
    // ONE CENSUS, OR NONE: every answer must be of the generation the root
    // was, vendor-wide and for its own part.
    const at = censusLag(answer.headers).generation;
    if (at !== lag.generation || body.generation !== row.generation) {
      return fail(`the census moved between the answers (${row.segment}: generation ${group(row.generation)}, then ${group(body.generation)})`);
    }
    const decoded = await decodeCensus(body.census, feed, signal);
    let bars = 0;
    const head = `${body.exchange}-${row.segment}-${underlying}`;
    for (const item of decoded) {
      if (item.instrument !== head && !String(item.instrument).startsWith(`${head}-`)) {
        return fail(`${row.segment} answered a row of ${item.instrument}, which is not ${underlying}`);
      }
      bars += item.rows;
    }
    if (decoded.length !== body.held.months || bars !== body.held.rows) {
      return fail(`${row.segment} counts ${group(body.held.months)} row(s) of ${group(body.held.rows)} bars and carries ${group(decoded.length)} of ${group(bars)}`);
    }
    for (const [month, rung, most, keys] of body.denominators) {
      const key = denomKey(month, rung);
      if (fullest.has(key) && (fullest.get(key) !== most || support.get(key) !== keys)) {
        return fail(`two answers disagree on the fullest row of ${month} at ${rung}`);
      }
      fullest.set(key, most);
      support.set(key, keys);
    }
    for (const item of decoded) {
      const most = fullest.get(denomKey(item.month, item.timeframe));
      if (most === undefined || most < item.rows) return fail(`${row.segment} names no fullest row for ${item.month} at ${item.timeframe} that holds its own`);
    }
    read.push({ segment: row.segment, expiry: body.expiry ?? null, months: decoded.length, rows: bars, generation: body.generation });
    for (const item of decoded) rows.push(item);
  }
  if (rows.length === 0) {
    return fail(unread.length > 0
      ? `${underlying}'s rows could not be listed (${unread.map((u) => `${u.segment}: ${u.why}`).join('; ')})`
      : `${feed} holds no census row of ${underlying}`);
  }
  // THE WHOLE BODY'S ORDER: newest month first, and within a month the
  // segments in the order the root listed them (each answer is already in
  // the census's order, and the sort is stable).
  rows.sort((a, b) => (a.month < b.month ? 1 : a.month > b.month ? -1 : 0));
  return { ok: true, rows, observed: { fullest, support }, lag, readAt: now(), retried,
    scope: { underlying, expiry, read, unread, months: rows.length } };
}

/**
 * WHAT THE SCOPE ON SCREEN CANNOT ANSWER (WP-56b), decided in one place so a
 * test holds it: `null` when one instrument's rows answer the page as it
 * stands, else `why` the whole census is needed and whether the selection
 * on screen is `missing` from the rows read.
 *
 * `missing` IS NOT 'THE STORE HOLDS NONE'. One instrument's rows cannot say
 * that an instrument-month they do not list is absent from the store: only
 * the whole census can. So while a scope is on screen and the selection is
 * missing from it, the page shows its reading state and the whole census is
 * read; the grid's 'No instrument-month matches this query' is drawn from
 * the whole census only. (The second check found it drawn from a scope for
 * 1.3 s over an option the store holds 277,359 bars of.)
 *
 * @param {{scope: CensusScope | null, view: string, defaultView: string, filter: string, kind: string,
 *   reconciling: boolean, matched: number}} page `matched` is how many rows
 *   of the scope the selection on screen matches.
 * @returns {{why: string, missing: boolean} | null}
 */
export function scopeNeed({ scope, view, defaultView, filter, kind, reconciling, matched }) {
  if (!scope) return null;
  if (view !== defaultView) return { why: 'the Coverage grid lists every instrument', missing: false };
  if (filter !== '' && filter !== scope.underlying) {
    return { why: `${filter} was chosen and only ${scope.underlying} had been read`, missing: true };
  }
  if (filter !== '' && kind !== '' && !reconciling && matched === 0) {
    return { why: `the rows read for ${scope.underlying} hold nothing for the selection on screen`, missing: true };
  }
  return null;
}

/**
 * THE INSTRUMENT A FEED CHOSEN LATER IS READ FOR (WP-56b): the instrument
 * ON SCREEN, so a feed picked after the page opened is asked for what the
 * reader is looking at and not for the page's opening instrument (the
 * second check's follow-up: 'first' was read once, so the new feed's scope
 * missed the selection and its whole census was read). `null` when the page
 * needs every row (the Coverage grid): the whole census from the start.
 *
 * @param {{view: string, defaultView: string, filter: string, expiry: string | null | undefined, fallback: string}} page
 *   `fallback` is the page's opening instrument, for a page with none chosen.
 * @returns {ScopeWanted | null}
 */
export function scopeWanted({ view, defaultView, filter, expiry, fallback }) {
  if (view !== defaultView) return null;
  return { underlying: filter !== '' ? filter : fallback, expiry: expiry || null };
}

/**
 * WHAT ONE READ OF THE SHARED STORE DOES (WP-56b), decided in one place so
 * it can be held to its rules by a test:
 *
 *   - `scoped`: read one instrument's rows. Only when the caller named an
 *     instrument (`first`), no subscriber has asked this feed's WHOLE census
 *     (`wholeAsked`), and the store is cold for the feed or already holds a
 *     scope (a refresh of it).
 *   - `clear`: drop the value before the request leaves. Always on a feed
 *     change; and when the value is one instrument's rows and the caller
 *     wants the whole census, unless the caller is the page that drew them
 *     (`keep`). No other page is ever shown one instrument's rows as the
 *     store's.
 *   - `key`: what makes two calls the same request. A scope and a whole
 *     read of one (feed, generation) are different requests.
 *
 * @param {{feed: string, generation: number, first: ScopeWanted | null, valueFeed: string | null,
 *   valueScoped: boolean, wholeAsked: boolean, keep?: boolean}} call
 */
export function planRead({ feed, generation, first, valueFeed, valueScoped, wholeAsked, keep = false }) {
  const scoped = first !== null && !wholeAsked && (valueFeed !== feed || valueScoped);
  const wholeKey = `${feed}#${generation}`;
  return { scoped, wholeKey,
    key: scoped ? `${wholeKey}#scope:${/** @type {ScopeWanted} */ (first).underlying}:${/** @type {ScopeWanted} */ (first).expiry ?? ''}` : wholeKey,
    clear: valueFeed !== feed || (valueScoped && !scoped && !keep) };
}

/**
 * The scope as one clause: `INDEX 93 row(s)`, `FNO at 2019-06-27 412 row(s)`.
 * @param {CensusScope} scope
 */
export const scopeText = (scope) => scope.read.filter((part) => part.months > 0)
  .map((part) => `${part.segment}${part.expiry ? ` at ${part.expiry}` : ''} ${group(part.months)} row(s)`).join(', ');

/* ======================================================================
   THE REFRESH LOG (WP-56b; the WP-56 second agent's follow-up 11)
   ----------------------------------------------------------------------
   How a read of the census ended (a first read, a delta merged, unchanged,
   a whole read and why, a failure and why) was said in the banner only, and
   the next read replaced the sentence: nobody could look up later why this
   page read 33 MB at 14:02. Every read now leaves one entry in a log kept
   in the browser's storage (the newest `REFRESH_LOG_KEEP`, across reloads
   and tabs of this origin), which /db lists and filters.

   IT IS THE PAGE'S LOG, NOT THE SERVER'S. The API has no route a page can
   write an event to, and its access log keeps the route and the selection
   of a sampled request, not why a client fell back. Until it has one, this
   is where the reason is kept; a storage that refuses the write is said
   (`fault`) and the log then holds this tab's reads only.
   ====================================================================== */
export const REFRESH_LOG_KEY = 'brutex.census.refreshes.v1';
export const REFRESH_LOG_KEEP = 200;

/**
 * @typedef {{at: number, feed: string, kind: 'first' | 'delta' | 'unchanged' | 'whole' | 'failed' | 'scoped', why: string | null,
 *   since: string | null, rows: number, added: number, replaced: number, generation: number | null, held: number | null,
 *   worker: string | null}} RefreshEntry
 */

/**
 * One read of the census as a log entry.
 * @param {{feed: string, at: number, how?: CensusHow | null, lag?: CensusLag | null, held?: number | null,
 *   worker?: string | null, error?: string | null, first?: boolean}} read
 * @returns {RefreshEntry}
 */
export function refreshEntry({ feed, at, how: read = null, lag = null, held = null, worker = null, error = null, first = false }) {
  const kind = error !== null ? 'failed' : first && (read === null || read.kind === 'whole') ? 'first' : read?.kind ?? 'whole';
  return { at, feed, kind, why: error ?? read?.why ?? null, since: read?.since ?? null, rows: read?.rows ?? 0,
    added: read?.added ?? 0, replaced: read?.replaced ?? 0, generation: lag?.generation ?? null, held, worker };
}

/** One entry as the sentence the log lists and a filter searches. @param {RefreshEntry} entry */
export function refreshText(entry) {
  const what = entry.kind === 'scoped' ? `read only the census rows of the instrument on screen from /census/facets.json (${entry.why ?? 'no instrument named'}; ${group(entry.rows)} row(s)), not the whole census`
    : entry.kind === 'first' ? `read the whole census (the first read of this feed on this page${entry.why ? `; ${entry.why}` : ''})`
    : entry.kind === 'delta' ? `read only the ${group(entry.rows)} row(s) changed since ${entry.since ?? 'the generation held'} (${group(entry.added)} new, ${group(entry.replaced)} changed in place)`
      : entry.kind === 'unchanged' ? (entry.why
        ? `asked for the whole census (${entry.why}) and the server answered that the census held is current; no body was read`
        : `found nothing changed since ${entry.since ?? 'the generation held'}`)
        : entry.kind === 'failed' ? `failed: ${entry.why ?? 'no reason named'}`
          : `read the whole census again${entry.why ? `: ${entry.why}` : ''}`;
  const now = entry.kind === 'failed' ? '' : `; generation ${entry.generation === null ? 'not named' : group(entry.generation)}` +
    `${entry.held === null ? '' : `, ${group(entry.held)} row(s) held`}`;
  return `${clockOf(entry.at)} · ${entry.feed} · ${what}${now}${entry.worker ? ` (${entry.worker})` : ''}.`;
}

/**
 * The entries whose sentence holds every word of `text` (case-insensitive),
 * newest first. An empty filter is every entry.
 * @param {RefreshEntry[]} entries @param {string} text
 */
export function searchRefreshes(entries, text) {
  const words = String(text ?? '').toLowerCase().split(/\s+/).filter(Boolean);
  const out = [];
  for (let i = entries.length - 1; i >= 0; i--) {
    const said = `${refreshText(entries[i])} ${entries[i].kind}`.toLowerCase();
    if (words.every((word) => said.includes(word))) out.push(entries[i]);
  }
  return out;
}

/**
 * The log: `note` appends one entry and saves the newest `keep`; `list` is
 * oldest first. `fault` is why the last save or the first read of the
 * storage failed, or null.
 * @param {{getItem: (key: string) => string | null, setItem: (key: string, value: string) => void} | null} storage
 * @param {{keep?: number, key?: string}} [options]
 */
export function createRefreshLog(storage, { keep = REFRESH_LOG_KEEP, key = REFRESH_LOG_KEY } = {}) {
  /** @type {RefreshEntry[]} */
  let entries = [];
  /** @type {string | null} */
  let fault = storage ? null : 'this browser offers no storage, so the log holds only what this tab read';
  /** Reads what other tabs and earlier loads saved; a log that cannot be read is said, never guessed. */
  function load() {
    if (!storage) return;
    try {
      const text = storage.getItem(key);
      if (text === null) return;
      const saved = JSON.parse(text);
      if (!Array.isArray(saved) || saved.some((entry) => !entry || typeof entry.at !== 'number' || typeof entry.feed !== 'string' || typeof entry.kind !== 'string')) {
        fault = 'the refresh log saved in this browser is not a list of entries, so it was not read; it is replaced by the next read';
        return;
      }
      entries = saved.slice(-keep);
    } catch (why) {
      fault = `the refresh log saved in this browser could not be read (${String(/** @type {any} */ (why)?.message ?? why)})`;
    }
  }
  load();
  return {
    /** @param {RefreshEntry} entry */
    note(entry) {
      // ANOTHER TAB MAY HAVE WRITTEN SINCE: its entries are taken first, so
      // two tabs do not overwrite each other's log.
      const mine = entries;
      const before = fault;
      fault = storage ? null : before;
      entries = [];
      load();
      const seen = new Set(entries.map((e) => `${e.at}|${e.feed}|${e.kind}`));
      for (const e of mine) if (!seen.has(`${e.at}|${e.feed}|${e.kind}`)) entries.push(e);
      entries.push(entry);
      entries.sort((a, b) => a.at - b.at);
      if (entries.length > keep) entries = entries.slice(-keep);
      if (!storage) return;
      try {
        storage.setItem(key, JSON.stringify(entries));
      } catch (why) {
        fault = `the refresh log could not be saved (${String(/** @type {any} */ (why)?.message ?? why)}), so it holds only what this tab read`;
      }
    },
    list: () => entries,
    get fault() { return fault; }
  };
}

/* ======================================================================
   THE BANNER'S WORDS (WP-56, D-3327; DB-022, DB-G03, DB-G11, DB-G60,
   DB-G78, DB-G84)
   ----------------------------------------------------------------------
   Plain functions over the census headers and the watch, so a test can
   hold every sentence /db's banner draws. Each line is one fact; a fact
   with nothing to say is not a line. The server's own sentence is quoted
   where it has one, never paraphrased into a new claim.
   ====================================================================== */
/** @typedef {{key: string, tone: 'info' | 'warn', text: string}} BannerLine */

/** `10 Oct 2026, 19:58:03` in IST, or `—`. @param {number | null | undefined} ms */
const clockOf = (ms) => (Number.isFinite(ms) ? `${dayLabel(/** @type {number} */ (ms))}, ${secondLabel(/** @type {number} */ (ms))} IST` : '—');

/** `INDEX 2,097,116 · CASH 2,114,847 · FNO 2,841,866`. @param {Record<string, number>} parts */
const partsText = (parts) => Object.entries(parts).map(([segment, n]) => `${segment} ${group(n)}`).join(' · ');

/**
 * The census's own state, in lines: when it was read and at which
 * generation, a census that cannot be called complete, headers this page
 * could not read, and a census decoded on the page because no worker could.
 *
 * `how` (optional) is how the rows came to be current (`CensusHow`): a
 * refresh that merged a delta says how many rows it read, one that found
 * nothing changed says so, and one that had to read the whole census again
 * says why, as a warning: a 33 MB read is never silent.
 *
 * `scope` (optional) is set while the page holds ONE INSTRUMENT'S rows and
 * not the whole census (`readScope`): the line then says exactly that, what
 * was and was not read, and that the lists below cover that instrument
 * alone. `scopeWhy` is why a first paint could not be drawn that way.
 * `reading` is true while the whole census is being read behind a scope.
 *
 * @param {{feed: string, at: number | null, lag: CensusLag | null, worker: string | null,
 *   head: HeadCheck | null, how?: CensusHow | null, scope?: CensusScope | null,
 *   scopeWhy?: string | null, reading?: boolean}} census
 * @returns {BannerLine[]}
 */
export function censusLines({ feed, at, lag, worker, head, how: read = null, scope = null, scopeWhy = null, reading = false }) {
  /** @type {BannerLine[]} */
  const out = [];
  const moved = censusMoved({ feed, lag }, head);
  const generation = lag?.generation ?? null;
  const checked = moved === false && head ? ` Checked again at ${clockOf(head.at)}: unchanged.` : '';
  const verb = read?.kind === 'delta'
    ? `Census refreshed at ${clockOf(at)} by reading only the ${group(read.rows)} row(s) changed since the generation this page held (${group(read.added)} new, ${group(read.replaced)} changed in place), now`
    : read?.kind === 'unchanged'
      ? `Census checked at ${clockOf(at)} and unchanged since this page read it,`
      : `Census read at ${clockOf(at)},`;
  if (scope) {
    // ONE INSTRUMENT, SAID AS ONE (WP-56b): the lists and totals below are
    // that instrument's alone, and the line says so and what was left out.
    out.push({ key: 'read', tone: 'info', text: `Read at ${clockOf(at)},` +
      (generation === null ? ' at a generation this API does not name' : ` generation ${group(generation)}`) +
      (lag?.parts ? ` (${partsText(lag.parts)})` : '') +
      `: only the census rows of ${scope.underlying} (${scopeText(scope)}) from /census/facets.json, not the whole census.` +
      ` Every list, total and offset on this page covers ${scope.underlying} alone until the whole census is read` +
      `${reading ? ', which is being read now.' : ', which happens when a control outside the bar rows is used.'}` +
      `${scope.unread.length > 0 ? ` Not read yet: ${scope.unread.map((part) => `${part.segment} (${part.why})`).join('; ')}.` : ''}${checked}` });
  } else {
    out.push({ key: 'read', tone: 'info', text: verb +
      (generation === null ? ' at a generation this API does not name' : ` generation ${group(generation)}`) +
      (lag?.parts ? ` (${partsText(lag.parts)})` : '') +
      `: every list, total and offset on this page is that snapshot.${checked}` });
  }
  if (!scope && scopeWhy) {
    out.push({ key: 'scope', tone: 'info', text: `This page read the whole census before its first paint, not only the instrument on screen: ${scopeWhy}.` });
  }
  if (read?.why && read.kind === 'unchanged') {
    // NO BODY WAS READ (WP-56b): the delta could not be asked or was not
    // believed, the whole census was asked for, and the server answered
    // that the copy this page holds is current. Saying "read the whole
    // census again" beside "unchanged" said two things, one of them false.
    out.push({ key: 'whole', tone: 'info', text: `This refresh could not ask for only what changed (${read.why}), so it asked for the whole census; the server answered that the census this page holds is current, and no body was read.` });
  } else if (read?.why && read.kind !== 'scoped') {
    out.push({ key: 'whole', tone: 'warn', text: `This refresh read the whole census again, not only what changed: ${read.why}.` });
  }
  if (!lag || lag.state === null) {
    out.push({ key: 'state', tone: 'warn', text: 'This API sends no census state, so whether the census is complete is not known.' });
  } else if (lag.state === 'behind') {
    const days = lag.behindDays !== null && lag.behindDays > 0
      ? `${group(lag.behindDays)} import day(s)${lag.behindFrom ? ` from ${dayLabel(lag.behindFrom)}` : ''} are on disk and not in this census, so their bars are not listed here`
      : 'the import journal names days this census has not counted, so their bars are not listed here';
    out.push({ key: 'state', tone: 'warn', text: `The ${feed} census is behind its import: ${days}.${lag.reason ? ` Server: ${lag.reason}` : ''}` });
  } else if (lag.state === 'capped') {
    const room = lag.capacity !== null && lag.headroom !== null
      ? `${group(lag.headroom)} of ${group(lag.capacity)} entries left in its fullest part, so what an import adds next may not be listed`
      : 'it is at or near the entries it can hold, so what an import adds next may not be listed';
    out.push({ key: 'state', tone: 'warn', text: `The ${feed} census is capped: ${room}.${lag.reason ? ` Server: ${lag.reason}` : ''}` });
  } else if (lag.reason) {
    out.push({ key: 'state', tone: 'warn', text: `The ${feed} census is ${lag.state}: ${lag.reason}` });
  }
  if (lag && lag.faults.length > 0) {
    out.push({ key: 'faults', tone: 'warn', text: `Census headers this page could not read: ${lag.faults.join('; ')}.` });
  }
  if (head && head.feed === feed && head.error) {
    out.push({ key: 'head', tone: 'info', text: `Whether the store moved since could not be checked at ${clockOf(head.at)}: ${head.error}.` });
  }
  if (worker) out.push({ key: 'worker', tone: 'info', text: `${worker[0].toUpperCase()}${worker.slice(1)}.` });
  return out;
}

/**
 * "The store moved", in lines: the generations, when it was seen, the
 * snapshot the page still shows, and — when the facets deltas for the
 * instrument on screen are in hand — what moved for it.
 *
 * @param {{from: number, to: number, segments: string[]}} moved
 * @param {HeadCheck | null} head
 * @param {number | null} readAt
 * @param {string | null} underlying
 * @param {Array<{ok: boolean, status: number, query: FacetQuery, body: any, reason: string | null}> | null} answers
 * @returns {Array<{key: string, text: string, through?: number}>}
 */
export function movedLines(moved, head, readAt, underlying, answers) {
  const out = [{ key: 'moved', text: `The store moved after this page read it: generation ${group(moved.from)} → ${group(moved.to)}` +
    `${moved.segments.length ? ` (${moved.segments.join(', ')} changed)` : ''}, seen at ${clockOf(head?.at)}. ` +
    `This page still shows the snapshot read at ${clockOf(readAt)}.` }];
  if (!underlying || !answers) return out;
  /** @type {string[]} */
  const changed = [];
  for (const answer of answers) {
    const segment = answer.query.segment;
    if (!answer.ok) {
      const why = answer.status === 410
        ? 'the server no longer holds the changes since this snapshot'
        : answer.status === 409
          ? "the server's census for it stands below this snapshot's generation (read whole since)"
          : answer.reason ?? 'the delta could not be read';
      out.push({ key: `delta-${segment}`, text: `${underlying} ${segment}: ${why}; Refresh reads it whole.` });
      continue;
    }
    const body = answer.body;
    const months = body.months.length, expiries = body.expiries.length;
    const last = body.underlyings[0]?.last_ts;
    if (months === 0 && expiries === 0 && body.underlyings.length === 0) continue;
    changed.push(segment);
    // THE PER-SERIES MARKER (DB-022, WP-56b): the store moved for THIS
    // series after the page read it, and the day its bars now reach is the
    // day an import or a pull has got to. `through` carries that stamp for
    // a reader that marks the series itself.
    const progress = Number.isFinite(last) ? `import or pull in progress through ${dayLabel(last / 1000)}: ` : '';
    out.push({ key: `delta-${segment}`, ...(Number.isFinite(last) ? { through: last } : {}),
      text: `${underlying} ${segment}: ${progress}${group(months)} month(s)` +
      `${expiries ? ` and ${group(expiries)} expiry(ies)` : ''} changed since this snapshot` +
      `${Number.isFinite(last) ? `; its newest bar is now ${clockOf(last / 1000)}` : ''}.` });
  }
  if (changed.length === 0 && answers.every((answer) => answer.ok)) {
    out.push({ key: 'delta-none', text: `Nothing changed for ${underlying} (${answers.map((a) => a.query.segment).join(', ')}): what moved is other instruments.` });
  }
  return out;
}
