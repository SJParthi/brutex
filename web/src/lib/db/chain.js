/**
 * /db — THE MONEYNESS LADDER'S NAMES
 *
 * How a chain offset becomes `ATM`, `ITM-k` or `OTM+k`, and how that name is
 * read back as a signed position.
 *
 * Moved out of `routes/db/+page.svelte` by WP-00 and owned by WP-53. The
 * names are the server's convention too: `/chain.json` (D-3329) names every
 * rung it counts this way, and the Moneyness control reads them back here, so
 * the list, the pick and the per-bar test are one string.
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
