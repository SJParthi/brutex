/**
 * THE IST OFFSET AND THE REGULAR NSE SESSION, AS NUMBERS — the one web copy.
 *
 * The Rust side holds these as `pull::session::IST_OFFSET_SECS`,
 * `pull::session::SESSION_OPEN_MINUTE` and
 * `pull::session::BARS_PER_REGULAR_SESSION`. The browser cannot import them,
 * so this module restates each ONCE and every page and module under `web/src`
 * imports it from here. Before D-3516 the offset was typed twelve times in
 * ten files (as seconds, milliseconds, microseconds and a BigInt) and the
 * session length in three; `web/tests/ist.test.js` now refuses another
 * spelling of either anywhere under `web/src`.
 *
 * `dates.js` stays the authority for RENDERING an IST instant, through `Intl`
 * and `Asia/Kolkata`. This module is for ARITHMETIC on one: a civil-day
 * number, a bucket edge, a minute of the session.
 */

/** IST is UTC+05:30 exactly, and India observes no daylight saving. */
export const IST_OFFSET_SECONDS = 19_800;

/** The same offset in milliseconds, the unit `Date` speaks. */
export const IST_OFFSET_MS = IST_OFFSET_SECONDS * 1_000;

/** The same offset in microseconds, the unit the store's stamps are in. */
export const IST_OFFSET_MICROS = IST_OFFSET_SECONDS * 1_000_000;

/** The same offset in microseconds as a BigInt, for the u64/i64 wire fields. */
export const IST_OFFSET_MICROS_BIG = BigInt(IST_OFFSET_SECONDS) * 1_000_000n;

/** Minutes from IST midnight to the regular open, 09:15. */
export const SESSION_OPEN_MINUTE = 9 * 60 + 15;

/** One-minute bars in a regular session, 09:15 to 15:29 inclusive. */
export const SESSION_MINUTES = 375;

const DAY_SECONDS = 86_400;

/** @param {number} a @param {number} n */
const mod = (a, n) => ((a % n) + n) % n;

/**
 * The start, in epoch seconds, of the `seconds`-wide candle that the bar
 * stamped `t` (epoch seconds) falls in — THE GRID `pull::fold` FILES THE
 * STORE'S RUNGS ON, so a candle a page derives from finer bars is the candle
 * the store holds at that rung.
 *
 * A width under a day is counted from the 09:15 IST open and restarts at every
 * open: `t − ((t + IST − 09:15) mod day) mod width`, the rule
 * `runner::resample` restates (D-1430). For every width that divides a day —
 * every rung the store files — that is edge for edge the continuous grid
 * `pull::fold` walks; for one that does not (7 or 45 minutes) each session
 * still begins a candle at 09:15. A day or wider keeps IST midnight, as
 * `pull::fold` does for a bucket of 86,400 seconds or more.
 *
 * Counting every width from IST midnight instead (what the markets page did
 * until D-3516) agrees only when the width divides 555 minutes: at 2, 10, 30
 * and 60 minutes the first candle of each session was stamped 09:14, 09:10,
 * 09:00 and 09:00 and held only part of a width, while the stored rung of the
 * same name begins at 09:15.
 *
 * @param {number} t epoch seconds
 * @param {number} seconds the candle width, a positive whole number of seconds
 * @returns {number} the candle's start, epoch seconds
 */
export function bucketStart(t, seconds) {
  if (seconds >= DAY_SECONDS) return t - mod(t + IST_OFFSET_SECONDS, seconds);
  const sinceOpen = t + IST_OFFSET_SECONDS - SESSION_OPEN_MINUTE * 60;
  return t - mod(mod(sinceOpen, DAY_SECONDS), seconds);
}
