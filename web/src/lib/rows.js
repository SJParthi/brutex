/**
 * WHETHER A CENSUS ROW CAN BE READ — the fault detector, with no runes in it,
 * so `web/tests/rows.test.js` can drive it under `node --test`.
 *
 * Extracted from `store.svelte.js`, which is 864 lines and imports
 * `$lib/ask.js`; node cannot resolve that alias, so this check — the one
 * standing between `/store.json` and every number two pages print — was
 * unreachable from a test runner even in principle. Its own doc block says why
 * it exists: *a promise is not a proof*. Nothing was proving the prover.
 */

/** The shape a census month is written in. One spelling of the test, shared. */
export const MONTH_KEY = /^\d{4}-\d{2}$/;

/**
 * `Number.isInteger`, restated as the type guard it already is at run time.
 *
 * @param {unknown} v
 * @returns {v is number}
 */
export const isWholeNumber = (v) => Number.isInteger(v);

/**
 * ONE CENSUS ROW AS IT ARRIVES — before `rowFault` has looked at it.
 *
 * The same four fields `rowFault` tests, each merely POSSIBLE. It is the honest
 * description of a row on entry: the endpoint says it sends a full row, and this
 * type says only what a reader may assume before checking, which is very nearly
 * nothing.
 *
 * @typedef {{ instrument?: unknown, month?: unknown, timeframe?: unknown, rows?: unknown }} WireRow
 */

/**
 * `null` when the row is readable; otherwise the reason it is not, in words.
 *
 * A ROW THAT FAILS IS KEPT, NEVER DROPPED AND NEVER COERCED. A census row whose
 * `rows` is absent, negative or fractional is a row whose count is UNKNOWN.
 * Dropping it makes it indistinguishable from a month that does not exist;
 * writing zero for it prints a measurement nobody took. It goes to `bad` with
 * the reason, and every total computed without it can say so.
 *
 * The reason names the FIELD and the VALUE, both. "malformed row" sends a
 * reader back to the wire to find out which of four fields it meant.
 *
 * @param {WireRow | null | undefined} row
 * @returns {string | null}
 */
export function rowFault(row) {
  if (typeof row?.instrument !== 'string' || row.instrument === '')
    return `its \`instrument\` field is ${JSON.stringify(row?.instrument)}, which is not an instrument key`;
  if (typeof row.month !== 'string' || !MONTH_KEY.test(row.month))
    return `its \`month\` field is ${JSON.stringify(row.month)}, which is not a YYYY-MM month`;
  if (typeof row.timeframe !== 'string' || row.timeframe === '')
    return `its \`timeframe\` field is ${JSON.stringify(row.timeframe)}`;
  if (!isWholeNumber(row.rows) || row.rows < 0)
    return `its \`rows\` field is ${JSON.stringify(row.rows)}, which is not a whole number of records`;
  return null;
}
