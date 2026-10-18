/**
 * /db — THE CHART PANEL'S PLAN (WP-57; DB-080, RA-175).
 *
 * Everything the chart decides that is not drawing: which one series it
 * draws, which rungs it offers and where each is read from, the query of one
 * page, how pages join, which instants are asked for greeks, and where the
 * halt, auction, gap and day-flag overlays sit. Plain functions only, so
 * `node --test` drives the module the panel imports.
 *
 * ONE PATH FOR EVERY FEED AND INSTRUMENT. Nothing here names a vendor or an
 * instrument. The rungs come from `/sources.json` (the store's own rung
 * table): a rung the feed STORES is paged through `/bars/window.json`; a rung
 * the server FOLDS on request is paged through `/sources/bars.json`, whose
 * fold is `pull::fold`. The selection's own rung needs no table at all — the
 * census plan the bar grid already made names it.
 *
 * COST, STATED. A page is one bar request ([`CHART_PAGE`] stored bars, or
 * [`FOLD_PAGE`] folded buckets), at most [`GREEK_REQUESTS`] greeks requests
 * of [`GREEK_CHUNK`] instants, and one day-flag request of at most
 * [`DAYFLAGS_PAGE_DAYS`] days: [`PAN_REQUESTS`] in all, whatever the history
 * holds. The chart keeps at most [`CHART_KEEP`] bars and refuses the page that
 * would pass it, by name. Joining a page is O(bars kept), bounded by that
 * ceiling; the hover readout is one Map probe.
 *
 * THE OPERATOR'S WINDOW BOUNDS EVERY RUNG (D-3338). The rung table names the
 * months a rung holds, inside the reader's window or not. [`windowChoice`]
 * clips every rung on offer to the window by the bar grid's own rule
 * (`outsideWindow`), unless the reader turned 'outside your window' on — and
 * then the bars drawn outside it are counted ([`outsideBars`]) and labelled.
 *
 * THE SPOT RULE IS SAID (D-3328). Under the opt-in 'last at or before t' a
 * point may be priced from an earlier spot: [`spotAges`] counts those points
 * and finds the oldest, [`spotRuleText`] is the sentence the panel prints,
 * and [`spotAgeWord`] is the hovered bar's own age.
 */
import { TF_SECS, fmt } from './notes-labels.js';
import { windowQuery, istInstant, istDayKey, monthSpan, MAX_WINDOW_MONTHS, WINDOW_PAGE_MAX } from './bars-plan.js';
import { cellOf, isHoleDay, DAYFLAGS_PAGE_DAYS } from './dayflags.js';
import { parseKey } from '../instrument.js';
import { GREEK_CHUNK, greekKey } from '../greeks.js';
import { ownMonth } from '../moneyness.js';
import { outsideWindow } from './selection-model.js';

/** Stored bars one page asks for: the window route's own maximum. */
export const CHART_PAGE = WINDOW_PAGE_MAX;
/** Folded buckets one page asks for (`sources::MAX_LIMIT`). */
export const FOLD_PAGE = 200;
/** Folded day buckets one page asks for (`sources::MAX_DAY_LIMIT`). */
export const FOLD_PAGE_DAY = 31;
/**
 * The most bars the chart holds; the page that would pass it is refused. One
 * whole one-second session (09:15 to 15:40 is 23,100 seconds) fits under it.
 */
export const CHART_KEEP = 24_000;
/** A selection of at most this many bars is read whole when the chart opens. */
export const CHART_AUTO_ALL = 4_000;
/** The most `/greeks.json` requests one page makes. */
export const GREEK_REQUESTS = 4;
/** The most requests one page (one pan) makes: bars, greeks, day flags. */
export const PAN_REQUESTS = 1 + GREEK_REQUESTS + 1;
/** The most requests opening the chart makes: the rung table and the auto-read pages. */
export const OPEN_REQUESTS = 1 + Math.ceil(CHART_AUTO_ALL / CHART_PAGE) * PAN_REQUESTS;
/** The most empty slots one band adds to the time axis. */
export const BAND_SLOTS = 120;
/** The most gap bands one page draws; the rest are counted, not drawn. */
export const GAP_BANDS = 200;
/** A hole on a sub-minute rung is a gap from this many missing bars (the server's `holes_5s`). */
export const SECOND_HOLE = 5;

export { DAYFLAGS_PAGE_DAYS, GREEK_CHUNK };

/** The greeks the chart can draw as a line, in the order the picker lists them. */
export const GREEK_LINES = [
  { field: 'delta', label: 'Delta', digits: 4 },
  { field: 'gamma', label: 'Gamma', digits: 6 },
  { field: 'theta', label: 'Theta', digits: 4 },
  { field: 'vega', label: 'Vega', digits: 4 },
  { field: 'rho', label: 'Rho', digits: 4 }
];

/** The overlay kinds, most important first: the first that covers a slot colours it. */
export const BAND_KINDS = [
  { kind: 'halt', word: 'trading halt', tone: 'warn' },
  { kind: 'gap', word: 'no bar where one was owed', tone: 'bad' },
  { kind: 'cas', word: 'closing auction', tone: 'info' }
];
const BAND_RANK = new Map(BAND_KINDS.map((k, i) => [k.kind, i]));

/** The phases of a bar's own `phase` word that are a halt, and the auction. */
const PHASE_BAND = new Map([
  ['halted', 'halt'],
  ['reopening_auction', 'halt'],
  ['cas', 'cas']
]);

/**
 * THE ONE SERIES THE CHART DRAWS, from the bar grid's own plan.
 *
 * A chart of candles is one series. A selection that reaches none says
 * nothing; one that the grid itself refuses repeats that refusal; one that
 * reaches several is refused by count, before any request.
 *
 * @param {{keys: string[], timeframe: string, from: string, to: string, why: string | null} | null | undefined} plan
 */
export function chartSubject(plan) {
  const none = { key: '', parsed: null, contract: '', option: false, why: /** @type {string | null} */ (null) };
  if (!plan || !Array.isArray(plan.keys) || plan.keys.length === 0) return none;
  if (plan.why) return { ...none, why: plan.why };
  if (plan.keys.length > 1) {
    return {
      ...none,
      why: `the chart draws one series and this selection reaches ${fmt(plan.keys.length)}: narrow it to one instrument, or to one expiry, strike and side`
    };
  }
  const key = plan.keys[0];
  const parsed = parseKey(key);
  if (parsed.why || !parsed.underlying) {
    return { ...none, why: `the key ${JSON.stringify(key)} could not be read${parsed.why ? ` (${parsed.why})` : ''}, so nothing is asked for it` };
  }
  const head = `${parsed.exchange}-${parsed.segment}-${parsed.underlying}`;
  const contract = key.length > head.length ? key.slice(head.length + 1) : '';
  return { key, parsed, contract, option: parsed.kind === 'option', why: null };
}

/**
 * The instrument of a subject as `/sources.json` and `/sources/bars.json`
 * name it.
 * @param {ReturnType<typeof chartSubject>} subject
 */
function instrumentQuery(subject) {
  const p = /** @type {any} */ (subject.parsed);
  const q = new URLSearchParams({ exchange: String(p.exchange), segment: String(p.segment), symbol: String(p.underlying) });
  if (subject.contract) q.set('contract', subject.contract);
  return q;
}

/** The `/sources.json` URL of a subject: the store's rung table for it. @param {ReturnType<typeof chartSubject>} subject */
export const sourcesUrl = (subject) => `/sources.json?${instrumentQuery(subject)}`;

/**
 * @typedef {object} RungChoice
 * @property {string} rung
 * @property {number} secs the rung's length in seconds
 * @property {'window' | 'fold'} source where a page of it is read
 * @property {string} kind the server's word: stored, derived, fold
 * @property {string} tag the server's source tag, e.g. `gdfl_fold`
 * @property {string} from the stored rung a fold reads, `''` for a stored rung
 * @property {string} firstMonth @property {string} lastMonth
 * @property {string} firstDay @property {string} lastDay
 * @property {string} state the server's word for how whole the rung is
 * @property {string} [derivedFrom] the rung a derived or folded rung is made from
 * @property {RungBehind | null} [behind] what a rung that is not whole misses
 */

/**
 * @typedef {object} RungBehind what the server's `behind_source` names
 * @property {number} months @property {string[]} monthsNamed
 * @property {number} sessions @property {string[]} sessionsNamed
 * @property {boolean} exact `false` where the server calls its session count a floor
 * @property {number} daysBehind
 */

/**
 * THE RUNG THE SELECTION IS ALREADY ON, from the census plan alone: no
 * request is needed to chart what the grid lists.
 *
 * @param {{timeframe: string, from: string, to: string}} plan
 * @returns {RungChoice | null}
 */
export function planChoice(plan) {
  const secs = TF_SECS.get(plan?.timeframe ?? '');
  if (!plan || !secs) return null;
  return {
    rung: plan.timeframe, secs, source: 'window', kind: 'stored', tag: '', from: '',
    firstMonth: plan.from, lastMonth: plan.to, firstDay: '', lastDay: '', state: '', derivedFrom: '', behind: null
  };
}

/**
 * EVERY RUNG THE FEED CAN SHOW THIS SERIES AT, from `/sources.json`, finest
 * first. A rung the feed stores (or derives and stores) is read through the
 * window route; a rung the server folds on request through the fold route.
 * A row of another feed, of a rung this page has no length for, or with no
 * month held is left out and counted in `skipped`.
 *
 * @param {any} body the `/sources.json` answer
 * @param {string} feed the active feed's wire name
 * @returns {{choices: RungChoice[], skipped: number}}
 */
export function rungChoices(body, feed) {
  if (!Array.isArray(body?.rungs)) {
    throw new Error(`/sources.json answered without a rung table${typeof body?.error === 'string' ? `: ${body.error}` : ''}`);
  }
  /** @type {Map<string, RungChoice>} */
  const by = new Map();
  let skipped = 0;
  for (const r of body.rungs) {
    if (r?.feed !== feed) continue;
    const secs = TF_SECS.get(String(r.rung));
    const fold = r.kind === 'fold' && r.stored !== true;
    const month = (/** @type {unknown} */ m) => (typeof m === 'string' && /^\d{4}-\d{2}$/.test(m) ? m : '');
    if (!secs || (!fold && r.stored !== true) || !month(r.first_month) || !month(r.last_month)) {
      skipped += 1;
      continue;
    }
    // A STORED RUNG OUTRANKS A FOLD OF THE SAME LENGTH: it is what is held.
    if (by.has(r.rung) && fold) continue;
    by.set(r.rung, {
      rung: String(r.rung), secs, source: fold ? 'fold' : 'window', kind: String(r.kind ?? ''), tag: String(r.tag ?? ''),
      from: fold ? String(r.derived_from ?? '') : '',
      firstMonth: month(r.first_month), lastMonth: month(r.last_month),
      firstDay: typeof r.first_day === 'string' ? r.first_day : '', lastDay: typeof r.last_day === 'string' ? r.last_day : '',
      state: String(r.state ?? ''), derivedFrom: typeof r.derived_from === 'string' ? r.derived_from : '', behind: behindOf(r.behind_source)
    });
  }
  return { choices: [...by.values()].sort((a, b) => a.secs - b.secs), skipped };
}

/** A count the server sent, or 0. @param {unknown} v */
const count = (v) => (typeof v === 'number' && Number.isSafeInteger(v) && v > 0 ? v : 0);
/** The strings of a list the server sent. @param {unknown} v @returns {string[]} */
const strings = (v) => (Array.isArray(v) ? v.filter((x) => typeof x === 'string') : []);

/** What a rung row's `behind_source` names, or `null` when the row has none. @param {any} b @returns {RungBehind | null} */
function behindOf(b) {
  if (!b || typeof b !== 'object') return null;
  return {
    months: count(b.months_missing), monthsNamed: strings(b.months_missing_named),
    sessions: count(b.sessions_missing), sessionsNamed: strings(b.sessions_missing_named),
    exact: b.sessions_missing_exact !== false, daysBehind: count(b.days_behind)
  };
}

/** The states in which a rung is whole: what the feed holds, or folds on request. */
const WHOLE_STATES = new Set(['', 'held', 'on_request']);
/** What a state word the server uses means, for the words this page knows. */
const STATE_SAYS = new Map([
  ['uncounted', 'not every month of it is counted in the census'],
  ['sessions_unchecked', 'its sessions could not be set against those of the rung it is made from'],
  ['unexpected', 'this feed is not expected to hold it at all']
]);

/** A named list and how many more there are. @param {string[]} named @param {number} n */
const namedList = (named, n) =>
  named.length === 0 ? '' : ` (${named.join(', ')}${n > named.length ? `, and ${fmt(n - named.length)} more` : ''})`;

/**
 * WHAT A RUNG THAT IS NOT WHOLE SAYS: the server's own state word, and the
 * months and sessions its `behind_source` names as missing. `''` for a rung
 * the feed holds whole or folds on request. A word this page does not know is
 * printed as it came, never dropped.
 *
 * @param {RungChoice | null | undefined} choice
 */
export function rungStateText(choice) {
  const state = choice?.state ?? '';
  if (!choice || WHOLE_STATES.has(state)) return '';
  const b = choice.behind ?? null;
  /** @type {string[]} */
  const misses = [];
  if (b && b.months > 0) misses.push(`${fmt(b.months)} month(s)${namedList(b.monthsNamed, b.months)}`);
  if (b && b.sessions > 0) misses.push(`${b.exact ? '' : 'at least '}${fmt(b.sessions)} session(s)${namedList(b.sessionsNamed, b.sessions)}`);
  const source = choice.derivedFrom || 'the rung it is made from';
  const said = [
    ...(STATE_SAYS.has(state) ? [/** @type {string} */ (STATE_SAYS.get(state))] : []),
    ...(misses.length ? [`it misses ${misses.join(' and ')} that ${source} holds`] : []),
    ...(b && b.daysBehind > 0 ? [`it is ${fmt(b.daysBehind)} day(s) behind ${source}`] : [])
  ];
  return `${choice.rung} is ${state}${said.length ? `: ${said.join('; ')}` : ''}. The chart draws the bars held; where one is missing it draws none.`;
}

/**
 * A real `YYYY-MM-DD` calendar day, or `''`: a string of the right shape that
 * names no day (a thirteenth month, the 30th of February) is not one, so no
 * garbage bound ever reaches date arithmetic.
 * @param {unknown} day
 */
function dayOf(day) {
  if (typeof day !== 'string' || !/^\d{4}-\d{2}-\d{2}$/.test(day)) return '';
  const ms = Date.parse(`${day}T00:00:00Z`);
  return Number.isFinite(ms) && new Date(ms).toISOString().slice(0, 10) === day ? day : '';
}
/** The month of a real `YYYY-MM-DD` day, or `''`. @param {unknown} day */
const monthOfDay = (day) => dayOf(day).slice(0, 7);
/** Whether a string is a `YYYY-MM` month. @param {unknown} month */
const isMonth = (month) => typeof month === 'string' && /^\d{4}-(0[1-9]|1[0-2])$/.test(month);
/** The last day of a `YYYY-MM` month. @param {string} month */
const lastDayOf = (month) => new Date(Date.UTC(Number(month.slice(0, 4)), Number(month.slice(5, 7)), 0)).toISOString().slice(0, 10);
/** The month after a `YYYY-MM` month. @param {string} month */
const monthAfter = (month) => new Date(Date.UTC(Number(month.slice(0, 4)), Number(month.slice(5, 7)), 1)).toISOString().slice(0, 7);

/**
 * The first and last month of a feed's operator window, or `null` where the
 * feed row carries none (a measured archive is not a window, D-3338).
 * @param {any} feedRow a `/feeds.json` row
 */
function windowMonths(feedRow) {
  const lo = monthOfDay(feedRow?.window?.from);
  const hi = monthOfDay(feedRow?.window?.to);
  return lo && hi && lo <= hi ? { lo, hi } : null;
}

/**
 * THE OPERATOR'S WINDOW OVER ONE RUNG (D-3338, operator answer 6).
 *
 * The server's rung table names every month a rung holds, including months
 * filed before the reader's window. Those are hidden by default and shown
 * only by the page's 'outside your window' toggle (`oow`), so every rung the
 * chart offers goes through here — the selection's own included, one path.
 *
 * - No window on the feed row (or one whose days are not real days): the rung as it came.
 * - Wholly inside the window: the same object, nothing said.
 * - Reaching outside with the toggle OFF: clipped to the window's months by
 *   the bar grid's own rule (`outsideWindow`, by month), and `hidden` says
 *   which months are held back. A rung wholly outside is not offered (`null`).
 * - Reaching outside with the toggle ON: read whole, and `outside` is true so
 *   the panel labels what it draws there.
 *
 * @param {RungChoice} choice
 * @param {any} feedRow the active feed's `/feeds.json` row, or nothing
 * @param {boolean} oow whether months outside the window are shown
 * @returns {{choice: RungChoice | null, hidden: string, outside: boolean}}
 */
export function windowChoice(choice, feedRow, oow) {
  const w = windowMonths(feedRow);
  // A RUNG WHOSE OWN MONTHS CANNOT BE READ is not clipped to anything: the
  // page request refuses it by name, which is the louder answer.
  if (!w || !isMonth(choice.firstMonth) || !isMonth(choice.lastMonth)) return { choice, hidden: '', outside: false };
  const before = outsideWindow(choice.firstMonth, feedRow) && choice.firstMonth < w.lo;
  const after = outsideWindow(choice.lastMonth, feedRow) && choice.lastMonth > w.hi;
  if (!before && !after) return { choice, hidden: '', outside: false };
  if (oow) return { choice, hidden: '', outside: true };
  const span = `${feedRow.window.from} to ${feedRow.window.to}`;
  const firstMonth = before ? w.lo : choice.firstMonth;
  const lastMonth = after ? w.hi : choice.lastMonth;
  if (firstMonth > lastMonth) {
    return {
      choice: null, outside: false,
      hidden: `${choice.rung} is held only for ${choice.firstMonth} to ${choice.lastMonth}, outside your window (${span}), so it is not offered: turn on 'outside your window' to chart it`
    };
  }
  const held = [
    ...(before ? [`from ${choice.firstMonth}, before it opens`] : []),
    ...(after ? [`to ${choice.lastMonth}, after it closes`] : [])
  ].join(' and ');
  // A DAY BOUND IS NEVER INVENTED: a row that names no day gains none, and a
  // held span that ends inside the window keeps its own end.
  const floor = `${w.lo}-01`;
  const ceil = lastDayOf(w.hi);
  return {
    choice: {
      ...choice, firstMonth, lastMonth,
      firstDay: choice.firstDay && choice.firstDay < floor ? floor : choice.firstDay,
      lastDay: choice.lastDay && choice.lastDay > ceil ? ceil : choice.lastDay
    },
    outside: false,
    hidden: `${choice.rung} is also held ${held} — outside your window (${span}). Those months are not drawn: turn on 'outside your window' to see them, labelled`
  };
}

/** How many of `bars` (ascending) are stamped before `t`. O(log n). @param {{t: number}[]} bars @param {number} t */
function barsBefore(bars, t) {
  let lo = 0;
  let hi = bars.length;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (bars[mid].t < t) lo = mid + 1;
    else hi = mid;
  }
  return lo;
}

/**
 * HOW MANY OF THE BARS HELD LIE OUTSIDE THE OPERATOR'S WINDOW, by the same
 * month rule: before the window's first month, or after its last (IST). Two
 * binary probes over bars that are already in order, whatever is held.
 *
 * @param {{t: number}[]} bars oldest first
 * @param {any} feedRow
 * @returns {{before: number, after: number, n: number}}
 */
export function outsideBars(bars, feedRow) {
  const w = windowMonths(feedRow);
  const opens = w ? istInstant(`${w.lo}-01`, '00:00') : null;
  const closes = w ? istInstant(`${monthAfter(w.hi)}-01`, '00:00') : null;
  if (opens === null || closes === null || bars.length === 0) return { before: 0, after: 0, n: 0 };
  const before = barsBefore(bars, opens);
  const after = bars.length - barsBefore(bars, closes);
  return { before, after, n: before + after };
}

/**
 * Whether one bar lies outside the operator's window: its IST month, asked of
 * the bar grid's own rule. @param {number} t unix seconds @param {any} feedRow
 */
export const isOutside = (t, feedRow) => windowMonths(feedRow) !== null && outsideWindow(istDayKey(t * 1000).slice(0, 7), feedRow);

/**
 * @typedef {{fromDay?: string, toDay?: string, fromTime?: string, toTime?: string}} Bounds
 */

/**
 * ONE PAGE OF A RUNG AS A REQUEST: the route, its query, how many rows it
 * asks for, and what the request leaves out (`notes`). `why` is a refusal
 * said before anything is sent.
 *
 * `at` is where the page starts: an offset into the newest-first order of a
 * stored rung (a seek, D-3326), or the fold route's own cursor.
 *
 * @param {string} feed
 * @param {ReturnType<typeof chartSubject>} subject
 * @param {RungChoice} choice
 * @param {Bounds} bounds the selection's day and time-of-day bounds
 * @param {number | null} at
 */
export function pageRequest(feed, subject, choice, bounds, at) {
  /** @type {string[]} */
  const notes = [];
  const refuse = (/** @type {string} */ why) => ({ route: '', query: '', limit: 0, notes, why });
  if (!subject.key) return refuse('no series is chosen');
  const fromDay = bounds.fromDay ?? '';
  const toDay = bounds.toDay ?? '';
  const timed = Boolean(bounds.fromTime || bounds.toTime);
  if (choice.source === 'fold') {
    const from = fromDay && fromDay > choice.firstDay ? fromDay : choice.firstDay;
    const to = toDay && (choice.lastDay === '' || toDay < choice.lastDay) ? toDay : choice.lastDay;
    if (!from || !to) return refuse(`the server's rung table names no first or last day for ${choice.rung}, so there is no span to fold`);
    if (from > to) return refuse(`the date window ${fromDay || '…'} to ${toDay || '…'} lies outside the held days ${choice.firstDay} to ${choice.lastDay}, so there is nothing to fold`);
    if (timed) notes.push(`the time-of-day bound is not applied to ${choice.rung}: the server folds whole buckets of ${choice.from || 'the stored rung'}`);
    const q = instrumentQuery(subject);
    const limit = choice.secs >= 86_400 ? FOLD_PAGE_DAY : FOLD_PAGE;
    q.set('rung', choice.rung);
    q.set('from', from);
    q.set('to', to);
    q.set('dir', 'desc');
    q.set('limit', String(limit));
    if (at !== null && at !== undefined) q.set('cursor', String(at));
    return { route: '/sources/bars.json', query: q.toString(), limit, notes, why: null };
  }
  let from = choice.firstMonth;
  let to = choice.lastMonth;
  if (fromDay && fromDay.slice(0, 7) > from) from = fromDay.slice(0, 7);
  if (toDay && toDay.slice(0, 7) < to) to = toDay.slice(0, 7);
  if (from > to) return refuse(`the date window ${fromDay || '…'} to ${toDay || '…'} lies outside the held months ${choice.firstMonth} to ${choice.lastMonth}, so there is nothing to read`);
  if (monthSpan(from, to) > MAX_WINDOW_MONTHS) {
    return refuse(`the months ${from} to ${to} span ${fmt(monthSpan(from, to))}, past the ${fmt(MAX_WINDOW_MONTHS)} one window request reads — narrow the dates`);
  }
  // A DAY BAR HAS NO TIME OF DAY TO BOUND: the grid hides the bound on a day
  // rung, and the chart does the same when its own rung is one.
  const dayRung = choice.secs >= 86_400;
  if (timed && dayRung) notes.push('the time-of-day bound is not applied to a day rung');
  const query = windowQuery(feed, { keys: [subject.key], timeframe: choice.rung, from, to }, {
    sort: 'ts', desc: true, offset: at ?? 0, limit: CHART_PAGE, fromDay, toDay,
    fromTime: dayRung ? '' : bounds.fromTime, toTime: dayRung ? '' : bounds.toTime, extremes: false
  });
  return { route: '/bars/window.json', query, limit: CHART_PAGE, notes, why: null };
}

/**
 * @typedef {object} ChartBar
 * @property {number} t unix seconds: the bar's stamp, or the bucket's start
 * @property {number} o paisa @property {number} h @property {number} l @property {number} c
 * @property {number} v @property {number | null} oi
 * @property {string | null} phase the server's phase word, stored rungs only
 * @property {number} at the stored instant greeks are asked at
 * @property {number | null} seconds how many stored bars a folded bucket holds
 */

const whole = (/** @type {unknown} */ v) => typeof v === 'number' && Number.isSafeInteger(v);

/** @param {any} r @param {number} t @param {number} at @param {number | null} seconds @returns {ChartBar | null} */
function barOf(r, t, at, seconds) {
  if (!whole(t) || !whole(r?.o) || !whole(r?.h) || !whole(r?.l) || !whole(r?.c)) return null;
  return {
    t, o: r.o, h: r.h, l: r.l, c: r.c, v: whole(r.v) ? r.v : 0, oi: whole(r.oi) ? r.oi : null,
    phase: typeof r.phase === 'string' ? r.phase : null, at, seconds
  };
}

/**
 * THE BARS OF ONE ANSWERED PAGE, oldest first, one per stamp. A row whose
 * prices are not whole paisa is not drawn and is counted (`unread`); a second
 * row of a stamp already taken is counted (`dupes`), never drawn over the
 * first. `more` is where the next older page starts, or `null` at the end.
 *
 * @param {'window' | 'fold'} source
 * @param {any} body the route's answer
 * @param {string} feed
 * @param {number} at the offset this page was asked at (stored rungs)
 */
export function pageBars(source, body, feed, at = 0) {
  /** @type {ChartBar[]} */
  const bars = [];
  let unread = 0;
  let other = 0;
  /** @type {number | null} */
  let more = null;
  let total = /** @type {number | null} */ (null);
  if (source === 'fold') {
    if (!Array.isArray(body?.rows)) throw new Error(String(body?.error ?? 'the fold route answered without rows'));
    for (const row of body.rows) {
      const cell = row?.cells?.[feed];
      if (!cell) {
        other += 1; // A BUCKET ONLY ANOTHER FEED HOLDS is not this feed's bar.
        continue;
      }
      const first = istInstant(istDayKey(Number(row.t) * 1000), String(cell.first_at ?? ''));
      const bar = barOf(cell, row.t, first ?? row.t, whole(cell.seconds) ? cell.seconds : null);
      if (bar) bars.push(bar);
      else unread += 1;
    }
    more = whole(body.next_cursor) ? body.next_cursor : null;
  } else {
    if (!Array.isArray(body?.bars) || !whole(body.total)) throw new Error(String(body?.error ?? 'the window route answered without bars'));
    for (const row of body.bars) {
      const bar = barOf(row, row?.t, row?.t, null);
      if (bar) bars.push(bar);
      else unread += 1;
    }
    total = body.total;
    const next = at + body.bars.length;
    more = body.bars.length > 0 && next < body.total ? next : null;
  }
  bars.sort((a, b) => a.t - b.t);
  let dupes = 0;
  const one = bars.filter((b, i) => {
    if (i > 0 && bars[i - 1].t === b.t) {
      dupes += 1;
      return false;
    }
    return true;
  });
  return { bars: one, more, total, unread, dupes, other, faults: typeof body?.faults === 'string' ? body.faults : null };
}

/**
 * JOIN A PAGE TO THE BARS HELD, oldest first, one bar per stamp. A stamp both
 * hold keeps the bar already held (a page read twice changes nothing).
 * O(held + page): the usual page lies wholly before what is held and is laid
 * in front of it; anything else is a two-way merge.
 *
 * @param {ChartBar[]} held oldest first @param {ChartBar[]} page oldest first
 */
export function joinBars(held, page) {
  if (page.length === 0) return { bars: held, added: 0, dupes: 0 };
  if (held.length === 0) return { bars: page, added: page.length, dupes: 0 };
  if (page[page.length - 1].t < held[0].t) return { bars: page.concat(held), added: page.length, dupes: 0 };
  if (page[0].t > held[held.length - 1].t) return { bars: held.concat(page), added: page.length, dupes: 0 };
  /** @type {ChartBar[]} */
  const out = [];
  let i = 0;
  let j = 0;
  let dupes = 0;
  while (i < held.length || j < page.length) {
    if (j >= page.length || (i < held.length && held[i].t < page[j].t)) out.push(held[i++]);
    else if (i >= held.length || page[j].t < held[i].t) out.push(page[j++]);
    else {
      out.push(held[i++]);
      j += 1;
      dupes += 1;
    }
  }
  return { bars: out, added: page.length - dupes, dupes };
}

/**
 * WHETHER ONE MORE PAGE MAY BE READ: refused by name when the bars held plus
 * a page would pass [`CHART_KEEP`]. `null` is "yes".
 * @param {number} held @param {number} page the rows the next page asks for
 */
export function keepRefusal(held, page) {
  if (held + page <= CHART_KEEP) return null;
  return `the chart holds ${fmt(held)} bars and keeps at most ${fmt(CHART_KEEP)}, so the earlier bars are not read: choose a coarser rung or narrow the dates to see them`;
}

/** A paisa integer as the rupee number a chart axis plots. @param {number} p */
export const rupees = (p) => p / 100;

/**
 * THE GREEKS TO ASK FOR ONE PAGE: the page's instants, newest first, in the
 * requests `/greeks.json` takes — one per contract month, [`GREEK_CHUNK`]
 * instants each — and no more than `max` of them. An instant that would need
 * one more request is not asked and is counted in `skipped`.
 *
 * @param {ChartBar[]} bars the page's bars
 * @param {ReturnType<typeof chartSubject>} subject
 * @param {string} tf the STORED rung the instants are bars of
 * @param {number} [max]
 */
export function greekPlan(bars, subject, tf, max = GREEK_REQUESTS) {
  /** @type {{instrument: string, tf: string, ts: number, side: string | null, month: string, t: number}[]} */
  const rows = [];
  if (!subject.option || !subject.parsed) return { rows, requests: 0, skipped: 0 };
  /** @type {Map<string, number>} */
  const perMonth = new Map();
  let requests = 0;
  let skipped = 0;
  for (let i = bars.length - 1; i >= 0; i -= 1) {
    const b = bars[i];
    const ts = b.at * 1000;
    const month = ownMonth({ ts });
    const n = perMonth.get(month) ?? 0;
    if (n % GREEK_CHUNK === 0) {
      if (requests >= max) {
        skipped += 1;
        continue;
      }
      requests += 1;
    }
    perMonth.set(month, n + 1);
    rows.push({ instrument: subject.key, tf, ts, side: subject.parsed.side, month, t: b.t });
  }
  return { rows, requests, skipped };
}

/**
 * FILE A PAGE'S GREEK ANSWERS UNDER THE BAR THEY PRICE: `at` maps a bar's
 * stamp to its cell. A bar never asked has no entry.
 *
 * @param {ReturnType<typeof greekPlan>['rows']} rows
 * @param {Map<string, any>} cells `readGreeks`' answer
 * @param {Map<number, any>} at filled in place
 */
export function fileGreeks(rows, cells, at) {
  for (const r of rows) {
    const cell = cells.get(greekKey(r));
    if (cell) at.set(r.t, cell);
  }
  return at;
}

/**
 * THE LINE OF ONE FIGURE over the bars held, and why the rest have none. A
 * priced bar gives a point; an unpriced one gives none and adds one to its
 * reason's count, so a line with holes says what the holes are.
 *
 * @param {ChartBar[]} bars oldest first
 * @param {Map<number, any>} at
 * @param {string} field `iv` or a greek's field
 */
export function greekLine(bars, at, field) {
  /** @type {{time: number, value: number, aged?: boolean}[]} */
  const points = [];
  /** @type {Map<string, number>} */
  const whys = new Map();
  let unasked = 0;
  for (const b of bars) {
    const cell = at.get(b.t);
    if (!cell) {
      unasked += 1;
      continue;
    }
    const v = cell[field];
    if (cell.why === null && typeof v === 'number' && Number.isFinite(v)) {
      const point = { time: b.t, value: field === 'iv' ? v * 100 : v };
      // A POINT PRICED FROM AN EARLIER SPOT IS MARKED, so the line can draw it apart (D-3328).
      points.push(typeof cell.age === 'number' && cell.age > 0 ? { ...point, aged: true } : point);
    } else {
      const why = String(cell.why ?? 'no_figure');
      whys.set(why, (whys.get(why) ?? 0) + 1);
    }
  }
  return { points, whys: [...whys.entries()].sort((a, b) => b[1] - a[1]), unasked };
}

/**
 * THE SPOT AGES OF THE POINTS DRAWN (D-3328): how many bars are priced, how
 * many of those from a spot EARLIER than the bar (`earlier`), the largest
 * such age in seconds (`oldest`), and how many priced cells carry no age at
 * all (`unsaid`). O(bars held), bounded by [`CHART_KEEP`].
 *
 * @param {ChartBar[]} bars
 * @param {Map<number, any>} at
 */
export function spotAges(bars, at) {
  let priced = 0;
  let earlier = 0;
  let oldest = 0;
  let unsaid = 0;
  for (const b of bars) {
    const cell = at.get(b.t);
    if (!cell || cell.why !== null) continue;
    priced += 1;
    if (typeof cell.age !== 'number' || !Number.isFinite(cell.age)) unsaid += 1;
    else if (cell.age > 0) {
      earlier += 1;
      if (cell.age > oldest) oldest = cell.age;
    }
  }
  return { priced, earlier, oldest, unsaid };
}

/**
 * THE SPOT RULE IN FORCE, as the IV note says it. Under the opt-in rule the
 * count of points priced from an earlier spot is always printed, zero
 * included, with the oldest age. Under the default rule an aged point is a
 * fault, and is said as one.
 *
 * @param {string} asof `prev` for 'last at or before t', else the exact second
 * @param {ReturnType<typeof spotAges>} ages
 */
export function spotRuleText(asof, ages) {
  const oldest = ages.earlier > 0 ? `, the oldest ${fmt(ages.oldest)} s before its bar` : '';
  if (asof === 'prev') {
    return `Spot rule: last at or before t (opted in; never a later spot). ${fmt(ages.earlier)} of the ${fmt(ages.priced)} priced point(s) drawn were priced from an earlier spot${oldest}; they are drawn in the warning colour, and the readout prints each bar's own spot age.${ages.unsaid > 0 ? ` ${fmt(ages.unsaid)} priced point(s) carry no spot age.` : ''}`;
  }
  return `Spot rule: exact second (a bar with no spot at its own second is not priced).${ages.earlier > 0 ? ` ${fmt(ages.earlier)} point(s) were priced from an earlier spot all the same${oldest}: the answer does not follow the rule asked for.` : ''}`;
}

/**
 * THE HOVERED BAR'S OWN SPOT AGE, for the readout beside its IV. Under the
 * opt-in rule every priced bar says where its spot came from; under the
 * default rule only a bar that was aged all the same does.
 *
 * @param {{why?: string | null, age?: number | null} | null | undefined} cell
 * @param {string} asof
 */
export function spotAgeWord(cell, asof) {
  if (!cell) return '';
  const age = typeof cell.age === 'number' && Number.isFinite(cell.age) ? cell.age : null;
  if (age !== null && age > 0) return `spot ${fmt(age)} s before this bar`;
  if (asof !== 'prev') return '';
  return age === 0 ? "spot at this bar's own second" : 'spot age not said by the server';
}

/**
 * The newest [`DAYFLAGS_PAGE_DAYS`] days of a page of bars, as the day-flag
 * request spans them. `clipped` says the page reaches further back than one
 * day-flag page reads.
 *
 * A PAGE THAT HOLDS NO BAR has no days of its own, so the selection's own day
 * bounds are asked instead (`bounds`): a session the store holds nothing of
 * is then named by its day record, not passed over as "no bar". With no last
 * day there is no newest day to count back from, and nothing is asked.
 *
 * @param {ChartBar[]} bars oldest first
 * @param {Bounds} [bounds] the selection's bounds, given only when no bar is held
 */
export function flagSpan(bars, bounds) {
  const floorOf = (/** @type {string} */ to) =>
    new Date(Date.parse(`${to}T00:00:00Z`) - (DAYFLAGS_PAGE_DAYS - 1) * 86_400_000).toISOString().slice(0, 10);
  if (bars.length === 0) {
    const to = dayOf(bounds?.toDay);
    const from = dayOf(bounds?.fromDay);
    if (!to || (bounds?.fromDay && !from) || (from && from > to)) return null;
    const floor = floorOf(to);
    return !from || from < floor ? { from: floor, to, clipped: true } : { from, to, clipped: false };
  }
  const to = istDayKey(bars[bars.length - 1].t * 1000);
  const first = istDayKey(bars[0].t * 1000);
  const floor = floorOf(to);
  return first < floor ? { from: floor, to, clipped: true } : { from: first, to, clipped: false };
}

/**
 * @typedef {object} Band
 * @property {'halt' | 'gap' | 'cas'} kind
 * @property {number} from unix seconds, inclusive @property {number} to exclusive
 * @property {string} day @property {string} label @property {string} detail
 */

/**
 * THE BANDS THE DAY RECORDS NAME: a halt from its own first and resuming
 * second, the closing auction where the day's own record says the series has
 * one, and a session day the store holds no bar of. The rule for what a day
 * is comes from `dayflags.js`, the same one the day band prints.
 *
 * @param {any[]} days `/dayflags.json`'s `days`
 * @param {{archive?: {from: string, to: string} | null}} [ctx]
 * @returns {{bands: Band[], cells: ReturnType<typeof cellOf>[]}}
 */
export function dayBands(days, ctx = {}) {
  /** @type {Band[]} */
  const bands = [];
  const cells = [];
  for (const rec of Array.isArray(days) ? days : []) {
    const cell = cellOf(rec, ctx);
    cells.push(cell);
    const day = cell.day;
    const halt = rec?.events?.halt;
    if (halt && typeof halt === 'object') {
      const from = istInstant(day, String(halt.seconds?.from ?? halt.halted?.from ?? ''));
      const to = istInstant(day, String(halt.seconds?.resume ?? ''));
      if (from !== null && to !== null && to > from) {
        bands.push({
          kind: 'halt', from, to, day,
          label: `halt (${String(halt.standing ?? 'unverified')}) ${String(halt.seconds?.from ?? '')}–${String(halt.seconds?.resume ?? '')} IST`,
          detail: String(halt.evidence ?? '')
        });
      }
    }
    const phases = Array.isArray(rec?.events?.phases) ? rec.events.phases : [];
    if (cell.kinds.includes('cas')) {
      // THE AUCTION'S MINUTES ARE THE DAY RECORD'S OWN. Where the record does
      // not know this share's eligibility it calls those minutes `unverified`
      // (D-3320), and the band says so instead of claiming an auction.
      for (const p of phases) {
        if (p?.phase !== 'cas' && p?.phase !== 'unverified') continue;
        const from = istInstant(day, String(p.from));
        const to = istInstant(day, String(p.to));
        if (from !== null && to !== null) {
          bands.push({
            kind: 'cas', from, to: to + 60, day,
            label: p.phase === 'cas'
              ? `closing auction ${p.from}–${p.to} IST`
              : `closing-auction window ${p.from}–${p.to} IST, unverified for this series (the day record does not know its eligibility)`,
            detail: ''
          });
        }
      }
    }
    if (cell.kinds.some((k) => k === 'store_gap' || k === 'refused' || k === 'incomplete')) {
      const open = phases.filter((/** @type {any} */ p) => p?.phase === 'continuous');
      const from = open.length ? istInstant(day, String(open[0].from)) : istInstant(day, '00:00');
      const last = open.length ? istInstant(day, String(open[open.length - 1].to)) : null;
      if (from !== null) {
        bands.push({
          kind: 'gap', from, to: last !== null ? last + 60 : from + 86_400, day,
          label: `${cell.word} on ${day}`, detail: cell.says.join('; ')
        });
      }
    }
  }
  return { bands, cells };
}

/**
 * THE HOLES IN THE BARS HELD, on the days the server flagged a gap: two
 * neighbouring bars of one IST day further apart than the rung, inside that
 * day's continuous session. A trade-driven series is never asked (its bars
 * are its trades, D-3336 §2), and a hole inside a halt is the halt.
 *
 * At most [`GAP_BANDS`] are returned; `more` counts the rest.
 *
 * @param {ChartBar[]} bars oldest first
 * @param {number} secs the rung's length
 * @param {ReturnType<typeof cellOf>[]} cells
 * @param {Band[]} halts
 * @param {boolean} tradeDriven
 * @returns {{bands: Band[], more: number}}
 */
export function holeBands(bars, secs, cells, halts, tradeDriven) {
  /** @type {Band[]} */
  const bands = [];
  let more = 0;
  if (tradeDriven || secs >= 86_400) return { bands, more };
  // A DAY WHOSE RECORD SAYS TIME IS MISSING (a minute gap, a hole of 5 s or
  // more, a hole most shares share); a second with two values is not one
  // (AH-03, D-5838).
  const flagged = new Set(cells.filter((c) => isHoleDay(c)).map((c) => c.day));
  if (flagged.size === 0) return { bands, more };
  const least = secs < 60 ? SECOND_HOLE : 1;
  for (let i = 1; i < bars.length; i += 1) {
    const a = bars[i - 1].t;
    const b = bars[i].t;
    const missing = Math.round((b - a) / secs) - 1;
    if (missing < least) continue;
    const day = istDayKey(a * 1000);
    if (!flagged.has(day) || istDayKey(b * 1000) !== day) continue;
    if (halts.some((h) => h.from < b && h.to > a + secs)) continue;
    if (bands.length >= GAP_BANDS) {
      more += 1;
      continue;
    }
    bands.push({ kind: 'gap', from: a + secs, to: b, day, label: `${fmt(missing)} bar(s) missing on ${day}`, detail: '' });
  }
  return { bands, more };
}

/** The first index of `times` (ascending) at or after `t`. @param {number[]} times @param {number} t */
function lowerBound(times, t) {
  let lo = 0;
  let hi = times.length;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (times[mid] < t) lo = mid + 1;
    else hi = mid;
  }
  return lo;
}

/**
 * THE BANDS AS SLOTS OF THE TIME AXIS. The chart's axis is the stamps it was
 * given, so a halt with no bar in it would have no width at all. Each band
 * therefore takes every held bar inside it, and where it holds none, empty
 * slots on the rung's grid — at most [`BAND_SLOTS`] of them a band — so the
 * halt is a shaded block with no candle in it, which is what happened.
 *
 * A bar's own `phase` word shades it too (the auction and halt minutes the
 * window route labels, D-3320), whether or not a day record was read.
 *
 * Bands are clipped to the bars held: nothing is drawn before the first or
 * after the last. One slot has one kind, the first of [`BAND_KINDS`].
 *
 * @param {Band[]} bands
 * @param {ChartBar[]} bars oldest first
 * @param {number} secs the rung's length
 * @returns {{time: number, kind: string}[]} ascending, one per stamp
 */
export function bandSlots(bands, bars, secs) {
  /** @type {Map<number, string>} */
  const at = new Map();
  const put = (/** @type {number} */ time, /** @type {string} */ kind) => {
    const had = at.get(time);
    if (had === undefined || (BAND_RANK.get(kind) ?? 99) < (BAND_RANK.get(had) ?? 99)) at.set(time, kind);
  };
  if (bars.length === 0) return [];
  const times = bars.map((b) => b.t);
  const lo = times[0];
  const hi = times[times.length - 1] + secs;
  for (const b of bars) {
    const kind = b.phase ? PHASE_BAND.get(b.phase) : undefined;
    if (kind) put(b.t, kind);
  }
  // THE RUNG'S OWN BUCKET GRID is the one its held bars stand on: buckets are
  // anchored at the session's open, which is not a multiple of every rung
  // from the epoch (a two-, ten-, thirty- or sixty-minute bucket is off it).
  const phase = ((lo % secs) + secs) % secs;
  for (const band of bands) {
    // AT A DAY RUNG THE CLOSING AUCTION SHADES NOTHING: it is part of every
    // session from its start date, so shading it would shade every candle.
    if (secs >= 86_400 && band.kind === 'cas') continue;
    const from = Math.max(band.from, lo);
    const to = Math.min(band.to, hi);
    if (!(to > from)) continue;
    // A HELD BAR IS SHADED WHERE IT STANDS when the band takes most of it, or
    // lies wholly inside it. A bar covers its own rung from its stamp: the
    // minute a halt begins ten seconds into is shaded, the minute trading
    // resumed one second into is not, and the one day candle of a day a
    // halt fell on is.
    for (let i = lowerBound(times, from - secs + 1); i < times.length && times[i] < to; i += 1) {
      const t = times[i];
      const shared = Math.min(t + secs, to) - Math.max(t, from);
      if (shared * 2 > secs || (t <= from && t + secs >= to)) put(t, band.kind);
    }
    // AND AN EMPTY SLOT a step apart wherever no held bar covers that step.
    // A day bucket is stamped at its own open, not on a UTC grid, so an
    // empty day slot sits at the band's own start.
    const step = Math.max(secs, Math.ceil((to - from) / BAND_SLOTS / secs) * secs);
    const start = secs >= 86_400 ? from : Math.ceil((from - phase) / secs) * secs + phase;
    for (let cursor = start; cursor < to; cursor += step) {
      const i = lowerBound(times, cursor - secs + 1);
      if (!(i < times.length && times[i] < Math.min(cursor + step, to))) put(cursor, band.kind);
    }
  }
  return [...at.entries()].sort((a, b) => a[0] - b[0]).map(([time, kind]) => ({ time, kind }));
}

/**
 * ONE MARKER PER FLAGGED DAY, on the day's first held bar, or on its first
 * band slot when the day holds no bar (or, when the selection holds no bar at
 * all, at the day's own midnight: a list entry, not a mark on a candle). A day whose only flag is the closing
 * auction is not marked — every session has one from its start date, and its
 * band already says so. Ascending, one per stamp.
 *
 * @param {ReturnType<typeof cellOf>[]} cells
 * @param {ChartBar[]} bars oldest first
 * @param {{time: number, kind: string}[]} slots
 */
export function flagMarkers(cells, bars, slots) {
  /** @type {Map<string, number>} */
  const first = new Map();
  for (const s of slots) {
    const day = istDayKey(s.time * 1000);
    if (!first.has(day)) first.set(day, s.time);
  }
  /** @type {Map<string, number>} */
  const firstBar = new Map();
  for (const b of bars) {
    const day = istDayKey(b.t * 1000);
    if (!firstBar.has(day)) firstBar.set(day, b.t);
  }
  const out = [];
  for (const c of cells) {
    if (!c.flagged) continue;
    const real = c.kinds.filter((k) => k !== 'cas' && k !== 'session');
    if (real.length === 0) continue;
    // WITH NO BAR HELD AT ALL the day is listed at its own midnight, so the
    // panel can still say what the day record says of it.
    const time = firstBar.get(c.day) ?? first.get(c.day) ?? (bars.length === 0 ? (istInstant(c.day, '00:00') ?? undefined) : undefined);
    if (time === undefined) continue;
    out.push({ time, day: c.day, kind: c.kind, tone: c.tone, word: c.word, says: c.says });
  }
  return out.sort((a, b) => a.time - b.time);
}

/**
 * Whether a series holds a bar only where a trade printed (D-3336 §2): an
 * option or a future, or a share at a sub-minute rung. The same rule
 * `dayflags.js` reads a held day by.
 * @param {ReturnType<typeof chartSubject>} subject @param {number} secs
 */
export const tradeDriven = (subject, secs) =>
  subject.parsed?.segment === 'FNO' || (subject.parsed?.segment === 'CASH' && secs < 60);

/**
 * A `#rgb` or `#rrggbb` colour with an alpha, as `#rrggbbaa`. Any other
 * spelling is returned as it came: a token this cannot read is drawn opaque,
 * never dropped.
 * @param {string} colour @param {number} alpha 0..1
 */
export function alphaHex(colour, alpha) {
  const c = String(colour ?? '').trim();
  const short = c.match(/^#([0-9a-f])([0-9a-f])([0-9a-f])$/i);
  const long = short ? `#${short[1]}${short[1]}${short[2]}${short[2]}${short[3]}${short[3]}` : c;
  if (!/^#[0-9a-f]{6}$/i.test(long)) return c;
  const a = Math.round(Math.min(1, Math.max(0, alpha)) * 255).toString(16).padStart(2, '0');
  return `${long}${a}`;
}

/**
 * What one read cost, as the panel says it: its requests against the bound.
 * @param {number} requests @param {number} ms
 */
export const costText = (requests, ms) => `${fmt(requests)} of at most ${fmt(PAN_REQUESTS)} request(s), ${fmt(Math.round(ms))} ms`;
