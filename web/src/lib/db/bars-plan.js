/**
 * /db — THE BAR GRID'S FIXED NUMBERS AND PURE ARITHMETIC
 *
 * Row metrics, page sizes, the IST day key, the pager's page list and the other
 * pure helpers the bar read and the pager use.
 *
 * Every line below was MOVED out of `routes/db/+page.svelte` by WP-00 (the /db
 * page split). The only edits to a moved line are the ones the move needs: a
 * page-level name that now lives in another file is imported, or, for state,
 * read as `db.<name>`; a plain module exports what it declares; and a callback
 * over such a value marks its parameter `any`, the way the page already did
 * elsewhere. Nothing was fixed in the move: every defect the audit found is
 * still present byte for byte, and is fixed by the package that owns this file
 * next.
 *
 * Owner after WP-00: WP-52 (bars data path, pager, go-to, extremes, export).
 *
 * A PLAIN MODULE, NOT A `.svelte.js` ONE: it holds constants and pure
 * functions only, so `node --test` can import it with no compiler. Imports
 * are relative for the same reason — `$lib` is a Vite alias node cannot see.
 */
import { fmt } from './notes-labels.js';

/* ---------------------------------------------------------------------
   THE PAGER'S STATE, and it governs BOTH grids.

   `page` is a 1-based ordinal and it is CLAMPED AT EVERY READ (`pageNow`)
   rather than written back: a narrowing that shortens the run must not
   leave the reader on page 40 of 3 looking at a blank grid, and a clamp
   that writes state from inside a derived is a loop.
   --------------------------------------------------------------------- */
export const PAGE_SIZES = [25, 50, 100, 250, 1000];

/* ======================================================================
   FIT — THE GRID HOLDS THE ROWS THE SCREEN CAN SHOW, AND NOT ONE MORE
   ======================================================================
   Every fixed page size is a guess about a viewport the script cannot see,
   and when the guess is high the surplus does not vanish — it becomes a
   second scrollbar inside the table. That is what this page was carrying:
   fifty rows fetched into a box with room for the first thirteen, so the
   wheel moved the rows under the pointer and the page under it everywhere
   else, and neither felt like the one in charge.

   There is no arrangement of two scrollers that reads well. The fix is to
   stop producing the surplus: measure the box, divide by the row height, and
   fetch exactly that. The inner scroll range then goes to ZERO — not hidden,
   not chained, not styled away, simply absent, because every row that was
   asked for is on screen.

   `fit` IS THE DEFAULT AND THE FIXED SIZES REMAIN. A reader who wants 250
   rows in one document to search or export still picks one and gets a
   scrolling box, deliberately. What changes is that they must ask.  */
/* THE TWO WINDOWING CONSTANTS, HOISTED TO THEIR FIRST USE. They belong to
   the virtual-scroll section far below — `ROW` places row N at `N * ROW` and
   `HEAD` is the sticky header the cursor arithmetic clears — and they are
   declared HERE because `rowsThatFit` divides by both and is written above
   that section. `--dbrow` and `--dbhead` in the stylesheet are the same two
   measurements in the other language; none of the four moves alone. */
export const ROW = 40;
export const HEAD = 30; /* the sticky header covers the top of the scroll box */
/* THE FIT'S TOLERANCE, HOISTED HERE FOR THE SAME REASON THE TWO ABOVE WERE.
   Two deriveds divide by it — `rowsThatFit` for the grid and `settledRows`
   for the fold — and the second is written above the first. It ran, because a
   `$derived` is lazy, and only `svelte-check` saw the temporal dead zone;
   that is the third time in this file, so the rule is now simply that a
   constant shared by two deriveds lives above both. Its reasoning stays with
   `rowsThatFit`, which is where the 24 was measured. */
/* EIGHT SINCE R3-03, AND THE SEVENTEEN IT HELD FOR `.bnotes` IS GONE: the notes
   strip is now always in the box at one line's height, so its arrival after a
   read no longer shrinks the box under a fitted page, and the heading's real
   height (its band and its labels) is measured rather than taken as `HEAD`.
   What remains is the damper — a fifth of a row for fonts and sub-pixels.
   Twenty-four counted a row fewer than the window holds whenever the box was
   within sixteen pixels of one more (the re-check's "Fit counts the rows that
   fit the window"). */
export const FIT_SLACK = 8;
/* `--s4`, WHICH IS `.board`'s `gap`. Written here because this file cannot
   read a stylesheet token, and named rather than inlined so the next reader
   knows which declaration it is shadowing. If `.board`'s gap changes, this
   changes with it; being wrong costs the fold's LABEL a few pixels of
   accuracy and nothing else, which is why it is worth an approximation
   rather than a `getComputedStyle` call on every resize. */
export const BOARD_GAP = 8;
/* TEN ROWS, AND THE SECOND CONDITION EXISTS BECAUSE "DOES IT FIT" ALONE
   PRODUCED A BIGGER WINDOW WITH FEWER ROWS.
   ---------------------------------------------------------------------
   MEASURED with only the overflow test: a 900px window did not fit, folded,
   and gave EIGHT rows; a 1,000px window fitted — barely, with the table
   sitting on its floor — so it did not fold, and gave FIVE. A hundred pixels
   more screen, three rows fewer, because "fits" was satisfied by a table too
   small to read. Fitting is the constraint; a usable grid is the point.

   So the readouts also fold when the grid is under ten rows and they are
   what is standing in its way. Ten is the count this page's own arithmetic
   keeps arriving at as the plain-view figure on an ordinary window, so it
   reads as "the grid is worse than ordinary here" rather than as a threshold
   picked to make a case pass.

   STILL MONOTONIC. This only ever folds, the guard above returns as soon as
   `compact` is true, and a reader's press stops it for good — so adding a
   second trigger adds no way for it to run twice. */
export const MIN_USEFUL_ROWS = 10;

/* ---------------------------------------------------------------------
   WHICH COLUMNS `/bars/window.json` ORDERS, IN THE GRID'S KEYS AND THE WIRE'S.

   `bars::SortKey::parse` accepts `ts`, `tod`, `o`, `h`, `l`, `c`, `v` and
   `oi` — the fields a stored `Bar` has, and its IST second of the day. The
   grid's own column keys are not all spelled that way, and the difference
   cost a full read: the Volume column is `vol`, `WINDOW_SORTS` held `v`, so
   `WINDOW_SORTS.has('vol')` was false and a Volume sort over GDFL NIFTY 1s
   asked for every one of 97 month files whole, about 3.5 GB of JSON, to sort
   them in the browser (DB-062, DB-G94). The Time column (`tod`) was not in the
   set either, and no row carried `tod`, so its arrows flipped the caret and
   not the rows (DB-006, DB-N05).

   So the grid's key is TRANSLATED here, once, for every reader: the request
   and the test of whether the server can order a column read the same table.
   D-3326. */
export const WIRE_SORT = Object.freeze({
  ts: 'ts',
  tod: 'tod',
  o: 'o',
  h: 'h',
  l: 'l',
  c: 'c',
  vol: 'v',
  oi: 'oi'
});
/** The grid's column keys the server orders. */
export const WINDOW_SORTS = new Set(Object.keys(WIRE_SORT));

/**
 * The wire spelling of a grid column key the server orders, or `null` for a
 * column it does not.
 * @param {string} key
 * @returns {string | null}
 */
export function wireSort(key) {
  return Object.hasOwn(WIRE_SORT, key) ? /** @type {any} */ (WIRE_SORT)[key] : null;
}

/* ---------------------------------------------------------------------
   THE COLUMNS THE BROWSER ORDERS, AND THE CEILING ON WHAT IT WILL READ TO DO IT.

   The change columns are folded on the server per bar, against the previous
   stored bar in time, and travel with the rows; the contract columns are one
   value per series. Neither is a field the server can order a window by. The
   page used to answer such a sort by reading EVERY matched month whole
   through `/bars.json` — 97 requests and 3.55 GB for NIFTY 1s, 603 requests
   and 563 MB for one option expiry, a renderer crash under a carried-over
   sort (DB-063, DB-N16, DB-A29, DB-B50).

   NOW IT IS BOUNDED BEFORE ANYTHING IS READ. The server's own count of the
   rows the query keeps decides it: at or under `LOCAL_SORT_BUDGET` the rows
   are read in time order through the window route, at most
   `LOCAL_SORT_BUDGET / WINDOW_PAGE_MAX` requests, and ordered here; past it
   the sort is REFUSED by name with the count, and the grid stays in time
   order. `/bars.json` is not called by /db at all any more. */
export const LOCAL_SORTS = new Set(['chg', 'oichg', 'exp', 'dte', 'side', 'strike']);
/** The most rows the browser reads to order a column the server cannot. */
export const LOCAL_SORT_BUDGET = 20_000;
/** The largest page `/bars/window.json` serves (`bars::MAX_WINDOW_LIMIT`). */
export const WINDOW_PAGE_MAX = 1000;
/** The most series one window request names (`bars::MAX_SERIES`). */
export const MAX_SERIES = 1024;
/** The most months one window request spans (`bars::MAX_WINDOW_MONTHS`). */
export const MAX_WINDOW_MONTHS = 240;
/** The most rows the whole-query export writes. */
export const EXPORT_CAP = 20_000;

/**
 * Months from `a` to `b` inclusive, both `YYYY-MM`; `0` when `b` is before `a`.
 * @param {string} a @param {string} b
 */
export function monthSpan(a, b) {
  const ya = Number(a.slice(0, 4));
  const ma = Number(a.slice(5, 7));
  const yb = Number(b.slice(0, 4));
  const mb = Number(b.slice(5, 7));
  const n = (yb - ya) * 12 + (mb - ma) + 1;
  return Number.isFinite(n) ? Math.max(0, n) : 0;
}

/**
 * THE SELECTION AS ONE WINDOW REQUEST NAMES IT (D-3326): its rung, every
 * series it reaches, and the month range — narrowed to the day window's months
 * when one narrows. Each series' census rows are kept beside it so the census
 * can be compared with the file's own count when the answer arrives.
 *
 * `why` is a refusal said before any request: more series than one request
 * reads, a range longer than the server spans, more than one rung, or a day
 * window wholly outside the held months.
 *
 * O(matched rows), once per change of the selection — never per page.
 *
 * @param {any[]} rows the matched census rows
 * @param {string} [fromDay] the day window's first day, when it narrows
 * @param {string} [toDay] the day window's last day, when it narrows
 */
export function seriesPlan(rows, fromDay, toDay) {
  /** @type {Map<string, {rows: number, census: any}>} */
  const series = new Map();
  const rungs = new Set();
  const months = new Set();
  for (const r of rows) {
    rungs.add(r.timeframe);
    months.add(r.month);
    const held = series.get(r.instrument);
    if (held) held.rows += Number(r.rows) || 0;
    else series.set(r.instrument, { rows: Number(r.rows) || 0, census: r });
  }
  /* The first and last month held, by sorting rather than by comparing, so
     a tie is not a second way to write the same answer (R3-03). */
  const held = [...months].sort();
  const lo = held[0] ?? '';
  const hi = held.at(-1) ?? '';
  const keys = [...series.keys()].sort();
  const timeframe = rungs.size === 1 ? String([...rungs][0]) : '';
  /* A day window narrows the months asked for, never widens them. An empty
     From sorts before every month, so it leaves the first held one. */
  const from = fromDay ? [lo, fromDay.slice(0, 7)].sort()[1] : lo;
  const to = toDay ? [hi, toDay.slice(0, 7)].sort()[0] : hi;
  /** @type {string | null} */
  let why = null;
  if (keys.length === 0) return { keys, series, timeframe, from, to, why };
  if (rungs.size > 1) {
    why = `this selection reaches ${fmt(rungs.size)} rungs (${[...rungs].join(', ')}); a page is read at one rung, so choose one`;
  } else if (keys.length > MAX_SERIES) {
    why = `too_many_series: this selection reaches ${fmt(keys.length)} series and one window request reads at most ${fmt(MAX_SERIES)} — choose an expiry, a strike or a side`;
  } else if (from > to) {
    why = `the date window ${fromDay || '…'} to ${toDay || '…'} lies outside the held months ${lo} to ${hi}, so there is nothing to read`;
  } else if (monthSpan(from, to) > MAX_WINDOW_MONTHS) {
    why = `the months ${from} to ${to} span ${fmt(monthSpan(from, to))}, past the ${fmt(MAX_WINDOW_MONTHS)} one window request reads — narrow the dates`;
  }
  return { keys, series, timeframe, from, to, why };
}

/**
 * A time bound as the wire spells it, `HH:MM:SS`.
 *
 * THE TO END OF A MINUTE IS ITS FIFTY-NINTH SECOND (DB-B48). The server reads
 * `tod_to=15:29` as `15:29:00`, inclusive, so on one-second bars it kept one
 * of the minute's sixty bars while this page's own picker meant the whole
 * minute. A minute is therefore sent as `HH:MM:00` at the From end and
 * `HH:MM:59` at the To end; a bound that already carries its second is sent
 * as it is. Anything else is `''`, no bound.
 *
 * @param {string | undefined} t @param {'from' | 'to'} end
 */
export function todWire(t, end) {
  const s = typeof t === 'string' ? t.trim() : '';
  if (/^\d{2}:\d{2}$/.test(s)) return `${s}:${end === 'to' ? '59' : '00'}`;
  if (/^\d{2}:\d{2}:\d{2}$/.test(s)) return s;
  return '';
}

/**
 * The query string of one `/bars/window.json` request over a planned
 * selection: every series as a `key=`, the month range, the day and
 * time-of-day bounds, the order, and either a page (`offset`, `limit`) or an
 * instant to find (`at`, no offset).
 *
 * `extremes` is sent only when asked: a fold is a scan (DB-G83), and a page
 * turn must cost a seek.
 *
 * @param {string} feed
 * @param {{keys: string[], timeframe: string, from: string, to: string}} plan
 * @param {{sort?: string, desc?: boolean, offset?: number, limit?: number,
 *   fromDay?: string, toDay?: string, fromTime?: string, toTime?: string,
 *   at?: number | null, extremes?: boolean}} o
 */
export function windowQuery(feed, plan, o) {
  const q = new URLSearchParams();
  q.set('feed', feed);
  q.set('timeframe', plan.timeframe);
  q.set('from', plan.from);
  q.set('to', plan.to);
  for (const k of plan.keys) q.append('key', k);
  q.set('sort', o.sort ?? 'ts');
  q.set('dir', o.desc === false ? 'asc' : 'desc');
  if (o.at !== undefined && o.at !== null) q.set('at', String(o.at));
  else q.set('offset', String(o.offset ?? 0));
  q.set('limit', String(o.limit ?? 50));
  if (o.fromDay) q.set('from_day', o.fromDay);
  if (o.toDay) q.set('to_day', o.toDay);
  const tf = todWire(o.fromTime, 'from');
  const tt = todWire(o.toTime, 'to');
  if (tf) q.set('tod_from', tf);
  if (tt) q.set('tod_to', tt);
  q.set('extremes', o.extremes ? '1' : '0');
  return q.toString();
}

/**
 * The identity of one stored bar on the page: its series and its second. A
 * series holds one bar per stamp, so this is unique across a merged page and
 * stable across a re-sort — which is what a highlight and an `{#each}` key
 * need.
 * @param {string} key @param {number} t unix seconds
 */
export const rowKey = (key, t) => `${key}|${t}`;

/**
 * An IST wall-clock instant as unix seconds, or `null` when either part is
 * not a day or a time.
 * @param {string} day `YYYY-MM-DD` @param {string} time `HH:MM` or `HH:MM:SS`
 */
export function istInstant(day, time) {
  const d = typeof day === 'string' ? day.match(/^(\d{4})-(\d{2})-(\d{2})$/) : null;
  const t = typeof time === 'string' ? time.match(/^(\d{2}):(\d{2})(?::(\d{2}))?$/) : null;
  if (!d || !t) return null;
  const [h, m, s] = [Number(t[1]), Number(t[2]), Number(t[3] ?? 0)];
  if (h > 23 || m > 59 || s > 59) return null;
  const midnight = Date.UTC(Number(d[1]), Number(d[2]) - 1, Number(d[3]));
  /* A day that does not exist rolls over (30 Feb is 2 Mar), so the day read
     back must be the day typed. */
  if (new Date(midnight).toISOString().slice(0, 10) !== `${d[1]}-${d[2]}-${d[3]}`) return null;
  return (midnight - IST_OFFSET_MS) / 1000 + h * 3600 + m * 60 + s;
}

/** The 1-based page holding 0-based row `index` at `size` rows a page. @param {number} index @param {number} size */
export const pageOfIndex = (index, size) => Math.floor(Math.max(0, index) / Math.max(1, size)) + 1;

/**
 * WHERE THE CENSUS AND THE FILE DISAGREE, by series (DB-G02, DB-G61, DB-022).
 *
 * The pager counts the FILE's own rows — the server's `series_totals` — so a
 * census behind a running import no longer strands the bars past its count.
 * The census is still compared, so its lag is said rather than hidden: one
 * entry per series whose census rows differ from the file's.
 *
 * Only meaningful when no day or time bound narrows the read: under a bound
 * the server's total is the kept rows, not the file's.
 *
 * @param {{key: string, total: number}[]} totals @param {Map<string, {rows: number}>} census
 */
export function censusBehind(totals, census) {
  /** @type {{key: string, file: number, census: number}[]} */
  const out = [];
  if (!Array.isArray(totals)) return out;
  for (const t of totals) {
    const c = census.get(t.key);
    if (c && Number.isSafeInteger(t.total) && c.rows !== t.total) out.push({ key: t.key, file: t.total, census: c.rows });
  }
  return out;
}

/**
 * A grid column's values compared for a local sort: nulls last in BOTH
 * directions, then the value, then a deterministic tie-break that does not
 * invert with the direction.
 * @param {string} k @param {boolean} desc
 */
export function localComparator(k, desc) {
  return (/** @type {any} */ a, /** @type {any} */ b) => {
    const A = a[k];
    const B = b[k];
    const na = A === null || A === undefined;
    const nb = B === null || B === undefined;
    if (na !== nb) return na ? 1 : -1;
    /* Text and numbers both order by `<`: an expiry, a side and a strike are
       each one type down a column; two unknowns are neither. */
    const d = A < B ? -1 : A > B ? 1 : 0;
    if (d !== 0) return desc ? -d : d;
    return a.ts < b.ts ? -1 : a.ts > b.ts ? 1 : a.rk < b.rk ? -1 : a.rk > b.rk ? 1 : 0;
  };
}

/**
 * WHICH CHANGES WERE MEASURED ACROSS THE OVERNIGHT GAP (R3-03).
 *
 * A bar's Change % is its close against the previous STORED bar of its
 * series, wherever that lies (D-3325). For the first bar of a session that is
 * the last bar of an earlier day, so the figure is the overnight move, not a
 * bar-to-bar one — and sorted by Change % those rows come out on top looking
 * like the biggest bars of the month (the 11 Oct re-check, NIFTY spot ·
 * Zerodha 1min, source row 57). Each such row now says so.
 *
 * Said only where it is KNOWN, never guessed:
 * - the previous bar of the same series is in `rows` and on an earlier IST
 *   day; or
 * - `edgeEarlier`: the read began at the first kept bar of a day window
 *   with no time-of-day bound, so the bar before a series' first row lies
 *   before the window's first day.
 * A time-of-day bound proves nothing: the bar before the first kept bar of a
 * day may be on that same day, outside the window, so no row is marked under
 * one. A day rung is a close-to-close series by definition and is not marked.
 *
 * O(rows · log rows), once per read — never per paint.
 *
 * @param {any[]} rows grid rows (`rk`, `key`, `ts`, `day`, `chg`), any order
 * @param {{timeNarrows: boolean, dayRung: boolean, edgeEarlier: boolean}} o
 * @returns {Map<string, number | null>} row key → the previous bar's ms, `null` when it is before the read
 */
export function acrossDay(rows, o) {
  /** @type {Map<string, number | null>} */
  const out = new Map();
  if (o.timeNarrows || o.dayRung) return out;
  /** @type {Map<string, any[]>} */
  const bySeries = new Map();
  for (const r of rows) {
    const list = bySeries.get(r.key);
    if (list) list.push(r);
    else bySeries.set(r.key, [r]);
  }
  for (const list of bySeries.values()) {
    list.sort((a, b) => a.ts - b.ts);
    list.forEach((r, i) => {
      if (typeof r.chg !== 'number') return;
      const prev = list[i - 1];
      if (prev ? prev.day < r.day : o.edgeEarlier) out.set(r.rk, prev ? prev.ts : null);
    });
  }
  return out;
}

/* `pooled` AND `IN_FLIGHT` STOOD HERE AND WENT IN R3-03: nothing imported
   them from this file any more. The one pool and its six-socket ceiling are
   `$lib/pooled.js`, which every reader that fans out already uses. */
/** Files whose bar count disagrees with the census row that named them. */
/* `barDisagree` HAS MOVED, to sit below `windowSaid` — it now has to ask
   whether the rows came from the window endpoint, and `windowSaid` is
   declared further down. Order is not decoration here; see its new home. */

/* ---------------------------------------------------------------------
   THE BAR ROWS. Built once per read - never per paint and never per
   keystroke: the comparator below runs O(n log n) times and must compare
   plain fields, exactly as the census comparator does.
   --------------------------------------------------------------------- */
export const IST_OFFSET_MS = 19800000; /* +05:30, and India keeps no DST */

/**
 * The IST calendar day an epoch-millisecond instant falls on, as the
 * `YYYY-MM-DD` KEY form and never a label. It is subtracted from an
 * expiry, so it has to stay comparable.
 * @param {number} ms
 */
export function istDayKey(ms) {
  const d = new Date(ms + IST_OFFSET_MS);
  return `${d.getUTCFullYear()}-${String(d.getUTCMonth() + 1).padStart(2, '0')}-${String(
    d.getUTCDate()
  ).padStart(2, '0')}`;
}

/**
 * Whole calendar days between two ISO days, or `null` if either will not
 * parse. Both ends are read at UTC midnight, so no zone offset can round a
 * day off the difference.
 * @param {string} fromIso
 * @param {string} toIso
 */
export function daysBetween(fromIso, toIso) {
  const a = Date.parse(`${fromIso}T00:00:00Z`);
  const b = Date.parse(`${toIso}T00:00:00Z`);
  if (!Number.isFinite(a) || !Number.isFinite(b)) return null;
  return Math.round((b - a) / 86400000);
}

/**
 * A price in paisa, with its two decimals and Indian digit grouping.
 *
 * INTEGER ARITHMETIC THROUGHOUT, exactly as `strikeText` does it: a `%`
 * and a subtraction, never a divide by 100 on the way to the screen.
 * Prices are `i64` paisa (`CLAUDE.md` section 7) and a float would round
 * one of them. Unlike `strikeText` the two decimals are ALWAYS printed -
 * an OHLC column holding 24,150 beside 24,150.25 has to align.
 *
 * @param {number | null | undefined} p
 */
export function paisaText(p) {
  if (p === null || p === undefined) return '-';
  const neg = p < 0;
  const a = neg ? -p : p;
  const frac = a % 100;
  const whole = (a - frac) / 100;
  return `${neg ? '−' : ''}${fmt(whole)}.${String(frac).padStart(2, '0')}`;
}

/**
 * THE DAYS THIS STORE ACTUALLY HOLDS — `[oldest, newest]` as `yyyy-mm-dd`,
 * or `['', '']` when nothing is loaded.
 *
 * # Why the calendar needs this and did not have it
 *
 * /ingest's day grid draws its bounds: days outside what the field may take
 * are struck through, and a paragraph under the grid names the window. /db's
 * grid drew every day plain, because it passed `$lib/DayField.svelte` no
 * `min` and no `max` — so the two calendars were the same COMPONENT showing
 * different amounts of truth.
 *
 * The bound here is a different fact from /ingest's and that is not an
 * inconsistency. /ingest's floor is the FEED's — how far back the vendor
 * answers, a rolling number that moves every day. This one is the STORE's:
 * the oldest and newest day actually on disk for the current selection. One
 * says what can be asked for, the other says what is held, which is the
 * standing difference between the two pages.
 *
 * ONE PASS over the rows already in memory, no sort and no new request —
 * `barRows` is what the table is about to draw anyway.
 */
/**
 * THE SPAN THE DAY FIELDS MAY REACH — /ingest's, not the store's.
 *
 * `dayBounds` below is the first and last day the LOADED ROWS hold, and
 * handing that to `DayField` as `min`/`max` made the calendar unable to leave
 * the store: with one month on disk the year pad offered exactly one chip,
 * `2026`, where /ingest's offers twelve. A reader could not navigate to 2015
 * to ask whether anything was there, because the control refused the question
 * before it was asked.
 *
 * So the BOUNDS are the whole span — 01 Jan 2015, the floor the server's own
 * picker offers, to today in IST — and `dayBounds` keeps its other job: it is
 * still what the bounds SENTENCE reports, so the reader is still told exactly
 * where the store starts and stops. The two were one value doing two jobs and
 * only one of them wanted the narrow number.
 *
 * `FLOOR_YEAR` is 2015 here for the same reason it is 2015 on /ingest: it is
 * the floor `crates/api/src/calendar.rs` offers, and the two pages must not
 * disagree about how far back a day may be named.
 */
export const FLOOR_YEAR = 2015;
export const spanFloor = `${FLOOR_YEAR}-01-01`;

/* ======================================================================
   WINDOWING — only the visible slice is in the DOM
   ====================================================================== */
/* 40px, AND IT IS THE SAME 40 THE `--dbrow` TOKEN SETS. Raised together with
   it: this places row N at `N * ROW`, so a disagreement drifts the rows away
   from the scrollbar one pixel per row and the list is unusable by the
   thousandth. 32 was cramped at the grid's new 14px type. */
/* A CHANGE IN BASIS POINTS AS A PERCENTAGE OF THE CELL'S WIDTH.
 *
 * Capped at 100 basis points — one percent — because that is where an
 * index minute-bar move stops being ordinary. Everything at or beyond it
 * draws a full bar, so the scale answers "is this one big" rather than
 * "how big", which is the question a column of fifty rows is actually
 * being scanned for. The exact figure is the text, unclipped, right there.
 *
 * `Math.abs` — direction is already carried by `data-dir`, which colours
 * the bar; length carries magnitude alone, so an up and a down of the same
 * size are the same length and are comparable at a glance.
 */
/* SCALED TO WHAT IS ACTUALLY IN THE SELECTION, NOT TO A GUESS.
 *
 * The first draft fixed full width at 100 basis points -- one percent -- on
 * the reasoning that an index minute-bar move stops being ordinary there.
 * Measured against the real rows: the 90th percentile is 5 bps, the 99th is
 * 8, and the largest on the page is 8. Every bar drew at 5-8% of the cell and
 * the column was blank. A scale nothing reaches is not a scale.
 *
 * Taken from `barRows` and not `barPage`, so it is every row IN MEMORY
 * rather than only the fifty drawn.
 *
 * THAT USED TO MEAN EVERY ROW THE QUERY MATCHED, AND SINCE `pagePlan` IT
 * DOES NOT. This comment read "every row the query matched ... so turning to
 * page 2 cannot silently change what a full bar means", and that was true
 * only because the grid read every matched instrument-month before drawing
 * anything - the 2,187-request storm. Now the exact path reads the file the
 * page sits in, so `barRows` holds that month and the scale is the widest
 * move IN THE LOADED WINDOW.
 *
 * So paging CAN change what a full bar means, and the honest description of
 * this scale is "widest move among the bars loaded", not "in the query". It
 * is stated here rather than quietly left as the old sentence because a
 * scale whose basis moved without saying so is the same defect as a receipt
 * that overstates: nothing on screen is wrong, and it is not the answer the
 * reader thinks they are getting.
 *
 * The fallback path is unchanged - a non-time sort still loads every matched
 * row, and there the old sentence still holds exactly.
 *
 * The floor stops a dead-flat selection from magnifying rounding into a
 * full-width bar: with every row at 0 or 1 bps there is nothing to compare
 * and the column stays quiet.
 */
export const CHG_FLOOR_BPS = 10;

/* A FLAT BAR STILL GETS A TICK. Zero range is a real state — a minute with
 * one print — and a row with no mark at all reads as a rendering fault. */
export const MIN_SPINE = 0.08;

export const SKELETON = Array.from({ length: 24 }, (_, i) => i);
