/**
 * PAISA AND COUNTS, AS AN INDIAN READER EXPECTS THEM — no runes, so
 * `web/tests/money.test.js` can drive it under `node --test`.
 *
 * Extracted from `routes/+page.svelte`, where `group` and `rupee` are called 52
 * times and neither had a test. The claim in `group`'s own comment —
 * "8,78,28,617 and never 87,828,617" — is not a property of the code. It is a
 * property of the RUNTIME's ICU data, and a Node or browser build without the
 * full set silently falls back to `en-US` grouping and prints 87,828,617. That
 * is a wrong number rendered confidently, in the digit grouping a reader uses
 * to judge magnitude at a glance, and nothing would have said so.
 */

/** Pinned, never inherited. The host's locale is not this product's. */
export const LOC = 'en-IN';

/**
 * Indian digit grouping — `8,78,28,617` and never `87,828,617`.
 *
 * @param {number | string} n
 * @returns {string}
 */
export const group = (n) => Number(n).toLocaleString(LOC);

/**
 * A paisa integer as a rupee string, always to two places.
 *
 * THE ONLY DIVIDE, AND IT IS AT THE EDGE. Prices are `i64` paisa everywhere
 * else (CLAUDE.md §7); this is the display boundary and the one place a
 * fractional rupee may exist at all. `paisa / 100` is exact enough for the job
 * because the quotient always has exactly two decimal places — there is never a
 * third digit for the float error to reach — but that holds only while the
 * paisa stays inside `Number.MAX_SAFE_INTEGER`, which at ₹90 billion it does by
 * a wide margin for an index quote.
 *
 * @param {number} paisa
 * @returns {string}
 */
export const rupee = (paisa) =>
  (paisa / 100).toLocaleString(LOC, { minimumFractionDigits: 2, maximumFractionDigits: 2 });

/**
 * THE EM DASH IS THE HONEST EMPTY, and `Intl` does not agree.
 *
 * `new Intl.NumberFormat('en-IN').format(NaN)` is the string `"NaN"` and
 * `format(Infinity)` is `"∞"`. Both are a non-answer rendered as though it were
 * one — the failure CLAUDE.md §4 names — and `/db` shipped it at 126 call
 * sites while `/audit` guarded the same values at its own. Two pages, one
 * quantity, two different words for not knowing.
 *
 * Two functions rather than one, because the pages genuinely differ and the
 * difference should be visible rather than accidental: `/audit` folds averages
 * and wants a whole number, `/db` counts records and must not round anything.
 * Naming both is what stops the next reader picking one at random.
 */

/**
 * A count, grouped; the em dash when there is no number.
 *
 * Does NOT round — a record count is already whole, and rounding one would hide
 * a fractional value that should never have arrived.
 *
 * @param {number} n
 * @returns {string}
 */
export const exact = (n) => (Number.isFinite(n) ? group(n) : '—');

/**
 * A fold or an average as a whole number, grouped; the em dash when there is
 * no number.
 *
 * @param {number} n
 * @returns {string}
 */
export const whole = (n) => (Number.isFinite(n) ? group(Math.round(n)) : '—');

/**
 * The same, to one decimal — for a ratio that would lose its meaning as an
 * integer.
 *
 * @param {number} n
 * @returns {string}
 */
export const oneDp = (n) =>
  Number.isFinite(n)
    ? new Intl.NumberFormat(LOC, { maximumFractionDigits: 1 }).format(n)
    : '—';
