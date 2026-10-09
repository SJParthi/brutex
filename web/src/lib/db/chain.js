/**
 * /db — THE MONEYNESS LADDER'S NAMES
 *
 * How a chain offset becomes `ATM`, `ITM-k` or `OTM+k`, and how that name is
 * read back as a signed position.
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
 * Owner after WP-00: WP-53 (option cells: own-month pricing, point-in-time moneyness).
 *
 * A PLAIN MODULE, NOT A `.svelte.js` ONE: it holds constants and pure
 * functions only, so `node --test` can import it with no compiler. Imports
 * are relative for the same reason — `$lib` is a Vite alias node cannot see.
 */

/**
 * The rung a chain offset sits on FOR ONE SIDE, and the sign is in the name.
 *
 * A call is in the money BELOW spot; a put is in the money ABOVE it. So one
 * strike is `ITM-2` for the CE and `OTM+2` for the PE, and the rung is a
 * function of both — which is exactly why moneyness cannot be a property of
 * the strike control alone, and why these stay two controls.
 *
 * @param {number} k offset from the money, negative below it
 * @param {'CE' | 'PE'} side
 */
export const rungOf = (k, side) =>
  k === 0 ? 'ATM' : ((side === 'CE') === (k < 0) ? 'ITM-' : 'OTM+') + Math.abs(k);

/* READ BACK OFF THE NAME, so the string on screen, the key in the selection,
   the key in the filter box and the key in the sort are ONE string and cannot
   drift into two. */
export const RUNG = /^(ITM|ATM|OTM)([+-]\d+)?$/;
/** @param {string} m */
export const famOf = (m) => (m.match(RUNG) ?? [null, m])[1];
/** The signed offset. `ATM` is 0; `ITM-10` is −10, which is DEEPER than −2. */
/** @param {string} m */
export const ladderPos = (m) => {
  const p = m.match(RUNG);
  return p && p[2] ? Number(p[2]) : 0;
};
