/**
 * HOW /ingest JUDGES ONE MONTH FILE AND ONE SERIES — the verdicts, the pull
 * journal's refusals joined to them, the span a feed can hold, the default
 * window and the spans one row's pull asks for (WP-61; DB-G20, DB-G23,
 * DB-B38, DB-B46).
 *
 * Pure, so `web/tests/ingest-months.test.js` drives every branch under
 * `node --test`. The page's census table calls these and lays the answers out.
 *
 * WHAT WAS WRONG, FINDING BY FINDING
 *
 *   DB-G23  A month the store holds no file for read "never pulled — nothing
 *           has asked the vendor for it yet", while the pull journal held that
 *           instrument's refusal for a day in that very month (a Muhurat the
 *           calendar could not size). `/ingest/refusals.json` serves the
 *           journal's refusals by month, and a month that is missing or short
 *           AND refused reads `requested and refused`, with the reason.
 *   DB-B38  One series holding 79 of 142 months read `short`, whose tooltip
 *           says "Every month file in this window has been written". A series
 *           with bars AND months that were never written is `partly never
 *           pulled`. The window opened at 2015-01-01, outside the operator's
 *           2018-01-01 window. A row's "Pull 63" asked the whole span from the
 *           first unsettled month to the last, all 142 months.
 *   DB-B46  GDFL was judged over 2015-01..2026-10 because the page read only
 *           `history`, which states nothing for an archive vendor. `/feeds.json`
 *           states the archive's measured span (`archive`); months outside it
 *           are `out of reach`, and the newest askable day is the archive's.
 *   DB-G20  The series verdict for the cell in flight is `in flight`, on the
 *           exact row only (`$lib/ingest-status.js` `flightRow`).
 */

/** The verdict keys, worst first: the order is a claim about what to do. */
export const VERDICT_ORDER = /** @type {const} */ ([
  'fail', 'refused', 'unread', 'never', 'partial', 'short', 'flight', 'unknown', 'beyond', 'ok'
]);

/** @typedef {typeof VERDICT_ORDER[number]} Verdict */

/**
 * [tone, label, what it means], spelled once. Read for the chip, the pill and
 * the sort ordinal.
 * @type {Record<Verdict, [string, string, string]>}
 */
export const VERDICTS = {
  fail: [
    'down',
    'failed',
    'The sweep ladder for this feed has HALTED and this series still has month files that are empty or short. Attempts have stopped — the halt reason is on the strip above, and a retry on its own will not help.'
  ],
  refused: [
    'down',
    'requested and refused',
    'At least one month file in this window is missing or short, AND the pull journal holds this instrument’s refusal for a day in that month (or for a whole window that covers it): it WAS asked for, and it was refused. The reason, as journalled, is in the cell’s title. A re-pull meets the same refusal until its cause is fixed, so read the reason before pressing Pull. The journal records no timeframe, so the refusal is this instrument’s for that day, at whichever rung was asked.'
  ],
  unread: [
    'down',
    'refusals unread',
    'At least one month file in this window is missing or short, and the pull journal’s refusals for this instrument were NOT read: the server could not read them, or the read failed or has not answered. Whether the month was asked for and refused is therefore NOT KNOWN, and neither “never pulled” nor “short” is claimed, because both say the journal names no refusal. The reason the refusals are unread is in the cell’s title and on the notice above the table. The bars stored are counted and exact.'
  ],
  never: [
    'warn',
    'never pulled',
    'No month file in this window holds a bar for this series, and the pull journal’s refusal index names no refusal of this instrument for any of them. That is one of two things, and this page cannot tell them apart: nothing has asked the vendor for it, or it was asked and the vendor answered with no bars and no refusal (a month before the instrument was listed reads this way: the journal indexes refusals per instrument, not the windows that were asked). The run records on /audit name the windows that were asked. A pull is warranted unless the instrument did not trade in these months.'
  ],
  partial: [
    'warn',
    'partly never pulled',
    'This series holds bars, and at least one month file in this window has never been written and has no refusal in the pull journal. The cells show which months: a pull of those months is warranted, and the row’s Pull asks for exactly those.'
  ],
  short: [
    'warn',
    'short',
    'Every month file in this window has been written, and at least one holds fewer bars than the NSE calendar expects for the days inside the window. The pull journal’s refusal index names no refusal for those months. That index holds refusals only: a member failure that is not a refusal (a minute coverage gap inside a day that was answered, for one) is NOT joined here and may exist; /dayflags and /audit list those by day.'
  ],
  flight: [
    'info',
    'in flight',
    '/ingest/status.json names exactly this row in flight right now: this feed, this timeframe, this instrument, the spot segment. Nothing is waiting on you; it settles into verified, short, refused or failed on its own.'
  ],
  unknown: [
    'info',
    'no yardstick',
    'This timeframe has no bars-per-session this page can state, so what is stored cannot be compared with the calendar. The bars are counted; the verdict is withheld rather than guessed.'
  ],
  beyond: [
    'info',
    'out of reach',
    'Every month file in this window lies outside what this feed holds or answers for at this timeframe: older than its floor, or outside the span its archive measures. No attempt was made and none would help — move the dates, the feed or the timeframe.'
  ],
  ok: [
    'up',
    'verified',
    'Every month file in the window holds at least the bars the NSE calendar expects for the days inside it.'
  ]
};

/**
 * THE SAME VERDICTS IN AN IMPORTED FEED'S WORDS (WP-61 repair).
 *
 * An archive feed's bars come from an import run, not from a request this page
 * can send, and it has no pull journal. Its rows read "partly never pulled …
 * a pull of those months is warranted, and the row's Pull asks for exactly
 * those" on a feed whose rows offer no Pull at all. The keys, tones and order
 * are the one table's; only the label and the sentence change for the three
 * verdicts that name a pull. Chosen by the feed's own `archive` field (the
 * caller passes `feedSpan(feed).kind === 'archive'`), never by a vendor name.
 *
 * @param {boolean} imported
 * @returns {Record<Verdict, readonly [string, string, string]>}
 */
export function verdictWords(imported) {
  if (!imported) return VERDICTS;
  return {
    ...VERDICTS,
    never: [
      VERDICTS.never[0],
      'never imported',
      'No month file in this window holds a bar for this series. This feed is imported from an archive, not pulled: either no import has reached the archive days of these months, or the archive holds no file of this series for them. The import panel above lists the days not done and the days refused, with reasons. No pull can be asked for; an import is what fills it.'
    ],
    partial: [
      VERDICTS.partial[0],
      'partly never imported',
      'This series holds bars, and at least one month file in this window has never been written. This feed is imported from an archive, not pulled: either the import has not reached those archive days, or the archive holds no file of this series for them (an instrument that was not listed yet, or was renamed, reads this way). The cells show which months; the import panel above lists the days not done and the days refused. No pull can be asked for.'
    ],
    short: [
      VERDICTS.short[0],
      'short',
      'Every month file in this window has been written, and at least one holds fewer bars than the NSE calendar expects for the days inside the window. This feed is imported from an archive, not pulled, so no pull journal exists to name a refusal: the import panel above lists the days not done and the days refused, and /db shows the day band of what is held.'
    ]
  };
}

/**
 * HOW MANY WHOLE-WINDOW REFUSALS OF `symbol` THE SERVER COUNTED AND DID NOT
 * LIST (WP-61 repair). `/ingest/refusals.json` lists the newest few and counts
 * the rest (`whole_count`); a month that only an unlisted one covers was read
 * as "no refusal", so it fell to "never pulled". With any unlisted, "the
 * journal names no refusal for this month" cannot be said of a month no listed
 * refusal covers: the caller marks such a month unread, with this count.
 *
 * @param {Map<string, SymbolRefusals> | null | undefined} bySymbol
 * @param {string} symbol
 * @returns {number} 0 when every whole-window refusal is listed
 */
export function wholeUnlisted(bySymbol, symbol) {
  const held = bySymbol instanceof Map ? bySymbol.get(symbol) : undefined;
  if (!held) return 0;
  const more = held.wholeCount - held.whole.length;
  return Number.isSafeInteger(more) && more > 0 ? more : 0;
}

/**
 * WHERE AN ARCHIVE'S MEASURED SPAN FALLS IN THE CHOSEN WINDOW (WP-61 repair).
 *
 * The reach strip said "reach not stated" for an imported feed whose
 * descriptor states its archive's first and last day on `/feeds.json`. The
 * span is the reach: nothing outside it can be imported. Percentages are of
 * the window, exact on three UTC midnights.
 *
 * @param {ReturnType<typeof feedSpan>} span
 * @param {string} from the window's first day, ISO
 * @param {string} to the window's last day, ISO
 * @returns {{ from: string, to: string, beforePct: number, livePct: number, afterPct: number } | null}
 *   `null` when the feed states no archive span or the window is not a span
 */
export function archiveLane(span, from, to) {
  if (!span || span.kind !== 'archive' || !span.from || !span.to) return null;
  const opens = Date.parse(from);
  const closes = Date.parse(to);
  const first = Date.parse(span.from);
  // AN OPEN LAST DAY (`to_binds: false`) cuts nothing after it: the vendor
  // delivers daily, so the lane stays live to the window's end.
  const openEnd = span.bindsTo === false;
  const last = openEnd ? Math.max(Date.parse(span.to), closes) : Date.parse(span.to);
  if (![opens, closes, first, last].every(Number.isFinite) || !(closes > opens) || last < first) return null;
  const width = closes - opens;
  const a = Math.min(width, Math.max(0, first - opens));
  const b = Math.min(width, Math.max(0, last - opens));
  const live = Math.max(0, b - a);
  // A window wholly outside the archive holds none of it.
  const overlap = last >= opens && first <= closes;
  const beforePct = overlap ? (a / width) * 100 : 100;
  const livePct = overlap ? (live / width) * 100 : 0;
  return { from: span.from, to: span.to, openEnd, beforePct, livePct, afterPct: Math.max(0, 100 - beforePct - livePct) };
}

/** A zeroed tally with one slot per verdict. @returns {Record<Verdict, number>} */
export function emptySeen() {
  return { fail: 0, refused: 0, unread: 0, never: 0, partial: 0, short: 0, flight: 0, unknown: 0, beyond: 0, ok: 0 };
}

/* ======================================================================
   THE PULL JOURNAL'S REFUSALS, JOINED TO MONTHS (DB-G23)
   ====================================================================== */

/**
 * One refusal as a month cell reads it.
 * @typedef {{
 *   count: number, days: string[], moreDayRecords: number, note: string, cut: boolean,
 *   record: number | null, atSecs: number | null, whole: boolean
 * }} MonthRefusal
 */

/**
 * @typedef {{
 *   months: Map<string, MonthRefusal>,
 *   whole: { from: string, to: string, note: string, cut: boolean, record: number | null, atSecs: number | null }[],
 *   wholeCount: number,
 *   count: number
 * }} SymbolRefusals
 */

/**
 * `/ingest/refusals.json`'s body as a lookup: symbol -> its refusals by month.
 * A symbol the server reported an error for is in `errors`, never an empty
 * entry: "no refusal" and "could not be read" are different facts.
 *
 * @param {any} body
 * @returns {{ bySymbol: Map<string, SymbolRefusals>, errors: Map<string, string>, rungNote: string }}
 */
export function refusalIndex(body) {
  /** @type {Map<string, SymbolRefusals>} */
  const bySymbol = new Map();
  /** @type {Map<string, string>} */
  const errors = new Map();
  for (const row of Array.isArray(body?.symbols) ? body.symbols : []) {
    if (!row || typeof row.symbol !== 'string') continue;
    /** @type {Map<string, MonthRefusal>} */
    const months = new Map();
    for (const [month, m] of Object.entries(row.months ?? {})) {
      const v = /** @type {any} */ (m);
      months.set(month, {
        count: Number.isSafeInteger(v?.count) ? v.count : 0,
        days: Array.isArray(v?.days) ? v.days.filter((/** @type {unknown} */ d) => typeof d === 'string') : [],
        moreDayRecords: Number.isSafeInteger(v?.more_day_records) ? v.more_day_records : 0,
        note: typeof v?.note === 'string' ? v.note : '',
        cut: v?.cut === true,
        record: Number.isSafeInteger(v?.record) ? v.record : null,
        atSecs: Number.isSafeInteger(v?.at_unix_secs) ? v.at_unix_secs : null,
        whole: false
      });
    }
    const whole = (Array.isArray(row.whole) ? row.whole : [])
      .filter((/** @type {any} */ w) => typeof w?.window?.from === 'string' && typeof w?.window?.to === 'string')
      .map((/** @type {any} */ w) => ({
        from: w.window.from,
        to: w.window.to,
        note: typeof w.note === 'string' ? w.note : '',
        cut: w.cut === true,
        record: Number.isSafeInteger(w.record) ? w.record : null,
        atSecs: Number.isSafeInteger(w.at_unix_secs) ? w.at_unix_secs : null
      }));
    bySymbol.set(row.symbol, {
      months,
      whole,
      wholeCount: Number.isSafeInteger(row.whole_count) ? row.whole_count : whole.length,
      count: Number.isSafeInteger(row.count) ? row.count : 0
    });
  }
  for (const e of Array.isArray(body?.errors) ? body.errors : []) {
    if (typeof e?.symbol === 'string') errors.set(e.symbol, String(e.error ?? 'the server named no reason'));
  }
  return { bySymbol, errors, rungNote: typeof body?.rung_note === 'string' ? body.rung_note : '' };
}

/**
 * The refusal the journal holds for `symbol` in `ym`, or `null`: a refusal
 * that names a day in the month, else a whole-window refusal whose window
 * covers it (the newest listed).
 *
 * @param {Map<string, SymbolRefusals>} bySymbol
 * @param {string} symbol
 * @param {string} ym `YYYY-MM`
 * @returns {MonthRefusal | null}
 */
export function refusedMonth(bySymbol, symbol, ym) {
  const held = bySymbol.get(symbol);
  if (!held) return null;
  const named = held.months.get(ym);
  if (named) return named;
  const whole = held.whole.find((w) => w.from.slice(0, 7) <= ym && ym <= w.to.slice(0, 7));
  if (!whole) return null;
  return { count: 1, days: [], moreDayRecords: 0, note: whole.note, cut: whole.cut, record: whole.record, atSecs: whole.atSecs, whole: true };
}

/**
 * WHETHER THE JOURNAL'S REFUSALS WERE READ FOR ONE INSTRUMENT (WP-61 repair).
 *
 * "Never pulled" and "short" both say the pull journal names no refusal, so
 * they may be said only of an instrument the journal answered for. The server
 * answers 200 with a per-symbol `errors` list when one instrument's index
 * could not be read; that list was stored and never looked at, so such an
 * instrument's months read "never pulled — nothing has asked the vendor for
 * it". An instrument is `read` only when the answer for THIS feed lists it
 * among `symbols` and not among `errors`; every other case is unread, with
 * the reason in words.
 *
 * @param {{
 *   feed: string | null, state: string, bySymbol: Map<string, SymbolRefusals>,
 *   errors: Map<string, string>, why: string | null
 * } | null | undefined} refusals the page's reading of `/ingest/refusals.json`
 * @param {string | null | undefined} feed the feed the table is drawn for
 * @param {string} symbol
 * @returns {{ read: true, why: '' } | { read: false, why: string }}
 */
export function refusalJoin(refusals, feed, symbol) {
  const unread = (/** @type {string} */ why) => /** @type {{ read: false, why: string }} */ ({ read: false, why });
  if (!refusals || !feed || refusals.feed !== feed)
    return unread('the pull journal’s refusals have not been read for this feed');
  const failed = refusals.errors instanceof Map ? refusals.errors.get(symbol) : undefined;
  if (failed !== undefined) return unread(`the server could not read this instrument’s refusals: ${failed}`);
  if (refusals.bySymbol instanceof Map && refusals.bySymbol.has(symbol)) return { read: true, why: '' };
  if (refusals.state === 'error') return unread(refusals.why ?? '/ingest/refusals.json could not be read, and no reason was kept');
  if (refusals.state === 'unsupported')
    return unread(`the server refused to answer for this feed’s refusals: ${refusals.why ?? 'it named no reason'}`);
  if (refusals.state === 'reading') return unread('/ingest/refusals.json is still being read');
  if (refusals.state === 'ready')
    return unread('the answer of /ingest/refusals.json names this instrument neither among its symbols nor among its errors');
  return unread('the pull journal’s refusals were not asked for');
}

/**
 * The per-symbol read failures of a refusals answer, as one sentence for the
 * notice above the table: how many, and the first few with the server's words.
 *
 * @param {Map<string, string>} errors
 * @param {number} [show] how many to spell out
 * @returns {string} empty when there are none
 */
export function refusalErrorsSentence(errors, show = 3) {
  if (!(errors instanceof Map) || errors.size === 0) return '';
  const listed = [...errors].slice(0, show).map(([symbol, why]) => `${symbol}: ${why}`);
  const more = errors.size - listed.length;
  return (
    `The pull journal’s refusals could not be read for ${errors.size.toLocaleString('en-IN')} instrument(s) — ` +
    `${listed.join('; ')}${more > 0 ? `; and ${more.toLocaleString('en-IN')} more` : ''}. ` +
    `Their missing or short months read “refusals unread” below: whether they were asked for and refused is not known, ` +
    `and “never pulled” is not claimed.`
  );
}

/**
 * A month's refusal in words, for a cell's title.
 * @param {MonthRefusal} r
 * @param {(secs: number) => string} [stamp] formats the record's time
 */
export function refusalSentence(r, stamp) {
  const where = r.whole
    ? 'a whole window that covers this month was refused before anything was appended'
    : `${r.count} refusal record(s) name ${r.days.join(', ')}${r.moreDayRecords > 0 ? ` and ${r.moreDayRecords} more record(s) of days not listed` : ''}`;
  const when = r.atSecs !== null && stamp ? `, newest written ${stamp(r.atSecs)}` : '';
  const which = r.record !== null ? ` (journal record ${r.record}; /audit.json carries its whole note)` : '';
  return `requested and refused — ${where}${when}: “${r.note}${r.cut ? '…' : ''}”${which}`;
}

/**
 * The symbols of a table, in the batches one refusals request may carry.
 * @param {Iterable<string>} symbols
 * @param {number} [max] `ingest_refusals::MAX_SYMBOLS`
 * @returns {string[][]}
 */
export function refusalBatches(symbols, max = 120) {
  const unique = [...new Set([...symbols].filter((s) => typeof s === 'string' && s !== ''))];
  /** @type {string[][]} */
  const out = [];
  for (let i = 0; i < unique.length; i += max) out.push(unique.slice(i, i + max));
  return out;
}

/* ======================================================================
   ONE MONTH, ONE SERIES
   ====================================================================== */

/**
 * ONE MONTH FILE'S VERDICT.
 *
 * @param {{ beyond: boolean, bars: number | null, expect: number | null, refused: boolean, unread?: boolean, sessions?: number }} m
 *   `bars` is `null` when the store holds no file for the month (which is not
 *   zero bars); `expect` is `null` when no denominator can be stated; `unread`
 *   is true when the pull journal's refusals were not read for the instrument
 *   (`refusalJoin`), so "no refusal names it" cannot be said; `sessions` is the
 *   number of sessions the calendar holds for the month's days inside the
 *   window, when the caller knows it.
 * @returns {Verdict}
 */
export function monthVerdict(m) {
  if (m.beyond) return 'beyond';
  const open = m.refused ? 'refused' : m.unread ? 'unread' : null;
  // NOTHING OWED IS NOT "NEVER PULLED": a month whose days inside the window
  // hold no session (expect 0) owes no bar, so a missing file is not a gap. A
  // journalled refusal is still shown; an unread journal changes nothing owed.
  // Said only when the caller states the month's session count is zero too:
  // an expectation of 0 that came from a denominator the page failed to look
  // up is not evidence that nothing is owed.
  if (m.bars === null && m.expect === 0 && m.sessions === 0) return m.refused ? 'refused' : 'ok';
  if (m.bars === null) return open ?? 'never';
  if (m.expect === null) return 'unknown';
  if (m.bars >= m.expect) return 'ok';
  return open ?? 'short';
}

/**
 * ONE SERIES' VERDICT, worst first, every branch with evidence behind it.
 *
 * `never` means NOTHING landed and nothing was refused. A series with bars and
 * unwritten months is `partial` (DB-B38), never `short`, whose text says every
 * month file has been written.
 *
 * @param {{
 *   seen: Record<Verdict, number>, months: number, got: number,
 *   inFlight: boolean, halted: boolean
 * }} s
 * @returns {Verdict}
 */
export function seriesVerdict(s) {
  const { seen } = s;
  const unread = seen.unread ?? 0;
  const missing = seen.never + seen.short + seen.refused + unread;
  if (s.months > 0 && seen.beyond === s.months) return 'beyond';
  if (s.inFlight) return 'flight';
  // NO MONTH WAS JUDGED, so nothing was verified: "verified" over zero month
  // files is a claim with no evidence. The verdict is withheld.
  if (!(s.months > 0)) return 'unknown';
  if (s.halted && missing > 0) return 'fail';
  // BEFORE `never`: a series whose refusals were not read is not "never pulled".
  if (unread > 0) return 'unread';
  if (seen.never > 0 && s.got === 0 && seen.refused === 0) return 'never';
  if (seen.refused > 0) return 'refused';
  if (seen.never > 0) return 'partial';
  if (seen.short > 0) return 'short';
  if (seen.unknown > 0) return 'unknown';
  return 'ok';
}

/**
 * THE SPANS ONE ROW'S PULL ASKS FOR (DB-B38): each run of consecutive
 * unsettled months, as the days the wire carries, clipped to the chosen
 * window. A row unsettled in 2020-11 and 2024-11 asks two months, never the
 * 49 between them.
 *
 * @param {{ month: string, k: Verdict }[]} months oldest first
 * @param {string} from the window's first day, ISO
 * @param {string} to the window's last day, ISO
 * @param {(ym: string) => string} lastDayOf the last day of a month, ISO
 * @returns {{ from: string, to: string, months: number }[]}
 */
export function unsettledSpans(months, from, to, lastDayOf) {
  /** @type {{ from: string, to: string, months: number }[]} */
  const out = [];
  let start = -1;
  const open = (/** @type {Verdict} */ k) => k !== 'ok' && k !== 'beyond';
  for (let i = 0; i <= months.length; i += 1) {
    const isOpen = i < months.length && open(months[i].k);
    // A RUN ALSO ENDS WHERE THE MONTHS STOP BEING CONSECUTIVE, which a caller's
    // list may be if it filtered: never join two months across a gap.
    const adjacent = i > 0 && i < months.length && nextMonth(months[i - 1].month) === months[i].month;
    if (start >= 0 && (!isOpen || !adjacent)) {
      const first = `${months[start].month}-01`;
      const last = lastDayOf(months[i - 1].month);
      const a = first > from ? first : from;
      const b = last < to ? last : to;
      if (a <= b) out.push({ from: a, to: b, months: i - start });
      start = -1;
    }
    if (isOpen && start < 0) start = i;
  }
  return out;
}

/** @param {string} ym */
function nextMonth(ym) {
  const [y, m] = ym.split('-').map(Number);
  return m === 12 ? `${String(y + 1).padStart(4, '0')}-01` : `${String(y).padStart(4, '0')}-${String(m + 1).padStart(2, '0')}`;
}

/* ======================================================================
   WHAT A FEED CAN HOLD, AND WHERE THE WINDOW OPENS (DB-B38, DB-B46)
   ====================================================================== */

/** @param {unknown} v @returns {string | null} */
const isoDay = (v) => (typeof v === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(v) ? v : null);

/**
 * THE SPAN A FEED'S DESCRIPTOR STATES, read off `/feeds.json` and nothing
 * else: an archive vendor's measured span (`archive`: its first day binds,
 * no file exists before it; its last day binds only when the server says
 * `to_binds` — a dated measurement of a vendor that delivers daily is a
 * lower bound, so a later day may be delivered and waiting, R3-05 repair 2),
 * or the operator's window for a pulled feed (`window`, which sets where the
 * form OPENS and binds nothing — the vendor answers outside it and a month
 * there is real).
 *
 * THE SERVER'S WORD ON THE OPERATOR'S WINDOW TRAVELS WITH IT (R3-05 repair
 * 3, the second agent's follow-up): a window the server REFUSED (`why`, no
 * days: the settings could not be read, or the window is inverted) is
 * `refused`, and a window served from degraded settings carries `degraded`,
 * so the page says why instead of opening on a default as if nothing were
 * wrong.
 *
 * @param {{ archive?: { from?: string, to?: string, source?: string, to_binds?: boolean } | null,
 *   window?: { from?: string, to?: string, source?: string, why?: string, degraded?: string | null } | null } | null | undefined} feed
 * @returns {{ kind: 'archive' | 'window' | null, from: string | null, to: string | null, binds: boolean, bindsTo: boolean, source: string, refused: string, degraded: string }}
 */
export function feedSpan(feed) {
  const a = feed?.archive;
  const w = feed?.window;
  const served = Boolean(w && isoDay(w.from) && isoDay(w.to));
  const refused = w && !served && typeof w.why === 'string' ? w.why : '';
  const degraded = w && typeof w.degraded === 'string' ? w.degraded : '';
  if (a && isoDay(a.from) && isoDay(a.to)) {
    // AN ABSENT `to_binds` IS AN OLDER SERVER, whose archive bound both ends.
    const bindsTo = a.to_binds !== false;
    return { kind: 'archive', from: /** @type {string} */ (a.from), to: /** @type {string} */ (a.to), binds: true, bindsTo, source: a.source ?? '', refused, degraded };
  }
  if (w && served) {
    return { kind: 'window', from: /** @type {string} */ (w.from), to: /** @type {string} */ (w.to), binds: false, bindsTo: false, source: w.source ?? '', refused: '', degraded };
  }
  return { kind: null, from: null, to: null, binds: false, bindsTo: false, source: '', refused, degraded };
}

/**
 * Whether a month file lies wholly outside a BINDING span (an archive's): no
 * file for it can exist, so it is out of reach, not a gap (DB-B46).
 *
 * @param {ReturnType<typeof feedSpan>} span
 * @param {string} ym
 * @param {(ym: string) => string} lastDayOf
 */
export function outsideSpan(span, ym, lastDayOf) {
  if (!span.binds || !span.from || !span.to) return false;
  return lastDayOf(ym) < span.from || (span.bindsTo && `${ym}-01` > span.to);
}

/**
 * WHERE THE WINDOW OPENS WHEN THE ADDRESS NAMES NONE (DB-B38, DB-B46).
 *
 * `from` is the feed's stated first day (the operator's 2018-01-01 for
 * Zerodha, the archive's first day for GDFL), never earlier than `floor`.
 * `to` is the newest askable day: the archive's last day for an archive
 * vendor (nothing newer can be imported from it), else `ceiling`.
 *
 * @param {ReturnType<typeof feedSpan>} span
 * @param {{ floor: string, ceiling: string }} bounds the page's own oldest and newest askable days
 * @returns {{ from: string, to: string, why: string }}
 */
export function defaultWindow(span, bounds) {
  const from = span.from && span.from > bounds.floor ? span.from : bounds.floor;
  const to = span.binds && span.bindsTo && span.to && span.to < bounds.ceiling ? span.to : bounds.ceiling;
  const why =
    span.kind === 'archive'
      ? span.bindsTo
        ? `the archive this feed was imported from spans ${span.from} to ${span.to}`
        : `the archive this feed is imported from begins ${span.from} and is delivered through at least ${span.to}`
      : span.kind === 'window'
        ? `your window for this feed opens ${span.from}`
        : 'this feed states no span, so the window opens at the oldest day this page can ask for';
  // A REFUSED OR DEGRADED WINDOW IS SAID, never opened on silently.
  const said = span.refused
    ? `; the server refused your window: ${span.refused}${span.degraded ? ` (${span.degraded})` : ''}`
    : span.degraded
      ? `; your settings are degraded: ${span.degraded}`
      : '';
  return { from: from <= to ? from : to, to, why: why + said };
}
