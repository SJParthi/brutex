/**
 * /db — WHAT THE COVERAGE GRID SAYS ABOUT ITSELF
 *
 * The census grid's columns, the sentence behind every withheld % change, and
 * the sentence behind a row that is its own denominator.
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
 * Owner after WP-00: WP-54 (coverage, coverage calendar and day flags).
 *
 * A PLAIN MODULE, NOT A `.svelte.js` ONE: it holds constants and pure
 * functions only, so `node --test` can import it with no compiler. Imports
 * are relative for the same reason — `$lib` is a Vite alias node cannot see.
 */

/* ---- THE PERCENTAGE COLUMNS ----------------------------------------
   `/store.json` sends INTEGER BASIS POINTS. 125 is +1.25%. It is never a
   float on the wire, because `CLAUDE.md` §7 bans the float for money and a
   ratio derived from money is the same arithmetic — see the handler's own
   comment, which promised this shape for a long time before D-0069 made it
   true. The decimal point is inserted HERE, for display, exactly as it is
   for prices.
   ==================================================================== */

/**
 * Basis points as a signed percentage with a fixed two decimals.
 *
 * THE DIVISION IS EXACT AND IT IS NOT `bps / 100`. The remainder is taken
 * with `%` — an integer operation — and the quotient is a multiple of 100
 * divided by 100, which IEEE returns exactly for every value inside 2^53.
 * A naive `(bps / 100).toFixed(2)` is a float rounding of a value that has an
 * exact decimal expansion, and it would be the one place on this page where a
 * price-derived number went through a float for no reason at all.
 *
 * TWO DECIMALS ALWAYS, AND THE SIGN IS PART OF THE NUMBER. Adaptive precision
 * moves the decimal point from row to row; a `+` that only appears on losses'
 * mirror image makes a column of gains read as a column of magnitudes. Zero
 * carries no sign because it has no direction — the cell is neutral-coloured
 * and says `0.00%`, which is a real answer and not an unknown.
 *
 * @param {number} bps INTEGER basis points off the wire; 125 is +1.25%
 */
// `bpsText` and `dirOf` live in `$lib/bps.js` beside `basisPoints`, because
// the three are one rule: what a ratio rounds to, how it reads, and what
// colour it earns. The `-0` note there explains why this formatter is safe
// only while that producer rounds away from zero.

/** up / down / flat / none — the direction green and red are reserved for.
    `none` is its own word rather than `flat`, because a cell nobody can
    compute and a month that did not move are different facts and must not
    share a colour. */
/** @param {number | null} bps */


/**
 * WHY a percentage is a dash. One sentence per server reason code.
 *
 * `CLAUDE.md` §4 — degrade loudly and NAME THE REASON. A single "n/a" over
 * five distinct facts is the fallback that hides a failure: "this equity can
 * never be shown until a threshold is sourced" and "re-ingest this month and
 * it will appear" are opposite instructions, and one dash cannot say both.
 *
 * An unrecognised code is reported as an unrecognised code, WITH the code.
 * Guessing a sentence for it would be this page inventing a fact about the
 * server, which is the exact failure the map exists to prevent.
 */
export const WHY = {
  corporate_action_unverified:
    'REFUSED, not missing. This instrument can split, bonus or consolidate, and no corporate-action threshold is sourced anywhere in this repository — docs/00-charter.md names no verified split-and-bonus feed. An unadjusted 1:5 split reads as an 80% overnight crash that is indistinguishable from a real move, so docs/05-decisions.md D-0018 refuses the window loudly rather than printing it. Indices never split and do show a number. This cell stays a dash until an operator supplies a threshold with a source.',
  not_recorded:
    'The census holds this month and no close was ever read for it — a manifest version-1 entry, or a version-2 one written before its bar file was priced (D-0067). Not zero, and not a price of zero: nobody has looked. Re-ingesting the month records both closes and this fills in.',
  no_earlier_month:
    'This feed holds no bars for the month immediately before this one, so there is no previous change to show. The first month an instrument holds is the ordinary case; a hole in the middle is the other one. It is unknown, never 0.00%.',
  base_not_positive:
    "The month's first close is zero paisa, so the ratio has no base and is undefined. Zero means zero here (CLAUDE.md §7), so this is a real stored price and not a missing one — but nothing can be divided by it.",
  overflow:
    'The move times 10,000 leaves a 64-bit integer, so the server refused it rather than wrapping it into a plausible small number.'
};

/**
 * THE PROBE IS WIDENED AT THE LOOKUP, NOT AT THE TABLE. `WHY` keeps the
 * five-key shape it is written in — that is the fact this page holds, and
 * narrowing it to an index signature would let a sixth key be added above
 * and read below with nothing noticing the sentence was never written. The
 * CODE, by contrast, comes off the wire and may be anything at all, which is
 * the entire reason the fallback line exists. So the widening happens
 * exactly where the unknown key is used, and `undefined` in the cast is what
 * makes the `??` a real branch rather than dead code. `barWhyText` does the
 * same for the bar grid's five reasons.
 *
 * `null` is the ordinary case here and not an error: `chg_why` is null on
 * every row that carries a NUMBER, and this sentence is only rendered where
 * the number is absent.
 *
 * @param {string | null} code
 */
export const whyText = (code) =>
  (code === null ? null : /** @type {Record<string, string | undefined>} */ (WHY)[code]) ??
  `The server named a reason this page does not have a sentence for: ${code ?? 'none given'}. That is a mismatch between this page and /store.json, not a fact about the data.`;

/** What the page says instead of a verdict it cannot support. */
export const SOLE_WHY =
  'This is the only instrument-month stored at this rung for this month, so the denominator is this row itself. ' +
  'Being short of itself by zero is a tautology, not a measurement: nothing here says the month is complete, and nothing says it is not. ' +
  'Pull a second instrument for this month, or read the Days column beside it.';

/* ======================================================================
   THE TWO PERCENTAGE COLUMNS — WHAT THEY ARE, AND WHY MOST CELLS ARE A DASH
   ----------------------------------------------------------------------
   Every number in this panel is COUNTED FROM THE ROWS ON SCREEN'S OWN
   DATASET, never typed in. The previous version of this block asserted that
   `/store.json` carries no price, which was true when it was written and
   false the day D-0069 landed — a hardcoded claim about the server outliving
   the server. Counting means the panel cannot say "0 refused" over a table
   full of dashes, or the reverse.
   ====================================================================== */

/** Bytes per bar on `/bars.json`, MEASURED: 668,251 bytes for 8,250 bars.
    A constant because it is a property of the wire format, not of the store;
    everything multiplied by it below is counted from the live data instead
    of typed in, so this panel cannot drift into asserting a row count the
    page beside it contradicts. */
export const BYTES_PER_BAR = 81.0;

/**
 * Bytes, at the magnitude that reads as a quantity rather than as `0.0`.
 *
 * @param {number} n
 */
export function bytesText(n) {
  if (n >= 1e9) return `${(n / 1e9).toFixed(1)} GB`;
  if (n >= 1e6) return `${(n / 1e6).toFixed(1)} MB`;
  if (n >= 1e3) return `${(n / 1e3).toFixed(1)} kB`;
  return `${Math.round(n)} B`;
}

/* ======================================================================
   SORT
   ----------------------------------------------------------------------
   Codepoint comparison, not `localeCompare`: instruments are `[A-Z0-9-_&]`
   by construction, so collation adds nothing and would make the order a
   function of the operator's locale. Same inputs, same order, everywhere.
   ====================================================================== */
export const COLS = [
  { key: 'instrument', label: 'Instrument', num: false },
  { key: 'month', label: 'Month', num: false },
  { key: 'timeframe', label: 'TF', num: false },
  { key: 'rows', label: 'Bars', num: true },
  { key: 'days', label: 'Sessions', num: true },
  { key: 'short', label: 'Short by', num: true },
  { key: 'pct', label: 'Completeness', num: true },
  // THESE TWO SORT NOW. They used to carry `key: null` with the comment "a
  // control that reorders 20,516 identical blanks is a control that lies
  // about having data" — which was right while they were blank and is wrong
  // now that they hold basis points. The rule the old comment states is
  // unchanged: EVERY COLUMN THAT HOLDS DATA SORTS. These hold data.
  {
    key: 'chg',
    label: '% Change',
    num: true,
    why: "this month's first close to its last close, in basis points"
  },
  {
    key: 'prevChg',
    label: 'Prev % Change',
    num: true,
    why: "the previous month's % change for the same instrument"
  }
];
