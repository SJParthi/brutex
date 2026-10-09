import { refusalFrom } from './refusal.js';
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

/**
 * An epoch day as `YYYY-MM-DD`. UTC midnight, like every other date on /ingest.
 *
 * @param {number} day
 * @returns {string}
 */
export function isoOfEpochDay(day) {
  return new Date(day * 86_400_000).toISOString().slice(0, 10);
}

/**
 * @typedef {{
 *   first: string, last: string,
 *   owed: Map<string, number|null>,
 *   indexOwed: Map<string, number|null>,
 *   withheld: Set<string>, withheldMonths: Set<string>,
 *   from: string[], clashes: number, why: string
 * }} Calendar
 */

/**
 * The calendar that measures nothing. `why` is the reason; `''` reads as "it
 * has not loaded yet" on the page.
 *
 * @param {string} why
 * @returns {Calendar}
 */
export function emptyCalendar(why) {
  return {
    first: '',
    last: '',
    owed: new Map(),
    indexOwed: new Map(),
    withheld: new Set(),
    withheldMonths: new Set(),
    from: [],
    clashes: 0,
    why
  };
}

/**
 * Read one feed's `/calendar.json` into the page's calendar. Never throws: a
 * refusal or a failure is an empty calendar that names its reason.
 *
 * @param {string} feed
 * @param {(url: string, init: RequestInit) => Promise<Response>} request
 * @param {AbortSignal} signal
 * @returns {Promise<Calendar>}
 */
export async function readCalendar(feed, request, signal) {
  try {
    const response = await request(`/calendar.json?feed=${encodeURIComponent(feed)}`, {
      cache: 'no-store',
      signal
    });
    // THE REASON IS READ, NOT DROPPED (CE-83, D-1789): a 429 names the
    // derivation ceiling and a 400 the unknown feed, and both say what to do.
    if (!response.ok) return emptyCalendar(await refusalFrom('/calendar.json', response));
    const body = await response.json();
    /** @type {Map<string, number|null>} */
    const owed = new Map();
    /** @type {Map<string, number|null>} */
    const indexOwed = new Map();
    for (const entry of body?.days ?? []) {
      const day = isoOfEpochDay(entry.day);
      owed.set(day, entry.owed ?? null);
      // MISSING IS UNKNOWN, NEVER "SAME AS EXCHANGE". An older API does not
      // carry the index-specific field and therefore cannot prove a common
      // index denominator on the systems-outage day. Falling back to `owed`
      // would silently restore the false 166-hole claim D-0420 refuses.
      indexOwed.set(day, entry.indexOwed ?? null);
    }
    // A DAY ABSENT FROM `days` IS A HOLIDAY ONLY IF IT IS NOT WITHHELD. The
    // server names, in `withheld`, every stretch whose daily rung it did not
    // read or could not trust (R9-api-law-0, D-1443); a malformed list throws
    // into the catch below and the whole calendar degrades loudly, rather than
    // its days becoming holidays. D-1507.
    const withheld = withheldDays(body?.withheld, isoOfEpochDay);
    return {
      first: owed.size ? isoOfEpochDay(body.firstDay) : '',
      last: owed.size ? isoOfEpochDay(body.lastDay) : '',
      owed,
      indexOwed,
      withheld: withheld.days,
      withheldMonths: withheld.months,
      from: body?.derivedFrom ?? [],
      clashes: (body?.disagreements ?? []).length,
      why: owed.size ? '' : 'the store holds no bars for this feed'
    };
  } catch (error) {
    return emptyCalendar(
      error instanceof Error
        ? `the calendar request failed (${error.message})`
        : 'the calendar request failed'
    );
  }
}

/**
 * ONE CALENDAR READ AT A TIME, AND ONLY THE CURRENT FEED'S MAY LAND (CE-71,
 * D-2732).
 *
 * `/ingest` read `/calendar.json` with no ticket: switching feed A to B while
 * A's read was in flight let A's slower answer replace B's calendar, and A's
 * calendar stood under B (and under "no feed") until anything answered. That
 * calendar is the denominator for the "NSE holiday" marks, the month span and
 * every owed-minute count in the coverage meter.
 *
 * `load(feed)` now clears the calendar to "not loaded" at once, revokes the
 * read in flight, and applies an answer only while its ticket is current —
 * `createPageRequests`' generation, which revokes a late reply even when the
 * transport ignores the abort.
 *
 * @param {{
 *   request: (url: string, init: RequestInit) => Promise<Response>,
 *   apply: (calendar: Calendar) => void,
 *   requests: { run: (work: (ticket: { signal: AbortSignal, current: () => boolean }) => Promise<void>) => Promise<void>, cancel: () => void }
 * }} options
 */
export function createCalendarLoader({ request, apply, requests }) {
  return {
    /** @param {string | null | undefined} feed */
    load(feed) {
      requests.cancel();
      if (!feed) {
        apply(emptyCalendar('no feed is selected'));
        return Promise.resolve();
      }
      apply(emptyCalendar(''));
      return requests.run(async (ticket) => {
        const calendar = await readCalendar(feed, request, ticket.signal);
        if (ticket.current()) apply(calendar);
      });
    }
  };
}
