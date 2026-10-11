/**
 * WHEN /markets READS A SERIES' NEWEST BARS INSTEAD OF ITS WHOLE MONTH FILES
 * (WP-61; DB-041, DB-080).
 *
 * `/bars.json` answers one whole month file: about 8,600 bars for a one-minute
 * series, 380,000 to 500,000 for a one-second one (52.8 MB of JSON, measured on
 * NIFTY 1s 2026-09). The rule here is the SIZE the census states for the months
 * asked, never the feed or the rung: past `MONTH_READ_MAX` records in any one
 * month file the page reads the newest `TAIL_PAGES` pages of `TAIL_PAGE` rows
 * through `/bars/window.json` and says how many of how many it drew.
 *
 * Pure, so `web/tests/markets-tail.test.js` drives every branch.
 */

/** The most records one month file may hold and still be read whole. */
export const MONTH_READ_MAX = 60_000;
/** Rows per `/bars/window.json` page: the route's own ceiling (`MAX_WINDOW_LIMIT`). */
export const TAIL_PAGE = 1_000;
/** Pages of the newest bars a bounded read takes: 5,000 bars at most. */
export const TAIL_PAGES = 5;

/**
 * @param {{ month: string, rows: number }[]} months the month files asked for, oldest first
 * @returns {{ bounded: boolean, from: string, to: string, pages: number, biggest: number, why: string }}
 */
export function tailPlan(months) {
  const held = Array.isArray(months)
    ? months.filter((m) => m && typeof m.month === 'string' && /^\d{4}-\d{2}$/.test(m.month))
    : [];
  if (held.length === 0) return { bounded: false, from: '', to: '', pages: 0, biggest: 0, why: '' };
  let biggest = 0;
  let from = held[0].month;
  let to = held[0].month;
  for (const m of held) {
    const rows = Number.isSafeInteger(m.rows) && m.rows > 0 ? m.rows : 0;
    if (rows > biggest) biggest = rows;
    if (m.month < from) from = m.month;
    if (m.month > to) to = m.month;
  }
  if (biggest <= MONTH_READ_MAX) return { bounded: false, from, to, pages: 0, biggest, why: '' };
  const most = TAIL_PAGE * TAIL_PAGES;
  return {
    bounded: true,
    from,
    to,
    pages: TAIL_PAGES,
    biggest,
    why:
      `A month file asked for holds ${biggest.toLocaleString('en-IN')} bars, more than the ` +
      `${MONTH_READ_MAX.toLocaleString('en-IN')} this page reads whole, so the chart draws the newest ` +
      `${most.toLocaleString('en-IN')} bars at most of ${from}${to === from ? '' : ` to ${to}`}, read ` +
      `${TAIL_PAGE.toLocaleString('en-IN')} at a time from /bars/window.json, and not the whole file.`
  };
}

/**
 * @typedef {{ t: number, o: number, h: number, l: number, c: number, v: number }} TailBar
 */

/**
 * FOLD BARS INTO BUCKETS OF `seconds`, on the integers they arrive as: min,
 * max and last are exact on integers. `offset` shifts the bucket edges (the
 * page passes IST's +05:30, so an hourly candle breaks on the hour a trader in
 * India sees). Bars arrive oldest first.
 *
 * @param {TailBar[]} raw
 * @param {number | null} seconds
 * @param {number} [offset] seconds added before bucketing, removed after
 * @returns {TailBar[]}
 */
export function foldBars(raw, seconds, offset = 0) {
  if (seconds === null || !(seconds > 0)) return [];
  /** @type {TailBar[]} */
  const out = [];
  /** @type {TailBar | null} */
  let cur = null;
  for (const b of raw) {
    const start = Math.floor((b.t + offset) / seconds) * seconds - offset;
    if (!cur || cur.t !== start) {
      cur = { t: start, o: b.o, h: b.h, l: b.l, c: b.c, v: b.v };
      out.push(cur);
    } else {
      if (b.h > cur.h) cur.h = b.h;
      if (b.l < cur.l) cur.l = b.l;
      cur.c = b.c;
      cur.v += b.v;
    }
  }
  return out;
}

/**
 * CUT A BOUNDED READ ON A BUCKET EDGE OF THE DRAWN RUNG (WP-61 repair).
 *
 * A read of the newest N bars ends at whatever second the Nth bar fell on. The
 * oldest bucket of the drawn rung then holds only the bars after that second,
 * and folding it gives a candle whose open, high and low are the fragment's:
 * measured on GDFL NIFTY 1s 2026-09 at 5m, the 14:05 IST candle of 24 Sep was
 * folded from 200 of its 300 seconds and drew an open 6.80 above the stored
 * one. A fragment is not a candle, so it is not drawn.
 *
 * The rule, on what was read and nothing else: when older bars exist that were
 * NOT read (`olderExist`) and the oldest bar read does not sit on its bucket's
 * first second, every bar of that bucket is dropped and the cut is reported. A
 * bucket whose first second WAS read is whole (the read is the newest
 * contiguous run, so nothing older can belong to it) and is kept. This can drop
 * a bucket that happens to be whole, when the series holds no bar on the
 * bucket's first second; that costs one candle and is said, where keeping it
 * would risk a wrong one.
 *
 * @param {TailBar[]} raw the bars read, in any order
 * @param {number | null} seconds the drawn rung's bucket length
 * @param {number} offset the bucketing offset `foldBars` is given
 * @param {boolean} olderExist whether the series holds bars older than the read
 * @returns {{ bars: TailBar[], cut: { bucket: number, until: number, dropped: number, first: number, last: number } | null }}
 */
export function tailCut(raw, seconds, offset, olderExist) {
  const bars = Array.isArray(raw) ? raw : [];
  if (!olderExist || bars.length === 0 || seconds === null || !(seconds > 0)) return { bars, cut: null };
  let oldest = Infinity;
  for (const b of bars) if (Number.isFinite(b?.t) && b.t < oldest) oldest = b.t;
  if (oldest === Infinity) return { bars, cut: null };
  const bucket = Math.floor((oldest + offset) / seconds) * seconds - offset;
  if (oldest === bucket) return { bars, cut: null };
  const until = bucket + seconds;
  /** @type {TailBar[]} */
  const kept = [];
  let dropped = 0;
  let last = oldest;
  for (const b of bars) {
    if (Number.isFinite(b?.t) && b.t < until) {
      dropped += 1;
      if (b.t > last) last = b.t;
    } else kept.push(b);
  }
  return { bars: kept, cut: { bucket, until, dropped, first: oldest, last } };
}
