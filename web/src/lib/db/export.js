/**
 * /db — THE CSV EXPORT'S COLUMNS
 *
 * The column set of each view's export file and the RFC 4180 quoting.
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

/* ======================================================================
   EXPORT — the matched set, written by the browser, out of memory
   ----------------------------------------------------------------------
   NO REQUEST, AND NO FILE IN THIS REPOSITORY.

   It writes exactly the rows `sorted` already holds — the same set the count
   above the button reports, in the order on screen rather than in the
   window's own slice — so it costs one pass over an array that is already
   built. It adds no endpoint, no round trip and nothing to the wire: every
   field it emits was decoded from `/store.json` when the store was read.

   THE BYTES GO TO THE BROWSER'S DOWNLOAD AND NOWHERE ELSE. `CLAUDE.md` §2
   lists the tracked extensions and `.csv` is not one of them, so a `.csv`
   written into this tree would be CI gate 1 failing the build. A `Blob` and
   an object URL never touch the filesystem this repository is checked out
   on, and the URL is revoked in the same statement pair it is created in.

   EVERY CELL IS THE WIRE FORM. `month` goes out `2024-09` and not `Sep
   2024`, `expiry` goes out `2026-10-29`, `strike` goes out in PAISA and
   `timeframe` goes out `1min` — the store's own spellings, which is what
   makes the file joinable against the store. Only the HEADER row is words.
   That is the rule at the top of `$lib/dates.js` applied to a file instead of
   to a screen: a display form that becomes a key is a key that matches
   nothing.

   A PERCENTAGE GOES OUT AS THE INTEGER BASIS POINTS IT ARRIVED AS, beside
   its reason code. Inserting the decimal point would make it a float, which
   `CLAUDE.md` §7 bans for money and this page has never done at either end;
   and an unknown goes out EMPTY beside its `chg_why`, never as `0`, because
   zero is a real month that closed where it opened.
   ====================================================================== */

/**
 * The columns, in the store's own order, with the wire value for each.
 *
 * `null` becomes the empty field and never `null`, `—`, or a zero: an
 * absence in a data file is an empty cell, and any of the other three would
 * be read back as a value.
 *
 * @type {{ label: string, v: (r: any) => string }[]}
 */
export const CSV_COLS = [
  { label: 'instrument', v: (r) => r.instrument },
  { label: 'month', v: (r) => r.month },
  { label: 'timeframe', v: (r) => r.timeframe },
  { label: 'rows', v: (r) => String(r.rows) },
  { label: 'fullest_in_month', v: (r) => String(r.denom) },
  { label: 'short_by', v: (r) => String(r.short) },
  { label: 'first_ts_micros', v: (r) => (r.firstAt === null ? '' : String(r.firstAt * 1000)) },
  { label: 'last_ts_micros', v: (r) => (r.lastAt === null ? '' : String(r.lastAt * 1000)) },
  { label: 'chg_bps', v: (r) => (r.chg === null ? '' : String(r.chg)) },
  { label: 'chg_why', v: (r) => r.chgWhy ?? '' },
  { label: 'prev_chg_bps', v: (r) => (r.prevChg === null ? '' : String(r.prevChg)) },
  { label: 'prev_chg_why', v: (r) => r.prevChgWhy ?? '' },
  { label: 'expiry', v: (r) => r.expiry ?? '' },
  { label: 'strike_paisa', v: (r) => (r.strike === null ? '' : String(r.strike)) },
  { label: 'side', v: (r) => r.side ?? '' }
];

/* THE BAR GRID EXPORTS BARS, NOT THE CENSUS. Writing instrument-months out
   of a screen showing bars would be a file that does not match the grid it
   was taken from, which is worse than no button. Every value is the store's
   own wire form: paisa integers, raw ISO expiries, integer basis points,
   an EMPTY FIELD for an unknown and never a zero.

   THE TEN UNSOURCED COLUMNS ARE NOT IN THIS FILE. A column of empty fields
   under a `delta` heading is a promise that the number could arrive; the
   reason it cannot is on the grid, where it can be read. */
export const BAR_CSV_COLS = [
  { label: 'instrument', v: (/** @type {any} */ r) => r.instrument },
  { label: 'month', v: (/** @type {any} */ r) => r.month },
  { label: 'timeframe', v: (/** @type {any} */ r) => r.tf },
  { label: 'ts_micros', v: (/** @type {any} */ r) => String(r.ts * 1000) },
  { label: 'ist_day', v: (/** @type {any} */ r) => r.day },
  { label: 'open_paisa', v: (/** @type {any} */ r) => String(r.o) },
  { label: 'high_paisa', v: (/** @type {any} */ r) => String(r.h) },
  { label: 'low_paisa', v: (/** @type {any} */ r) => String(r.l) },
  { label: 'close_paisa', v: (/** @type {any} */ r) => String(r.c) },
  { label: 'volume', v: (/** @type {any} */ r) => String(r.vol) },
  { label: 'chg_bps', v: (/** @type {any} */ r) => (r.chg === null ? '' : String(r.chg)) },
  { label: 'chg_why', v: (/** @type {any} */ r) => r.chgWhy ?? '' },
  { label: 'open_interest', v: (/** @type {any} */ r) => (r.oi === null ? '' : String(r.oi)) },
  { label: 'oi_chg_bps', v: (/** @type {any} */ r) => (r.oichg === null ? '' : String(r.oichg)) },
  { label: 'oi_chg_why', v: (/** @type {any} */ r) => r.oichgWhy ?? '' },
  { label: 'expiry', v: (/** @type {any} */ r) => r.exp ?? '' },
  { label: 'days_to_expiry', v: (/** @type {any} */ r) => (r.dte === null ? '' : String(r.dte)) },
  { label: 'side', v: (/** @type {any} */ r) => r.side ?? '' },
  { label: 'strike_paisa', v: (/** @type {any} */ r) => (r.strike === null ? '' : String(r.strike)) }
];

/**
 * RFC 4180 quoting, and the test is on the four characters that break a
 * field rather than on a guess about which ones might. A quote inside a
 * quoted field is doubled — that is the standard's own escape and not a
 * backslash, which spreadsheets read as a literal backslash.
 *
 * @param {string} s
 */
export function csvQuote(s) {
  return /["\r\n,]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
}
