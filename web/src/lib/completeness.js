/**
 * WHAT "COMPLETE" MEANS ON `/db`, AS PLAIN FUNCTIONS.
 *
 * # Against the calendar, never against a peer (D-3336, WP-54)
 *
 * Every row used to be judged against the FULLEST stored series at the same
 * (month, rung) — `denominators()` below, a max. That made the verdict a
 * statement about the store measured by the store (DB-014, DB-G34, DB-G63):
 *
 * - a hole every series shares read `full`: Zerodha 2024-11, every index at
 *   1,875 one-minute bars, 5 of 19 sessions;
 * - a share in a closing-auction month read short against the index: 7,560 =
 *   21 x 360 against 7,875, although 360 is exactly what a CAS-eligible share
 *   owes (D-3320);
 * - a one-second option read 0.0065% covered against an index's every second,
 *   and GDFL NIFTY 2018-09 read `full 100.00%` at 4,00,200 bars against 18 x
 *   22,500 = 4,05,000 owed.
 *
 * Now [`judgeRow`] takes the bars owed from the calendar of record
 * (`$lib/calendar-owed.js` `owedLedger`, built from `/calendar.json`): the
 * sessions `pull::calendar` sources in the month, times what each session owes
 * at the row's rung, a special session by its windows, a halted minute
 * neither owed nor missing. One-second shares and options print only when a
 * trade does, so they are `trade` — trade-driven, measured by the sessions in
 * their span, never short of an index's every second (D-3336 §2).
 *
 * `denominators()` stays, and so does `fullest`: no longer a denominator, it
 * is the CORROBORATION that names a hole the whole store shares ("store-wide
 * hole", DB-012). If even the fullest series at a (month, rung) is short of
 * what the calendar owes an index, every series is, and the hole is the
 * store's, not one instrument's.
 *
 * No runes, no imports but the ledger's span, no DOM: `web/tests/
 * completeness.test.js` drives these under `node --test`.
 */
import { ledgerSpan, monthSpan, monthsBetween, epochDayOfIso } from './calendar-owed.js';

/**
 * The separator inside a composite key: NUL, which no month or rung name can
 * carry. Written as an escape, so the source stays searchable text.
 */
const SEP = '\u0000';

/**
 * One stored instrument-month at one rung, as the table reads it.
 *
 * @typedef {object} Row
 * @property {string} month `"YYYY-MM"`.
 * @property {string} timeframe the rung.
 * @property {number} rows bars held.
 */

/**
 * The denominator key for one row: its month AND its rung, never the month.
 *
 * @param {string} month
 * @param {string} timeframe
 * @returns {string}
 */
export function denomKey(month, timeframe) {
  return `${month}${SEP}${timeframe}`;
}

/**
 * The fullest row and the row COUNT at each (month, rung).
 *
 * O(n) once over the whole store, then O(1) per lookup. NOT A DENOMINATOR any
 * more (D-3336): [`judgeRow`] reads `fullest` only to tell a hole one series
 * has from a hole the whole store shares.
 *
 * @param {Row[] | null | undefined} rows
 * @returns {{ fullest: Map<string, number>, support: Map<string, number> }}
 */
export function denominators(rows) {
  const fullest = new Map();
  const support = new Map();
  for (const r of rows ?? []) {
    const at = denomKey(r.month, r.timeframe);
    fullest.set(at, Math.max(fullest.get(at) ?? 0, r.rows));
    support.set(at, (support.get(at) ?? 0) + 1);
  }
  return { fullest, support };
}

/**
 * Whether this row is the only one stored at its (month, rung).
 *
 * @param {Map<string, number>} support the `support` half of `denominators`.
 * @param {Row} row
 * @returns {boolean}
 */
export function isSole(support, row) {
  return (support.get(denomKey(row.month, row.timeframe)) ?? 0) < 2;
}

/* ======================================================================
   THE VERDICT AGAINST THE CALENDAR
   ====================================================================== */

/**
 * The census segment of a store key: its second token, `INDEX`, `CASH` or
 * `FNO`, or `''` for a key that has none.
 *
 * @param {string} instrument
 */
export function segmentOfKey(instrument) {
  const parts = instrument.split('-');
  return parts.length > 2 ? parts[1] : '';
}

/**
 * Whether a series is TRADE-DRIVEN: it holds a bar only where a trade
 * printed, so it owes no fixed count (D-3336 §2). A one-second share, and an
 * option at any rung. A one-second INDEX is computed every second and owes
 * 22,500 a regular session.
 *
 * @param {string} timeframe
 * @param {string} segment `INDEX`, `CASH` or `FNO`
 * @param {number | null | undefined} strike the option's strike, `null` for a future or spot
 */
export function isTradeDriven(timeframe, segment, strike) {
  // A FUTURE AT ONE SECOND is trade-driven too: the calendar's derivatives
  // row at `1s` is a ceiling, not a count owed (`tradeDriven`, SC-CLOSE-16).
  if (segment === 'FNO') return (strike !== null && strike !== undefined) || timeframe === '1s';
  return timeframe === '1s' && segment === 'CASH';
}

/**
 * What [`judgeRow`] is told besides the row.
 *
 * @typedef {object} JudgeContext
 * @property {import('./calendar-owed.js').OwedLedger | null} ledger the feed's EXCHANGE calendar at the row's rung: what every day owes
 * @property {string} [ledgerWhy] why there is none
 * @property {import('./calendar-owed.js').OwedLedger | null} [held] the row's own series' calendar at its rung, whose held digits count the days it holds; its span is the series' own, so it never decides what is owed
 * @property {number | null} [storeLastDay] epoch day of the last bar any series holds at this rung: how far the store reaches
 * @property {number | null} [seriesFirstDay] epoch day of the series' first stored bar at this rung
 * @property {number | null} [seriesLastDay] epoch day of its last
 * @property {number | null} [fullest] the most bars any stored series holds at this month and rung
 * @property {number} [support] how many series are stored at this month and rung
 * @property {{ from: string, to: string } | null} [window] the operator's window, `YYYY-MM-DD`
 * @property {{ from: string, to: string } | null} [archive] the vendor archive, `YYYY-MM-DD`
 * @property {boolean} [derived] the rung is folded from the feed's 1min (D-3000, D-3321)
 * @property {{ rows: number, lastAt: number | null, cover?: Cover } | null} [source] the 1min row of the same instrument-month, judged against the 1min calendar
 * @property {number} [rungMs] the rung's own length, milliseconds
 * @property {{ n: number, atMost: number } | null} [peers] how many calendar-owed series are stored at this month and rung, and how many of them hold no more bars than this row (`peersAtMost`)
 * @property {boolean | null} [casFlag] a share's dated closing-auction flag for the span: `true` eligible, `false` not, `null` (the census carries none) unknown, so a count between the two is not called full (SC-CLOSE-17)
 */

/**
 * A row to judge: a decorated census row, or an absent month.
 *
 * @typedef {object} Judgeable
 * @property {string} instrument
 * @property {string} month
 * @property {string} timeframe
 * @property {number} rows
 * @property {number | null} firstAt epoch milliseconds of the first bar
 * @property {number | null} lastAt epoch milliseconds of the last bar
 * @property {string | null} [expiry] the contract's ISO expiry
 * @property {number | null} [strike]
 * @property {string} [underlying]
 * @property {boolean} [absent] no file: an absent month
 */

/**
 * @typedef {object} Cover
 * @property {'calendar' | 'trade' | 'none'} basis what the verdict is measured against
 * @property {'full' | 'near' | 'gap' | 'absent' | 'trade' | 'unmeasured' | 'over'} state
 * @property {number | null} owed bars the calendar owes over the measured span
 * @property {number | null} owedAlt for a share on auction days, the count were it NOT eligible
 * @property {number | null} missing bars owed and not held
 * @property {number} extra bars held beyond what is owed (bars on days owed nothing)
 * @property {number | null} ratio held over owed, 0..1, `null` where nothing is owed
 * @property {number | null} sessions sessions in the measured span
 * @property {number | null} heldSessions sessions held: counted from the series' own day digits when `heldExact`, else bars over the average session
 * @property {boolean} heldExact
 * @property {number | null} spanFrom epoch day the measure starts
 * @property {number | null} spanTo epoch day it ends
 * @property {number} unknownDays sessions of unrecorded length in the span, owed nothing here
 * @property {number} auctionDays
 * @property {number} specialDays
 * @property {number} haltDays
 * @property {number} outsideDays days of the span outside the calendar's own
 * @property {number} sharedMissing of `missing`, the bars every stored series at this month and rung is short of
 * @property {string[]} notes codes; see `NOTE_TEXT` in `$lib/db/notes-coverage.js`
 * @property {string} why the reason, where `basis` is `none`
 */

/**
 * The share of the series stored at a (month, rung) that must hold no more
 * bars than a short row for its hole to be called the store's. CHOSEN, not
 * sourced: a hole nine in ten series share is a pull or import that did not
 * land those days, not one instrument's gap. Named on the page.
 */
export const STORE_WIDE_SHARE = 0.9;

/**
 * How many of the series stored at one (month, rung) hold at most `rows`
 * bars, over a SORTED list of their counts: one binary search.
 *
 * @param {Int32Array | number[]} sorted ascending
 * @param {number} rows
 * @returns {{ n: number, atMost: number }}
 */
export function peersAtMost(sorted, rows) {
  let lo = 0;
  let hi = sorted.length;
  while (lo < hi) {
    const mid = (lo + hi) >>> 1;
    if (sorted[mid] <= rows) lo = mid + 1;
    else hi = mid;
  }
  return { n: sorted.length, atMost: lo };
}

/**
 * WHICH OF THE LEDGER'S COUNTS A ROW OWES, by its segment, whether its span
 * holds closing-auction days, and a share's dated flag (SC-CLOSE-17).
 *
 * - a future: `deriv`, the derivatives venue's own count (never an index's);
 * - an index: `index`;
 * - a share: `share`, or on auction days `cas` when it is eligible and
 *   `share` when it is not. With no flag (the census carries none) it is
 *   held to `cas`, the lower, and `unresolved` says the other count stands
 *   beside it.
 *
 * `owed`, `unknown` and `unheld` are field names of a `ledgerSpan` answer.
 *
 * @param {string} segment
 * @param {boolean} auction the span holds at least one auction day
 * @param {boolean | null} flag the share's dated closing-auction flag, `null` unknown
 * @returns {{ owed: string, unknown: string[], unheld: string, cas: boolean, unresolved: boolean }}
 */
export function owedKind(segment, auction, flag) {
  if (segment === 'FNO') return { owed: 'deriv', unknown: ['unknownDeriv'], unheld: 'unheldDeriv', cas: false, unresolved: false };
  if (segment !== 'CASH') return { owed: 'index', unknown: ['unknownIndex'], unheld: 'unheldIndex', cas: false, unresolved: false };
  if (!auction || flag === false) return { owed: 'share', unknown: ['unknownShare'], unheld: 'unheldShare', cas: false, unresolved: false };
  if (flag === true) return { owed: 'cas', unknown: ['unknownCas'], unheld: 'unheldCas', cas: true, unresolved: false };
  return { owed: 'cas', unknown: ['unknownCas', 'unknownShare'], unheld: 'unheldCas', cas: true, unresolved: true };
}

/**
 * A coverage ratio as the page prints it: two decimals, ROUNDED DOWN, so a
 * month missing one bar of 4,72,500 reads 99.99% and never 100.00%
 * (R3-04). `toFixed` rounds half up and printed exactly that. A ratio above
 * zero that floors to nothing reads `<0.01%`, never `0.00%`, which would
 * say nothing is held; `null` (nothing can be owed) is a dash.
 *
 * @param {number | null | undefined} ratio held over owed, 0..1
 * @returns {string}
 */
export function coverPct(ratio) {
  // `Number.isFinite` is false for `null` and `undefined` as for NaN.
  if (!Number.isFinite(ratio)) return '—';
  const r = /** @type {number} */ (ratio);
  if (r >= 1) return '100.00%';
  // `1e-9` absorbs the float's own error (0.29 x 10,000 is 2,899.999...);
  // the cap keeps a ratio under 1 under 100.00% whatever that adds.
  const hundredths = Math.min(9_999, Math.floor(r * 10_000 + 1e-9));
  if (hundredths <= 0) return r > 0 ? '<0.01%' : '0.00%';
  return `${Math.floor(hundredths / 100)}.${String(hundredths % 100).padStart(2, '0')}%`;
}

/** Milliseconds in a day. */
const DAY_MS = 86_400_000;
/** IST's offset from UTC, ms: a bar stamp's IST day is `floor((t + IST) / day)`. */
const IST_MS = 19_800_000;

/**
 * The IST epoch day of an instant in epoch milliseconds.
 *
 * @param {number} ms
 */
export const istDayOf = (ms) => Math.floor((ms + IST_MS) / DAY_MS);

/**
 * JUDGE ONE INSTRUMENT-MONTH AGAINST THE CALENDAR OF RECORD.
 *
 * O(1): two prefix-sum reads on the ledger, whatever the span.
 *
 * - The span is the month; a series' first month is measured from its first
 *   bar (a share that listed mid-month owes nothing before it), and a
 *   contract's last month ends at its expiry.
 * - An index (and a future) owes `index`; a share owes `share`, and on
 *   auction days (D-3320) a CAS-eligible share owes `cas`. Eligibility is per
 *   share, from the dated NSE master (charter §9), and is not on the census
 *   wire: a share is judged against the eligible count and `owedAlt` names
 *   the other, so a share at 375 a session reads `full` too, and one between
 *   the two counts says it cannot be told apart.
 * - A trade-driven series owes no count: it is `trade`, with the sessions its
 *   span covers (D-3336 §2).
 *
 * @param {Judgeable} row
 * @param {JudgeContext} ctx
 * @returns {Cover}
 */
export function judgeRow(row, ctx) {
  const segment = segmentOfKey(row.instrument);
  /** @type {string[]} */
  const notes = [];
  /** @type {Cover} */
  const cover = {
    basis: 'none',
    state: 'unmeasured',
    owed: null,
    owedAlt: null,
    missing: null,
    extra: 0,
    ratio: null,
    sessions: null,
    heldSessions: null,
    heldExact: false,
    spanFrom: null,
    spanTo: null,
    unknownDays: 0,
    auctionDays: 0,
    specialDays: 0,
    haltDays: 0,
    outsideDays: 0,
    sharedMissing: 0,
    notes,
    why: ''
  };
  const { first, last } = monthSpan(row.month);
  // A month is in the window when its first day is not after its end and
  // its last is not before its start (`-31` bounds every day as text).
  if (ctx.window && !(`${row.month}-01` <= ctx.window.to && `${row.month}-31` >= ctx.window.from)) {
    notes.push('outside_window');
  }
  if (ctx.derived) {
    notes.push('derived');
    if (!ctx.source) notes.push('derived_no_source');
  }
  // THE SPAN. A series' first stored month is measured from its first bar:
  // owing a listing month its days before the listing would call every new
  // share short. The note says so, so a front the import refused is still
  // visible as a note and as the series' first day (D-3315).
  //
  // NO DAY OF A ROW'S SPAN CAN LIE OUTSIDE THESE BOUNDS, so they are a max
  // and a min and nothing more: a series' first bar is never after a month
  // it holds or lies between, a contract never holds a month after its
  // expiry, and the store's reach is never before a month it holds.
  let from = first;
  let to = last;
  const seriesFirst = ctx.seriesFirstDay ?? -Infinity;
  if (seriesFirst > first) {
    from = seriesFirst;
    notes.push('series_start');
  }
  to = Math.min(to, row.expiry ? epochDayOfIso(row.expiry) : Infinity);
  // THE SPAN'S END. A month the store has not finished reaching is owed only
  // to the store's reach (the last bar any series holds at this rung: GDFL's
  // archive ends 2026-09-24). A series whose last stored bar falls short of
  // that, in the store's last month, owes the rest and reads short: a pull
  // that stopped is a hole, never "full". A series whose last month is an
  // earlier one ENDED there (a delisting, a rename, a pull that stopped
  // months ago): it is measured to its last bar and the note says so, with
  // the store's reach beside it, so the end is seen and judged by a reader.
  const reach = ctx.storeLastDay ?? Infinity;
  to = Math.min(to, reach);
  // ENDED, NOT STOPPED: the store reaches past this month, and the series'
  // last bar is short of the span's end.
  const seriesLast = ctx.seriesLastDay ?? Infinity;
  if (seriesLast < to && reach > last) {
    to = seriesLast;
    notes.push('series_end');
  }
  cover.spanFrom = from;
  cover.spanTo = to;
  const ledger = ctx.ledger;
  if (!ledger) {
    cover.why = ctx.ledgerWhy || 'no calendar of record was read for this feed and rung';
    return cover;
  }
  const span = ledgerSpan(ledger, from, to);
  cover.sessions = span.sessions;
  cover.auctionDays = span.auction;
  cover.specialDays = span.specials;
  cover.haltDays = span.halted;
  cover.outsideDays = span.outside;
  if (span.outside > 0) notes.push('outside_span');
  if (span.specials > 0) notes.push('special');
  if (span.halted > 0) notes.push('halt');
  // THE SERIES' OWN DAY DIGITS, when its own calendar was read: "20 of 21
  // sessions" is then a count of days, not bars over 22,500. A day of the
  // span outside the series' own calendar holds no bar of it (its census
  // span ends there), so only the series' own span is counted.
  const mine = ctx.held ?? ledger;
  const own =
    mine.series !== null &&
    mine.series.symbol === (row.underlying ?? '') &&
    mine.series.segment === segment &&
    mine.held !== null;
  const heldSessions = own ? ledgerSpan(/** @type {any} */ (mine), from, to).heldSessions : null;
  if (isTradeDriven(row.timeframe, segment, row.strike)) {
    cover.basis = 'trade';
    cover.state = row.absent ? 'absent' : 'trade';
    notes.push('trade_driven');
    // A MONTH A TRADE-DRIVEN SERIES DOES NOT HOLD is a month nothing traded
    // in it (or the import has not reached it), never a hole to re-pull.
    if (row.absent) notes.push('no_trade_month');
    if (own) {
      cover.heldSessions = heldSessions;
      cover.heldExact = true;
    }
    return cover;
  }
  // A FUTURE IS OWED THE DERIVATIVES VENUE'S COUNT, never an index's
  // (SC-CLOSE-17): an answer that serves no derivatives row judges no future.
  const future = segment === 'FNO';
  if (future && !Array.isArray(ledger.derivatives)) {
    cover.why = 'the calendar of record serves no derivatives figure at this rung, so a future is owed no count here (SC-CLOSE-17)';
    return cover;
  }
  cover.basis = 'calendar';
  const kind = owedKind(segment, span.auction > 0, ctx.casFlag ?? null);
  /** The span's counts by field name, for the kind's names. */
  const count = /** @type {Record<string, number>} */ (/** @type {unknown} */ (span));
  const owed = count[kind.owed];
  const unknown = kind.unknown.reduce((most, k) => Math.max(most, count[k]), 0);
  cover.unknownDays = unknown;
  if (unknown > 0) notes.push('unknown_days');
  if (kind.cas) notes.push('cas');
  // THE OTHER COUNT, NAMED: a share on auction days whose dated flag the
  // census does not carry is held to the eligible count and the ineligible
  // one is beside it. A flag that is known decides, and names nothing.
  if (kind.unresolved && span.share !== span.cas) cover.owedAlt = span.share;
  cover.owed = owed;
  const sessionsKnown = span.sessions - unknown;
  const perSession = sessionsKnown > 0 && owed > 0 ? owed / sessionsKnown : ledger.regular;
  if (own) {
    cover.heldSessions = heldSessions;
    cover.heldExact = true;
  } else if (perSession) {
    // NULL OR ZERO HAS NO AVERAGE SESSION TO DIVIDE BY.
    cover.heldSessions = row.rows / perSession;
  }
  if (row.absent) {
    cover.state = 'absent';
    cover.missing = owed;
    cover.ratio = owed > 0 ? 0 : null;
    return cover;
  }
  // THE MOST A SERIES CAN HOLD WITHOUT HOLDING TOO MUCH: what it owes (the
  // ineligible count, for a share on auction days), plus the bars a halt
  // leaves unowed — a frozen bar in a halted minute is labelled, not extra.
  // A month with days of unknown length cannot be said to hold too much.
  const ceiling = (cover.owedAlt ?? owed) + (future ? 0 : span.haltBars);
  cover.extra = unknown > 0 ? 0 : Math.max(0, row.rows - ceiling);
  // PER DAY, NOT PER MONTH (R3-04, DB-014). What the sessions it holds no
  // bar on owe is missing whatever the other days hold: a surplus on one
  // day never pays for a hole on another. Read off the series' own held
  // digits; with none read, the floor is nothing and the count decides.
  const unheld = own
    ? Number(/** @type {Record<string, number>} */ (/** @type {unknown} */ (ledgerSpan(/** @type {any} */ (mine), from, to)))[kind.unheld]) || 0
    : 0;
  const missing = Math.max(0, owed - row.rows, unheld);
  // NAMED ONLY WHERE THE FLOOR DECIDES: a session with no bar owes more than
  // the month total says is missing (never a surplus month with none).
  if (unheld > 0 && unheld > owed - row.rows) notes.push('unheld_sessions');
  // THE OTHER COUNT DECIDES BELOW IT (SC-CLOSE-17). `owedAlt` stands only
  // for a share whose flag the census does not carry, so `alt > owed` is
  // that share. Under the ineligible count its verdict hangs on the flag;
  // at or above what an eligible share owes and under the other count,
  // nothing is known missing and nothing is known whole — a flag-0 share
  // at 360 a session is short, an eligible one full — so neither verdict
  // is claimed.
  const alt = cover.owedAlt ?? owed;
  if (alt > owed && row.rows < alt) notes.push('cas_unresolved');
  cover.missing = missing === 0 && row.rows < alt ? null : missing;
  // A SURPLUS IS NOT PLACED BY A MONTH TOTAL. A month holding more than it
  // owes may hold it on one day and lack it on another, so its count
  // proves nothing missing and nothing whole: no bar it is known to lack
  // makes it `over`, never `full` and never 100.00%, until a per-day count
  // places the surplus.
  if (cover.extra > 0 && cover.missing === 0) cover.missing = null;
  cover.ratio = owed > 0 && cover.missing !== null ? (owed - cover.missing) / owed : null;
  if (owed === 0) {
    cover.state = unknown > 0 ? 'unmeasured' : row.rows > 0 ? 'over' : 'full';
  } else if (cover.missing === null) {
    cover.state = cover.extra > 0 ? 'over' : 'unmeasured';
  } else if (cover.missing === 0) {
    cover.state = 'full';
  } else {
    // ONE SESSION AT THIS ROW'S COUNT: an index's regular session, or the
    // average of the span's own (a future's era, a ledger with no regular
    // count). `perSession` is a number here: owed is above zero, so a day of
    // the span was sized and `sessionsKnown` is at least one.
    const per = /** @type {number} */ (future || ledger.regular === null ? perSession : ledger.regular);
    cover.state = cover.missing < per ? 'near' : 'gap';
  }
  if (cover.extra > 0) {
    notes.push('extra');
    // OWED MORE THAN ZERO: the surplus may stand on a day another lacks.
    if (owed > 0) notes.push('surplus_unplaced');
  }
  // A HOLE THE WHOLE STORE SHARES (DB-012, DB-G34): if even the fullest
  // series stored at this (month, rung) is short of what the calendar owes
  // an index over the WHOLE month, every series is, and the hole is the
  // store's. Two series at least, or there is nothing to share it with.
  //
  // TWO MEASURES, BECAUSE ONE OUTLIER DEFEATS EITHER ALONE. `peers` counts
  // the series at this (month, rung) that hold no more bars than this row:
  // at least STORE_WIDE_SHARE of them (Zerodha 2024-11: 271 of 272 series
  // hold 1,875 or fewer; SWIGGY, pulled later, holds 4,093) is the store's
  // hole. Failing that, `sharedMissing` is the shortfall EVERY series has —
  // the month's index count less the fullest series' (2020-02: no series
  // holds the 13th's 10:32 minute) — and a row missing more owns the rest.
  const short = cover.missing;
  if (short !== null && short > 0) {
    const peers = ctx.peers ?? null;
    const whole = ledgerSpan(ledger, first, last);
    const shared =
      (ctx.support ?? 0) >= 2 && Number.isFinite(ctx.fullest)
        ? Math.max(0, whole.index - /** @type {number} */ (ctx.fullest))
        : 0;
    cover.sharedMissing = Math.min(shared, short);
    if (peers && peers.n >= 2 && peers.atMost >= STORE_WIDE_SHARE * peers.n) {
      notes.push('store_wide');
    } else if (shared > 0) {
      notes.push(short <= shared ? 'store_wide' : 'store_wide_part');
    }
  }
  // A DERIVED RUNG BEHIND OR SHORT OF ITS 1min SOURCE (DB-013, DB-G12,
  // DB-G35): its last bar is earlier than the source's last minute by more
  // than one bucket, or it holds fewer sessions' worth than the source.
  if (ctx.derived && ctx.source) {
    const rungMs = ctx.rungMs ?? 0;
    if (row.lastAt !== null && row.lastAt + rungMs < (ctx.source.lastAt ?? -Infinity)) {
      notes.push('derived_behind');
    }
    if (ctx.source.cover && derivedPartial(cover, ctx.source.cover)) notes.push('derived_partial');
  }
  return cover;
}

/**
 * Whether a derived row is short of its 1min source (DB-013, DB-G36): the
 * buckets it misses, in source minutes, exceed what the source itself misses
 * by more than one bucket. A bucket is worth `source.owed / derived.owed`
 * minutes — the two calendars' own counts, so a closing-auction month and a
 * special session are weighed the same way. 360ONE 60min 2023-02 holds 126
 * of 140 buckets while its 1min month misses 30 of 7,500 minutes: 840
 * minutes withheld against 30 lost, partial. Days held cannot see this: a
 * day with six of seven hourly buckets is still a day held.
 *
 * @param {Cover} derived
 * @param {Cover} source
 */
export function derivedPartial(derived, source) {
  // NOTHING UNKNOWN OR UNOWED IS EVER PARTIAL, by the arithmetic itself: an
  // unknown count reads 0, so an unowed derived month makes the bucket
  // infinite (Infinity > Infinity, or NaN, is false) and an unowed source
  // makes it 0 (0 > a count is false). Only an unknown source shortfall
  // must be refused by name — it would read as nothing lost.
  if (source.missing === null) return false;
  const bucket = Number(source.owed) / Number(derived.owed);
  return Number(derived.missing) * bucket > source.missing + bucket;
}

/**
 * THE MONTHS A SERIES DOES NOT HOLD, between the first and the last it does.
 *
 * A month nobody stored had no row, so it was never drawn and never counted
 * (DB-012: Zerodha's 2023-10 and 2023-11, and the 2020-11 / 2021-11 / 2022-11
 * / 2024-10 months of the Muhurat holes, read as nothing at all). Each is
 * returned as an absent row the grid can draw and [`judgeRow`] can measure:
 * zero bars, against everything the calendar owes.
 *
 * O(months between the two ends).
 *
 * @param {string} firstMonth the series' first held `YYYY-MM`
 * @param {string} lastMonth its last
 * @param {Set<string>} held every month it holds
 * @returns {string[]} the months it does not hold, oldest first
 */
export function absentMonths(firstMonth, lastMonth, held) {
  // An empty first month would count from the year 0; an empty last, or a
  // last before the first, counts nothing on its own.
  if (!firstMonth) return [];
  return monthsBetween(firstMonth, lastMonth).filter((m) => !held.has(m));
}

/* ======================================================================
   THE MONTH ROLL-UP
   ====================================================================== */

/**
 * One month rolled up across its judged rows.
 *
 * @typedef {object} MonthRoll
 * @property {string} month
 * @property {number} n rows in the month, absent ones included
 * @property {number} bars bars held by calendar-judged rows
 * @property {number} owed bars owed to them
 * @property {number} missing
 * @property {number} full
 * @property {number} absent
 * @property {number} trade trade-driven rows, owed no count
 * @property {number} unmeasured rows no calendar could judge
 * @property {number} over rows holding more than they owe and nothing known missing: a surplus no month total can place (R3-04)
 * @property {number} storeWide rows short by a hole the whole store shares
 * @property {string | false | undefined} tf the rung all rows share, `false` when two disagree
 * @property {any | null} worst the row missing the most bars
 */

/**
 * The month roll-up, one card per month, ORDER-INDEPENDENT. `items` carry a
 * `cover` from [`judgeRow`]. Owed and held are summed only over calendar-
 * judged rows, each at its own rung: no rung's count is applied to another's
 * (the 25,033.33% card of a month held at 1day and 1min).
 *
 * @param {Array<{ month: string, timeframe: string, rows: number, cover: Cover }> | null | undefined} items
 * @returns {(MonthRoll & ReturnType<typeof monthVerdict>)[]}
 */
export function rollUpMonths(items) {
  /** @type {Map<string, MonthRoll>} */
  const by = new Map();
  for (const it of items ?? []) {
    let m = by.get(it.month);
    if (!m) {
      m = {
        month: it.month,
        n: 0,
        bars: 0,
        owed: 0,
        missing: 0,
        full: 0,
        absent: 0,
        trade: 0,
        unmeasured: 0,
        over: 0,
        storeWide: 0,
        tf: undefined,
        worst: null
      };
      by.set(it.month, m);
    }
    m.n += 1;
    m.tf = m.tf === undefined ? it.timeframe : m.tf === it.timeframe ? m.tf : false;
    const c = it.cover;
    if (c.state === 'absent') m.absent += 1;
    if (c.notes.includes('store_wide') || c.notes.includes('store_wide_part')) m.storeWide += 1;
    if (c.basis === 'trade') {
      m.trade += 1;
      continue;
    }
    // A ROW WHOSE MISSING COUNT IS UNKNOWN joins no sum: an unplaced surplus
    // (`over`) or a share whose auction flag is unread. Summing its count
    // would let it fill another row's hole, the month-grain fault R3-04
    // removes from the rows.
    if (c.basis !== 'calendar' || c.missing === null) {
      if (c.state === 'over') m.over += 1;
      else m.unmeasured += 1;
      continue;
    }
    // A CALENDAR ROW ALWAYS CARRIES ITS OWED COUNT; its missing one is known here.
    const owedHere = /** @type {number} */ (c.owed);
    m.owed += owedHere;
    // HELD IS OWED LESS MISSING, so a per-day floor lowers the month too.
    m.bars += owedHere - c.missing;
    m.missing += c.missing;
    if (c.state === 'full') m.full += 1;
    // THE WORST ROW, ORDER-INDEPENDENT: the most missing, ties broken on
    // the row's own identity rather than on which arrived first.
    const miss = c.missing;
    const was = m.worst ? m.worst.cover.missing : -1;
    // `was` is -1 only before a first row, and `miss` is never negative.
    if (miss > was || (miss === was && rowId(it) < rowId(m.worst))) m.worst = it;
  }
  return [...by.values()].map((m) => ({ ...m, ...monthVerdict(m) }));
}

/**
 * A row's identity for a deterministic tie-break.
 *
 * @param {{ month: string, timeframe: string, instrument?: string }} it
 */
const rowId = (it) => `${it.instrument}|${it.timeframe}`;

/**
 * A month's coverage and its band.
 *
 * `pct` is `null` — not 1, and not 0 — where no row of the month is owed a
 * count: trade-driven rows, or rows no calendar could judge. A ratio nobody
 * can compute is a dash.
 *
 * A month with an `over` row and nothing known missing is `over`, never
 * `full`: the surplus is not placed (R3-04).
 *
 * @param {{ n: number, owed: number, bars: number, missing: number,
 *           trade: number, unmeasured: number, absent: number, over?: number }} m
 * @returns {{ pct: number | null, state: 'trade' | 'unmeasured' | 'full' | 'near' | 'gap' | 'absent' | 'over' }}
 */
export function monthVerdict(m) {
  const over = m.over ?? 0;
  const judged = m.n - m.trade - m.unmeasured - over;
  const pct = judged > 0 && m.owed > 0 ? m.bars / m.owed : null;
  if (judged === 0) return { pct, state: over > 0 ? 'over' : m.trade > 0 ? 'trade' : 'unmeasured' };
  if (m.absent === m.n) return { pct, state: 'absent' };
  if (m.missing === 0) return { pct, state: over > 0 ? 'over' : 'full' };
  return { pct, state: m.bars >= 0.99 * m.owed ? 'near' : 'gap' };
}
