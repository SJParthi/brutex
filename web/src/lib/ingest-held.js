/**
 * WHAT THE STORE HOLDS FOR ONE /ingest ROW — the held-months join, out of the
 * page (R4-01, V1).
 *
 * # THE DEFECT
 *
 * `/ingest?feed=gdfl` read every share as "0 / unverified · NEVER IMPORTED ·
 * 97/97 unproved" while the census the page reads held 97 one-second months of
 * each. Measured on a shadow of the operator's store, 14 Oct 2026, GET only:
 * `/instruments.json?feed=gdfl` files RELIANCE as exchange NSE, segment CASH,
 * and `/store.json?feed=gdfl&encoding=census-tuples-v1` holds
 * `NSE-CASH-RELIANCE` at `1s` for 2018-09..2026-09 — so the key join was
 * right. What was wrong was WHEN and HOW it was read:
 *
 *   1. Until the census lands, every probe answers "no file", and the page
 *      turned "not looked up yet" into "never imported" and "0". On GDFL that
 *      is the first seconds of every load (2.1 million month rows, 184 MB),
 *      and for good when the read fails. `censusReadiness` names the state and
 *      the month verdict is `loading` until it is ready.
 *   2. A month held at a rung with no bars-per-session count (one second) read
 *      `unknown` and counted as unproved. It is `held` now (`monthVerdict`
 *      `countOnly`), counted, never compared.
 *   3. The rung list said a second "has no store directory" on every feed,
 *      false for an archive feed since D-2802. `rungStored` reads it per feed
 *      from `/feeds.json`'s `rungs` and `derived_rungs`.
 *
 * # COST
 *
 * A spot month is ONE probe of the shared `store.byCell` index. A contract
 * segment sums every contract of its underlying, which no shared index holds:
 * that fold walks the census once per census STAMP (feed, generation, read
 * time), in slices, and is reused by every later render and every tick change
 * until the census moves (`passes` counts the walks, for the test that pins
 * it). Pure apart from the slicing hook, so `web/tests/ingest-held.test.js`
 * drives it under `node --test`.
 */

/**
 * The census spells an instrument `EXCHANGE-SEGMENT-SYMBOL`.
 * @param {{ exchange: string, segment: string, symbol: string }} row
 */
export function censusKey(row) {
  return `${row.exchange}-${row.segment}-${row.symbol}`;
}

/**
 * @typedef {{ ready: boolean, word: 'ready' | 'loading' | 'failed' | 'none', why: string }} Readiness
 */

/**
 * WHETHER THE CENSUS ON HAND ANSWERS FOR THIS FEED, and in words when not.
 *
 * `storeFeed` is the feed the shared value was published for (`store.feed`):
 * it is set only when a read of that feed has been folded, so it is the one
 * field that says "these counts are this feed's". A refresh of the same feed
 * keeps the previous value and stays ready; a feed switch clears it first.
 *
 * @param {{ feed: string | null | undefined, storeFeed: string | null | undefined, state?: string, error?: string | null }} s
 * @returns {Readiness}
 */
export function censusReadiness({ feed, storeFeed, state = '', error = null }) {
  if (!feed) return { ready: false, word: 'none', why: 'No feed is selected, so no census is read.' };
  if (storeFeed === feed) return { ready: true, word: 'ready', why: '' };
  if (typeof error === 'string' && error !== '') {
    return { ready: false, word: 'failed', why: `The store census of ${feed} could not be read: ${error}. No month is judged against a census that did not load; Refresh reads it again.` };
  }
  return {
    ready: false,
    word: 'loading',
    why: state === 'reading'
      ? `The store census of ${feed} is being read (/store.json). No month is judged until it lands.`
      : `The store census of ${feed} has not been read yet. No month is judged until it lands.`
  };
}

/**
 * WHETHER `feed` FILES BARS AT `dir`, from `/feeds.json` — never from a list
 * in this file. `rungs` are what the feed serves and the store files;
 * `derived_rungs` are folded from finer bars and filed too (D-3321).
 *
 * @param {{ wire?: string, display?: string, rungs?: unknown, derived_rungs?: unknown } | null | undefined} feed
 * @param {string} dir
 * @returns {{ stored: boolean | null, why: string }} `null` when this server
 *   sent no `rungs` field, which is a missing field, not a "no".
 */
export function rungStored(feed, dir) {
  const name = feed?.display ?? feed?.wire ?? 'this feed';
  if (!feed || !Array.isArray(feed.rungs)) {
    return { stored: null, why: `This server sent no rungs for ${name} on /feeds.json, so whether the store files ${dir} bars for it is not stated here. A missing field, not a refusal: a server that predates it.` };
  }
  const derived = Array.isArray(feed.derived_rungs) ? feed.derived_rungs : [];
  if (feed.rungs.includes(dir)) return { stored: true, why: `${name} serves ${dir} and the store files it.` };
  if (derived.includes(dir)) return { stored: true, why: `${name}'s ${dir} bars are folded from its finer bars and filed beside them.` };
  const all = [...feed.rungs, ...derived];
  return {
    stored: false,
    why: `${name} files no ${dir} bars: /feeds.json lists what it stores as ${all.length > 0 ? all.join(', ') : 'nothing'}. Not a store limit — the store has a directory for it — but nothing of this feed is filed there.`
  };
}

/**
 * @typedef {{ instrument: string, timeframe: string, month: string, rows: number }} Cell
 * @typedef {{ stamp: string | null, published: string | null, by: Map<string, number>, passes: number, ready: boolean, folding: string | null, error: string | null, abort: AbortController | null }} HeldJoin
 *   `published` is the stamp `by` was folded from — the previous census while
 *   a newer one is walked.
 */

/** A join with nothing folded yet. @returns {HeldJoin} */
export function createHeldJoin() {
  return { stamp: null, published: null, by: new Map(), passes: 0, ready: false, folding: null, error: null, abort: null };
}

/**
 * Stop the walk in flight, if any — the page leaving. A later fold of the same
 * stamp walks again.
 * @param {HeldJoin} join
 */
export function stopFold(join) {
  join.abort?.abort();
  join.abort = null;
}

/**
 * The stamp one census reading is known by, and only another stamp is folded
 * again.
 *
 * KEYED ON THE SERVER'S CENSUS, NOT ON THE READ. When the answer carried the
 * census's own generation (`x-brutex-census-generation`, with its epoch), the
 * stamp is (feed, epoch, generation): a re-read of an unchanged census is the
 * SAME stamp and walks nothing. Before this the stamp was this browser's read
 * counter and read time, so every re-read of an unchanged answer re-walked all
 * 2.1 M cells (about 35 s on the R4-01 shadow) and put every contract row back
 * to `census loading` (R4-01 repair round 1). With no server generation the
 * read itself is the stamp, as before: walked again, never assumed unchanged.
 *
 * @param {{ feed: string | null, generation?: number, at?: number | null, server?: number | null, epoch?: string | null }} s
 *   `generation` and `at` are this browser's read; `server` and `epoch` the
 *   census's own, from the answer's headers (`store.lag`)
 */
export const censusStamp = ({ feed, generation = 0, at = null, server = null, epoch = null }) =>
  Number.isSafeInteger(server) && /** @type {number} */ (server) >= 0
    ? `${feed ?? ''}#census#${epoch ?? ''}#${server}`
    : `${feed ?? ''}#${generation}#${at ?? ''}`;

/** The feed a stamp is of. @param {string | null} stamp */
const feedOfStamp = (stamp) => (stamp === null ? null : stamp.slice(0, stamp.indexOf('#')));

/**
 * FOLD THE CONTRACT SEGMENTS OF ONE CENSUS, ONCE PER STAMP.
 *
 * Every derivative cell's rows are summed under (underlying, segment, rung,
 * month) for EVERY underlying, so ticking another one never walks again. The
 * walk is `cells.length` steps and runs in slices (`slices`, the census
 * fold's own `inSlices`), so no task holds the page. A stamp already folded,
 * or being folded, is not walked again.
 *
 * THE WALK IS CANCELLED BY A NEWER STAMP, NEVER BY A RE-RENDER. Its abort is
 * the join's own: a caller re-asking for the stamp in flight gets `false` and
 * the walk carries on. A page effect that aborted on every re-run left the
 * walk dead and the stamp marked in flight — every contract row read
 * `census loading` for good (seen on the R4-01 shadow before this rule).
 *
 * @param {HeldJoin} join
 * @param {string} stamp
 * @param {readonly Cell[]} cells
 * @param {{
 *   parse: (instrument: string) => { underlying: string | null },
 *   segmentOf: (parsed: any) => string | null,
 *   slices?: (n: number, work: (start: number, end: number) => void, signal: AbortSignal) => Promise<void>
 * }} how
 * @returns {Promise<boolean>} true when this call walked the census.
 */
export async function foldDerivatives(join, stamp, cells, how) {
  if (join.stamp === stamp && (join.ready || join.folding === stamp)) return false;
  join.abort?.abort();
  const abort = new AbortController();
  join.abort = abort;
  const by = new Map();
  join.stamp = stamp;
  join.ready = false;
  join.folding = stamp;
  join.error = null;
  join.passes += 1;
  const work = (/** @type {number} */ start, /** @type {number} */ end) => {
    for (let i = start; i < end; i += 1) {
      const c = cells[i];
      const p = how.parse(c.instrument);
      const seg = how.segmentOf(p);
      if (seg === null || seg === 'spot' || p.underlying === null) continue;
      const k = `${p.underlying}|${seg}|${c.timeframe}|${c.month}`;
      by.set(k, (by.get(k) ?? 0) + c.rows);
    }
  };
  try {
    if (how.slices) await how.slices(cells.length, work, abort.signal);
    else work(0, cells.length);
  } catch (why) {
    // A NEWER STAMP REPLACED THIS ONE, OR THE WALK FAILED: either way the sums
    // are partial and are never published as the store's.
    if (join.folding === stamp) {
      join.folding = null;
      join.error = why instanceof Error ? why.message : String(why);
    }
    return true;
  }
  if (join.stamp !== stamp) return true;
  join.by = by;
  join.published = stamp;
  join.ready = true;
  join.folding = null;
  if (join.abort === abort) join.abort = null;
  return true;
}

/**
 * THE BARS ONE (instrument, segment, rung, month) HOLDS.
 *
 * @param {HeldJoin} join the contract fold (read for a non-spot segment only)
 * @param {(instrument: string, timeframe: string, month: string) => number | undefined} probe
 *   one lookup of the shared census index (`store.byCell`)
 * @param {Readiness} census
 * @param {{ exchange: string, segment: string, symbol: string }} m the catalogue row
 * @param {string} seg `spot` or a contract segment
 * @param {string} dir the rung
 * @param {string} ym `YYYY-MM`
 * @returns {{ bars: number | null, loading: boolean, stale: boolean }} `bars`
 *   is null when the store holds no file for it (which is not zero bars);
 *   `loading` is true when it has not been looked up, so neither a count nor
 *   an absence is known; `stale` is true when the count is the PREVIOUS
 *   census's of the same feed, shown while a newer one is walked — labelled
 *   so, never passed off as the newer census's.
 */
export function heldMonth(join, probe, census, m, seg, dir, ym) {
  if (!census.ready) return { bars: null, loading: true, stale: false };
  if (seg === 'spot') {
    const bars = probe(censusKey(m), dir, ym);
    return { bars: bars === undefined ? null : bars, loading: false, stale: false };
  }
  const key = `${m.symbol}|${seg}|${dir}|${ym}`;
  if (join.ready) {
    const bars = join.by.get(key);
    return { bars: bars === undefined ? null : bars, loading: false, stale: false };
  }
  // A NEWER CENSUS OF THE SAME FEED IS BEING WALKED: the previous one's sums
  // stand, labelled. Another feed's sums, or a walk that failed, never stand.
  if (join.published !== null && join.error === null && join.folding !== null && feedOfStamp(join.published) === feedOfStamp(join.stamp)) {
    const bars = join.by.get(key);
    return { bars: bars === undefined ? null : bars, loading: false, stale: true };
  }
  return { bars: null, loading: true, stale: false };
}
