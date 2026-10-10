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
 * The per-day minute denominators of a D-3336 `/calendar.json` answer.
 *
 * Since D-3336 the server answers the calendar of record as a dense string,
 * `days`, one code per epoch day from `from`: `.` closed, `R` and `X` regular
 * sessions, `P` a special session and `H` one with a suspected halt (each
 * sized in `special`), `U` a session of unrecorded length, `-` outside the
 * calendar. Minutes are read from `minuteOwed` and `special[].minuteOwed`,
 * whatever rung the answer was built at, because this page's denominators
 * are one-minute bars. A closed day is omitted, as the older array did; an
 * unsized day is `null`, never `0` or `375`.
 *
 * @param {any} body
 * @returns {{ owed: Map<string, number|null>, indexOwed: Map<string, number|null> }}
 */
export function denseOwed(body) {
  /** @type {Map<string, number|null>} */
  const owed = new Map();
  /** @type {Map<string, number|null>} */
  const indexOwed = new Map();
  const days = typeof body?.days === 'string' ? body.days : '';
  const from = body?.from;
  if (!days || !Number.isSafeInteger(from)) return { owed, indexOwed };
  const regular = Number.isSafeInteger(body?.minuteOwed?.regular) ? body.minuteOwed.regular : null;
  /** @type {Map<number, any>} */
  const special = new Map();
  for (const entry of Array.isArray(body?.special) ? body.special : []) special.set(entry?.day, entry);
  for (let at = 0; at < days.length; at += 1) {
    const code = days[at];
    if (code === '.' || code === '-') continue;
    const day = from + at;
    const iso = isoOfEpochDay(day);
    const sized = special.get(day);
    if (sized) {
      owed.set(iso, sized.minuteOwed ?? null);
      // MISSING IS UNKNOWN, NEVER "SAME AS EXCHANGE" (D-0420).
      indexOwed.set(iso, sized.minuteIndexOwed ?? null);
    } else if (code === 'R' || code === 'X') {
      owed.set(iso, regular);
      indexOwed.set(iso, regular);
    } else {
      owed.set(iso, null);
      indexOwed.set(iso, null);
    }
  }
  return { owed, indexOwed };
}

/**
 * How many times a `"building"` calendar is asked for again before the page
 * shows it as still building: at the server's 250 ms hint, about half a
 * minute, longer than the slowest first build measured (6.55 s, DB-N19).
 */
export const CALENDAR_BUILDING_TRIES = 120;

/**
 * Wait `ms`, or until `signal` aborts.
 *
 * @param {number} ms
 * @param {AbortSignal | undefined} signal
 * @returns {Promise<void>}
 */
function pause(ms, signal) {
  return new Promise((resolve, reject) => {
    if (signal?.aborted) {
      reject(signal.reason ?? new Error('aborted'));
      return;
    }
    const timer = setTimeout(resolve, ms);
    signal?.addEventListener(
      'abort',
      () => {
        clearTimeout(timer);
        reject(signal.reason ?? new Error('aborted'));
      },
      { once: true }
    );
  });
}

/**
 * `/calendar.json` for `feed`, asked again while the server answers `202`
 * `"building"` (DB-N19): it builds a calendar off the request path and names
 * that state rather than holding the request for seconds. The last response
 * and, when it was read, its body.
 *
 * @param {string} url
 * @param {(url: string, init: RequestInit) => Promise<Response>} request
 * @param {AbortSignal} signal
 * @param {(ms: number, signal: AbortSignal) => Promise<void>} wait
 * @returns {Promise<{ response: Response, body: any }>}
 */
async function settledCalendar(url, request, signal, wait) {
  for (let asked = 1; ; asked += 1) {
    const response = await request(url, { cache: 'no-store', signal });
    if (response.status !== 202) return { response, body: undefined };
    const body = await response.json();
    if (body?.status !== 'building' || asked >= CALENDAR_BUILDING_TRIES) return { response, body };
    const hint = Number(body.retry_after_ms);
    await wait(Number.isFinite(hint) && hint >= 50 ? hint : 250, signal);
  }
}

/**
 * Read one feed's `/calendar.json` into the page's calendar. Never throws: a
 * refusal or a failure is an empty calendar that names its reason. A calendar
 * the server is still building is asked for again at its hint, and one still
 * building after [`CALENDAR_BUILDING_TRIES`] is shown empty with the server's
 * own reason — never as a calendar with no sessions.
 *
 * @param {string} feed
 * @param {(url: string, init: RequestInit) => Promise<Response>} request
 * @param {AbortSignal} signal
 * @param {(ms: number, signal: AbortSignal) => Promise<void>} [wait]
 * @returns {Promise<Calendar>}
 */
export async function readCalendar(feed, request, signal, wait = pause) {
  try {
    const { response, body: building } = await settledCalendar(
      `/calendar.json?feed=${encodeURIComponent(feed)}`,
      request,
      signal,
      wait
    );
    // THE REASON IS READ, NOT DROPPED (CE-83, D-1789): a 429 names the
    // derivation ceiling and a 400 the unknown feed, and both say what to do.
    if (!response.ok) return emptyCalendar(await refusalFrom('/calendar.json', response));
    if (building?.status === 'building') {
      return emptyCalendar(
        `the server is still building this calendar: ${building.reason ?? 'no reason given'}`
      );
    }
    const body = building ?? (await response.json());
    /** @type {Map<string, number|null>} */
    let owed = new Map();
    /** @type {Map<string, number|null>} */
    let indexOwed = new Map();
    if (typeof body?.days === 'string') {
      // THE D-3336 CALENDAR OF RECORD: a dense day-status string.
      ({ owed, indexOwed } = denseOwed(body));
    } else {
      for (const entry of body?.days ?? []) {
        const day = isoOfEpochDay(entry.day);
        owed.set(day, entry.owed ?? null);
        // MISSING IS UNKNOWN, NEVER "SAME AS EXCHANGE". An older API does not
        // carry the index-specific field and therefore cannot prove a common
        // index denominator on the systems-outage day. Falling back to `owed`
        // would silently restore the false 166-hole claim D-0420 refuses.
        indexOwed.set(day, entry.indexOwed ?? null);
      }
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
      // A WITHHELD ANSWER NAMES ITS REASON (D-3336): read it, never guess one.
      why: owed.size
        ? ''
        : typeof body?.reason === 'string' && body.reason
          ? body.reason
          : 'the store holds no bars for this feed'
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
