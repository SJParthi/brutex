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
  'fail', 'refused', 'never', 'partial', 'short', 'flight', 'unknown', 'beyond', 'ok'
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
  never: [
    'warn',
    'never pulled',
    'No month file in this window holds a bar for this series, and the pull journal names no refusal of this instrument for any of them: nothing has asked the vendor for it. A pull IS warranted.'
  ],
  partial: [
    'warn',
    'partly never pulled',
    'This series holds bars, and at least one month file in this window has never been written and has no refusal in the pull journal. The cells show which months: a pull of those months is warranted, and the row’s Pull asks for exactly those.'
  ],
  short: [
    'warn',
    'short',
    'Every month file in this window has been written, and at least one holds fewer bars than the NSE calendar expects for the days inside the window. No refusal in the pull journal names those months.'
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

/** A zeroed tally with one slot per verdict. @returns {Record<Verdict, number>} */
export function emptySeen() {
  return { fail: 0, refused: 0, never: 0, partial: 0, short: 0, flight: 0, unknown: 0, beyond: 0, ok: 0 };
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
 * @param {{ beyond: boolean, bars: number | null, expect: number | null, refused: boolean }} m
 *   `bars` is `null` when the store holds no file for the month (which is not
 *   zero bars); `expect` is `null` when no denominator can be stated.
 * @returns {Verdict}
 */
export function monthVerdict(m) {
  if (m.beyond) return 'beyond';
  if (m.bars === null) return m.refused ? 'refused' : 'never';
  if (m.expect === null) return 'unknown';
  if (m.bars >= m.expect) return 'ok';
  return m.refused ? 'refused' : 'short';
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
  const missing = seen.never + seen.short + seen.refused;
  if (s.months > 0 && seen.beyond === s.months) return 'beyond';
  if (s.inFlight) return 'flight';
  if (s.halted && missing > 0) return 'fail';
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
 * else: an archive vendor's measured span (`archive`, both ends bind: no file
 * exists outside it), or the operator's window for a pulled feed (`window`,
 * which sets where the form OPENS and binds nothing — the vendor answers
 * outside it and a month there is real).
 *
 * @param {{ archive?: { from?: string, to?: string, source?: string } | null,
 *   window?: { from?: string, to?: string, source?: string } | null } | null | undefined} feed
 * @returns {{ kind: 'archive' | 'window' | null, from: string | null, to: string | null, binds: boolean, source: string }}
 */
export function feedSpan(feed) {
  const a = feed?.archive;
  if (a && isoDay(a.from) && isoDay(a.to)) {
    return { kind: 'archive', from: /** @type {string} */ (a.from), to: /** @type {string} */ (a.to), binds: true, source: a.source ?? '' };
  }
  const w = feed?.window;
  if (w && isoDay(w.from) && isoDay(w.to)) {
    return { kind: 'window', from: /** @type {string} */ (w.from), to: /** @type {string} */ (w.to), binds: false, source: w.source ?? '' };
  }
  return { kind: null, from: null, to: null, binds: false, source: '' };
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
  return lastDayOf(ym) < span.from || `${ym}-01` > span.to;
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
  const to = span.binds && span.to && span.to < bounds.ceiling ? span.to : bounds.ceiling;
  const why =
    span.kind === 'archive'
      ? `the archive this feed was imported from spans ${span.from} to ${span.to}`
      : span.kind === 'window'
        ? `your window for this feed opens ${span.from}`
        : 'this feed states no span, so the window opens at the oldest day this page can ask for';
  return { from: from <= to ? from : to, to, why };
}
