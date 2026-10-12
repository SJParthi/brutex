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
 * | LTP, Open, High, Low, Volume, OI | `/bars/window.json`, ONE bar | ONE seek PER SERIES |
 * | Change, Change % | that bar's own `chg`: against the previous STORED bar at this timeframe, never a day change (DB-G17) | same request |
 * | OI Change % | that bar's own `oichg`, with `oichg_why` (DB-G15) | same request |
 * | Change, OI Change (in rupees, in contracts) | DERIVED from the value and its whole basis points: every whole change that pair allows (`moveRange`); plain when that is one figure, `≈` with the range on hover when it is not | none |
 * | the marks beside a figure (`behind`, `bar HH:MM:SS`, `partial`, a spot's age) | `marksOf` and `spotMark`: one list for the lead ticker, the strip, the grid row and the Snapshot copy | none |
 * | Spot Price, Premium | `/spot.json`, at the bar's own instant (D-2807, D-3328) | ONE request per underlying and month, at most 400 instants each; an answer already read is not asked for again |
 * | index strip order | `/aliases.json` `index`, in the order served (DB-G13) | ONE request per page |
 * | a rung behind its source | the census cells' own `last_ts` (DB-G12) | free |
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

import { ask } from './ask.js';
import { pooled, IN_FLIGHT } from './pooled.js';
import { parseKey } from './instrument.js';
/* THE EXCHANGE'S DAY, NOT THE BROWSER'S. `./dates.js` owns the `Asia/Kolkata`
   clock and `./db/bars-plan.js` the IST day-and-time to instant conversion
   `/db` uses for Go to; a second spelling of either is forbidden.
   RELATIVE IMPORTS, so `web/tests/terminal.test.js` loads this module as it
   is, with no bundler alias. */
import { istDay, secondLabel } from './dates.js';
import { group } from './money.js';
import { istInstant } from './db/bars-plan.js';
import { readSpots, spotKey, SPOT_CHUNK } from './moneyness.js';

export { SPOT_CHUNK };

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
 * Which asset classes this feed's store can actually fill, right now, AND the
 * census keys that could be filed under none of them.
 *
 * Derived from the census keys on every call: nothing here is a list of
 * instruments, and adding a symbol to the store is the only action that makes
 * it appear. O(n) over census keys, once per census generation (the caller is
 * a `$derived`).
 *
 * A KEY THAT WILL NOT PARSE IS COUNTED AND NAMED, NOT DROPPED (DB-G32). It
 * cannot be filed under a segment it never named, and until this returned
 * `unparsed` it vanished from every count on the page, so the store looked
 * smaller than it is. The page prints the count and the first few keys.
 *
 * @param {string[]} keys every instrument key the census holds for this feed
 * @returns {{tabs: {id: string, label: string, keys: string[], why: string|null}[],
 *            indices: string[],
 *            unparsed: {count: number, sample: string[]},
 *            unplaced: {count: number, sample: string[]}}}
 *          `indices` is every INDEX-segment key, in the order given: the
 *          strip's candidates, which are in no tab.
 */
export function tabsAndRest(keys) {
  /** @type {Map<string, string[]>} */
  const bySegment = new Map();
  /** @type {Map<string, string[]>} */
  const byKind = new Map();
  const unparsed = { count: 0, sample: /** @type {string[]} */ ([]) };
  /* A KEY THAT PARSES AND THAT NO TAB TAKES is not on the strip either unless
     it is an index: it was in no count at all. Counted and named like the
     unparsed ones; the tabs' own `match` words decide it, not a list here. */
  const unplaced = { count: 0, sample: /** @type {string[]} */ ([]) };
  const taken = (/** @type {{segment: string|null, kind: string|null}} */ p) =>
    p.segment === 'INDEX' || TABS.some((tab) => (tab.sourced === 'segment' ? p.segment : p.kind) === tab.match);
  for (const key of keys) {
    const p = parseKey(key);
    if (!p.underlying || !p.segment || !p.kind) {
      unparsed.count += 1;
      if (unparsed.sample.length < UNPARSED_SAMPLE) unparsed.sample.push(String(key));
      continue;
    }
    if (!taken(p)) {
      unplaced.count += 1;
      if (unplaced.sample.length < UNPARSED_SAMPLE) unplaced.sample.push(String(key));
    }
    if (!bySegment.has(p.segment)) bySegment.set(p.segment, []);
    (bySegment.get(p.segment) ?? []).push(key);
    if (!byKind.has(p.kind)) byKind.set(p.kind, []);
    (byKind.get(p.kind) ?? []).push(key);
  }

  const tabs = TABS.map((tab) => {
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
  return { tabs, indices: bySegment.get('INDEX') ?? [], unparsed, unplaced };
}

/** How many unreadable census keys are named; the count is always whole. */
export const UNPARSED_SAMPLE = 5;

/**
 * The tabs alone; see {@link tabsAndRest}.
 * @param {string[]} keys
 */
export const tabsFrom = (keys) => tabsAndRest(keys).tabs;

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
 * A timeframe's length in seconds, or `null` for a name this page cannot read.
 *
 * `1s` is one second (DB-G32): the old pattern knew `min` and `day` only, so
 * `1s` came back as "unknown", sorted as the COARSEST rung and would have been
 * the default beside any minute rung.
 *
 * @param {unknown} t `1s`, `5min`, `1day`, ...
 * @returns {number|null}
 */
export function timeframeSeconds(t) {
  const m = String(t ?? '').match(/^([1-9]\d*)(s|min|day)$/);
  if (!m) return null;
  return Number(m[1]) * (m[2] === 's' ? 1 : m[2] === 'min' ? 60 : 86_400);
}

/**
 * Timeframe order by duration: `1s` below `1min`, `2min` below `10min`, `1day`
 * last. A name that will not read sorts after every name that does, so it can
 * never be taken for the finest rung.
 * @param {unknown} t @returns {number}
 */
export function timeframeOrder(t) {
  return timeframeSeconds(t) ?? Number.MAX_SAFE_INTEGER;
}

/**
 * WHETHER A CELL'S LAST BAR IS BEHIND THE FINER RUNG IT IS FOLDED FROM (DB-G12).
 *
 * A derived rung (D-3000) is folded from the feed's finer bars. When that fold
 * stopped early, the rung's month ends days before its source does, and its
 * "last bar" is an old bar under the month's name: Zerodha NIFTY 60min for
 * Sep 2026 ended on the 4th while 1min reached the 30th, so the page printed
 * the 4th's close as the month's.
 *
 * The census already carries each cell's `last` stamp, so this costs no
 * request: it reads the instrument's own cells for the month, takes the
 * FINEST rung held there as the source (whatever the feed calls it), and
 * answers `behind` when the source holds a bar at or after the END of this
 * cell's last bucket: a bucket this rung should hold and does not.
 *
 * @param {{month: string, timeframe: string, last: number|null}[]|undefined} cells
 *        the instrument's census cells; `last` in MICROSECONDS
 * @param {string} timeframe @param {string} month
 * @returns {{source: string, last: number, sourceLast: number, lastDay: string, sourceDay: string, lastTime: string, sourceTime: string, text: string}|null}
 *          `lastTime` and `sourceTime` are the two stamps' IST clock times, `HH:MM:SS`
 */
export function behindSource(cells, timeframe, month) {
  const own = timeframeSeconds(timeframe);
  if (!cells || own === null) return null;
  let mine = null;
  let finest = null;
  let finestSecs = own;
  for (const cell of cells) {
    if (cell.month !== month || typeof cell.last !== 'number') continue;
    if (cell.timeframe === timeframe) {
      mine = cell;
      continue;
    }
    const secs = timeframeSeconds(cell.timeframe);
    if (secs !== null && secs < finestSecs) {
      finest = cell;
      finestSecs = secs;
    }
  }
  if (!mine || !finest) return null;
  const last = /** @type {number} */ (mine.last);
  const sourceLast = /** @type {number} */ (finest.last);
  if (sourceLast < last + own * 1_000_000) return null;
  const lastDay = istDay(last / 1000);
  const sourceDay = istDay(sourceLast / 1000);
  const lastTime = secondLabel(last / 1000);
  const sourceTime = secondLabel(sourceLast / 1000);
  return {
    source: finest.timeframe,
    last,
    sourceLast,
    lastDay,
    sourceDay,
    lastTime,
    sourceTime,
    // The day says nothing when both rungs stop on it: then the mark names the time.
    text: `behind ${finest.timeframe} (last ${lastDay}${lastDay === sourceDay ? ` ${lastTime}` : ''})`
  };
}

/**
 * The sentence a behind mark carries, for the day in view.
 *
 * It names both stamps to the second, because 210 of the 536 behind cells of
 * the Zerodha census stop on the SAME day as their source and "ends on the
 * 29th, the source reaches the 29th" explains nothing. And it says what the
 * figure beside it is for the day chosen: with no day it is the stale bar;
 * on the day the rung stops it is no later than that bar; after it the rung
 * holds nothing.
 *
 * @param {NonNullable<ReturnType<typeof behindSource>>} b
 * @param {string} timeframe @param {string} [day] `YYYY-MM-DD`, or `''`
 */
export function behindWhy(b, timeframe, day = '') {
  const stops = `${b.lastDay} ${b.lastTime}`;
  const head =
    `This ${timeframe} month’s last bar is stamped ${stops}, and the same series at ` +
    `${b.source} reaches ${b.sourceDay} ${b.sourceTime}. The ${timeframe} bars after ` +
    `${stops} were never derived`;
  if (!day) return `${head}, so the figure beside this is the bar stamped ${stops} and not the month’s last.`;
  if (day > b.lastDay) return `${head}, so this rung holds no bar for ${day}.`;
  return (
    `${head}, so on ${day} this rung stops at ${b.lastTime}` +
    (b.sourceDay === b.lastDay ? ` while ${b.source} reaches ${b.sourceTime}` : '') +
    `: the figure beside this is no later than that bar.`
  );
}

/**
 * EVERY MARK A FIGURE CARRIES, FOR EVERY PLACE THAT DRAWS THE FIGURE.
 *
 * The grid row, the index strip and the lead ticker draw the same quote, and
 * the row alone said when its bar was older than the time asked. A level on
 * the strip from 112 minutes before the time chosen read as the level at that
 * time. So the marks are decided here, once, and the page renders this list
 * at all three sites and copies it into a Snapshot:
 *
 *   behind   the rung's month stops before the rung it is folded from
 *            (DB-G12). Not drawn for a day before the rung's last: that day's
 *            figure is its own.
 *   lag      a time was asked and the bar is stamped earlier than it. On a
 *            rung coarser than a second the sentence also says how far the
 *            bar reaches, because a bar is named by its OPENING stamp and its
 *            close can be later than the time asked.
 *   partial  a record in the window would not read.
 *
 * @param {ReturnType<typeof behindSource>} behind
 * @param {Quote|undefined|null} quote
 * @param {{timeframe: string, day?: string, time?: string}} asked
 * @returns {{kind: 'behind'|'lag'|'partial', text: string, why: string}[]}
 */
export function marksOf(behind, quote, asked) {
  const { timeframe, day = '', time = '' } = asked;
  /** @type {{kind: 'behind'|'lag'|'partial', text: string, why: string}[]} */
  const out = [];
  if (behind && (!day || day >= behind.lastDay)) {
    out.push({ kind: 'behind', text: behind.text, why: behindWhy(behind, timeframe, day) });
  }
  if (quote && quote.at !== null && quote.lag !== null && quote.lag > 0) {
    const stamp = secondLabel(quote.at * 1000);
    const own = timeframeSeconds(timeframe);
    let cover = '';
    if (own !== null && own > 1) {
      const until = quote.at + own;
      cover =
        ` A bar is named by its opening stamp: at ${timeframe} this one covers up to ` +
        `${secondLabel(until * 1000)}` +
        (quote.asked !== null && until > quote.asked
          ? `, which is after the time asked, so its close can be later than ${time}.`
          : '.');
    }
    out.push({
      kind: 'lag',
      text: `bar ${stamp}`,
      why:
        `The last bar stamped at or before ${time} on ${day} is stamped ${stamp}, ` +
        `${group(quote.lag)} s earlier. A bar stamped later is never shown.${cover}`
    });
  }
  if (quote?.faults) {
    out.push({ kind: 'partial', text: 'partial', why: `A record in this window would not read: ${quote.faults}` });
  }
  return out;
}

/** How many indices the strip draws. */
export const STRIP_MAX = 6;

/**
 * THE INDEX STRIP, IN THE SERVER'S OWN ORDER (DB-G13).
 *
 * The strip was the first six INDEX names in alphabetical order out of 135,
 * which on Zerodha is BANKNIFTY and five bond indices, with NIFTY not shown.
 * `/aliases.json` serves the index names this store keys, the F&O index
 * underlyings first and NIFTY then BANKNIFTY at their head, each with the key
 * it is stored under. So the order is read, not typed here:
 *
 *   1. the F&O index underlyings the census holds, in the order served. They
 *      appear even when the chosen cell is absent; their cell then says why.
 *   2. the other served index names that hold the chosen cell, in that order;
 *   3. every other index that holds the chosen cell, by name.
 *
 * @param {string[]} indexKeys every INDEX-segment key the census holds, sorted
 * @param {(key: string) => boolean} holdsCell whether a key holds the chosen cell
 * @param {{key: string, fno: boolean}[]} served `/aliases.json` `index`, in order; `[]` when unread
 * @param {number} [limit]
 * @returns {string[]}
 */
export function stripOf(indexKeys, holdsCell, served, limit = STRIP_MAX) {
  const held = new Set(indexKeys);
  /** @type {string[]} */
  const out = [];
  const seen = new Set();
  /** @param {string} key */
  const take = (key) => {
    if (out.length < limit && !seen.has(key)) {
      seen.add(key);
      out.push(key);
    }
  };
  for (const s of served) if (s.fno && held.has(s.key)) take(s.key);
  for (const s of served) if (!s.fno && held.has(s.key) && holdsCell(s.key)) take(s.key);
  for (const key of indexKeys) {
    if (out.length >= limit) break;
    if (holdsCell(key)) take(key);
  }
  return out;
}

/**
 * A change in a value's own units from the value and its change in integer
 * basis points: `value - value / (1 + bps / 10000)`.
 *
 * DERIVED, never measured: the wire carries the value and the ratio, not the
 * earlier value. `null` when there is nothing to derive from, and at exactly
 * -10000 bps, where the divisor is zero and the earlier value cannot be
 * recovered from a ratio at all.
 *
 * @param {number|null|undefined} value @param {number|null|undefined} bps
 * @returns {number|null}
 */
export function moveFromBps(value, bps) {
  if (typeof value !== 'number' || typeof bps !== 'number') return null;
  if (bps === -10_000) return null;
  const before = value / (1 + bps / 10_000);
  if (!Number.isFinite(before)) return null;
  return value - before;
}

/**
 * The server's own change in basis points, in whole numbers: the same
 * arithmetic as `crates/api/src/month_change.rs::basis_points`, which rounds
 * half AWAY from zero. `null` where that function refuses (a base that is not
 * positive) or where a product would leave the safe integers.
 *
 * @param {number} base the earlier value @param {number} value the later one
 * @returns {number|null}
 */
export function bpsBetween(base, value) {
  if (!Number.isSafeInteger(base) || !Number.isSafeInteger(value) || base <= 0) return null;
  const scaled = (value - base) * 10_000;
  if (!Number.isSafeInteger(scaled)) return null;
  // `%` is exact on safe integers and keeps the sign of `scaled`, as Rust's does.
  const rest = scaled % base;
  const whole = (scaled - rest) / base;
  const magnitude = Math.abs(rest);
  return magnitude >= base - magnitude ? whole + Math.sign(rest) : whole;
}

/** How far a float estimate of a bound is searched before the pair is called inconsistent. */
const SETTLE = 4;

/**
 * EVERY WHOLE CHANGE THE STORED PAIR ALLOWS (the OI Change and Change cells).
 *
 * The wire carries a value and its change against the previous stored bar in
 * WHOLE basis points, not the earlier value. Rebuilding the difference from
 * the ratio and printing it as an integer was wrong on real rows: an open
 * interest of 45,075 at -17 bp printed -77 where the store's own difference
 * is -75, and any real change under half a basis point printed 0.
 *
 * The earlier value is a whole number (contracts, or paisa), and the server's
 * rounding is known, so the earlier values that give exactly these basis
 * points are a run of consecutive integers. This returns the change at both
 * ends of that run. When the run is ONE value the change is exact, and only
 * then may it be drawn as a plain figure.
 *
 * O(1): two float estimates, each settled by at most `2 * SETTLE + 1` exact
 * evaluations.
 *
 * @param {number|null|undefined} value @param {number|null|undefined} bps
 * @returns {{least: number, most: number, mid: number, exact: boolean}|null}
 *          `null` when there is nothing to derive from, at -10000 bp or
 *          below, or when no whole earlier value gives these basis points
 */
export function moveRange(value, bps) {
  if (!Number.isSafeInteger(value) || !Number.isSafeInteger(bps)) return null;
  const v = /** @type {number} */ (value);
  const b = /** @type {number} */ (bps);
  if (v <= 0 || b <= -10_000 || v > Number.MAX_SAFE_INTEGER / 20_000) return null;
  const hits = (/** @type {number} */ base) => base >= 1 && bpsBetween(base, v) === b;
  // bpsBetween falls as the base rises, so the bases that hit are one run.
  let low = Math.ceil((20_000 * v) / (2 * b + 20_001)) - SETTLE;
  const lowEnd = low + 2 * SETTLE;
  while (low <= lowEnd && !hits(low)) low += 1;
  let high = Math.floor((20_000 * v) / (2 * b + 19_999)) + SETTLE;
  const highEnd = high - 2 * SETTLE;
  while (high >= highEnd && !hits(high)) high -= 1;
  if (low > lowEnd || high < highEnd || high < low) return null;
  // The estimate was more than SETTLE off: the run's true end is not known.
  if (hits(low - 1) || hits(high + 1)) return null;
  const least = v - high;
  const most = v - low;
  const point = Math.round(/** @type {number} */ (moveFromBps(v, b)));
  return { least, most, mid: Math.min(most, Math.max(least, point)), exact: low === high };
}

/**
 * One derived change as a cell: a plain figure when it is exact, and `≈` with
 * the whole range on hover when it is not. Never an approximate figure in an
 * exact one's clothes.
 *
 * @param {number|null|undefined} value @param {number|null|undefined} bps
 * @param {(n: number) => string} format the unit's own formatter
 * @param {string} of what the value is, for the sentence: `open interest`, `close`
 * @returns {{text: string, dir: number, approx: boolean, why: string}|null}
 */
export function moveCell(value, bps, format, of) {
  const r = moveRange(value, bps);
  if (!r) return null;
  const dir = r.least > 0 ? 1 : r.most < 0 ? -1 : 0;
  if (r.exact) {
    return {
      text: format(r.mid),
      dir,
      approx: false,
      why:
        `Derived, and exact: the wire carries the ${of} and its change against the previous ` +
        `stored bar in whole basis points, and ${format(r.mid)} is the only whole change ` +
        `that gives those basis points.`
    };
  }
  return {
    text: `≈ ${format(r.mid)}`,
    dir,
    approx: true,
    why:
      `Approximate. The stored change is one figure from ${format(r.least)} to ${format(r.most)}, ` +
      `and the wire cannot say which: it carries the ${of} and its change against the previous ` +
      `stored bar in whole basis points, not the earlier figure, and every change in that range ` +
      `rounds to the same basis points. The percentage beside it is the store’s own figure.`
  };
}

/**
 * Whether a row's ranking value is still ON ITS WAY, as opposed to landed and
 * absent.
 *
 * A chip that filters carries a row whose value is not known YET, so a
 * refetch does not empty the grid. It carried a LANDED absence the same way:
 * OI Gainers listed 23 contracts with no OI change beside 7 that gained.
 * A quote that has landed with nothing in the field is not a gainer.
 *
 * @param {Quote|undefined|null} quote the row's quote, if it has landed
 * @param {string} on the field ranked on
 * @param {boolean} hasSpot whether the row's spot answer has landed
 * @returns {boolean}
 */
export function awaited(quote, on, hasSpot) {
  if (!quote) return true;
  if (on === 'spot' || on === 'premium') return quote.at !== null && !hasSpot;
  return false;
}

/**
 * A SNAPSHOT'S FIRST LINE: which window the copied figures are of.
 * A copy named neither the day, the time nor the spot rule, so a figure from
 * an earlier bar or an earlier spot read as a current one once pasted.
 *
 * @param {{feed: string, tab: string, timeframe: string, month: string, day?: string, time?: string, spot?: ''|'exact'|'prev'}} w
 */
export function windowLine(w) {
  const parts = [w.feed, w.tab, w.timeframe, w.month];
  if (!w.day) parts.push('the month’s last bar of each series');
  else if (!w.time) parts.push(`the last bar of ${w.day}`);
  else parts.push(`the last bar stamped at or before ${w.time} IST on ${w.day}`);
  if (w.spot === 'exact') parts.push('spot at the bar’s own instant');
  if (w.spot === 'prev') parts.push('spot: the last at or before the bar, its age beside it');
  return `# ${parts.join(' · ')}`;
}

/** A name with its marks, as copied text. @param {string} label @param {{text: string}[]} marks */
export const copyName = (label, marks) => [label, ...marks.map((m) => `[${m.text}]`)].join(' ');

/** A cell with its mark, as copied text. A dash stays a dash. @param {{text: string, mark?: string|null, pending?: boolean}} cell */
export const copyCell = (cell) => (cell.text || '—') + (cell.mark ? ` [${cell.mark}]` : '');

/**
 * The age mark of a spot, or `null` when the cell carries none.
 * The opted-in rule ALWAYS shows the spot's age (D-3328), 0 s included; an
 * exact answer has none to show.
 * @param {{close: number|null, age: number|null, asof: string}} spot
 */
export const spotMark = (spot) =>
  spot.close !== null && spot.age !== null && (spot.asof === 'prev' || spot.age > 0)
    ? `age ${group(spot.age)} s`
    : null;

/**
 * A bar column's header: the name, and whose change or volume it is (DB-G17).
 * `Change %` copied a broker terminal's DAY change, but the cell is the last
 * bar's move against the bar before it at the chosen timeframe: one second on
 * a `1s` rung.
 * @param {string} name @param {string} timeframe
 */
export const barHeader = (name, timeframe) => `${name} · last bar (${timeframe || 'no timeframe'})`;

/* ====================================================================
   THE ONE REQUEST POOL, AND ITS MEASURED COUNT (DB-G86, RA-143)
   --------------------------------------------------------------------
   Every request this page makes goes through `gated`: the quote seeks, the
   spot joins and the strip's order. At most `traffic.cap` are in flight at
   once, page-wide, however many batches are alive; before this, each batch
   had its own six and a superseded batch's started reads ran beside the new
   one's. The counters are what the page prints, so the fan-out is a number
   on screen and not an estimate: `sent` requests, `peak` in flight.
   ==================================================================== */

/** The page's request counters. `peak` never exceeds `cap`. */
export const traffic = { cap: IN_FLIGHT, inFlight: 0, peak: 0, sent: 0, waited: 0 };

/** Waiters for a slot, read from `head` so taking one is O(1). @type {(() => void)[]} */
let waiting = [];
let head = 0;

/** The transport. Replaced only by tests. @type {typeof ask} */
let transport = ask;

/**
 * Points every request of this module at `request`, and empties its caches
 * and counters. For tests; the page never calls it.
 * @param {typeof ask} [request]
 */
export function useTransport(request = ask) {
  transport = request;
  forgetQuotes();
  aliases = null;
  traffic.inFlight = 0;
  traffic.peak = 0;
  traffic.sent = 0;
  traffic.waited = 0;
  waiting = [];
  head = 0;
}

function release() {
  if (head < waiting.length) {
    const next = waiting[head];
    head += 1;
    if (head === waiting.length) {
      waiting = [];
      head = 0;
    }
    // The slot passes straight to the next waiter: `inFlight` does not move.
    next();
  } else {
    traffic.inFlight -= 1;
  }
}

/**
 * One request through the page's pool.
 * @param {string} url @param {{signal?: AbortSignal}} [options]
 * @returns {Promise<Response>}
 */
export async function gated(url, options = {}) {
  if (traffic.inFlight < traffic.cap) {
    traffic.inFlight += 1;
  } else {
    traffic.waited += 1;
    await new Promise((resolve) => waiting.push(() => resolve(undefined)));
  }
  if (traffic.inFlight > traffic.peak) traffic.peak = traffic.inFlight;
  try {
    options.signal?.throwIfAborted();
    traffic.sent += 1;
    return await transport(url, options);
  } finally {
    release();
  }
}

/**
 * One quote's cache key: the six things that change the answer.
 *
 * @param {string} feed @param {string} key @param {string} timeframe
 * @param {string} month @param {string} [day] @param {string} [time]
 * @returns {string}
 */
export const quoteKey = (feed, key, timeframe, month, day = '', time = '') =>
  `${feed}|${key}|${timeframe}|${month}|${day}|${time}`;

/**
 * CACHED ON THE PROMISE, NOT THE ANSWER, so two rows that want the same series
 * in one frame make one request. A failure is cached as a failure: a cell that
 * could not be read must not retry on every repaint.
 *
 * BOUNDED: at most {@link QUOTE_CACHE} entries, oldest out first (a `Map`
 * keeps insertion order, so the eviction is one `delete`). Scrubbing through
 * times on a `1s` rung would otherwise grow it for the life of the tab.
 *
 * @type {Map<string, Promise<Quote>>}
 */
const inflight = new Map();

/** The most quotes held at once. */
export const QUOTE_CACHE = 4_096;

/**
 * @typedef {object} Quote
 * @property {string} key         the census key this answers for
 * @property {number|null} close  the bar's close, PAISA INTEGER, or null
 * @property {number|null} open
 * @property {number|null} high
 * @property {number|null} low
 * @property {number|null} volume null when the bar has no traded volume (`volumeWhy`)
 * @property {string|null} volumeWhy the server's word for a volume that is not one (`index_no_volume`)
 * @property {number|null} oi     open interest, or null when the store filed the null sentinel
 * @property {number|null} chg    change against the previous STORED bar, INTEGER BASIS POINTS, or null
 * @property {string|null} chgWhy the reason `chg` is null, or null
 * @property {number|null} oichg  open-interest change against the previous stored bar, INTEGER BASIS POINTS, or null
 * @property {string|null} oichgWhy the reason `oichg` is null, or null
 * @property {number|null} at     the bar's timestamp, epoch SECONDS
 * @property {number|null} asked  the instant asked for with a time, epoch seconds, or null
 * @property {number|null} lag    seconds from the bar to the instant asked, 0 for an exact hit, or null
 * @property {string|null} faults the server's word when a record in the window would not read
 * @property {string|null} why    why this quote holds nothing at all
 */

/**
 * ONE BAR AS A QUOTE: the single mapping from the wire's row to this page's.
 *
 * @param {string} key
 * @param {any} bar
 * @returns {Quote}
 */
export function fromBar(key, bar) {
  const volumeWhy = typeof bar.v_why === 'string' ? bar.v_why : null;
  return {
    key,
    close: typeof bar.c === 'number' ? bar.c : null,
    open: typeof bar.o === 'number' ? bar.o : null,
    high: typeof bar.h === 'number' ? bar.h : null,
    low: typeof bar.l === 'number' ? bar.l : null,
    // A `0` beside `index_no_volume` is not a traded volume of zero (D-3322).
    volume: volumeWhy === null && typeof bar.v === 'number' ? bar.v : null,
    volumeWhy,
    // NULL IS THE WIRE'S OWN WORD: the server writes `null` for `OI_NULL`, and
    // the sentinel means absent while zero means zero (`CLAUDE.md` section 7).
    oi: typeof bar.oi === 'number' ? bar.oi : null,
    chg: typeof bar.chg === 'number' ? bar.chg : null,
    chgWhy: typeof bar.chg_why === 'string' ? bar.chg_why : null,
    // DB-G15: the window has carried these since D-3325 and this mapping
    // dropped them, so the OI Change columns were drawn as "no source".
    oichg: typeof bar.oichg === 'number' ? bar.oichg : null,
    oichgWhy: typeof bar.oichg_why === 'string' ? bar.oichg_why : null,
    at: typeof bar.t === 'number' ? bar.t : null,
    asked: null,
    lag: null,
    faults: null,
    why: null
  };
}

/** A quote that answers nothing, and says which reason. @param {string} key @param {string} why @returns {Quote} */
const nothing = (key, why) => ({
  key,
  close: null,
  open: null,
  high: null,
  low: null,
  volume: null,
  volumeWhy: null,
  oi: null,
  chg: null,
  chgWhy: null,
  oichg: null,
  oichgWhy: null,
  at: null,
  asked: null,
  lag: null,
  faults: null,
  why
});

/**
 * THE ONE-BAR QUESTION, AS A URL (DB-G16, DB-G86).
 *
 * Always `limit=1&sort=ts&dir=desc`, which `/bars/window.json` answers with a
 * seek and says so (`scanned: false`):
 *
 *   no day        the month's last bar;
 *   a day         `from_day` and `to_day` both that day: the day's last bar;
 *   a day, a time `at=` that IST instant as well: the last bar AT OR BEFORE
 *                 it on that day. Never a later bar.
 *
 * Until the window took day bounds this page read the first 512 bars of the
 * month and looked through them, which reaches 09:23:31 of the first day of a
 * `1s` month and nothing else.
 *
 * @param {string} feed @param {string} key @param {string} timeframe
 * @param {string} month `YYYY-MM` @param {string} [day] `YYYY-MM-DD` or `''`
 * @param {string} [time] `HH:MM` or `HH:MM:SS` in IST, or `''`
 * @returns {{url: string, at: number|null} | {why: string}}
 */
export function windowUrl(feed, key, timeframe, month, day = '', time = '') {
  const parts = seriesParams(key);
  if (!parts) return { why: `${key} is not a series key this store can name.` };
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
    limit: '1'
  });
  let at = null;
  if (day) {
    if (!/^\d{4}-\d{2}-\d{2}$/.test(day) || day.slice(0, 7) !== month) {
      return { why: `${day} is not a day of ${month}, so there is no bar of that month to read for it.` };
    }
    q.set('from_day', day);
    q.set('to_day', day);
    if (time) {
      at = istInstant(day, time);
      if (at === null) return { why: `${time} is not an IST time of day (HH:MM or HH:MM:SS).` };
      q.set('at', String(at));
    }
  } else if (time) {
    return { why: `A time needs a day: ${time} was asked with no date.` };
  } else {
    q.set('offset', '0');
  }
  return { url: `/bars/window.json?${q}`, at };
}

/**
 * The quote one `/bars/window.json` answer holds, or the named reason it
 * holds none. Pure: the answer in, the quote out.
 *
 * @param {string} key
 * @param {any} body the parsed answer, or null when it was not JSON
 * @param {number} status the HTTP status
 * @param {{timeframe: string, month: string, day?: string, time?: string, at?: number|null}} asked
 * @returns {Quote}
 */
export function quoteFrom(key, body, status, asked) {
  const { timeframe, month, day = '', time = '', at = null } = asked;
  if (!body || !Array.isArray(body.bars)) {
    return nothing(
      key,
      typeof body?.error === 'string'
        ? body.error
        : `/bars/window.json answered HTTP ${status} with no window, so there is no bar ` +
            `to read and no reason recorded for it.`
    );
  }
  const faults = typeof body.faults === 'string' ? body.faults : null;
  const bar = body.bars[0];
  if (!bar) {
    /* THREE ABSENCES, NOT ONE. A damaged record is a fact about this read; a
       day with no bars is a fact about the store; and neither says the market
       did not trade. */
    if (faults) {
      return nothing(
        key,
        `The store holds ${month} for this series but a record would not read: ${faults}` +
          (day ? `. Whether it holds ${day} is not something this read can say.` : '')
      );
    }
    if (day) {
      return nothing(
        key,
        `The store holds no ${timeframe} bars for this series on ${day}. The day was ` +
          `asked by its own bounds, so this is the whole day: the series did not trade ` +
          `then, or its bars were never stored.`
      );
    }
    return nothing(key, `The store holds no ${timeframe} bars for this series in ${month}.`);
  }
  const quote = fromBar(key, bar);
  quote.faults = faults;
  if (at !== null) {
    if (quote.at === null || quote.at > at) {
      /* NEVER A LATER BAR FOR AN EARLIER TIME. With no bar at or before the
         instant, the window's page holds the day's FIRST bar, which is after
         it; showing that would be look-ahead. */
      return nothing(
        key,
        `This series has no ${timeframe} bar at or before ${time} on ${day}` +
          (quote.at === null ? '.' : `: its first bar that day is stamped ${secondLabel(quote.at * 1000)}.`) +
          ` A later bar is never shown for an earlier time.`
      );
    }
    quote.asked = at;
    quote.lag = at - quote.at;
  }
  return quote;
}

/**
 * The quote for one series: the month's last bar, a day's last bar, or the
 * last bar at or before a time on that day. ONE seek on the wire in every
 * case (see {@link windowUrl}).
 *
 * NEVER THROWS. A refused parameter, a transport error, a timeout or a body
 * that is not a window comes back as a `Quote` whose numbers are `null` and
 * whose `why` is the reason; a rejection would take out the other rows of the
 * same batch.
 *
 * @param {string} feed @param {string} key @param {string} timeframe
 * @param {string} month @param {string} [day] `YYYY-MM-DD`, or `''`
 * @param {string} [time] `HH:MM[:SS]` in IST, or `''` for the day's last bar
 * @returns {Promise<Quote>}
 */
export function quoteOn(feed, key, timeframe, month, day = '', time = '') {
  const id = quoteKey(feed, key, timeframe, month, day, time);
  const held = inflight.get(id);
  if (held) return held;

  const run = (async () => {
    const asked = windowUrl(feed, key, timeframe, month, day, time);
    if ('why' in asked) return nothing(key, asked.why);
    try {
      const res = await gated(asked.url);
      const body = await res.json().catch(() => null);
      return quoteFrom(key, body, res.status, { timeframe, month, day, time, at: asked.at });
    } catch (error) {
      const e = /** @type {any} */ (error);
      return nothing(key, String(e && e.message ? e.message : e));
    }
  })();

  if (inflight.size >= QUOTE_CACHE) {
    const oldest = inflight.keys().next().value;
    if (oldest !== undefined) inflight.delete(oldest);
  }
  inflight.set(id, run);
  return run;
}

/**
 * Quotes for the rows on screen, as of one day.
 *
 * @param {string} feed
 * @param {{key: string, timeframe: string, month: string}[]} rows
 * @param {string} day `YYYY-MM-DD`, or `''`
 * @param {string} [time]
 * @param {AbortSignal} [signal] Cancels queued work; shared reads already started remain cached.
 * @returns {Promise<Quote[]>}
 */
export function quotesOn(feed, rows, day, time = '', signal) {
  return pooled(rows, IN_FLIGHT, (r) => quoteOn(feed, r.key, r.timeframe, r.month, day, time), signal);
}

/**
 * THE UNDERLYING'S SPOT FOR EACH CONTRACT QUOTE, AT THAT BAR'S OWN INSTANT
 * (DB-G15).
 *
 * `/spot.json` already joins a contract's bar to its underlying's series, and
 * `/db` already reads it through `readSpots`; this is that reader, not a
 * second one. One request per underlying, timeframe and month, each holding at
 * most {@link SPOT_CHUNK} (400) instants, so seventeen rows of one chain are
 * one request and a widened 250-row view of one underlying is still one.
 *
 * `exact` is the default (D-2807): the spot stamped at the bar's own second.
 * `prev` opts in to the last spot at or before it, and the answer then
 * carries its age (D-3328). Never a later spot.
 *
 * @param {string} feed
 * @param {Quote[]} quotes the quotes on screen; only contracts with a bar are asked
 * @param {string} timeframe
 * @param {AbortSignal} signal
 * @param {'exact'|'prev'} [asof]
 * @returns {Promise<Map<string, import('./moneyness.js').SpotAnswer>>} by census key
 */
export async function spotsFor(feed, quotes, timeframe, signal, asof = 'exact') {
  /** @type {{instrument: string, tf: string, ts: number, side: string, id: string}[]} */
  const rows = [];
  /** @type {Map<string, import('./moneyness.js').SpotAnswer>} */
  const out = new Map();
  for (const q of quotes) {
    if (!q || q.at === null) continue;
    const p = parseKey(q.key);
    if (p.kind !== 'future' && p.kind !== 'option') continue;
    /* AN ANSWER ALREADY READ IS NOT ASKED FOR AGAIN. The page calls this each
       time a batch of quotes lands, with every quote it holds, so the rows of
       the batch before were re-read for nothing. The answer for one contract
       bar under one rule does not change while the census generation stands
       (`forgetQuotes` drops these with the quotes). */
    const id = `${feed}|${asof}|${timeframe}|${q.key}|${q.at}`;
    const known = spotsHeld.get(id);
    if (known) {
      out.set(q.key, known);
      continue;
    }
    // `readSpots` asks for every row that names a side; a future has none of
    // its own and is asked the same question, so it is given the word FUT.
    rows.push({ instrument: q.key, tf: timeframe, ts: q.at * 1000, side: p.side ?? 'FUT', id });
  }
  if (rows.length === 0) return out;
  const read = await readSpots(feed, rows, signal, /** @type {typeof ask} */ (gated), asof);
  for (const row of rows) {
    const spot = read.get(spotKey(row));
    if (!spot) continue;
    out.set(row.instrument, spot);
    // A FAILED REQUEST IS NOT REMEMBERED: the next batch asks again.
    if (spot.why === 'spot_request_failed') continue;
    if (spotsHeld.size >= QUOTE_CACHE) {
      const oldest = spotsHeld.keys().next().value;
      if (oldest !== undefined) spotsHeld.delete(oldest);
    }
    spotsHeld.set(row.id, spot);
  }
  return out;
}

/** Spot answers already read, by (feed, rule, timeframe, contract, instant);
    at most {@link QUOTE_CACHE}, oldest out first.
    @type {Map<string, import('./moneyness.js').SpotAnswer>} */
const spotsHeld = new Map();

/** The strip's order, cached on the promise. @type {Promise<{served: {key: string, fno: boolean}[], why: string|null}>|null} */
let aliases = null;

/**
 * The index names `/aliases.json` serves, in its order, each with the key it
 * is stored under and whether it is an F&O index underlying. ONE request per
 * page load: the list is the build's, so a read that succeeded is never
 * repeated. A failed read is an EMPTY list beside its reason, so the strip
 * falls back to name order and says so; it never guesses an order, and the
 * failure is not kept, so the next call asks again.
 *
 * @returns {Promise<{served: {key: string, fno: boolean}[], why: string|null}>}
 */
export function servedIndices() {
  if (aliases) return aliases;
  const run = (async () => {
    try {
      const res = await gated('/aliases.json');
      const body = await res.json().catch(() => null);
      if (!body || !Array.isArray(body.index)) {
        return {
          served: [],
          why:
            typeof body?.error === 'string'
              ? body.error
              : `/aliases.json answered HTTP ${res.status} with no index list.`
        };
      }
      const served = [];
      for (const row of body.index) {
        if (typeof row?.key !== 'string') continue;
        served.push({ key: row.key, fno: row.in_fno_universe === true });
      }
      return { served, why: null };
    } catch (error) {
      const e = /** @type {any} */ (error);
      return { served: [], why: String(e && e.message ? e.message : e) };
    }
  })();
  aliases = run;
  run.then((answer) => {
    if (answer.why !== null && aliases === run) aliases = null;
  });
  return run;
}

/**
 * The first and last calendar day of a month, `YYYY-MM-DD`: the Date control's
 * bounds. Any day of the chosen month may be asked for; one that holds no bars
 * answers per row with that reason.
 * @param {string} month `YYYY-MM` @returns {{min: string, max: string}|null}
 */
export function monthSpan(month) {
  const m = String(month ?? '').match(/^(\d{4})-(0[1-9]|1[0-2])$/);
  if (!m) return null;
  const last = new Date(Date.UTC(Number(m[1]), Number(m[2]), 0)).getUTCDate();
  return { min: `${m[0]}-01`, max: `${m[0]}-${String(last).padStart(2, '0')}` };
}

/**
 * The first and last IST day a set of census cells covers: what the series in
 * view hold, said beside the Date control. Free: the census carries each
 * cell's stamps.
 *
 * @param {({first: number|null, last: number|null}|undefined)[]} cells `first` and `last` in MICROSECONDS
 * @returns {{min: string, max: string}|null}
 */
export function daySpan(cells) {
  let lo = Infinity;
  let hi = -Infinity;
  for (const cell of cells) {
    if (!cell) continue;
    if (typeof cell.first === 'number' && cell.first < lo) lo = cell.first;
    if (typeof cell.last === 'number' && cell.last > hi) hi = cell.last;
  }
  if (!Number.isFinite(lo) || !Number.isFinite(hi)) return null;
  const min = istDay(lo / 1000);
  const max = istDay(hi / 1000);
  return min && max ? { min, max } : null;
}

/**
 * Forgets every cached quote.
 *
 * CALLED WHEN THE CENSUS GENERATION CHANGES: a pull or an import writes new
 * bars under keys the cache already holds answers for, and a terminal that
 * kept serving the earlier close would report a superseded measurement.
 */
export function forgetQuotes() {
  inflight.clear();
  spotsHeld.clear();
}
