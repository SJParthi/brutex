/**
 * /db — WHAT AN OPTION CELL SAYS ABOUT ITSELF
 *
 * The greek columns, the short word a refused cell shows and the sentence
 * behind it, the per-page tally of those words, the sentence that names a
 * whole underlying's missing spot once, and the Moneyness control's test of
 * one bar's rung.
 *
 * Moved out of `routes/db/+page.svelte` by WP-00; owned by WP-53 (option
 * cells: own-month pricing, point-in-time moneyness, refusal words), which
 * fixed DB-038, DB-068, DB-069, DB-075, DB-B14, DB-G53 and DB-G64 here
 * (D-3328, D-3329, D-3330).
 *
 * A PLAIN MODULE, NOT A `.svelte.js` ONE: it holds constants and pure
 * functions only, so `node --test` can import it with no compiler. Imports
 * are relative for the same reason — `$lib` is a Vite alias node cannot see.
 */
import { GREEK_FLAG, GREEK_WHY, ivText, greekText, tenorText } from '../greeks.js';
import { RUNG_WHY, SPOT_WHY } from '../moneyness.js';
import { paisaText } from './bars-plan.js';
import { famOf } from './chain.js';
import { fmt } from './notes-labels.js';

/**
 * WHAT THE MONEYNESS CONTROL IS NOW (D-3329, DB-075). A rung is a distance
 * from spot AT ONE INSTANT along the chain stored by then, so it belongs to a
 * BAR and is asked of `/chain.json` at each bar's own stamp. The control keeps
 * the bars whose own rung is the one picked. Written once: it is the control's
 * title and the clause under it.
 */
export const RUNG_PER_BAR =
  'a rung is a distance from spot at ONE instant, counted along the strikes of this expiry stored by then (/chain.json, D-3329), so it belongs to a bar and not to a month. Picking one keeps the bars whose own rung it is, on each page the server sends.';

/**
 * The short word a refused cell shows, per reason code: at most ten
 * characters where a greek column (86px) must hold it, the rung's own codes
 * up to the Moneyness column's width. The cell's title and the page's tally
 * carry the whole sentence (DB-038: a refusal used to be the same `—` as
 * "not an option"); `unimported` is said in full once per underlying.
 */
export const REFUSAL_WORD = {
  no_spot_bar: 'no spot',
  no_spot_in_session: 'no prior',
  no_spot_file: 'unimported',
  spot_unreadable: 'unreadable',
  spot_being_written: 'writing',
  spot_order_broken: 'unordered',
  spot_request_failed: 'failed',
  t_outside_month: 'bad month',
  t_off_grid: 'off grid',
  census_absent: 'no census',
  census_state_unknown: 'lag unread',
  census_degraded: 'degraded',
  census_behind: 'census behind',
  expiry_not_in_census: 'not counted',
  no_strike_stored_by_then: 'no strike',
  ladder_too_sparse: 'too sparse',
  strike_not_stored_by_then: 'not stored',
  rung_request_failed: 'failed',
  rate_unset: 'no rate',
  rate_unreadable: 'rate fault',
  rate_before_series: 'pre-rates',
  rate_stale: 'rate stale',
  tenor_refused: 'no tenor',
  iv_unsolved: 'unsolved',
  below_zero_carry_bound: '≤ floor',
  no_option_bar: 'no bar',
  no_option_file: 'no file',
  option_unreadable: 'unreadable',
  option_being_written: 'writing',
  option_order_broken: 'unordered',
  greeks_request_failed: 'failed'
};

/** The reasons that are a wait, not a refusal: drawn as pending, never tallied. */
export const PENDING = new Set(['spot_pending', 'rung_pending', 'greeks_pending']);

/** The word for one refusal code; an unknown code shows itself. @param {string} why */
export const refusalWord = (why) => /** @type {Record<string, string>} */ (REFUSAL_WORD)[why] ?? why;

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
 * Where a spot came from, for a title: the exact second, or the earlier bar
 * opted in and how old it is (D-3328: the age is always shown).
 * @param {{spotT?: number|null, age?: number|null, asof?: string|null}} s
 */
export function spotAgeText(s) {
  if (s.age === null || s.age === undefined) return '';
  if (s.age === 0) return 'at this bar’s own second';
  return `from the underlying’s last bar ${fmt(s.age)} s before this one (last at or before t, opted in; never a later bar)`;
}

/**
 * The title over a solved cell: what it was solved from, the tenor in years,
 * and every flag the server set (D-3330, DB-G64).
 * @param {Record<string, any>|null} r the server's row
 */
export function greekInputs(r) {
  if (!r) return '';
  const tenor = tenorText(typeof r.tenor_y === 'number' ? r.tenor_y : null, typeof r.tenor_s === 'number' ? r.tenor_s : null);
  const age = typeof r.spot_age_s === 'number' && r.spot_age_s > 0 ? ` (${fmt(r.spot_age_s)} s old: last spot at or before t)` : '';
  const flags = [
    r.below_band === true ? `FLAGGED below_band: ${GREEK_FLAG.below_band}` : '',
    r.above_rate_tenor === true ? `FLAGGED above_rate_tenor: ${GREEK_FLAG.above_rate_tenor}` : ''
  ].filter(Boolean);
  return `solved on the server from close ${paisaText(r.premium)}, spot ${paisaText(r.spot)}${age}, tenor ${tenor} (${fmt(r.tenor_s)} s) to the settled expiry’s session close, rate ${(Number(r.rate) * 100).toFixed(4)}% continuous (RBI 91-day T-bill released ${r.auction}), zero carry${flags.length ? `. ${flags.join(' ')}` : ''}`;
}

/**
 * The cell's reason, and the server's own fault sentence when it sent one.
 * @param {string|null} why
 * @param {{detail?: string|null}} cell
 */
export const spotWhyText = (why, cell) =>
  `${/** @type {Record<string, string>} */ (SPOT_WHY)[why ?? ''] ?? `unknown (${why})`}${cell.detail ? ` — ${cell.detail}` : ''}`;

/**
 * A rung's reason — a chain refusal, or the spot's own word when there was no
 * spot to count from — and the server's sentence after it.
 * @param {string|null} why
 * @param {{mnyDetail?: string|null}} cell
 */
export const rungWhyText = (why, cell) =>
  `${/** @type {Record<string, string>} */ (RUNG_WHY)[why ?? ''] ??
    /** @type {Record<string, string>} */ (SPOT_WHY)[why ?? ''] ??
    `unknown (${why})`}${cell.mnyDetail ? ` — ${cell.mnyDetail}` : ''}`;

/**
 * Every sentence a code can have, from whichever table holds it.
 * @param {string} why
 */
export const reasonText = (why) =>
  /** @type {Record<string, string>} */ (RUNG_WHY)[why] ??
  /** @type {Record<string, string>} */ (SPOT_WHY)[why] ??
  /** @type {Record<string, string>} */ (GREEK_WHY)[why] ??
  `unknown (${why})`;

/**
 * WHETHER ONE BAR'S OWN RUNG IS THE ONE PICKED (D-3329, DB-075). `ATM` is
 * exact; `ITM` or `OTM` alone is the whole family, at any depth; `ITM-2` is
 * exactly that rung. A bar with no rung (refused, or not yet read) is not on
 * any rung and is never kept by a pick.
 * @param {Set<string>} pick @param {string|null|undefined} rung
 */
export function rungMatches(pick, rung) {
  if (pick.size === 0) return true;
  if (!rung) return false;
  return pick.has(rung) || pick.has(String(famOf(rung)));
}

/**
 * @typedef {object} Tally
 * @property {number} options  option rows on the page
 * @property {Map<string, number>} mny   refusal code -> rows, moneyness
 * @property {Map<string, number>} spot  refusal code -> rows, spot (intrinsic, extrinsic)
 * @property {Map<string, number>} greeks refusal code -> rows, IV and greeks
 * @property {Map<string, number>} flags  flag -> priced rows carrying it
 * @property {number} pending  rows with any cell still being read
 * @property {number} earlier  rows priced from an earlier spot (age > 0)
 * @property {number} oldest   the oldest such spot, seconds
 * @property {number} named    rows with a rung
 * @property {number} priced   rows with IV and greeks
 */

/**
 * THE REFUSALS ON ONE PAGE, COUNTED (D-3330, DB-038): per column group, every
 * reason code and how many option rows carry it; the flags on priced rows;
 * and how many were priced from an earlier spot. One pass over the page.
 * @param {any[]} rows the rows on the page
 * @param {(r: any) => any} oc a row's option cells
 * @param {(r: any) => any} gc a row's greek cell
 * @returns {Tally}
 */
export function tallyOf(rows, oc, gc) {
  /** @type {Tally} */
  const t = { options: 0, mny: new Map(), spot: new Map(), greeks: new Map(), flags: new Map(),
    pending: 0, earlier: 0, oldest: 0, named: 0, priced: 0 };
  const bump = (/** @type {Map<string, number>} */ m, /** @type {string} */ k) => m.set(k, (m.get(k) ?? 0) + 1);
  for (const r of rows) {
    const o = oc(r);
    const g = gc(r);
    if (!o && !g) continue;
    t.options += 1;
    let waiting = false;
    if (o) {
      if (o.mny !== null) t.named += 1;
      else if (PENDING.has(o.mnyWhy)) waiting = true;
      else bump(t.mny, o.mnyWhy);
      if (o.why !== null) {
        if (PENDING.has(o.why)) waiting = true;
        else bump(t.spot, o.why);
      } else if (typeof o.age === 'number' && o.age > 0) {
        t.earlier += 1;
        t.oldest = Math.max(t.oldest, o.age);
      }
    }
    if (g) {
      if (g.why === null) {
        t.priced += 1;
        for (const f of g.flags ?? []) bump(t.flags, f);
      } else if (PENDING.has(g.why)) waiting = true;
      else bump(t.greeks, g.why);
    }
    if (waiting) t.pending += 1;
  }
  return t;
}

/**
 * `code n · code n`, largest first — a tally map for reading.
 * @param {Map<string, number>} m
 * @returns {{code: string, word: string, n: number}[]}
 */
export const tallyRows = (m) =>
  [...m.entries()]
    .sort((a, b) => b[1] - a[1] || (a[0] < b[0] ? -1 : 1))
    .map(([code, n]) => ({ code, word: refusalWord(code), n }));

/**
 * ONE SENTENCE PER UNDERLYING WHOSE SPOT IS NOT IMPORTED (DB-069, DB-G53):
 * every bar of its options refuses `no_spot_file`, so the page names the
 * underlying once, with the cause the alias table (D-3332, `/aliases.json`)
 * can state — an index series, a dated symbol change, or a name with no row
 * whose end is UNVERIFIED — rather than repeating a generic reason on each
 * cell. Nothing here is a claim the table does not make.
 *
 * @param {string} underlying the option's underlying, as stored
 * @param {{segment: string|null, key: string|null}} read what `/spot.json` looked for
 * @param {any} alias the `/aliases.json?symbol=` answer, or null when it did not read
 * @param {string[]} months the bar months this page asked it in
 */
export function familyNote(underlying, read, alias, months) {
  const where = `${read.segment ?? '?'}/${read.key ?? underlying}`;
  const span = months.length === 1 ? months[0] : `${months[0]} to ${months[months.length - 1]}`;
  const head = `${underlying} options: underlying spot not imported — /spot.json read ${where} for ${span} and this feed’s store holds no such file, so no bar of these options has moneyness, intrinsic, extrinsic, IV or greeks.`;
  if (!alias || typeof alias !== 'object') {
    return `${head} The alias table did not answer, so the cause is not stated here.`;
  }
  const later = 'D-3333 imports it, stored and never swept (a store write, WP-80 step 4); until then it is absent, not zero.';
  if (alias.matched === 'index') {
    const nse = alias.index?.nse_name ? ` (${alias.index.nse_name})` : '';
    return `${head} ${underlying} is an index option: its index series${nse} files under ${alias.store_key ?? read.key}, which GDFL ships as a separate index file. ${later}`;
  }
  const changes = Array.isArray(alias.changes) ? alias.changes : [];
  if (changes.length > 0) {
    const steps = changes.map((/** @type {any} */ c) => `${c.old} → ${c.new} on ${c.effective}`).join(', then ');
    return `${head} Renamed: ${steps} (NSE symbolchange.csv, D-3332). Its spot is read under the newest symbol’s key, ${alias.store_key ?? read.key}, whose series on this feed starts at the rename; the days before it are in the old-name file. ${later}`;
  }
  if (alias.not_aliased && typeof alias.not_aliased.why === 'string') {
    return `${head} Not renamed: ${alias.not_aliased.why}. ${later}`;
  }
  return `${head} No symbol change or alias is recorded for ${underlying} (D-3332), so its spot is its own share series. An underlying that left F&O, was delisted or merged keeps its own key; which of these applies is UNVERIFIED. ${later}`;
}

/**
 * WHETHER AN EXPIRY'S TRADING ENDED BEFORE ITS STATED DAY, AS STORED (DB-B14).
 *
 * `last` is the newest stored bar of ANY contract of this underlying and
 * expiry; `frontier` the newest stored option bar of the whole feed (how far
 * the import reached). Ended early when the expiry lies inside what the
 * import reached and no contract of it has a bar on or after its own day.
 * Not claimed while the census is behind from a day on or before the expiry
 * (strikes stored since are not counted, D-3327). Why trading ended is
 * UNVERIFIED: no source here says.
 *
 * @param {string} expiry `YYYY-MM-DD`
 * @param {string|null} last IST day of the newest stored bar of the expiry
 * @param {string|null} frontier IST day of the newest stored option bar of the feed
 * @param {string|null} behindFrom IST day the census is behind from, or null
 * @returns {{ended: false, why: string} | {ended: true, last: string, days: number}}
 */
export function endedEarly(expiry, last, frontier, behindFrom) {
  if (!last) return { ended: false, why: 'no bar of this expiry is stored' };
  if (behindFrom && behindFrom <= expiry) return { ended: false, why: 'the census is behind from a day on or before this expiry' };
  if (!frontier || frontier < expiry) return { ended: false, why: 'the import has not reached this expiry' };
  if (last >= expiry) return { ended: false, why: 'a contract of this expiry has a bar on its own day' };
  const days = Math.round((Date.parse(`${expiry}T00:00:00Z`) - Date.parse(`${last}T00:00:00Z`)) / 86_400_000);
  return { ended: true, last, days };
}
