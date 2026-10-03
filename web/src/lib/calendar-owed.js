/**
 * Fold per-day exchange and spot-index minute denominators to month grain.
 *
 * `indexOwed` is deliberately independent of `exchangeOwed`. A market can be
 * open while an index is not being computed; copying the exchange count into a
 * missing index count would turn an unknown into a precise-looking loss.
 *
 * @param {string} from inclusive ISO day
 * @param {string} to inclusive ISO day
 * @param {Map<string, number|null>} exchangeOwed
 * @param {Map<string, number|null>} indexOwed
 * @param {(day: string, n: number) => string} addDay
 * @returns {{
 *   exchange: Map<string, number>,
 *   index: Map<string, number>,
 *   indexUnknown: Set<string>
 * }}
 */
export function foldMinuteOwed(from, to, exchangeOwed, indexOwed, addDay) {
  /** @type {Map<string, number>} */
  const exchange = new Map();
  /** @type {Map<string, number>} */
  const index = new Map();
  /** @type {Set<string>} */
  const indexUnknown = new Set();
  if (!from || !to || from > to) return { exchange, index, indexUnknown };
  let day = from;
  while (day <= to) {
    const owed = exchangeOwed.get(day);
    if (typeof owed === 'number') {
      const month = day.slice(0, 7);
      exchange.set(month, (exchange.get(month) ?? 0) + owed);
      const indexCount = indexOwed.get(day);
      if (typeof indexCount === 'number') {
        index.set(month, (index.get(month) ?? 0) + indexCount);
      } else {
        // One unproved index day withholds the WHOLE month's denominator. A
        // partial sum would still look exact and could be ranked as complete.
        indexUnknown.add(month);
      }
    }
    day = addDay(day, 1);
  }
  return { exchange, index, indexUnknown };
}

/**
 * The days `/calendar.json` WITHHOLDS, as ISO days and as `YYYY-MM` months.
 *
 * A withheld day is inside the calendar's span and absent from `days`, exactly
 * as a holiday is — the difference is that nobody proved the exchange shut: the
 * month's daily rung was not read, or failed its checks (R9-api-law-0, D-1443).
 * Reading such a day as a holiday is the failure dressed as a fact that
 * `CLAUDE.md` §4 bans, so the page asks this set first. D-1507.
 *
 * Each run is `{from, to}` in epoch days, inclusive. A run that is not two
 * integers, or runs backwards, is REFUSED rather than skipped: dropping it would
 * turn its days back into holidays.
 *
 * Cost: one step per withheld day — the server withholds whole months, so this
 * is bounded by the days the store cannot speak for, not by the span.
 *
 * @param {unknown} runs `body.withheld`; `undefined` from an older API is no runs
 * @param {(day: number) => string} isoOfEpochDay
 * @returns {{ days: Set<string>, months: Set<string> }}
 */
export function withheldDays(runs, isoOfEpochDay) {
  /** @type {Set<string>} */
  const days = new Set();
  /** @type {Set<string>} */
  const months = new Set();
  if (runs === undefined || runs === null) return { days, months };
  if (!Array.isArray(runs)) throw new Error('/calendar.json `withheld` is not a list');
  for (const run of runs) {
    const from = run?.from;
    const to = run?.to;
    if (!Number.isSafeInteger(from) || !Number.isSafeInteger(to) || to < from) {
      throw new Error(`/calendar.json \`withheld\` carries a malformed run: ${JSON.stringify(run)}`);
    }
    for (let day = from; day <= to; day += 1) {
      const iso = isoOfEpochDay(day);
      days.add(iso);
      months.add(iso.slice(0, 7));
    }
  }
  return { days, months };
}
