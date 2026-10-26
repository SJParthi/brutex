/**
 * WHICH EXPIRED-DERIVATIVE SEGMENTS A FEED CAN BE ASKED FOR, by the `fno` word
 * `/feeds.json` states for it (R4-28 repair 5, the r5 check's follow-up).
 *
 * `/ingest` kept this inline and knew four words. The server states five
 * since D-7186, and the fifth, `by_token` (Zerodha: the day candles of each
 * futures contract through a token resolved by date against the dated Kite
 * master snapshots, stored and never swept), fell through to "states no
 * expired-derivative history", which is false for it. One table of the five
 * words answers the page's three questions: which segments a feed is asked
 * for, why a feed is declined, and why a feed is served for one segment only.
 *
 *   by_name           futures and options: a discovery walk over contract
 *                     names.
 *   by_strike_offset  options only: ATM±n through `v2/charts/rollingoption`,
 *                     which publishes no expired-futures series.
 *   by_token          futures only: `POST /pull/fno` refuses expired options
 *                     for this feed by name.
 *   local_folder      none: an archive, whose contracts are files on disk.
 *   none              none: the feed states no expired-derivative history.
 *
 * A WORD THIS TABLE DOES NOT KNOW is declined and named as unknown, never read
 * as `none`: a page older than its server would otherwise call a new shape
 * "no history", which is how `by_token` was misreported.
 */

/** @type {Readonly<Record<string, { segments: readonly string[], why: string }>>} */
const MODES = Object.freeze({
  by_name: { segments: Object.freeze(['futures', 'options']), why: '' },
  by_strike_offset: {
    segments: Object.freeze(['options']),
    why: 'its endpoint is v2/charts/rollingoption, which addresses options by ATM±n and publishes no expired-futures series.'
  },
  by_token: {
    segments: Object.freeze(['futures']),
    why: 'day candles of each futures contract, through a token resolved by date against the dated master snapshots; the server refuses expired options for it by name.'
  },
  local_folder: {
    segments: Object.freeze([]),
    why: 'is a folder of files, not an endpoint — there is no expiry list to ask it for.'
  },
  none: { segments: Object.freeze([]), why: 'states no expired-derivative history.' }
});

/** The five words `/feeds.json` states, in this table's order. */
export const FNO_MODES = Object.freeze(Object.keys(MODES));

/** @param {unknown} how */
const known = (how) => typeof how === 'string' && Object.hasOwn(MODES, how);

/**
 * The segments a feed stating `how` is asked for: `futures`, `options`, both
 * or neither. A word this table does not know asks for neither.
 *
 * @param {unknown} how
 * @returns {string[]}
 */
export function fnoSegmentsOf(how) {
  return known(how) ? [...MODES[/** @type {string} */ (how)].segments] : [];
}

/**
 * Why a feed stating `how` is asked for no segment at all, or `''` when it is
 * asked for one. An unknown word is named as such.
 *
 * @param {unknown} how
 * @returns {string}
 */
export function fnoDeclinedWhy(how) {
  if (!known(how)) {
    const said = how === undefined || how === null ? 'none stated' : JSON.stringify(how);
    return `states an expired-derivative mode this page does not know (${said}); nothing is sent to it.`;
  }
  const mode = MODES[/** @type {string} */ (how)];
  return mode.segments.length === 0 ? mode.why : '';
}

/**
 * Why a feed stating `how` is asked for one segment and not both, or `''`
 * when it is asked for both or for neither.
 *
 * @param {unknown} how
 * @returns {string}
 */
export function fnoPartialWhy(how) {
  if (!known(how)) return '';
  const mode = MODES[/** @type {string} */ (how)];
  return mode.segments.length === 1 ? `serves expired ${mode.segments[0]} only — ${mode.why}` : '';
}
