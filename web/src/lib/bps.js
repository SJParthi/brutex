/**
 * BASIS POINTS, ROUNDED THE WAY THE ENGINE ROUNDS THEM — the arithmetic half,
 * with no runes in it, so `web/tests/bps.test.js` can drive it under
 * `node --test`.
 *
 * # The divergence this exists to remove
 *
 * `/db` computed a bar's change with `Math.round(((b.c - before.c) * 10000) /
 * before.c)`. `Math.round` rounds a half toward +Infinity, so a +312.5 bp move
 * returned `313` and its mirror image returned `-312`. The engine rounds half
 * AWAY FROM ZERO in both directions — `crates/api/src/server.rs::basis_points`,
 * pinned by `basis_points_round_half_away_from_zero_in_both_directions`, whose
 * own comment names the consequence: "a gain and its mirror-image loss printing
 * different magnitudes, which is visible the moment the column is sorted."
 *
 * The two spellings met in ONE TABLE through ONE FORMATTER. `bpsText` renders
 * the server's month change and this browser-computed bar change in adjacent
 * columns, so the same quantity was rounded two ways on one screen. Worse,
 * `Math.round(-0.5)` is `-0`: `bpsText` prints that as an unsigned `0.00%` and
 * `dirOf` colours it `flat`. A real loss rendered as a month that did not move
 * is exactly the distinction the rest of that page is built to keep — zero is a
 * real answer, and it must never stand in for one that could not be computed.
 *
 * The values also LEAVE THE MACHINE. `/db`'s CSV button writes `chg_bps` and
 * `oi_chg_bps` under a tooltip promising the file holds the basis points "they
 * arrived as … so the file joins against the store". Under `Math.round` they
 * had not arrived; the browser made them, by a different rule, and the file did
 * not join.
 *
 * # Why the guard is `Number.isSafeInteger` and not `checked_mul`
 *
 * The engine multiplies in `i64` and returns `Unknown::Overflow` when the scale
 * leaves the type. A JavaScript number is an f64: past 2^53 it does not wrap,
 * it silently stops representing consecutive integers, which is the quieter and
 * worse failure. So the browser's guard is the f64 one, placed at the same step,
 * and its `null` is the same verdict — the caller turns it into the `overflow`
 * reason `WHY` already carries a sentence for.
 *
 * Both halves of the ratio are read from `Closes::known` upstream, so a
 * non-positive base is refused before this function is called; that case is the
 * caller's `previous_close_zero` / `previous_oi_zero` reason, not this one's.
 */

/**
 * One ratio in integer basis points, rounded half away from zero.
 *
 * @param {number} base  the earlier value. Must be > 0; the caller has already
 *                       named the reason when it is not.
 * @param {number} later the later value.
 * @returns {number | null} integer basis points, or `null` where the engine
 *                          would return `Unknown::Overflow`.
 */
export function basisPoints(base, later) {
  const scaled = (later - base) * 10_000;
  if (!Number.isSafeInteger(scaled)) {
    return null;
  }
  // `%` carries the sign of the DIVIDEND in JavaScript exactly as it does in
  // Rust, so `rest` is signed and `scaled - rest` is divisible by `base` with
  // nothing left over. Taking the remainder first and subtracting it keeps the
  // division exact, rather than trusting a float quotient to truncate the way
  // an integer divide would.
  const rest = scaled % base;
  const whole = (scaled - rest) / base;
  const step = rest >= 0 ? 1 : -1;
  const magnitude = rest >= 0 ? rest : -rest;
  // Written as a subtraction rather than `magnitude * 2 >= base` for the same
  // reason the engine writes it that way: `magnitude < base` makes
  // `base - magnitude` safe, and the doubling would not be.
  return magnitude >= base - magnitude ? whole + step : whole;
}
