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
  // THE COVERAGE VERDICT IS THE CALENDAR'S (WP-54, D-3336): what the
  // calendar of record owes the row at its rung, what is missing, and the
  // state the grid shows. The peer "fullest in month" is no longer a verdict.
  { label: 'owed_by_calendar', v: (r) => (Number.isFinite(r.cover?.owed) ? String(r.cover.owed) : '') },
  { label: 'missing_by_calendar', v: (r) => (Number.isFinite(r.cover?.missing) ? String(r.cover.missing) : '') },
  { label: 'coverage_state', v: (r) => r.cover?.state ?? '' },
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

/* THE BAR GRID EXPORTS BARS, NOT THE CENSUS — WITH THE GRID'S OWN COLUMNS.
   Writing instrument-months out of a screen showing bars would be a file that
   does not match the grid it was taken from. Every value is the wire form:
   paisa integers, raw ISO expiries, integer basis points, an EMPTY FIELD for
   an unknown beside its reason code, and never a zero.

   THE FEED IS THE FIRST COLUMN AND THE OPTION ANALYTICS ARE IN IT (DB-047,
   DB-G101, D-3326). The file used to carry no feed — a Zerodha and a GDFL
   export of NSE-INDEX-NIFTY were indistinguishable once renamed — and left
   out spot, moneyness, intrinsic, extrinsic, IV and the five greeks under a
   comment calling them "the ten unsourced columns", stale since D-2807 and
   D-2830 sourced them. They are written here exactly as the grid shows them:
   the underlying's close at the bar's own instant on its own feed, and the
   server's solve — or the reason each is not a number.

   Each column reads the row and its context: `{ feed, oc, gc }`, the feed
   and the row's option cells (`optionCells`) and greek cell (`greekCell`),
   `null` on a row that is not an option. */
/** @typedef {{ feed: string, oc: any, gc: any }} BarCsvContext */
/** @type {{ label: string, v: (r: any, x: BarCsvContext) => string }[]} */
export const BAR_CSV_COLS = [
  { label: 'feed', v: (_r, x) => x.feed },
  { label: 'instrument', v: (r) => r.instrument },
  { label: 'month', v: (r) => r.month },
  { label: 'timeframe', v: (r) => r.tf },
  { label: 'ts_micros', v: (r) => String(r.ts * 1000) },
  { label: 'ist_day', v: (r) => r.day },
  { label: 'ist_time', v: (r) => istClock(r.ts) },
  { label: 'open_paisa', v: (r) => String(r.o) },
  { label: 'high_paisa', v: (r) => String(r.h) },
  { label: 'low_paisa', v: (r) => String(r.l) },
  { label: 'close_paisa', v: (r) => String(r.c) },
  /* AN INDEX'S VOLUME IS NOT A MEASUREMENT (D-3322): empty, with the reason. */
  { label: 'volume_shares', v: (r) => (r.vWhy ? '' : String(r.vol)) },
  { label: 'volume_why', v: (r) => r.vWhy ?? '' },
  { label: 'chg_bps', v: (r) => (r.chg === null ? '' : String(r.chg)) },
  { label: 'chg_why', v: (r) => r.chgWhy ?? '' },
  { label: 'open_interest_shares', v: (r) => (r.oi === null ? '' : String(r.oi)) },
  { label: 'open_interest_why', v: (r) => r.oiWhy ?? '' },
  { label: 'oi_chg_bps', v: (r) => (r.oichg === null ? '' : String(r.oichg)) },
  { label: 'oi_chg_why', v: (r) => r.oichgWhy ?? '' },
  { label: 'expiry', v: (r) => r.exp ?? '' },
  { label: 'days_to_expiry', v: (r) => (r.dte === null ? '' : String(r.dte)) },
  { label: 'side', v: (r) => r.side ?? '' },
  { label: 'strike_paisa', v: (r) => (r.strike === null ? '' : String(r.strike)) },
  { label: 'spot_paisa', v: (_r, x) => (x.oc?.spot === null || x.oc?.spot === undefined ? '' : String(x.oc.spot)) },
  { label: 'moneyness', v: (_r, x) => x.oc?.mny ?? '' },
  { label: 'intrinsic_paisa', v: (_r, x) => (x.oc?.intr === null || x.oc?.intr === undefined ? '' : String(x.oc.intr)) },
  { label: 'extrinsic_paisa', v: (_r, x) => (x.oc?.extr === null || x.oc?.extr === undefined ? '' : String(x.oc.extr)) },
  { label: 'spot_why', v: (_r, x) => (x.oc ? (x.oc.mny === null ? (x.oc.mnyWhy ?? x.oc.why ?? '') : (x.oc.why ?? '')) : '') },
  /* THE SERVER'S FIGURES AS IT SENT THEM: IV as a fraction (0.1834, not
     18.34), the greeks in the units their columns name on the grid. */
  { label: 'iv', v: (_r, x) => num(x.gc?.iv) },
  { label: 'delta', v: (_r, x) => num(x.gc?.delta) },
  { label: 'gamma', v: (_r, x) => num(x.gc?.gamma) },
  { label: 'theta', v: (_r, x) => num(x.gc?.theta) },
  { label: 'vega', v: (_r, x) => num(x.gc?.vega) },
  { label: 'rho', v: (_r, x) => num(x.gc?.rho) },
  { label: 'greeks_why', v: (_r, x) => (x.gc ? (x.gc.why ?? '') : '') }
];

/** A server figure as written, or the empty field. @param {unknown} v */
function num(v) {
  return typeof v === 'number' && Number.isFinite(v) ? String(v) : '';
}

/** `HH:MM:SS` in IST of an epoch-millisecond instant. @param {number} ms */
export function istClock(ms) {
  const s = Math.floor(ms / 1000) + 19_800;
  const d = ((s % 86_400) + 86_400) % 86_400;
  const hh = String(Math.floor(d / 3600)).padStart(2, '0');
  const mm = String(Math.floor((d % 3600) / 60)).padStart(2, '0');
  const ss = String(d % 60).padStart(2, '0');
  return `${hh}:${mm}:${ss}`;
}

/**
 * The bar CSV's text: a header of words, then one record per row, CRLF.
 * @param {any[]} rows @param {(r: any) => BarCsvContext} context
 */
export function barCsvText(rows, context) {
  const lines = [BAR_CSV_COLS.map((c) => csvQuote(c.label)).join(',')];
  for (const r of rows) {
    const x = context(r);
    lines.push(BAR_CSV_COLS.map((c) => csvQuote(c.v(r, x))).join(','));
  }
  return `${lines.join('\r\n')}\r\n`;
}

/**
 * THE FILE IS NAMED FOR THE ROWS IT HOLDS (DB-047). It used to be named for
 * every matched month — `…_2023-01_2026-09.csv` over eight rows of one day.
 * Now: the feed, the rung, the instrument (or `selection`), the first and
 * last IST day the written rows hold, and how many rows that is — built from
 * keys only, so two exports of the same rows share a name on every machine.
 *
 * @param {string} feed @param {string} tf @param {string} what @param {any[]} rows
 */
export function barCsvName(feed, tf, what, rows) {
  let lo = '';
  let hi = '';
  for (const r of rows) {
    if (lo === '' || r.day < lo) lo = r.day;
    if (hi === '' || r.day > hi) hi = r.day;
  }
  const safe = String(what || 'selection').replace(/[^A-Za-z0-9._-]+/g, '-');
  return `brutex-db-bars_${feed || 'no-feed'}_${tf || 'all-rungs'}_${safe}_${lo || 'none'}_${hi || 'none'}_${rows.length}rows.csv`;
}

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
