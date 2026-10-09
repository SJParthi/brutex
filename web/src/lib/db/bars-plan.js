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
import { pooled as readPool } from '../pooled.js';
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
export const FIT_SLACK = 24;
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
   WHICH COLUMNS `/bars/window.json` CAN ORDER BY, AND IT IS NOT ALL OF THEM.

   `bars::SortKey::parse` accepts exactly these seven — they are the fields a
   stored `Bar` HAS. This grid draws twenty-five columns, and the other
   eighteen are derived here: the change columns, the expiry arithmetic, the
   moneyness, every greek. The endpoint has no way to order by a number the
   store never held.

   WHAT HAPPENED WITHOUT THIS GUARD, measured by clicking the column: sorting
   on OI CHG % sent `sort=oichg`, the server answered **400 — "oichg" is not a
   column this grid sorts on** — and the grid emptied. Fifty rows became none,
   on a store holding 623,498 bars, because a control the page offers asked a
   question the route it now uses cannot take.

   A COLUMN THE GRID OFFERS MUST NOT EMPTY THE GRID. The window route is for
   the seven the store can order; everything else keeps the read-and-sort-here
   path it always had, which is slower and correct. `CLAUDE.md` §4: degrade
   loudly and name the reason, or refuse — never a control that silently
   returns nothing. */
export const WINDOW_SORTS = new Set(['ts', 'o', 'h', 'l', 'c', 'v', 'oi']);

/* ---------------------------------------------------------------------
   HOW MANY OF THOSE REQUESTS ARE IN FLIGHT AT ONCE - and it is a constant,
   which is the whole point.

   `Promise.all(want.map(read))` starts EVERY request in the same tick. The
   budget above deliberately reads every matched instrument-month, so `want`
   is as long as the query is wide, and the number of open sockets was
   therefore a function of how much history the store holds.

   MEASURED on this build, 2026-08-22, a store holding 2019-12..2026-08 for
   NIFTY, BANKNIFTY and INDIAVIX at nine rungs - 81 months x 9 x 3 = 2,187
   matched instrument-months:

     requests fired                     498+ (the capture cut off)
     succeeded                          0
     every one of them                  net::ERR_INSUFFICIENT_RESOURCES

   Chrome refuses to open more sockets long before 2,187 and fails the
   REQUEST rather than queueing it, so the grid rendered nothing at all. Not
   slow - empty, with a console full of failures and no sentence on the page
   saying why. That is the shape `CLAUDE.md` §4 bans: a failure wearing an
   empty table's clothes.

   A POOL, NOT A SMALLER BUDGET. Capping `readBudget` would answer a narrower
   question than the operator asked, which the block above rejects in the
   operator's own words - "a page that answers a narrower question than the
   one it was asked is worse than a slow one". Every matched month is still
   read. What is bounded is how many are in flight, so the cost per reader is
   constant no matter how wide the query gets, and a store ten years deep
   costs the same sockets as one month does.

   SIX because that is what a browser gives a single origin over HTTP/1.1;
   asking for more does not make more, it makes a queue this code cannot see
   and cannot report on.
   --------------------------------------------------------------------- */
export const IN_FLIGHT = 6;

/**
 * Runs `job` over `items` with at most `limit` outstanding at any moment.
 *
 * Results come back in the ORDER OF `items`, not the order they finished,
 * because the caller indexes them against its own plan and a pool that
 * reordered would silently shuffle the grid.
 *
 * A job that throws lands as a rejection in its own slot rather than taking
 * the pool down - `readBarFile` already returns failures as values, so this
 * is the belt to that braces.
 *
 * @template T, R
 * @param {T[]} items
 * @param {number} limit
 * @param {(item: T) => Promise<R>} job
 * @param {AbortSignal} [signal]
 * @returns {Promise<R[]>}
 */
export async function pooled(items, limit, job, signal) {
  return readPool(items, limit, job, signal);
}
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

/**
 * `HH:MM` to minutes since IST midnight, or `null` if it is not a time.
 *
 * VALIDATES THE PARTS, not just the shape: `25:00` and `10:73` both match a
 * digits-colon-digits pattern and neither is a minute of any day. Returning
 * `null` for them is what lets the caller say "that is not a time" instead
 * of landing on a bar chosen by arithmetic on nonsense.
 *
 * @param {string | null | undefined} hhmm
 */
export function minuteOf(hhmm) {
  const m = String(hhmm ?? '')
    .trim()
    .match(/^(\d{1,2}):(\d{2})$/);
  if (!m) return null;
  const h = Number(m[1]);
  const mi = Number(m[2]);
  if (h > 23 || mi > 59) return null;
  return h * 60 + mi;
}

/**
 * Which page buttons to draw. Always first, last, current and its
 * neighbours; the rest elided - a 1,840-page run must not render 1,840
 * buttons, which is the constant-per-paint rule applied to a control.
 *
 * @param {number} cur
 * @param {number} total
 */
export function pageList(cur, total) {
  /* ONE SHAPE FROM BOTH BRANCHES. The short branch returned bare numbers
     and the long one returned records, so the `{#each}` key expression was
     reading `.k` off a number for every run of seven pages or fewer. */
  if (total <= 7) {
    return Array.from({ length: total }, (_, i) => ({ k: `p${i + 1}`, p: i + 1, gap: false }));
  }
  const want = new Set([1, total, cur, cur - 1, cur + 1]);
  if (cur <= 3) {
    want.add(2);
    want.add(3);
    want.add(4);
  }
  if (cur >= total - 2) {
    want.add(total - 1);
    want.add(total - 2);
    want.add(total - 3);
  }
  const list = [...want]
    .filter((p) => p >= 1 && p <= total)
    .sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
  /** @type {{ k: string, p: number, gap: boolean }[]} */
  const out = [];
  list.forEach((p, i) => {
    if (i && p - list[i - 1] > 1) out.push({ k: `gap${p}`, p, gap: true });
    out.push({ k: `p${p}`, p, gap: false });
  });
  return out;
}

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
