/**
 * THE BACKTEST PERFORMANCE TAB'S HOLDING SERIES, WITH NO RUNES IN IT, so
 * `web/tests/hold-series.test.js` can drive the same arithmetic the page draws.
 *
 * Both functions read the current store's bars (`{t, o, c}`: `t` in epoch
 * SECONDS, prices in paisa) and say nothing about the strategy.
 *
 * # `holdPeriods` measured the wrong interval and had no year (CE-70, D-2730)
 *
 * Each bucket was its own LAST close minus its own FIRST BAR's close. That
 * drops the move of the bucket's first bar and every gap between buckets
 * (overnight, a weekend), so the buckets did not sum to the holding P&L the
 * chart's caption names, and on a `1day` series every daily bucket held one
 * bar and read exactly 0. The daily and weekly keys were `d/m` with no year, so
 * on a 1,000-bar daily window (about four years) 5 Mar 2023 and 5 Mar 2024 were
 * one bucket whose value was a year-over-year change.
 *
 * Now each bucket is measured from the PREVIOUS bucket's last close, and the
 * first bucket from its first bar's OPEN — the earliest price the window holds;
 * the gap into the window's first bar is not on screen and is not invented.
 * The buckets therefore sum exactly to `last close - first open`. Daily and
 * weekly keys are the IST `YYYY-MM-DD` that `istDay` produces.
 *
 * # `returnHistogram` had a second rounding rule (CE-73, D-2731)
 *
 * It rounded a float ratio with `Math.round`, which rounds half toward
 * +Infinity: a +312.5 bp bar landed in 313 and its mirror image in -312, and
 * `Math.round(-0.4)` is `-0`, which `r < 0` does not count as a loser.
 * `basisPoints` is the engine's rule (half away from zero, integer-exact); a
 * bar where it returns `null` (overflow) is skipped and COUNTED, so the page
 * can say how many it left out.
 */

import { basisPoints } from './bps.js';
import { istDay } from './dates.js';
import { IST_OFFSET_MS } from './ist.js';

const DAY_MS = 86_400_000;

/**
 * The bucket key for one bar's instant.
 *
 * @param {number} t epoch seconds
 * @param {'daily'|'weekly'|'quarterly'|'yearly'} scale
 * @returns {string}
 */
export function periodKey(t, scale) {
  const ms = t * 1000;
  // Shift only for calendar projection; the instant itself is zone-neutral.
  const d = new Date(ms + IST_OFFSET_MS);
  const y = d.getUTCFullYear();
  if (scale === 'yearly') return `${y}`;
  if (scale === 'quarterly') return `Q${Math.floor(d.getUTCMonth() / 3) + 1} '${String(y).slice(2)}`;
  if (scale === 'daily') return istDay(ms);
  // Weekly: the IST Monday that starts the bar's week, with its year.
  return istDay(ms - ((d.getUTCDay() + 6) % 7) * DAY_MS);
}

/**
 * The bars bucketed by period, each bucket's change from the previous
 * bucket's close (the first bucket's from its first open), in paisa.
 *
 * Ordered by first appearance, which is chronological because the bars are.
 *
 * @param {ReadonlyArray<{t: number, o: number, c: number}>} bars
 * @param {'daily'|'weekly'|'quarterly'|'yearly'} scale
 * @returns {{label: string, v: number}[]}
 */
export function holdPeriods(bars, scale) {
  if (bars.length < 2) return [];
  /** @type {Map<string, {label: string, last: number}>} */
  const buckets = new Map();
  for (const b of bars) {
    const k = periodKey(b.t, scale);
    const at = buckets.get(k);
    if (at) at.last = b.c;
    else buckets.set(k, { label: k, last: b.c });
  }
  let base = bars[0].o;
  const out = [];
  for (const x of buckets.values()) {
    out.push({ label: x.label, v: x.last - base });
    base = x.last;
  }
  return out;
}

/**
 * Half away from zero, for the mean of integer basis points — the same rule
 * `basisPoints` applies, so the average line does not reintroduce `Math.round`.
 *
 * @param {number} x
 */
function roundHalfAway(x) {
  const r = Math.round(Math.abs(x));
  return x < 0 ? -r : r;
}

/**
 * The distribution of per-bar returns (open to close), in integer basis
 * points, bucketed into at most 19 bins. `null` when there is nothing to
 * distribute.
 *
 * @param {ReadonlyArray<{o: number, c: number}>} bars
 */
export function returnHistogram(bars) {
  if (bars.length < 2) return null;
  /** @type {number[]} */
  const rets = [];
  let overflowed = 0;
  for (const b of bars) {
    if (!(b.o > 0)) continue;
    const bps = basisPoints(b.o, b.c);
    if (bps === null) overflowed += 1;
    else rets.push(bps);
  }
  if (rets.length === 0) return null;
  const lo = Math.min(...rets);
  const hi = Math.max(...rets);
  const width = Math.max(1, Math.ceil((hi - lo) / 18));
  /** @type {Map<number, number>} */
  const counts = new Map();
  for (const r of rets) {
    const slot = Math.floor((r - lo) / width);
    counts.set(slot, (counts.get(slot) ?? 0) + 1);
  }
  const bins = [];
  for (let i = 0; i <= Math.floor((hi - lo) / width); i += 1) {
    const from = lo + i * width;
    bins.push({ from, mid: from + width / 2, n: counts.get(i) ?? 0 });
  }
  const losses = rets.filter((r) => r < 0);
  const gains = rets.filter((r) => r > 0);
  /** @param {number[]} xs */
  const mean = (xs) =>
    xs.length === 0 ? null : roundHalfAway(xs.reduce((a, b) => a + b, 0) / xs.length);
  return {
    bins,
    max: Math.max(1, ...bins.map((b) => b.n)),
    lo,
    hi,
    avgLoss: mean(losses),
    avgGain: mean(gains),
    losers: losses.length,
    winners: gains.length,
    flat: rets.length - losses.length - gains.length,
    total: rets.length,
    overflowed
  };
}
