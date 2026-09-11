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

/**
 * Integer basis points as a signed percentage. `125` -> `+1.25%`.
 *
 * BUILT FROM THE INTEGER, NEVER FROM A DIVIDE. `(mag - frac) / 100` divides a
 * number already known to be a multiple of 100, so the quotient is exact and
 * there is no float to round. `bps / 100` with two fraction digits would look
 * identical and would be a second rounding rule beside `basisPoints`'s.
 *
 * ZERO CARRIES NO SIGN, because it has no direction — the cell reads `0.00%`
 * and is neutral-coloured, which is a real answer and not an unknown.
 *
 * `-0` IS RENDERED AS `0.00%`, AND THAT IS ONLY SAFE BECAUSE OF `basisPoints`.
 * JavaScript's `-0` is a negative that lost its magnitude, and this formatter
 * cannot tell it from a true zero — `Math.abs(-0)` is `0` and the sign test
 * `bps < 0` is false. It reached here once: `/db` computed its change with
 * `Math.round`, `Math.round(-0.5)` is `-0`, and a real loss printed as a flat
 * month. The formatter was not the fault and is not the fix. `basisPoints`
 * rounds away from zero in both directions, so a negative can never arrive with
 * zero magnitude, and `-0` is now genuinely unreachable from the only producer
 * this page has. Any future producer owes the same guarantee.
 *
 * @param {number} bps INTEGER basis points; 125 is +1.25%
 * @returns {string}
 */
export function bpsText(bps) {
  const sign = bps > 0 ? '+' : bps < 0 ? '-' : '';
  const mag = Math.abs(bps);
  const frac = mag % 100;
  return `${sign}${(mag - frac) / 100}.${String(frac).padStart(2, '0')}%`;
}

/**
 * `up` / `down` / `flat` / `none` — the direction green and red are reserved for.
 *
 * `none` IS ITS OWN WORD RATHER THAN `flat`. A cell nobody can compute and a
 * month that did not move are different facts and must not share a colour: one
 * is an absence with a reason beside it, the other is a measurement.
 *
 * @param {number | null | undefined} bps
 * @returns {'up' | 'down' | 'flat' | 'none'}
 */
export const dirOf = (bps) =>
  bps === null || bps === undefined ? 'none' : bps > 0 ? 'up' : bps < 0 ? 'down' : 'flat';
