/**
 * /db — RUNG LENGTHS, SESSION SIZE AND THE TWO FORMATTERS EVERY READER USES
 *
 * The bar-length ladder (`TF_SECS`), the session size the completeness column
 * divides by (`BARS_PER_SESSION`), the label under the Timeframe control
 * (`tfNote`), the number formatter (`fmt`) and the three-way comparator (`txt`).
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
 * Owner after WP-00: WP-55 (price-basis and close-semantics labels; it fixes the `BARS_PER_SESSION` comment).
 *
 * A PLAIN MODULE, NOT A `.svelte.js` ONE: it holds constants and pure
 * functions only, so `node --test` can import it with no compiler. Imports
 * are relative for the same reason — `$lib` is a Vite alias node cannot see.
 */
import { exact } from '../money.js';

/**
 * A session is 375 one-minute bars, 09:15–15:29 inclusive.
 *
 * NSE's Closing Auction Session moved the close: from 2026-08-03 the
 * continuous session ends at 15:29. Stated because a completeness figure with
 * an unstated denominator is a number nobody can check.
 */
export const BARS_PER_SESSION = 375;

/**
 * Bars one session holds **at the rung the row is stored at**.
 *
 * # The defect this removes
 *
 * `BARS_PER_SESSION` is 375 because that is how many ONE-MINUTE bars a
 * session holds. It was applied to every row regardless of rung, and the
 * server compounded it by stamping `"timeframe":"1m"` on every census row —
 * so a complete 1-day month, 21 bars for 21 trading days, was divided by 375
 * and reported as `0.06 sessions observed`, i.e. ~99.7% missing. The store
 * was right; both readers were wrong in the same direction.
 *
 * A DAILY session is ONE bar by definition. That is not an approximation:
 * the rung IS the session for `1day`.
 *
 * A rung this function has no number for returns `null`, and every caller
 * renders a dash rather than a figure — an unknown denominator produces no
 * percentage at all, never a plausible one. The same rule the % change
 * columns already follow.
 *
 * @param {string | undefined} tf a rung name as the census spells it
 * @returns {number | null} `null` means NO RECORDED SESSION SIZE, never zero
 */
export const barsPerSession = (tf) => {
  if (tf === '1min' || tf === '1m') return BARS_PER_SESSION;
  if (tf === '1day') return 1;
  return null;
};

/**
 * THE BAR-LENGTH LADDER — a rung's own length in SECONDS.
 *
 * # Why a number and not a list position
 *
 * `store::path::Timeframe` is `{ secs: u32, name: &'static str }`, and the
 * seconds are the field that MEANS something: `1min` is 60 and `1day` is
 * 86,400 because those are the numbers in `crates/store/src/path.rs`, not
 * because of where they sit in an array here. Every value below is copied
 * from a `Timeframe` constant in that file and nothing is interpolated
 * between them — a rung this table has no second count for is not given a
 * guessed one.
 *
 * `1m` IS AN ALIAS AND IS MARKED AS SUCH. It is what the server stamped on
 * every census row before it learned to write the real rung, and it may still
 * be in a cached response, so it gets the length this table already knows
 * rather than falling off the end of the ladder.
 *
 * `1hr` WAS A SECOND ALIAS AND IS GONE. This comment used to read: *"`1hr` is
 * `pull::vendor::Granularity`'s spelling of the rung the store files under
 * `60min`"* — a plain statement that one rung had two names, absorbed here by
 * hand because the two crates disagreed. D-0132 made them one word: the
 * store's, `60min`, which is the one that becomes a path. Nothing emits `1hr`
 * any more, and the server refuses it by name as an unknown rung, so keeping
 * a row for it would map a spelling nothing sends onto a length — a workaround
 * outliving the defect it worked around.
 *
 * # Why this exists at all
 *
 * Sorted as TEXT, the rungs read `15min`, `1day`, `1min`, `30min`, `3min`,
 * `5min`, `60min` — ordered by first character. That is the same defect as a
 * month column ordered by `Apr`, and it is why `$lib/dates.js` opens with the
 * rule it does. A bar-length axis has ONE order and it is the length.
 *
 * @type {ReadonlyMap<string, number>}
 */
export const TF_SECS = new Map([
  ['1s', 1],
  ['1min', 60],
  ['1m', 60],
  // 2min AND 10min WERE MISSING, AND THE STORE HAS BOTH.
  //
  // `store::path::Timeframe::KNOWN` lists SECOND_1, MINUTE_1, MINUTE_2,
  // MINUTE_3, MINUTE_5, MINUTE_10, MINUTE_15, MINUTE_30, MINUTE_60 and
  // DAY_1 — ten rungs — and this table carried seven of them. The two
  // absentees sort to `Infinity` and land at the END of the ladder, after
  // 1day, each labelled "length not recorded". Measured on the live page:
  // the rung menu read 1min, 3min, 5min, 15min, 30min, 60min, 1day, 10min,
  // 2min. Two of the nine rungs the store actually holds were out of order
  // and disclaiming a length the store publishes.
  //
  // The seconds are `Timeframe::MINUTE_2.secs` = 120 and
  // `MINUTE_10.secs` = 600, read from `crates/store/src/path.rs`, not
  // arithmetic done here.
  ['2min', 120],
  ['3min', 180],
  ['5min', 300],
  ['10min', 600],
  ['15min', 900],
  ['30min', 1800],
  ['60min', 3600],
  ['1day', 86400]
]);

/**
 * A rung's position on that ladder.
 *
 * A rung the table has no length for sorts LAST — `Infinity`, which is past
 * every real length — and is never folded into the middle, because a string
 * nobody measured has no measured relation to the rungs either side of it.
 * The control draws it, says `length not recorded`, and lets it be chosen:
 * the store holds those rows and hiding the only way to reach them would be
 * worse than admitting the ladder does not reach that far.
 *
 * @param {string} tf
 */
export const tfSecs = (tf) => TF_SECS.get(tf) ?? Number.POSITIVE_INFINITY;

/**
 * THREE-WAY, and it is not a formality.
 *
 * `a < b ? -1 : 1` answers "greater" for two EQUAL inputs, in both
 * directions at once, and a comparator that contradicts itself leaves the
 * order of those elements unspecified — the same store could render its
 * rungs in a different order on the next pass. Equal lengths fall through to
 * `txt`, which is itself three-way, so two spellings of one length are
 * ordered and stay ordered.
 *
 * The example this used to give was `1hr` beside `60min`, and D-0132 removed
 * it: they were one rung with two names and there is now one. The rule is
 * kept because `1m` beside `1min` is still a live pair, and because a
 * comparator that only happens to be consistent is not one.
 *
 * @param {string} a
 * @param {string} b
 */
export const tfCmp = (a, b) => {
  const sa = tfSecs(a);
  const sb = tfSecs(b);
  return sa < sb ? -1 : sa > sb ? 1 : txt(a, b);
};

/* `tfNote` — the third of these — is NOT here, and its absence is the rule
   the FORMATTING block below states: it calls `fmt`, so it is declared after
   `fmt` and not before it. The two above call nothing. */

/* ======================================================================
   FORMATTING
   ----------------------------------------------------------------------
   DECLARED HERE, ABOVE EVERY READER. These sat at the bottom of the script
   and the refusal notice — a `$derived` that formats live counts — closed
   over `fmt` from above it. It ran, because a derived is lazy and the const
   is initialised by the time anything reads it, but `svelte-check` called it
   what it is: a temporal-dead-zone reference that works by scheduling
   accident. One reordering of the evaluation and it throws.
   ====================================================================== */
/* PINNED TO en-IN, LIKE EVERY OTHER PAGE. An unqualified `Intl.NumberFormat()`
   inherits whatever locale the host happens to run, so the same store renders
   8,78,28,617 on one machine and 878,286,617 on another. That makes a
   screenshot unreproducible and a bug report unfalsifiable — and this one
   formatter feeds `fmt`, which is called 36 times across the headline
   counters, the month cards, the receipt prose and every table cell.
   `audit/+page.svelte:145` pins the same way. */
// AND IT NOW DASHES AN UNKNOWN. `nf.format(NaN)` is the string "NaN" and
// `format(Infinity)` is "∞" — a non-answer rendered as though it were one, at
// all 126 of this page's call sites, while `/audit` guarded the same values.
// `exact` does not round, because a record count is already whole and
// rounding one would hide a fractional value that should never have arrived.
export const fmt = exact;

/**
 * What one bar-length rung says about itself under the Timeframe control —
 * its length, and the session size the completeness column divides by.
 *
 * DECLARED HERE AND NOT BESIDE `TF_SECS`, because it calls `fmt` and `fmt`
 * is declared in the block above this line. That is the whole rule the
 * FORMATTING block states, applied rather than quoted.
 *
 * Never a figure this page does not hold: a rung with no recorded
 * bars-per-session says so rather than printing a denominator nobody chose —
 * the same rule `sessions` and the two percentage columns already follow.
 *
 * @param {string} tf
 */
export const tfNote = (tf) => {
  const per = barsPerSession(tf);
  const secs = TF_SECS.get(tf);
  const len = secs === undefined ? 'length not recorded' : `${fmt(secs)}s bars`;
  return per === null ? `${len} · no recorded session size` : `${len} · ${fmt(per)}/session`;
};

/**
 * Three-way codepoint comparison, and STRINGS ONLY.
 *
 * Every one of the fourteen call sites passes a string — a month key, an
 * instrument, a rung name, a reason code — and the block above says why:
 * these keys are `[A-Z0-9-_&]` by construction, so `<` on the codepoints is
 * the whole ordering and `localeCompare` would only make it a function of
 * the operator's locale. Declaring the parameters as strings is what keeps
 * that true: a number reaching here would compare numerically on one
 * argument and lexically on the other the moment one side arrived as text,
 * and the caller — `sorted`, at 20,516 rows — is the last place that would
 * show it.
 *
 * @param {string} a
 * @param {string} b
 */
export function txt(a, b) {
  return a < b ? -1 : a > b ? 1 : 0;
}
