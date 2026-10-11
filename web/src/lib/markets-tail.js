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
