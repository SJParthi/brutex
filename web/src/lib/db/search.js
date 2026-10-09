/**
 * /db — THE SEARCH BOX'S TWO CONSTANTS
 *
 * The prefix length the census index is cut at, and how many rows the find
 * list draws.
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
 * Owner after WP-00: WP-51 (one search box over tickers, natural names, aliases and expiries).
 *
 * A PLAIN MODULE, NOT A `.svelte.js` ONE: it holds constants and pure
 * functions only, so `node --test` can import it with no compiler. Imports
 * are relative for the same reason — `$lib` is a Vite alias node cannot see.
 */

/* ======================================================================
   THE MONTH WINDOW — a range over the row's own unit
   ----------------------------------------------------------------------
   A ROW HERE IS AN INSTRUMENT-MONTH, so the honest range control is a month
   range and not a day picker: a from-day and a to-day would have to be
   rounded to months to be applied, and a control whose value is silently
   coarsened is a control that lies about what it did.

   THERE IS NO `<input type="date">` ON THIS PAGE AND THERE MUST NOT BE. It
   renders in the OS locale — `02/09/2024`, which is two different days
   depending on the reader — and it cannot be restyled. The endpoints are
   chosen from the months the store ACTUALLY HOLDS, through the same one
   dropdown every other control on this page uses, so the label is
   `Sep 2024` (never `Sept`, per `$lib/dates.js`), the value stays raw
   `YYYY-MM`, and the control cannot offer a month that would match nothing.
   ====================================================================== */

/* ======================================================================
   THE PREFIX INDEX — one Map probe per keystroke
   ----------------------------------------------------------------------
   Every 1..4-character prefix of the full instrument, displayed symbol,
   underlying and month maps to its rows. The complete store index is built
   in yielding chunks once per census. Longer text and selected membership
   intersect a bucket; broad prefixes may still include the whole store.
   ====================================================================== */
export const MAX_PREFIX = 4;

/** A rendering budget, not an authority on what matches. See `findMore`. */
export const FIND_ROWS = 10;
