/**
 * /db — THE BAR GRID'S COLUMNS AND WHAT EACH ONE SAYS
 *
 * Every column of the bar grid with its width and its reason, the groups over
 * them, the read budgets, and the sentence behind every withheld change.
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
   WHAT HAS NO SOURCE, SAID ONCE EACH.

   Every sentence below reaches the screen twice - on the disabled sort
   button's title and on every cell in that column - so the reason cannot
   be on one and missing from the other.
   --------------------------------------------------------------------- */
export const NO_PREMKT =
  'a pre-market percentage needs a pre-open session price, and the store’s bar record carries none: docs/02-store-format.md fixes the record at timestamp, open, high, low, close, volume and open interest, and /bars.json returns exactly those seven fields. There is nothing on this wire to divide.';
export const NO_SPOT_SORT =
  'spot is read for the rows on this page only (/spot.json, D-2807), so there is no order over the whole selection to sort by.';
/* SOURCED SINCE D-2830. The greeks were "no source" because no rate and no
   time to expiry were measured; D-2810 sourced the rate (RBI 91-day T-bill)
   and D-2831 states the expiry instant, and `/greeks.json` solves each bar
   in Rust. Like spot, they are read for the page's rows only. */
export const NO_GREEK_SORT =
  'greeks are solved for the rows on this page only (/greeks.json, D-2830), so there is no order over the whole selection to sort by.';

/* ---------------------------------------------------------------------
   THE BAR GRID'S COLUMNS - all twenty-four, in the approved order.

   `none` is the reason a column has no source. A column with `none` set
   renders a NAMED dash in every cell and its sort button is DISABLED
   wearing that same sentence: a sort control over a column of unknowns
   reorders nothing while claiming to have worked.
   --------------------------------------------------------------------- */
export const BAR_COLS = [
  /* THE DAY AND THE MINUTE ARE TWO COLUMNS, because a grid is read DOWN a
     column. One `18 Aug 2026, 15:29` cell put every row's minute at a
     different x — the day in front of it is a different width on the 9th and
     the 18th — so the one field a reader scans a bar table for could not be
     scanned at all. Sorting still keys on `ts`, the instant, so splitting the
     DISPLAY changes no order. */
  {
    key: 'ts',
    label: 'Date',
    w: 116,
    why: 'the bar’s own opening day, IST - the store holds the instant in microseconds'
  },
  {
    key: 'tod',
    label: 'Time',
    w: 84,
    why: 'the bar’s own opening minute, IST - to the SECOND on a 1s rung, where sixty bars share each minute; absent on a day rung, where every bar opens at the same one. Sorting orders every bar of the query by its IST time of day, on the server (DB-006)'
  },
  {
    key: 'tf',
    label: 'TF',
    w: 68,
    why: 'the bar length; a page is read at one rung, so every row carries the same one'
  },
  { key: 'o', label: 'Open', w: 104, num: true, why: 'paisa integer, straight from the store' },
  { key: 'h', label: 'High', w: 104, num: true, why: 'paisa integer, straight from the store' },
  { key: 'l', label: 'Low', w: 104, num: true, why: 'paisa integer, straight from the store' },
  { key: 'c', label: 'Close', w: 108, num: true, why: 'paisa integer, straight from the store' },
  {
    key: 'vol',
    label: 'Volume',
    w: 116,
    num: true,
    why: 'SHARES traded inside this bar — for an option or a future, shares of the underlying (lots × lot size), not lots; no lot is shown, because no lot size is served for a contract (D-3343). An index has no traded volume and reads n/a (D-3322)'
  },
  { key: 'premkt', label: 'Pre-mkt %', w: 102, num: true, none: NO_PREMKT },
  {
    key: 'chg',
    label: 'Change %',
    w: 102,
    num: true,
    why: 'this bar’s close against the PREVIOUS STORED BAR’s close of its series, wherever it lies — another month, another day, outside the date window — in integer basis points; refused by name where that bar is not the comparable one (D-3325)'
  },
  {
    key: 'oi',
    label: 'Open interest',
    w: 124,
    num: true,
    why: 'the stored open interest, in SHARES of the underlying, not lots (D-3343). n/a on an index or a share, which has none (D-3322); i64::MIN is the null sentinel and renders as unknown; a zero is a real zero'
  },
  {
    key: 'oichg',
    label: 'OI chg %',
    w: 100,
    num: true,
    why: 'this bar’s open interest against the previous stored bar’s, in integer basis points'
  },
  {
    key: 'exp',
    label: 'Expiry',
    w: 116,
    why: 'the contract’s expiry day, parsed from the stored name'
  },
  {
    key: 'dte',
    label: 'Days to exp',
    w: 106,
    num: true,
    why: 'whole calendar days from this bar’s IST day to the expiry day - negative after it'
  },
  { key: 'side', label: 'Type', w: 72, why: 'CE or PE, from the stored name' },
  { key: 'strike', label: 'Strike', w: 96, num: true, why: 'paisa integer, from the stored name' },
  /* `num: true` — MEASURED, NOT ASSUMED. Every neighbour on this side of the
     table carries it (`dte`, `strike`, `intr`, `extr`, `iv`, and the four
     Greeks) and this one did not, so its HEADER rendered left-aligned over a
     CELL that rendered right: the only column on the table whose label and
     value face opposite edges. Found by reading the computed styles of all
     twenty-four columns rather than by looking, which is also how the two
     "clipped" columns turned out not to be clipped at all. */
  /* SOURCED SINCE D-2807, PER BAR. Spot is the underlying's stored close at
     this bar's own instant, feed and rung (`/spot.json`), read for the rows
     on THIS PAGE only — so no order over the whole selection exists to sort
     by, and the sort button says so rather than ordering one page. */
  {
    key: 'mny',
    label: 'Moneyness',
    w: 106,
    num: true,
    nosort: NO_SPOT_SORT,
    why: 'ATM, ITM-k or OTM+k: k strikes of this expiry from the strike nearest the underlying’s close at this bar’s own instant, counted over the strikes stored by then (/chain.json, D-3329); a refusal shows its reason word'
  },
  {
    key: 'intr',
    label: 'Intrinsic',
    w: 100,
    num: true,
    nosort: NO_SPOT_SORT,
    why: 'the underlying’s close at this bar’s instant over the strike (CE) or the strike over it (PE), never below zero, in paisa'
  },
  {
    key: 'extr',
    label: 'Extrinsic',
    w: 100,
    num: true,
    nosort: NO_SPOT_SORT,
    why: 'this bar’s close less intrinsic, in paisa; not clipped, so a close below intrinsic shows negative'
  },
  /* SOLVED PER BAR, ON THE SERVER (D-2830). Each figure is
     `pull::pricing::bar_greeks` on this bar's close, the underlying's close
     at the same instant, the time to the expiry day's session close (D-2831)
     and the T-bill rate as of the release before this bar's day (D-2810).
     Zero carry: no dividend yield is sourced. Units are in each `why`. */
  {
    key: 'iv',
    label: 'IV %',
    w: 86,
    num: true,
    nosort: NO_GREEK_SORT,
    why: 'the Black-Scholes volatility that reproduces this bar’s close, in percent, solved on the server'
  },
  {
    key: 'delta',
    label: 'Delta',
    w: 86,
    num: true,
    nosort: NO_GREEK_SORT,
    why: 'change in premium per unit change in the underlying, at the solved volatility'
  },
  {
    key: 'gamma',
    label: 'Gamma',
    w: 96,
    num: true,
    nosort: NO_GREEK_SORT,
    why: 'change in delta per rupee of the underlying'
  },
  {
    key: 'theta',
    label: 'Theta',
    w: 88,
    num: true,
    nosort: NO_GREEK_SORT,
    why: 'rupees of premium per calendar day (365-day year), negative when time decays it'
  },
  {
    key: 'vega',
    label: 'Vega',
    w: 86,
    num: true,
    nosort: NO_GREEK_SORT,
    why: 'rupees of premium per one percentage point of volatility'
  },
  {
    key: 'rho',
    label: 'Rho',
    w: 84,
    num: true,
    nosort: NO_GREEK_SORT,
    why: 'rupees of premium per one percentage point of the risk-free rate'
  }
];
/** By key, so a cell reaches its own column's reason without an index. */
export const BAR_META = (() => {
  /** @type {Record<string, any>} */
  const m = {};
  for (const c of BAR_COLS) m[c.key] = c;
  return m;
})();

export const OPTION_COLS = new Set([
  'side',
  'strike',
  'mny',
  'intr',
  'extr',
  'iv',
  'delta',
  'gamma',
  'theta',
  'vega',
  'rho'
]);
export const FUTURE_COLS = new Set(['exp', 'dte']);
/* ------------------------------------------------------------------
   FIVE GROUPS, AND THEY ARE THE SCHEMA'S OWN SHAPE, NOT A DECORATION.

   Twenty-five columns is a wall. Nothing in it says that Open/High/Low/Close
   are one reading of one bar, or where the contract facts stop and the
   greeks begin, so every column costs the same amount of looking as every
   other one. The grouping below invents nothing: each group is a CONTIGUOUS
   RUN of `BAR_COLS` in the order that list already had —

     When      1..3    ts, tod, tf
     Price     4..7    o, h, l, c
     Flow      8..12   vol, premkt, chg, oi, oichg
     Contract  13..19  exp, dte, side, strike, mny, intr, extr
     Greeks    20..25  iv, delta, gamma, theta, vega, rho

   — and that contiguity is why a band can be drawn over them at all. A group
   that had to skip a column could not be one heading.

   THE BAND IS ONE CELL PER COLUMN, NOT `colspan`. Columns are hidden by
   `display: none` at fixed `nth-child` positions — see the long note over
   those rules for why hiding beats filtering here — and a `colspan` counts
   SLOTS, not visible columns, so a spanning cell would drift the moment a
   group lost a member. One `<th>` per column, carrying the label on the
   group's first VISIBLE column and nothing on the rest, cannot disagree with
   the grid. The existing hide rules match `thead > tr > th:nth-child(N)` in
   ANY row of the head, so they gate this row for free.
   ------------------------------------------------------------------ */
export const BAR_GROUPS = [
  { id: 'when', label: 'When', keys: ['ts', 'tod', 'tf'] },
  { id: 'price', label: 'Price', keys: ['o', 'h', 'l', 'c'] },
  { id: 'flow', label: 'Flow', keys: ['vol', 'premkt', 'chg', 'oi', 'oichg'] },
  {
    id: 'contract',
    label: 'Contract',
    keys: ['exp', 'dte', 'side', 'strike', 'mny', 'intr', 'extr']
  },
  { id: 'greeks', label: 'Greeks', keys: ['iv', 'delta', 'gamma', 'theta', 'vega', 'rho'] }
];


/* EVERY COLUMN IS IN EXACTLY ONE GROUP, AND ONE ADDED LATER WITHOUT A GROUP
   IS FINDABLE RATHER THAN SILENT. A new key nobody grouped would carry no
   band label, and the band would read as a gap the eye blames on the layout
   rather than on the column list. The table publishes any such key on
   `data-ungrouped` — the same debug-attribute convention `data-boxh` and
   `data-fit` already earned on this grid — so the answer is one inspection
   away instead of a puzzle. `CLAUDE.md` §4: degrade loudly, name the reason. */
/**
 * The keys of `cols` no group in `groups` names, in column order.
 * @param {{key: string}[]} cols @param {{keys: string[]}[]} groups
 */
export function ungrouped(cols, groups) {
  const named = new Set(groups.flatMap((g) => g.keys));
  return cols.filter((c) => !named.has(c.key)).map((c) => c.key);
}
export const BAR_UNGROUPED = ungrouped(BAR_COLS, BAR_GROUPS);


/** A PARTITION, AND IT SUMS: sourced + unsourced = every column drawn. */
export const BAR_SOURCED = BAR_COLS.filter((c) => !c.none).length;
export const BAR_UNSOURCED = BAR_COLS.length - BAR_SOURCED;

/* THE READ BUDGET (`READ_BUDGETS`, `budgetLabel`) WENT IN R3-03: the control
   left with the per-month reads (D-3326) and nothing read either name since. */

/**
 * Why one bar's percentage is not a number. One code, one sentence — EVERY
 * code the server can send (D-3325; `web/tests/bar-why.test.js` reads them out
 * of `bars.rs` and `bars_change.rs`). Each one is a fact about the bar, never a
 * zero and never a blank.
 */
export const BAR_WHY = {
  no_earlier_stored_bar:
    'no earlier bar of this series is stored at all, so there is nothing to measure it against: this is the first stored bar of the series (for a GDFL contract, its first bar in an archive that starts on 3 Sep 2018). It is not a zero.',
  earlier_month_not_read:
    'the bar before this one is in an earlier month the request was told not to read (lookback off). It is not a zero.',
  first_bar_in_window:
    'the bar before this one lies outside the date or time window the request was told to stay inside (lookback off). It is not a zero.',
  /* KEPT FOR AN ANSWER CACHED BEFORE D-3325: the server no longer sends it. */
  first_bar_in_file:
    'this was the first bar of the month the request read; the server no longer answers this way (D-3325), so this answer predates it — read again for the change against the previous stored bar.',
  previous_close_zero: 'the previous close is zero paisa, and a ratio against zero is not a number.',
  contract_adjusted:
    'a corporate action re-based this contract at its ex-date, between the previous bar and this one (the import’s lineage ledger, D-3331), so the two prices are not on one basis.',
  corporate_action_unverified:
    'this is the first bar of a session on a series a corporate action can re-base — a share, or an option or future on one — and no adjustment is verified, so an overnight move is refused rather than shown as a possibly fake crash (D-0018, D-3325).',
  previous_session_not_adjacent:
    'the previous stored bar is from a session earlier than the one immediately before this bar’s, so sessions are missing between them and a change across the hole would not be a bar-to-bar move.',
  halt_suspected:
    'a suspected trading halt lies between the previous bar and this one (D-3320), so the move is not one of continuous trading.',
  phase_boundary:
    'the previous bar is in a different phase of the session — pre-open, continuous, closing auction — so the two prices are not comparable bar to bar (D-3320).',
  previous_bar_not_adjacent:
    'on this index rung the previous stored bar is not the previous bucket — minutes are missing between them — so the change would span a hole.',
  vendor_oi_unadjusted:
    'the vendor carried this contract’s open interest unscaled past its ex-date (D-3331), so it is not comparable with the bar before.',
  not_a_derivative:
    'not applicable: an index or a share has no open interest, on any feed (D-3322). A Zerodha month stores 0 here, which is not a measurement; it is not shown as one.',
  oi_null:
    'open interest on this bar is the store’s null sentinel, i64::MIN — this feed stamps none for this series. It is NOT zero; a zero here would be a real zero.',
  oi_null_before:
    'the previous bar carries no open interest, so there is nothing to measure this one against.',
  previous_oi_zero: 'the previous open interest is zero, and a ratio against zero is not a number.',
  oi_to_zero:
    'open interest printed exactly 0 straight after a non-zero value, so the change is withheld rather than shown as −100%: GDFL carries lone zero prints between non-zero ones (NIFTY 28 Mar 2019 11,600 CE: 8,025, 0, 8,325 on consecutive bars), and which zeros are real closes is UNVERIFIED. The 0 itself is shown as stored.',
  previous_unreadable:
    'the bar before this one failed its checksum and was not read, so there is no trusted value to measure this one against. Its own value is shown; only the change is withheld.',
  overflow:
    'the change does not fit the integer range the server measures in, so it is withheld rather than shown wrapped. Both values are real.'
};
/** @param {string | null} code */
export const barWhyText = (code) =>
  /** @type {any} */ (BAR_WHY)[String(code)] ??
  'unknown, and this page did not record why - which is a defect here, not a fact about the store.';

/* ---------------------------------------------------------------------
   R3-03 — WHAT THE GRID OFFERS, AND WHAT IT SAYS WHEN IT CANNOT.
   --------------------------------------------------------------------- */

/** The segment a census key names, its second token: `INDEX`, `CASH` or `FNO`. @param {string} key */
export const segmentOfKey = (key) => String(key).split('-')[1] ?? '';

export const NO_VOLUME_SORT =
  'an index has no traded volume (D-2802, D-3322): every row of this selection reads n/a, so there is nothing to order — the rows would come back in time order under a Volume caret.';
export const NO_OI_SORT =
  'an index or a share has no open interest (D-3322): every row of this selection reads n/a, so there is nothing to order — the rows would come back in time order under this caret.';

/**
 * WHETHER A SORT MEANS SOMETHING ON THIS SELECTION (R3-03): `null` when it
 * does, else the reason its button is disabled.
 *
 * Volume on an index and open interest on an index or a share are n/a on
 * every row, by the server's own rule (`bars_change.rs`: an `INDEX` bar has no
 * traded volume, an `INDEX` or `CASH` bar no open interest, D-3322). Ordering
 * them answered time order wearing a Volume caret (the 11 Oct re-check, NIFTY
 * spot · Zerodha 1min). A selection with any derivative keeps the sort.
 *
 * @param {string} key the grid's column key @param {string[]} keys the selection's series
 * @returns {string | null}
 */
export function sortRefusal(key, keys) {
  if (keys.length === 0) return null;
  const segs = keys.map(segmentOfKey);
  if (key === 'vol' && segs.every((s) => s === 'INDEX')) return NO_VOLUME_SORT;
  if ((key === 'oi' || key === 'oichg') && segs.every((s) => s === 'INDEX' || s === 'CASH')) return NO_OI_SORT;
  return null;
}

/**
 * WHAT TO DO WHEN THE BROWSER WILL NOT ORDER A COLUMN (R3-03, DB-B50): advice
 * that WORKS for this rung and this many series.
 *
 * The old sentence told every reader to "narrow to a day". A whole IST day
 * holds at most `86,400 / rung seconds` bars of one series — 1,440 one-minute
 * bars, but 86,400 one-second bars, past the 20,000 the browser orders — so
 * on a one-second series the advice could not work. A day is offered only
 * where a day of every series fits; otherwise the time of day it must be
 * narrowed to as well is named in minutes. Every figure is arithmetic on the
 * rung and the budget; no session length is assumed.
 *
 * @param {{label: string, total: number, budget: number, tfSecs: number, series: number, stored: string[]}} o
 */
export function localSortAdvice({ label, total, budget, tfSecs, series, stored }) {
  const n = Math.max(1, series);
  const perDay = n * Math.ceil(86_400 / Math.max(1, tfSecs));
  const ways = [];
  if (perDay <= budget) {
    ways.push(`narrow to one day (a day holds at most ${fmt(perDay)} bar(s) of ${n === 1 ? 'this series' : `these ${fmt(n)} series`} at this rung)`);
  } else {
    const minutes = Math.floor(budget / (n * Math.max(1, Math.ceil(60 / Math.max(1, tfSecs)))));
    ways.push(
      minutes >= 1
        ? `narrow to one day AND a time of day of at most ${fmt(minutes)} minute(s) — a whole day of ${n === 1 ? 'this series' : `these ${fmt(n)} series`} can hold ${fmt(perDay)} bar(s) at this rung, so a day alone is not enough`
        : `choose fewer series — one minute of these ${fmt(n)} series is already past it`
    );
  }
  if (n > 1) ways.push(`choose one strike or one side (${fmt(n)} series now)`);
  ways.push(`or sort by a stored column (${stored.join(', ')}), which the server orders over any number`);
  return `${label} cannot be ordered here: this query keeps ${fmt(total)} bar(s), and the browser orders at most ${fmt(budget)} of a column the server cannot sort. The grid keeps the order it had. To order by ${label}: ${ways.join('; ')}.`;
}

/**
 * WHY A PAGE OF BARS IS NOT ON SCREEN, IN WORDS THAT ARE TRUE ABOUT WHO SAID
 * IT (R3-03).
 *
 * The grid called every missing page "a failed or refused read" and "the
 * server's own" sentence, with Try again under it — also when the PAGE had
 * refused the selection before asking anything (too many series, two rungs, a
 * window outside the held months), where no server had spoken and pressing
 * Try again asked the same impossible question. Three cases now:
 *
 * - `page`: this page refused before any request; nothing was read; the fix
 *   is to change the selection, so no Try again.
 * - `server`: the server answered 4xx with its reason; the same question gets
 *   the same answer, so no Try again.
 * - `failed`: no answer, a 5xx, a 408/429 past its retries, or an answer
 *   that did not check; asking again can help, so Try again is offered.
 *
 * @param {{made: 'page' | 'read', status: number}} o
 * @returns {{kind: 'page' | 'server' | 'failed', title: string, says: string, retry: boolean}}
 */
export function readFailure({ made, status }) {
  if (made === 'page') {
    return {
      kind: 'page',
      title: 'This selection is not read as one page',
      says: 'This page refused it before asking the server, so nothing was read and no store was consulted. Change the selection as the sentence above says.',
      retry: false
    };
  }
  if (status >= 400 && status < 500 && status !== 408 && status !== 429) {
    return {
      kind: 'server',
      title: 'The server refused this page',
      says: `The sentence above is the server's own (HTTP ${status}). Asking again asks the same question, so it is not offered: change what it names.`,
      retry: false
    };
  }
  return {
    kind: 'failed',
    title: 'This page of bars could not be read',
    says: status !== 0 && status !== 200
      ? `The server answered HTTP ${status} and no page. Nothing is drawn rather than a plausible blank; asking again may answer.`
      : 'No answer arrived, or the answer did not check. Nothing is drawn rather than a plausible blank; asking again may answer.',
    retry: true
  };
}
