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
 * `/calendar.json` at `url` for any page, asked again while the server answers
 * `202` `"building"` (DB-N19), at its hint, at most [`CALENDAR_BUILDING_TRIES`]
 * times. `/db` reads its calendar of record through this (WP-50's date
 * fields): read once with a bare `fetch`, a `202` is `ok`, so the first click
 * after a restart showed "answered with no day string" for good instead of
 * the calendar a moment later.
 *
 * The last response, and its body: the parsed JSON of an `ok` answer -- a
 * calendar, or one still `"building"` after every try, which the caller must
 * show as building -- and `null` for a refusal, whose body is left unread for
 * the caller's own reason. Throws what the request or the wait throws (an
 * abort among them).
 *
 * @param {string} url
 * @param {(url: string, init: RequestInit) => Promise<Response>} request
 * @param {AbortSignal} signal
 * @param {(ms: number, signal: AbortSignal) => Promise<void>} [wait]
 * @returns {Promise<{ response: Response, body: any }>}
 */
export async function askCalendar(url, request, signal, wait = pause) {
  const { response, body } = await settledCalendar(url, request, signal, wait);
  if (!response.ok) return { response, body: null };
  return { response, body: body ?? (await response.json()) };
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

/* ======================================================================
   THE CALENDAR OF RECORD, FOLDED FOR COVERAGE (D-3336 §1-2, WP-54)
   ----------------------------------------------------------------------
   /db judged every instrument-month against the FULLEST stored series at the
   same month and rung (DB-014, DB-G34, DB-G63). A hole every index shares
   read "full" (Zerodha 2024-11: every index 1,875 bars, 5 of 19 sessions),
   a share in a closing-auction month read short against the index (7,560 =
   21 x 360 against 7,875), and a one-second option read 0.0065% covered
   against an index's every second. The denominator is now the calendar's:
   `/calendar.json?feed&rung` answers a dense day-status string and the bars
   each kind of day owes AT THAT RUNG (`owed.regular`, `owed.casEligible`,
   `special[].owed`), built by the server from `pull::calendar` and
   `pull::fold`, the one calendar and the one fold. Nothing here re-derives a
   session length: every count below is the server's.

   `owedLedger` folds one answer into prefix sums once, so the bars owed over
   ANY day span — a month, a month from a series' first bar, a contract's life
   inside a month — is two array reads (`ledgerSpan`). A census of 250,025
   Zerodha rows is judged in one Map probe and two subtractions a row.
   ====================================================================== */

/** The day codes that are sessions: owed something, or of unknown length. */
const SESSION_CODES = new Set(['R', 'X', 'P', 'H', 'U']);

/**
 * The epoch day of `YYYY-MM-DD` (UTC midnight, as every day on these pages).
 *
 * @param {string} iso
 * @returns {number}
 */
export function epochDayOfIso(iso) {
  return Math.round(
    Date.UTC(Number(iso.slice(0, 4)), Number(iso.slice(5, 7)) - 1, Number(iso.slice(8, 10))) /
      86_400_000
  );
}

/**
 * The first and last epoch day of a `YYYY-MM` month.
 *
 * @param {string} month
 * @returns {{ first: number, last: number }}
 */
export function monthSpan(month) {
  const y = Number(month.slice(0, 4));
  const m = Number(month.slice(5, 7));
  return {
    first: Math.round(Date.UTC(y, m - 1, 1) / 86_400_000),
    last: Math.round(Date.UTC(y, m, 0) / 86_400_000)
  };
}

/**
 * Every `YYYY-MM` from `from` to `to` inclusive, in order. Empty when `from`
 * is after `to`. Bounded by the caller's two months, never by a store.
 *
 * @param {string} from
 * @param {string} to
 * @returns {string[]}
 */
export function monthsBetween(from, to) {
  /** @type {string[]} */
  const out = [];
  let y = Number(from.slice(0, 4));
  let m = Number(from.slice(5, 7));
  const endY = Number(to.slice(0, 4));
  const endM = Number(to.slice(5, 7));
  while (y < endY || (y === endY && m <= endM)) {
    out.push(`${String(y).padStart(4, '0')}-${String(m).padStart(2, '0')}`);
    m += 1;
    if (m > 12) {
      m = 1;
      y += 1;
    }
  }
  return out;
}

/**
 * ONE FEED'S CALENDAR OF RECORD AT ONE RUNG, as prefix sums.
 *
 * `index`, `share` and `cas` are the three ways a day can be owed:
 * - `index` — an index (and a future): a regular or auction day owes
 *   `owed.regular`; a special or halt day its `indexOwed` (the systems-outage
 *   day of 2021-02-24 has none: SEBI recorded no common index computation,
 *   D-0420, so it is UNKNOWN, never the exchange's 220).
 * - `share` — a share that is NOT closing-auction eligible: an auction day
 *   owes `owed.regular`.
 * - `cas` — a CAS-eligible share from 2026-08-03 (D-3320): an auction day
 *   owes `owed.casEligible` (360 at 1min: Zerodha's landing ends at 15:14).
 *
 * A `U` day (a session of unmeasured length) and a special day with no count
 * are UNKNOWN in the kind they lack: counted in `unknown*`, contributing
 * nothing to the owed sum, so a reader can say "owed excludes N days" rather
 * than invent a length (D-3336 §1, CLAUDE.md §3 rule 1).
 *
 * Refuses a body with no day string or a held string of the wrong length:
 * a malformed calendar is a named failure, never an empty one.
 *
 * @param {any} body a `/calendar.json` answer with `status: "ok"`
 * @returns {OwedLedger}
 */
export function owedLedger(body) {
  const codes = typeof body?.days === 'string' ? body.days : null;
  const from = body?.from;
  if (codes === null || codes.length === 0 || !Number.isSafeInteger(from)) {
    throw new Error(
      `/calendar.json carried no day string${typeof body?.reason === 'string' && body.reason ? ` (${body.reason})` : ''}`
    );
  }
  const n = codes.length;
  const heldText = typeof body.held === 'string' ? body.held : null;
  if (heldText !== null && heldText.length !== n) {
    throw new Error(`/calendar.json held string is ${heldText.length} days against ${n} day codes`);
  }
  const int = (/** @type {unknown} */ v) => (Number.isSafeInteger(v) ? /** @type {number} */ (v) : null);
  const regular = int(body?.owed?.regular);
  const casEligible = int(body?.owed?.casEligible);
  /** @type {Map<number, any>} */
  const special = new Map();
  for (const entry of Array.isArray(body.special) ? body.special : []) {
    if (Number.isSafeInteger(entry?.day)) special.set(entry.day, entry);
  }
  /** @type {Map<number, any>} */
  const halts = new Map();
  for (const halt of Array.isArray(body.halts) ? body.halts : []) {
    if (Number.isSafeInteger(halt?.day)) halts.set(halt.day, halt);
  }
  const prefix = () => new Float64Array(n + 1);
  const sessions = prefix();
  const index = prefix();
  const share = prefix();
  const cas = prefix();
  const unknownIndex = prefix();
  const unknownShare = prefix();
  const unknownCas = prefix();
  const auction = prefix();
  const specials = prefix();
  const halted = prefix();
  const unmeasured = prefix();
  // BARS A HALT LEAVES UNOWED: a halt day owes its regular count less the
  // halted minutes (or seconds); a series that kept printing frozen bars
  // through them holds those bars, labelled halted, neither owed nor extra.
  const haltBars = prefix();
  const heldDays = prefix();
  const heldSessions = prefix();
  const closedHeld = prefix();
  for (let at = 0; at < n; at += 1) {
    const code = codes[at];
    const day = from + at;
    const session = SESSION_CODES.has(code);
    /** @type {number | null} */
    let i = 0;
    /** @type {number | null} */
    let s = 0;
    /** @type {number | null} */
    let c = 0;
    if (code === 'R') {
      i = regular;
      s = regular;
      c = regular;
    } else if (code === 'X') {
      i = regular;
      s = regular;
      c = casEligible;
    } else if (code === 'P' || code === 'H') {
      const sized = special.get(day);
      i = int(sized?.indexOwed);
      s = int(sized?.owed);
      c = s;
    } else if (code === 'U') {
      i = null;
      s = null;
      c = null;
    }
    const held = heldText === null ? 0 : heldText[at] !== '-' && heldText[at] !== '0' ? 1 : 0;
    sessions[at + 1] = sessions[at] + (session ? 1 : 0);
    index[at + 1] = index[at] + (i ?? 0);
    share[at + 1] = share[at] + (s ?? 0);
    cas[at + 1] = cas[at] + (c ?? 0);
    unknownIndex[at + 1] = unknownIndex[at] + (session && i === null ? 1 : 0);
    unknownShare[at + 1] = unknownShare[at] + (session && s === null ? 1 : 0);
    unknownCas[at + 1] = unknownCas[at] + (session && c === null ? 1 : 0);
    auction[at + 1] = auction[at] + (code === 'X' ? 1 : 0);
    specials[at + 1] = specials[at] + (code === 'P' ? 1 : 0);
    halted[at + 1] = halted[at] + (code === 'H' ? 1 : 0);
    unmeasured[at + 1] = unmeasured[at] + (code === 'U' ? 1 : 0);
    haltBars[at + 1] =
      haltBars[at] + (code === 'H' && regular !== null && s !== null ? Math.max(0, regular - s) : 0);
    heldDays[at + 1] = heldDays[at] + held;
    heldSessions[at + 1] = heldSessions[at] + (session ? held : 0);
    closedHeld[at + 1] = closedHeld[at] + (code === '.' ? held : 0);
  }
  const heldBy = Array.isArray(body.heldBy) ? body.heldBy.map(String) : [];
  const target = body?.target ?? null;
  return {
    feed: typeof body.feed === 'string' ? body.feed : '',
    rung: typeof body.rung === 'string' ? body.rung : '',
    from,
    to: from + n - 1,
    codes,
    held: heldText,
    heldBy,
    // THE SERIES THE HELD DIGITS COUNT, when the answer was asked for one:
    // `symbol` and `segment` of `target`, else `null` (the exchange's, held
    // by NIFTY and BANKNIFTY).
    series:
      target?.kind === 'series' && typeof target.symbol === 'string'
        ? { symbol: target.symbol, segment: String(target.segment ?? '') }
        : null,
    regular,
    casEligible,
    special,
    halts,
    sum: { sessions, index, share, cas, unknownIndex, unknownShare, unknownCas, auction, specials, halted, unmeasured, haltBars, heldDays, heldSessions, closedHeld }
  };
}

/**
 * @typedef {object} OwedLedger
 * @property {string} feed
 * @property {string} rung
 * @property {number} from first epoch day the calendar answers for
 * @property {number} to last epoch day
 * @property {string} codes one day code per day
 * @property {string | null} held one digit per day, or `null`
 * @property {string[]} heldBy
 * @property {{ symbol: string, segment: string } | null} series
 * @property {number | null} regular bars a regular session owes at this rung
 * @property {number | null} casEligible bars an auction day owes a CAS-eligible share
 * @property {Map<number, any>} special
 * @property {Map<number, any>} halts
 * @property {Record<string, Float64Array>} sum prefix sums, one more than days
 */

/**
 * What a ledger says about the days `fromDay..=toDay` (epoch days). Two
 * array reads per field: O(1) whatever the span.
 *
 * `outside` is how many asked days lie outside the calendar's own span: they
 * are owed nothing because nobody can say, never because they were closed.
 *
 * @param {OwedLedger} ledger
 * @param {number} fromDay
 * @param {number} toDay
 */
export function ledgerSpan(ledger, fromDay, toDay) {
  const asked = Math.max(0, toDay - fromDay + 1);
  const lo = Math.max(fromDay, ledger.from);
  const hi = Math.min(toDay, ledger.to);
  /** @type {Record<string, number>} */
  const out = {};
  for (const [name, sums] of Object.entries(ledger.sum)) {
    out[name] = hi < lo ? 0 : sums[hi - ledger.from + 1] - sums[lo - ledger.from];
  }
  return {
    asked,
    inside: hi < lo ? 0 : hi - lo + 1,
    outside: asked - (hi < lo ? 0 : hi - lo + 1),
    sessions: out.sessions,
    index: out.index,
    share: out.share,
    cas: out.cas,
    unknownIndex: out.unknownIndex,
    unknownShare: out.unknownShare,
    unknownCas: out.unknownCas,
    auction: out.auction,
    specials: out.specials,
    halted: out.halted,
    unmeasured: out.unmeasured,
    haltBars: out.haltBars,
    heldDays: out.heldDays,
    heldSessions: out.heldSessions,
    closedHeld: out.closedHeld
  };
}

/**
 * One day of a ledger: its code, the held digit, and its special or halt
 * record. `null` outside the ledger's span.
 *
 * @param {OwedLedger} ledger
 * @param {number} day epoch day
 */
export function ledgerDay(ledger, day) {
  if (day < ledger.from || day > ledger.to) return null;
  const at = day - ledger.from;
  const digit = ledger.held === null ? null : ledger.held[at];
  return {
    day,
    code: ledger.codes[at],
    held: digit === null || digit === '-' ? null : Number(digit),
    special: ledger.special.get(day) ?? null,
    halt: ledger.halts.get(day) ?? null
  };
}

/**
 * The `/calendar.json` URL for one feed, rung and (optionally) one series.
 * The series' segment travels with its symbol so a share is never answered
 * with an index of the same name.
 *
 * @param {string} feed
 * @param {string} rung
 * @param {{ symbol: string, segment: string } | null} [series]
 */
export function calendarUrl(feed, rung, series = null) {
  const q = new URLSearchParams({ feed, rung });
  if (series) {
    q.set('symbol', series.symbol);
    q.set('segment', series.segment);
  }
  return `/calendar.json?${q}`;
}

/**
 * Read one ledger, never throwing: `{ ledger }` or `{ why }` naming the
 * refusal, the build still in progress, or the failure. A calendar still
 * building after every try is a `why`, never a ledger with no sessions.
 *
 * @param {string} url
 * @param {(url: string, init: RequestInit) => Promise<Response>} request
 * @param {AbortSignal} signal
 * @param {(ms: number, signal: AbortSignal) => Promise<void>} [wait]
 * @returns {Promise<{ ledger: OwedLedger | null, why: string }>}
 */
export async function readLedger(url, request, signal, wait = pause) {
  try {
    const { response, body } = await askCalendar(url, request, signal, wait);
    if (!response.ok) return { ledger: null, why: await refusalFrom('/calendar.json', response) };
    if (body?.status === 'building') {
      return {
        ledger: null,
        why: `the server is still building this calendar: ${body.reason ?? 'no reason given'}`
      };
    }
    if (body?.status === 'withheld') {
      return { ledger: null, why: `the calendar is withheld: ${body.reason ?? 'no reason given'}` };
    }
    return { ledger: owedLedger(body), why: '' };
  } catch (error) {
    if (signal.aborted) throw error;
    return {
      ledger: null,
      why: `the calendar could not be read (${error instanceof Error ? error.message : String(error)})`
    };
  }
}
