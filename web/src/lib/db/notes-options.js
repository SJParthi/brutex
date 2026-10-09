/**
 * /db — WHAT AN OPTION CELL SAYS ABOUT ITSELF
 *
 * The greek columns, the sentence behind a moneyness the month-level control
 * cannot anchor, and the words behind a spot or greek the server refused.
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
 * Owner after WP-00: WP-53 (option cells: own-month pricing, point-in-time moneyness, refusal words).
 *
 * A PLAIN MODULE, NOT A `.svelte.js` ONE: it holds constants and pure
 * functions only, so `node --test` can import it with no compiler. Imports
 * are relative for the same reason — `$lib` is a Vite alias node cannot see.
 */
import { ivText, greekText } from '../greeks.js';
import { paisaText } from './bars-plan.js';
import { fmt } from './notes-labels.js';
import { SPOT_WHY } from '../moneyness.js';

/**
 * WHY THE LADDER CANNOT BE OPENED, or `null` when it can — and WHICH of the
 * two reasons it is.
 *
 * The second one is permanent on today's wire and it is the more important
 * of the two, because it is the one an operator would otherwise assume away:
 * a chain is not a ladder. Rungs are distances FROM THE MONEY, and nothing
 * this page reads says where the money is.
 */
/* THE PRICE CLAUSE, WRITTEN ONCE. It is the reason on the control's own
   tooltip AND the reason on the counted line under the row, and two spellings
   of one refusal is how one of them goes stale. */
export const NO_SPOT =
  'a rung is a DISTANCE FROM SPOT AT ONE INSTANT, and this control filters stored instrument-MONTHS, which have no single spot: the underlying moves through every bar of the month, so one rung per month would be a guess at which instant was meant. /store.json’s chg_bps is a RATIO and carries no price at all. The Bars view names each option bar’s rung, intrinsic and extrinsic from the underlying’s stored close at that bar’s own instant (/spot.json, D-2807); this month-level control stays unanchored rather than anchored to a guess.';
/** The six greek cells, in column order, each with its reading format. */
export const GREEK_FIELDS = [
  { key: 'iv', label: 'IV %', text: (/** @type {any} */ c) => ivText(c.iv) },
  { key: 'delta', label: 'Delta', text: (/** @type {any} */ c) => greekText(c.delta, 4) },
  { key: 'gamma', label: 'Gamma', text: (/** @type {any} */ c) => greekText(c.gamma, 6) },
  { key: 'theta', label: 'Theta', text: (/** @type {any} */ c) => greekText(c.theta, 2) },
  { key: 'vega', label: 'Vega', text: (/** @type {any} */ c) => greekText(c.vega, 2) },
  { key: 'rho', label: 'Rho', text: (/** @type {any} */ c) => greekText(c.rho, 2) }
];
/**
 * The title over a solved cell: what it was solved from.
 * @param {Record<string, any>|null} r
 */
export const greekInputs = (r) =>
  r
    ? `solved on the server from close ${paisaText(r.premium)}, spot ${paisaText(r.spot)}, ${fmt(r.tenor_s)} s to the expiry session close, rate ${(Number(r.rate) * 100).toFixed(4)}% continuous (RBI 91-day T-bill released ${r.auction}), zero carry${r.below_band ? '; the tenor is below the band the greeks crate validates itself in' : ''}`
    : '';

/**
 * The cell's reason, and the server's own fault sentence when it sent one.
 * @param {string|null} why
 * @param {{detail: string|null}} cell
 */
export const spotWhyText = (why, cell) =>
  `${/** @type {Record<string, string>} */ (SPOT_WHY)[why ?? ''] ?? `unknown (${why})`}${cell.detail ? ` — ${cell.detail}` : ''}`;
