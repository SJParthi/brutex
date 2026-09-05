/**
 * THE TERMINAL'S RUNTIME MODEL — what `/terminal` may show, and what it may
 * not, decided by the store rather than by this file.
 *
 * # The rule this module exists to keep
 *
 * `/terminal` recreates a broker's markets terminal. A broker's terminal is fed
 * by a live tick stream and shows eight asset classes; this store holds monthly
 * OHLCV bar files and holds them only for what has actually been pulled. Those
 * two facts do not line up, and every honest thing this page does follows from
 * refusing to pretend they do.
 *
 * The source terminal resolves the mismatch by printing `0.00` and `-100.00 %`
 * for instruments it has no quote for — measured across the captures this page
 * was built from: whole screens of `SEP FUT` rows at `LTP 0.00`, `Change %
 * -100.00 %`, `Open Interest 0`, every one of them an absence wearing a
 * measurement's clothes. `CLAUDE.md` section 4 bans exactly that shape, and
 * this module is where the ban is enforced: **nothing here ever returns a
 * number it did not read.** An unknown is `null` beside a `why`, and the page
 * renders the `why`.
 *
 * # Where every figure comes from, and what it costs
 *
 * | Column | Source | Cost |
 * |---|---|---|
 * | which instruments exist | `/store.json` census, via `$lib/store.svelte.js` | ONE request per (feed, generation), ETag → 304 |
 * | which months, which rungs | the same census | free, same pass |
 * | Change, Change % | the census's own `chg_bps` | free, one Map probe |
 * | LTP, Open, High, Low, Volume, OI | `/bars/window.json` | ONE request PER SERIES |
 *
 * That last row is the whole design problem. The census cannot carry a price:
 * `chg_bps` is a RATIO, and `/db` already recorded why a ratio is not enough —
 * *"a ratio has no scale, so no absolute price can be recovered from it."* A
 * last close exists only in a bar file.
 *
 * So a table of N instruments would be N requests, and the engine surface is
 * 213 equities plus two indices. 215 requests to fill one screen is not a
 * table, it is an outage.
 *
 * # Therefore: incremental, and the viewport is the increment
 *
 * A quote is fetched for a row when the page ASKS for that row, and the page
 * asks only for rows a reader can currently see. The source terminal shows
 * about seventeen. Seventeen requests at six outstanding is two round trips,
 * and scrolling asks for the next seventeen.
 *
 * Every answer is cached on `(feed, key, timeframe, month)` and the cache is
 * keyed on the PROMISE, not the result, so two rows that want the same series
 * in the same frame make one request rather than two. A failure is cached as a
 * failure — a cell that could not be read must not silently retry on every
 * repaint, which is how one broken series becomes a request storm.
 */

import { ask } from '$lib/ask.js';
import { pooled, IN_FLIGHT } from '$lib/pooled.js';
import { parseKey } from '$lib/instrument.js';

/**
 * The tab strip — THREE ASSET CLASSES, AND DELIBERATELY NOT THE SOURCE
 * TERMINAL'S EIGHT.
 *
 * THE LABELS ARE DESIGN AND THE CONTENTS ARE NOT. This array is the one part of
 * the terminal that is chosen rather than derived, because it is a picture of a
 * tab strip and a tab strip is a fixed list of words. What goes UNDER each tab
 * is decided entirely by `tabsFrom` below, against the store, at run time.
 *
 * `sourced` names how a tab is filled:
 *
 *   'segment' — rows are census keys whose parsed `segment` matches.
 *   'kind'    — rows are census keys whose parsed `kind` matches.
 *
 * # Why five tabs were removed rather than drawn empty
 *
 * The source terminal offers Live, Options, Stocks, Commodity, Futures, ETFs,
 * Bonds and Index. Five of those can never hold anything here and were drawn
 * carrying an explanation instead:
 *
 *   Commodity — MCX, and `CLAUDE.md` section 1 narrows the engine to NSE.
 *   ETFs, Bonds — no segment in this store's path grammar; there is no shelf
 *                 for them to be missing from.
 *   Live — a merge of the other tabs, which is a fourth view of three sets.
 *   Index — the two spot indices, which the strip along the top already shows.
 *
 * A tab that exists only to say why it is empty is a tab an operator opens
 * once. Three tabs that are always about something the store can actually hold
 * is the surface this console needs, and it is the operator's own instruction.
 *
 * WHAT THE REMOVAL COSTS, STATED SO IT IS NOT DISCOVERED: there is no longer a
 * GRID view of `NSE-INDEX-NIFTY` or `NSE-INDEX-BANKNIFTY`. They remain on the
 * index strip, which is a different surface with a different shape — a level
 * and a move, not a row of columns. If an index grid is wanted, Index returns
 * as a fourth entry here and nothing else changes.
 */
export const TABS = [
  { id: 'stocks', label: 'Stocks', sourced: 'segment', match: 'CASH' },
  { id: 'futures', label: 'Futures', sourced: 'kind', match: 'future' },
  { id: 'options', label: 'Options', sourced: 'kind', match: 'option' }
];

/**
 * Which asset classes this feed's store can actually fill, right now.
 *
 * Derived from the census keys on every call — NOTHING here is a list of
 * instruments, and adding a symbol to the store is the only action that makes
 * it appear. That is the whole point: a hardcoded universe is correct the day
 * it is written and silently wrong the first time the store changes underneath
 * it, which is the defect `/db` recorded as a hardcoded span throwing away 40
 * of 121 months while reporting no hole.
 *
 * O(n) over census keys, once per call, where n is instruments HELD and not
 * instruments listed. The caller is a `$derived`, so it runs when the census
 * generation changes and not per repaint.
 *
 * @param {string[]} keys every instrument key the census holds for this feed
 * @returns {{id: string, label: string, keys: string[], why: string|null}[]}
 */
export function tabsFrom(keys) {
  /** @type {Map<string, string[]>} */
  const bySegment = new Map();
  /** @type {Map<string, string[]>} */
  const byKind = new Map();
  for (const key of keys) {
    const p = parseKey(key);
    // A KEY THAT WILL NOT PARSE IS DROPPED FROM THE TABS AND COUNTED NOWHERE.
    // It cannot be filed under a segment it never named, and filing it under a
    // guess is the invention section 3 rule 1 forbids. `parseKey` already
    // carries the `why`; the page surfaces those separately so a store with
    // unreadable keys does not look like a store with fewer instruments.
    if (!p.underlying || !p.segment || !p.kind) continue;
    if (!bySegment.has(p.segment)) bySegment.set(p.segment, []);
    (bySegment.get(p.segment) ?? []).push(key);
    if (!byKind.has(p.kind)) byKind.set(p.kind, []);
    (byKind.get(p.kind) ?? []).push(key);
  }

  return TABS.map((tab) => {
    const from = tab.sourced === 'segment' ? bySegment : byKind;
    const found = from.get(/** @type {string} */ (tab.match)) ?? [];
    return {
      id: tab.id,
      label: tab.label,
      keys: found,
      why: found.length
        ? null
        : `The store holds no ${tab.label.toLowerCase()} series for this feed. ` +
          `Nothing has been pulled into it, which is a different fact from a ` +
          `market with nothing to show.`
    };
  });
}

/**
 * The `/bars/window.json` parameters for one census key.
 *
 * REBUILT FROM `parseKey`, NOT RE-SPLIT. `/db` splits the key inline and hunts
 * for a `YYYY-MM-DD` triple by index; that works and it is a second parser for
 * a grammar `$lib/instrument.js` already owns, which is the two-copies defect
 * this tree keeps removing. `parseKey` handles the case that breaks the naive
 * split — an underlying containing a hyphen, `BAJAJ-AUTO` — so this cannot
 * disagree with the rest of the product about what a key means.
 *
 * `contract` is reassembled from the parsed parts rather than sliced off the
 * string, so the tail this sends is the tail `parseKey` recognised and not
 * whatever happened to follow the last hyphen.
 *
 * @param {string} key
 * @returns {{exchange: string, segment: string, symbol: string, contract: string}|null}
 */
export function seriesParams(key) {
  const p = parseKey(key);
  if (!p.underlying || !p.exchange || !p.segment) return null;
  let contract = '';
  if (p.kind === 'future' && p.expiry) contract = `${p.expiry}-FUT`;
  else if (p.kind === 'option' && p.expiry && p.strike !== null && p.side) {
    contract = `${p.expiry}-${p.strike}-${p.side}`;
  }
  return { exchange: p.exchange, segment: p.segment, symbol: p.underlying, contract };
}

/**
 * One quote's cache key. The four things that change the answer, and no more.
 *
 * @param {string} feed
 * @param {string} key
 * @param {string} timeframe
 * @param {string} month
 * @returns {string}
 */
export const quoteKey = (feed, key, timeframe, month) =>
  `${feed}|${key}|${timeframe}|${month}`;

/**
 * CACHED ON THE PROMISE, NOT THE ANSWER.
 *
 * A cache of finished reads lets two rows that want the same series in the same
 * frame both miss, both fetch, and both fill it — one request wasted per
 * duplicate, every time. Storing the in-flight promise means the second asker
 * awaits the first asker's request. `/db` learned this on bar files and the
 * note there says the same thing; this is that lesson applied to quotes.
 *
 * @type {Map<string, Promise<Quote>>}
 */
const inflight = new Map();

/**
 * @typedef {object} Quote
 * @property {string} key         the census key this answers for
 * @property {number|null} close  last close, PAISA INTEGER, or null
 * @property {number|null} open
 * @property {number|null} high
 * @property {number|null} low
 * @property {number|null} volume
 * @property {number|null} oi     open interest, or null when the store filed the null sentinel
 * @property {number|null} chg    change on the bar, INTEGER BASIS POINTS, or null
 * @property {string|null} chgWhy the reason `chg` is null, or null
 * @property {number|null} at     the bar's timestamp, epoch SECONDS
 * @property {string|null} why    why this quote holds nothing at all
 */

/**
 * ONE BAR AS A QUOTE — the single mapping from the wire's row to this page's.
 *
 * Extracted so the two fetch paths cannot disagree about what a bar means. The
 * cheap path asks for `limit=1` and reads the month's last bar; the day path
 * reads a whole month and picks a bar out of it. Two copies of this mapping
 * would drift the first time a field is added, and the drift would be invisible
 * — both paths would render, and only one would be right.
 *
 * @param {string} key
 * @param {any} bar
 * @returns {Quote}
 */
function fromBar(key, bar) {
  return {
    key,
    close: typeof bar.c === 'number' ? bar.c : null,
    open: typeof bar.o === 'number' ? bar.o : null,
    high: typeof bar.h === 'number' ? bar.h : null,
    low: typeof bar.l === 'number' ? bar.l : null,
    volume: typeof bar.v === 'number' ? bar.v : null,
    // NULL IS THE WIRE'S OWN WORD HERE, not a coercion. The server writes
    // `null` when the record carries `OI_NULL`, and `CLAUDE.md` section 7 says
    // the sentinel means absent while zero means zero. Reading a missing open
    // interest as `0` is the single defect most visible in the captures this
    // page recreates.
    oi: typeof bar.oi === 'number' ? bar.oi : null,
    chg: typeof bar.chg === 'number' ? bar.chg : null,
    chgWhy: typeof bar.chg_why === 'string' ? bar.chg_why : null,
    at: typeof bar.t === 'number' ? bar.t : null,
    why: null
  };
}

/** A quote that answers nothing, and says which reason. @param {string} key @param {string} why */
const nothing = (key, why) => ({
  key,
  close: null,
  open: null,
  high: null,
  low: null,
  volume: null,
  oi: null,
  chg: null,
  chgWhy: null,
  at: null,
  why
});

/**
 * The newest bar this store holds for one series, or a named reason it holds
 * none.
 *
 * ONE ROW, ASKED FOR AS ONE ROW. `limit=1&sort=ts&dir=desc` is a seek, not a
 * scan — `/bars/window.json` distinguishes the two on the wire and says which
 * it did, and asking for the whole month to read its last line would be the
 * scan this endpoint was built to avoid. Measured elsewhere in this tree at
 * about 81 bytes per bar, so a month asked for whole to find one close is
 * roughly 8,000 wasted bytes per row per repaint.
 *
 * NEVER THROWS. Every failure — a refused parameter, a transport error, a
 * timeout, a body that is not a window — comes back as a `Quote` whose numbers
 * are `null` and whose `why` is the reason, because a table renders cells and a
 * cell has to say something. A rejection here would take out the other sixteen
 * rows in the same pool.
 *
 * @param {string} feed
 * @param {string} key
 * @param {string} timeframe
 * @param {string} month `YYYY-MM`
 * @returns {Promise<Quote>}
 */
export function quote(feed, key, timeframe, month) {
  const id = quoteKey(feed, key, timeframe, month);
  const held = inflight.get(id);
  if (held) return held;

  const run = (async () => {
    const parts = seriesParams(key);
    if (!parts) {
      return nothing(key, `${key} is not a series key this store can name.`);
    }
    const q = new URLSearchParams({
      feed,
      exchange: parts.exchange,
      segment: parts.segment,
      symbol: parts.symbol,
      contract: parts.contract,
      timeframe,
      from: month,
      to: month,
      sort: 'ts',
      dir: 'desc',
      offset: '0',
      limit: '1'
    });
    try {
      const res = await ask(`/bars/window.json?${q}`);
      const body = await res.json().catch(() => null);
      if (!body || !Array.isArray(body.bars)) {
        return nothing(
          key,
          body?.error ??
            `/bars/window.json answered HTTP ${res.status} with no window, so ` +
              `there is no bar to read and no reason recorded for it.`
        );
      }
      const bar = body.bars[0];
      if (!bar) {
        // 206 AND AN EMPTY WINDOW ARE DIFFERENT ABSENCES. `faults` means a
        // record would not read — the month exists and part of it is damaged.
        // No faults and no bars means the month simply is not held. A reader
        // must be able to tell those apart; both are absences and only one is a
        // fault.
        return nothing(
          key,
          body.faults
            ? `The store holds ${month} for this series but a record would not ` +
              `read: ${body.faults}`
            : `The store holds no ${timeframe} bars for this series in ${month}.`
        );
      }
      return fromBar(key, bar);
    } catch (error) {
      const e = /** @type {any} */ (error);
      return nothing(key, String(e && e.message ? e.message : e));
    }
  })();

  inflight.set(id, run);
  return run;
}

/**
 * Quotes for the rows a reader can currently see.
 *
 * THE INCREMENT IS THE VIEWPORT AND THIS IS THE FUNCTION THAT MAKES IT ONE. A
 * caller hands it the visible slice; it fetches at `IN_FLIGHT` outstanding and
 * resolves in the order given, so the caller can index the answers straight
 * against the rows it drew.
 *
 * Already-cached series cost nothing but a Map probe, so re-asking on every
 * scroll frame is cheap and correct: the pool only ever carries the rows that
 * newly came into view.
 *
 * @param {string} feed
 * @param {{key: string, timeframe: string, month: string}[]} rows
 * @returns {Promise<Quote[]>}
 */
export function quotesFor(feed, rows) {
  return pooled(rows, IN_FLIGHT, (r) => quote(feed, r.key, r.timeframe, r.month));
}

/**
 * Forgets every cached quote.
 *
 * CALLED WHEN THE FEED CHANGES OR A PULL FINISHES, and at no other time. The
 * cache is keyed on the feed already, so a feed switch does not strictly need
 * this — but a pull writes new bars under keys the cache already holds answers
 * for, and a terminal that kept serving the pre-pull close would be reporting a
 * measurement that has been superseded, which is the stale-value half of the
 * rule `store.svelte.js` keeps on the census.
 */
export function forgetQuotes() {
  inflight.clear();
}
