/**
 * /sources — ALL SOURCES FOR ONE INSTRUMENT, the arithmetic-free half
 * (WP-58; D-3339, with D-3323 and D-3324 for the labels; findings DB-079,
 * DB-G41, DB-G58, DB-G99).
 *
 * # Why this exists
 *
 * `/db` is one feed at a time by design, so the operator reconciled Zerodha
 * against GDFL by hand: NIFTY's day close differs from GDFL's last second on
 * 19 of 19 days of 2019-03 because one is the official close and the other the
 * last print; RELIANCE's Zerodha prices sit at GDFL ÷ 2.098 because Zerodha
 * back-adjusts shares and GDFL does not. WP-37 built three read-only routes
 * that put the feeds side by side (`/sources.json`, `/sources/bars.json`,
 * `/sources/day.json`); this module turns their answers into what the page
 * draws, with no runes in it, so `web/tests/sources.test.js` drives every line
 * under `node --test`.
 *
 * # What this module never does
 *
 * - It never compares two prices itself. Every difference, flag, factor and
 *   tolerance is the server's (`crates/api/src/sources.rs`), where the fold
 *   authority and the named thresholds live. The one thing computed here is
 *   whether two paisa integers the server sent are the same integer, where
 *   the server's comparison carries no `equal` of its own.
 * - It never merges feeds. Every row it returns names its feed and its tag
 *   (stored, derived or fold), because storage stays per vendor (D-0225).
 * - It never hides a code it does not know. A flag, state or reason this file
 *   has no word for is shown verbatim, toned as a warning, never dropped.
 * - It never keeps a list the data already knows: feeds, rungs, tags, spans
 *   and links into /db all come from the answers. A link's feed, key and rung
 *   are the server's; its segment is left for /db to derive from the key,
 *   because the server writes the store's segment name and /db has its own
 *   (`dbLinkOf`).
 */

import { parseKey } from './instrument.js';
import { rupee, exact } from './money.js';
import { decodeDb, dbHref } from './urlstate.js';
import { SEGMENT_KEYS } from './db/selection-model.js';

// ══ The selection and its address ════════════════════════════════════════════

/**
 * What /sources opens on when the address names nothing: the instrument /db
 * opens on (the operator's answer 11), at the finest rung both feeds can show
 * side by side, newest first.
 */
export const SOURCES_DEFAULTS = Object.freeze({ key: 'NSE-INDEX-NIFTY', rung: '1min', dir: 'desc' });

/** The address keys, in the one order `encodeSources` writes them. */
export const SOURCES_KEYS = Object.freeze(['key', 'rung', 'day', 'dir', 'cursor', 'month']);

const DAY = /^\d{4}-\d{2}-\d{2}$/;
const MONTH = /^\d{4}-\d{2}$/;
const RUNG = /^\d{1,3}(s|min|day)$/;
const CURSOR = /^-?\d{1,12}$/;

/**
 * @typedef {object} SourcesSelection
 * @property {string} [key]    the instrument key, contract tail included
 * @property {string} [rung]   the timeline's rung, the store's spelling
 * @property {string} [day]    the timeline's day, `YYYY-MM-DD`
 * @property {'asc'|'desc'} [dir]
 * @property {string} [cursor] unix seconds, the timeline's continuation
 * @property {string} [month]  the day table's month, `YYYY-MM`
 */

/**
 * Whether a day or a month is a real calendar date, not merely the right
 * shape: `2019-02-30` and `2019-13` are refused by name here rather than sent
 * to the server to be refused there.
 *
 * @param {string} text `YYYY-MM-DD` or `YYYY-MM`
 */
export function realDate(text) {
  const m = /^(\d{4})-(\d{2})(?:-(\d{2}))?$/.exec(String(text ?? ''));
  if (!m) return false;
  const [y, mo, d] = [Number(m[1]), Number(m[2]), m[3] === undefined ? 1 : Number(m[3])];
  if (mo < 1 || mo > 12 || d < 1) return false;
  return d <= new Date(Date.UTC(y, mo, 0)).getUTCDate();
}

/**
 * One instrument key as the three routes ask for it, or why it cannot be.
 *
 * A future is refused by name: the All sources routes take a spot series or
 * an option contract, and a future's tail would be sent as a symbol it never
 * was.
 *
 * @param {unknown} key
 * @returns {{ok: true, key: string, exchange: string, segment: string, symbol: string, contract: string|null} | {ok: false, key: string, why: string}}
 */
export function instrumentOf(key) {
  const text = typeof key === 'string' ? key.trim().toUpperCase() : '';
  if (!text) return { ok: false, key: '', why: 'no instrument is named' };
  const p = parseKey(text);
  if (p.why !== null || !p.exchange || !p.segment || !p.underlying) {
    return {
      ok: false,
      key: text,
      why: `${JSON.stringify(text)} is not an instrument key (${p.why ?? 'it names no exchange, segment and symbol'}). A key reads EXCHANGE-SEGMENT-SYMBOL, like NSE-INDEX-NIFTY, with -YYYY-MM-DD-STRIKEPAISA-CE or -PE after it for one option contract`
    };
  }
  if (p.kind === 'future') {
    return { ok: false, key: text, why: `${text} is a future: the All sources routes compare a spot series or one option contract (D-3339)` };
  }
  const contract = p.kind === 'option' ? text.slice(`${p.exchange}-${p.segment}-${p.underlying}-`.length) : null;
  return { ok: true, key: text, exchange: p.exchange, segment: p.segment, symbol: p.underlying, contract };
}

/**
 * The page's address back to a selection, plus every field it could not read.
 *
 * It reads three spellings of the same instrument, so a link from anywhere
 * lands: this page's own `key=`, the routes' `exchange=&segment=&symbol=
 * [&contract=]`, and /db's (`key=`, `tf=` for the rung, `from=` for the day).
 * A malformed field is left out of `sel` and named in `refused`; nothing is
 * coerced.
 *
 * @param {string | null | undefined} search
 * @returns {{sel: SourcesSelection, refused: Array<{key: string, value: string, why: string}>}}
 */
export function decodeSources(search) {
  const p = new URLSearchParams(String(search ?? '').replace(/^\?/, ''));
  /** @param {string} k */
  const text = (k) => (p.has(k) ? String(p.get(k)).trim() : '');
  /** @type {SourcesSelection} */
  const sel = {};
  /** @type {Array<{key: string, value: string, why: string}>} */
  const refused = [];

  let key = text('key').toUpperCase();
  if (!key && text('symbol')) {
    const tail = text('contract');
    key = [text('exchange') || 'NSE', text('segment'), text('symbol')].join('-').toUpperCase() + (tail ? `-${tail.toUpperCase()}` : '');
  }
  if (key) {
    const inst = instrumentOf(key);
    if (inst.ok) sel.key = inst.key;
    else refused.push({ key: 'key', value: key, why: inst.why });
  }

  const rung = text('rung') || text('tf');
  if (rung) {
    if (RUNG.test(rung)) sel.rung = rung;
    else refused.push({ key: 'rung', value: rung, why: 'a rung is spelled like 1s, 1min, 5min or 1day' });
  }

  const day = text('day') || text('from');
  if (day) {
    if (DAY.test(day) && realDate(day)) sel.day = day;
    else refused.push({ key: 'day', value: day, why: 'a day is YYYY-MM-DD and must be a real date' });
  }

  const month = text('month');
  if (month) {
    if (MONTH.test(month) && realDate(month)) sel.month = month;
    else refused.push({ key: 'month', value: month, why: 'a month is YYYY-MM and must be a real month' });
  }

  const dir = text('dir');
  if (dir) {
    if (dir === 'asc' || dir === 'desc') sel.dir = dir;
    else refused.push({ key: 'dir', value: dir, why: 'a direction is asc or desc' });
  }

  const cursor = text('cursor');
  if (cursor) {
    if (CURSOR.test(cursor)) sel.cursor = cursor;
    else refused.push({ key: 'cursor', value: cursor, why: 'a cursor is a whole number of unix seconds' });
  }
  return { sel, refused };
}

/**
 * A /sources selection as a query string, no leading `?`. Idempotent: one
 * selection is one address, byte for byte — keys in `SOURCES_KEYS` order,
 * defaults left out.
 *
 * @param {SourcesSelection | null | undefined} sel
 */
export function encodeSources(sel) {
  const p = new URLSearchParams();
  /** @param {string} k @param {unknown} v */
  const put = (k, v) => {
    const s = v === null || v === undefined ? '' : String(v).trim();
    if (s) p.append(k, s);
  };
  put('key', sel?.key ? String(sel.key).toUpperCase() : '');
  if (sel?.rung && sel.rung !== SOURCES_DEFAULTS.rung) put('rung', sel.rung);
  put('day', sel?.day);
  if (sel?.dir && sel.dir !== SOURCES_DEFAULTS.dir) put('dir', sel.dir);
  put('cursor', sel?.cursor);
  put('month', sel?.month);
  return p.toString();
}

/** @param {SourcesSelection | null | undefined} sel */
export function sourcesHref(sel) {
  const q = encodeSources(sel);
  return q ? `/sources?${q}` : '/sources';
}

/**
 * The ONE series a /db selection is made of, by the census's own key, or `''`
 * when it is none or more than one. /db's anchor prints an underlying
 * (`NIFTY`), which is a name and not a series; the census rows the selection
 * matched each carry the store's key (`NSE-INDEX-NIFTY`). The walk stops at
 * the first row of a second series, so it is bounded by one series' months.
 *
 * @param {unknown} rows the matched census rows, each `{instrument}`
 */
export function oneInstrument(rows) {
  if (!Array.isArray(rows)) return '';
  let key = '';
  for (const row of rows) {
    const k = typeof row?.instrument === 'string' ? row.instrument : '';
    if (!k) return '';
    if (key === '') key = k;
    else if (k !== key) return '';
  }
  return key;
}

/**
 * The link /db puts beside its instrument: the same instrument, the same rung
 * and the same first day, on /sources. `null` when /db names no single
 * instrument, so the caller draws no link rather than one that opens on
 * another instrument.
 *
 * @param {{key?: string|null, tf?: string|null, from?: string|null}} db
 */
export function sourcesHrefOfDb(db) {
  const inst = instrumentOf(db?.key ?? '');
  if (!inst.ok) return null;
  /** @type {SourcesSelection} */
  const sel = { key: inst.key };
  if (db?.tf && RUNG.test(db.tf)) sel.rung = db.tf;
  if (db?.from && DAY.test(db.from) && realDate(db.from)) sel.day = db.from;
  return sourcesHref(sel);
}

// ══ The three requests ═══════════════════════════════════════════════════════

/** The most rows one timeline page asks for below the day rung: /db's page. */
export const TIMELINE_LIMIT = 50;

/** The most days one day table page holds (the server's `PAGE_DAYS`): a month always fits one page. */
export const PAGE_DAYS = 31;

/**
 * @param {ReturnType<typeof instrumentOf>} inst
 */
function instrumentQuery(inst) {
  if (!inst.ok) return null;
  const q = new URLSearchParams({ exchange: inst.exchange, segment: inst.segment, symbol: inst.symbol });
  if (inst.contract) q.set('contract', inst.contract);
  return q;
}

/** `/sources.json` for one instrument key, or `null` for a key that is not one. @param {string} key */
export function rungsUrl(key) {
  const q = instrumentQuery(instrumentOf(key));
  return q ? `/sources.json?${q}` : null;
}

/** One day in seconds: the length of the day rung. */
const DAY_SECONDS = 86_400;

/**
 * What the timeline does at one rung:
 *
 * - `'day'`  the rung is the day rung THE RUNG TABLE NAMES (one day long, and
 *   spelled exactly as a row of the table spells it): the timeline is the day
 *   table, and `/sources/bars.json` is asked nothing.
 * - `'ask'`  every other rung is asked of the server, whose answer or refusal
 *   is drawn. That includes a rung that only LOOKS like a day rung: `2day`,
 *   `999day`, `0day` and `01day` are not rungs the store has, the server
 *   refuses each by name, and the page draws that refusal. It used to match
 *   `/day$/`, ask nothing, and print "At 2day the timeline is the day table
 *   below" over the 1day table: a garbage parameter coerced into an answer.
 * - `'wait'` the rung is one day long and the rung table has not answered
 *   yet, so whether the table names it is not known. Nothing is asked until
 *   it is; a table that fails or is refused ends the wait as `'ask'`.
 *
 * WHY THE DAY RUNG IS NOT ASKED OF `/sources/bars.json`. A page of day
 * buckets folds a month of seconds on every request and the route keeps none
 * of it. Measured twice on the operator's store, 12 Oct 2026, 200 distinct
 * months of three instruments each time: a first read took p50 196 and 260 ms,
 * p99 272 and 827 ms; the same months again took p50 34 and 289 ms, so what a
 * second read costs is the operating system's file cache and the machine's
 * load, not anything kept. `/sources/day.json` holds the same days, tagged the
 * same way, and IS kept per day by the server (p50 2.5 ms, p99 3.2 ms once
 * kept).
 *
 * @param {unknown} rung the address's rung
 * @param {{rows: Array<{rung: string}>} | null | undefined} table the rung table, once it has answered
 * @param {boolean} [answered] whether the rung table's request has ended at all (an answer, a refusal or a failure)
 * @returns {'day' | 'ask' | 'wait'}
 */
export function timelineMode(rung, table, answered = Boolean(table)) {
  if (rungSeconds(rung) !== DAY_SECONDS) return 'ask';
  if (table) return table.rows.some((r) => r.rung === String(rung)) ? 'day' : 'ask';
  return answered ? 'ask' : 'wait';
}

/**
 * Whether a rung is the day rung the rung table names. Never true of a
 * spelling the table does not hold, and never true without a table.
 *
 * @param {unknown} rung
 * @param {{rows: Array<{rung: string}>} | null | undefined} table
 */
export const isDayRung = (rung, table) => timelineMode(rung, table, true) === 'day';

/**
 * `/sources/bars.json` for one page of the timeline, or `null` with nothing
 * to ask. The page is ONE IST DAY, so a fold reads at most that day's seconds
 * whatever the rung. Whether to ask at all is `timelineMode`'s decision; this
 * only spells the request, for any rung, so a rung the store does not have
 * reaches the server and is refused there by name.
 *
 * @param {SourcesSelection} sel `key`, `rung` and `day` are required
 */
export function barsUrl(sel) {
  const q = instrumentQuery(instrumentOf(sel?.key ?? ''));
  if (!q || !sel?.rung || !sel?.day || !DAY.test(sel.day)) return null;
  q.set('rung', sel.rung);
  q.set('from', sel.day);
  q.set('to', sel.day);
  q.set('limit', String(TIMELINE_LIMIT));
  q.set('dir', sel.dir === 'asc' ? 'asc' : 'desc');
  if (sel.cursor) q.set('cursor', sel.cursor);
  return `/sources/bars.json?${q}`;
}

/**
 * `/sources/day.json` for one month of the day table, or `null`.
 *
 * @param {string} key
 * @param {string} month `YYYY-MM`
 */
export function dayUrl(key, month) {
  const q = instrumentQuery(instrumentOf(key));
  if (!q || !MONTH.test(String(month ?? ''))) return null;
  q.set('from', month);
  return `/sources/day.json?${q}`;
}

/**
 * The month before or after one, `YYYY-MM`, by integer arithmetic.
 *
 * @param {string} month
 * @param {number} step whole months, negative for earlier
 */
export function shiftMonth(month, step) {
  if (!MONTH.test(String(month ?? ''))) return '';
  const at = Number(month.slice(0, 4)) * 12 + (Number(month.slice(5, 7)) - 1) + step;
  // Outside 0000-01..9999-12 there is no month this page's own address accepts.
  if (at < 0 || at > 9999 * 12 + 11) return '';
  return `${String(Math.floor(at / 12)).padStart(4, '0')}-${String((at % 12) + 1).padStart(2, '0')}`;
}

/**
 * What a response says, as one of three outcomes the page draws differently:
 * an answer, a refusal in the server's own words, or a failure. A refusal is
 * never shown as an empty table and a failure is never shown as a refusal.
 *
 * @param {number} status
 * @param {unknown} body the parsed JSON, or `null` when it would not parse
 * @returns {{phase: 'ok', body: any} | {phase: 'refused'|'failed', why: string, status: number}}
 */
export function outcomeOf(status, body) {
  const said = body && typeof body === 'object' && typeof (/** @type {any} */ (body).error) === 'string' ? /** @type {any} */ (body).error : '';
  if (status >= 200 && status < 300) {
    if (body && typeof body === 'object' && !said) return { phase: 'ok', body };
    return { phase: 'failed', status, why: said || `the server answered ${status} with a body that is not this route's JSON` };
  }
  if (status >= 400 && status < 500) return { phase: 'refused', status, why: said || `the server refused with ${status} and gave no reason` };
  return { phase: 'failed', status, why: said || `the server answered ${status}` };
}

// ══ Words ════════════════════════════════════════════════════════════════════

/** @typedef {'ok'|'quiet'|'info'|'warn'|'bad'} Tone */

/**
 * Every flag the three routes raise (`sources.rs`), worded. `tone` says how
 * loud: `bad` a disagreement past the named tolerance, `warn` a vendor fault
 * or an unmeasured comparison, `info` a labelled difference that is not a
 * disagreement (official against last print, a basis factor).
 *
 * @type {Record<string, {word: string, tone: Tone, why: string}>}
 */
export const FLAG_WORDS = Object.freeze({
  o: { word: 'open disagrees', tone: 'bad', why: 'the two opens differ by more than the price tolerance' },
  h: { word: 'high disagrees', tone: 'bad', why: 'the two highs differ by more than the price tolerance' },
  l: { word: 'low disagrees', tone: 'bad', why: 'the two lows differ by more than the price tolerance' },
  c: { word: 'close disagrees', tone: 'bad', why: 'the two closes differ by more than the price tolerance' },
  v: { word: 'volume disagrees', tone: 'bad', why: 'the two volumes differ by more than the volume tolerance' },
  open_mismatch: { word: 'open differs', tone: 'bad', why: "GDFL's first print and Zerodha's official open differ by more than the price tolerance" },
  minute_open_mismatch: { word: 'minute open ≠ day open', tone: 'warn', why: "Zerodha's first minute opens away from its own day bar's open (DB-G41)" },
  minute_extremes_outside_day: { word: 'minute extreme outside the day range', tone: 'warn', why: "Zerodha's minutes go above its day high or below its day low: the day bar is not a fold of its minutes (DB-G41)" },
  gdfl_extremes_outside_day: { word: 'GDFL extreme outside the day range', tone: 'warn', why: "GDFL's seconds go above Zerodha's day high or below its day low" },
  official_vs_last_print: { word: 'close: official against last print', tone: 'info', why: "Zerodha's day close is the exchange's official close and the other close is the last print: labelled, never counted as a disagreement (D-3324)" },
  gdfl_first_print_after_open: { word: 'GDFL starts late', tone: 'warn', why: "GDFL's first print of the day is after the opening minute, so it is not set against the official open" },
  minute_volume_off_day: { word: 'minute volume off the day’s', tone: 'warn', why: "the sum of Zerodha's minute volumes differs from its day bar's volume by more than the volume tolerance (DB-G41)" },
  basis_differs: { word: 'price basis differs: factor shown', tone: 'info', why: "Zerodha's share prices are back-adjusted and GDFL's are as traded: the factor replaces a disagreement (D-3323, DB-G58)" },
  basis_unmeasured: { word: 'basis not measured', tone: 'warn', why: 'no pair of prices of the same instant exists on this row, so no factor is read and nothing is judged' },
  basis_step: { word: 'factor stepped', tone: 'warn', why: 'the factor moved by more than the factor tolerance since the last day measured the same way' },
  minute_basis_differs_from_day: { word: 'minutes on another basis than the day bar', tone: 'warn', why: "the factor read from Zerodha's minutes differs from the one read from its day bar" }
});

/**
 * One flag code as a chip. A code this file has no word for is shown as the
 * code itself, loudly, never dropped.
 *
 * @param {unknown} code
 * @returns {{code: string, word: string, tone: Tone, why: string}}
 */
export function flagOf(code) {
  const text = String(code ?? '');
  const known = Object.hasOwn(FLAG_WORDS, text) ? FLAG_WORDS[text] : null;
  if (known) return { code: text, ...known };
  return { code: text, word: text || 'unnamed flag', tone: 'warn', why: `the server raised ${JSON.stringify(text)}, which this page has no words for; shown as sent` };
}

/**
 * Why a comparison holds no number, or why a source holds no record of a day
 * (`absent_why` and the `why` of a comparison in `sources.rs`).
 *
 * @type {Record<string, string>}
 */
export const WHY_WORDS = Object.freeze({
  basis_differs: 'basis differs',
  basis_unmeasured: 'basis not measured',
  gdfl_first_print_after_open: 'GDFL starts late',
  index_no_volume: 'an index has no volume',
  no_volume_to_compare: 'no volume to compare',
  not_a_price: 'not a price',
  not_a_derivative: 'not a derivative',
  not_recorded: 'not recorded',
  no_feed_stores_this_rung: 'no feed stores this rung',
  none_of_this_instrument: 'the feed holds none of this instrument',
  before_archive: 'before the feed’s archive begins',
  after_archive: 'after the feed’s archive ends',
  outside_your_window: 'outside your window',
  closed: 'the exchange was closed'
});

/** A reason code in words; an unknown one verbatim, underscores opened. @param {unknown} code */
export function whyOf(code) {
  const text = String(code ?? '');
  if (!text) return 'no reason was sent';
  return Object.hasOwn(WHY_WORDS, text) ? WHY_WORDS[text] : text.replaceAll('_', ' ');
}

/**
 * A source's tag in the plan's words: `zerodha 1min stored`, `zerodha 5min
 * derived`, `gdfl 1s→1min fold`. One rule for every feed: the words are made
 * from the feed, the kind and the rungs the answer itself names.
 *
 * @param {{feed?: unknown, kind?: unknown, rung?: unknown, from?: unknown}} s
 *   `from` is the rung the cells are read from (`derived_from` on a rung row,
 *   `read_rung` on a timeline source).
 */
export function tagWord(s) {
  const feed = String(s?.feed ?? '?');
  const rung = String(s?.rung ?? '?');
  const from = s?.from ? String(s.from) : '';
  const kind = String(s?.kind ?? '');
  if (kind === 'fold') return `${feed} ${from || '?'}→${rung} fold`;
  if (kind === 'derived') return `${feed} ${rung} derived`;
  if (kind === 'stored') return `${feed} ${rung} stored`;
  return `${feed} ${rung} ${kind || 'kind not sent'}`;
}

/** A paisa integer as rupees, exact; anything else is said not to be sent. @param {unknown} paisa */
export function priceText(paisa) {
  return typeof paisa === 'number' && Number.isSafeInteger(paisa) ? rupee(paisa) : '—';
}

/** A whole count, grouped; anything else a dash. @param {unknown} n */
export function countText(n) {
  return typeof n === 'number' && Number.isFinite(n) ? exact(n) : '—';
}

/** A signed number of basis points, two places, with a real minus sign. @param {number} bps */
function bpsText(bps) {
  const abs = Math.abs(bps).toFixed(2);
  if (Number(abs) === 0) return '0.00 bps';
  return `${bps < 0 ? '−' : '+'}${abs} bps`;
}

/**
 * A price basis as the badges a row carries (D-3323, D-3324, DB-G58).
 *
 * @param {any} basis the server's `basis` object
 * @returns {{price: string, volume: string, close: string, adjusted: boolean, source: string, unverified: string, tone: Tone}}
 */
export function basisOf(basis) {
  const b = basis && typeof basis === 'object' ? basis : {};
  const unverified = typeof b.unverified === 'string' ? b.unverified : '';
  return {
    price: typeof b.price_label === 'string' ? b.price_label : unverified ? 'UNVERIFIED' : 'not sent',
    volume: typeof b.volume_label === 'string' ? b.volume_label : unverified ? 'UNVERIFIED' : 'not sent',
    close: typeof b.close_label === 'string' ? b.close_label : 'not sent',
    adjusted: b.adjusted_as_of === 'pull_date',
    source: typeof b.source === 'string' ? b.source : '',
    unverified,
    tone: unverified ? 'warn' : b.adjusted_as_of === 'pull_date' ? 'info' : 'quiet'
  };
}

/**
 * One of the server's links into /db, as an address /db ITSELF shows rows on.
 *
 * THE SEGMENT IS /db's TO NAME, NOT THE STORE'S. The server's link carries the
 * store's segment (`seg=INDEX`, `seg=CASH`, `seg=FNO`), and /db's `seg` is its
 * own segment key (`spot`, `options`, `options-cut`, …: `SEGMENT_KEYS`). /db
 * takes `seg` as given and matches census rows against it, so every link this
 * page passed on unchanged opened an EMPTY grid ("No instrument-month matches
 * this query") although the rows are stored. With `key` in the link and no
 * `seg`, /db derives the segment from the key, today's day and the feed's
 * archive end — which is also the only way a contract that settled after the
 * archive ended lands on its own segment. So the link is read through /db's
 * codec and written back without the store's segment; a `seg` is kept only
 * when it is one of /db's own keys and no `key` is there to derive it from.
 *
 * `day` narrows the link to one day (both bounds). `oow` asks /db to show
 * months outside the operator's window, which it hides by default (D-3338):
 * without it a day out there would open an empty grid too.
 *
 * @param {unknown} link the server's `db`
 * @param {{day?: string, oow?: boolean}} [change]
 * @returns {string | null} `null` when the row carries no link into /db
 */
export function dbLinkOf(link, change = {}) {
  if (typeof link !== 'string' || !/^\/db(\?|$)/.test(link)) return null;
  const at = link.indexOf('?');
  const { sel } = decodeDb(at < 0 ? '' : link.slice(at + 1));
  const { seg, ...rest } = sel;
  /** @type {Record<string, any>} */
  const out = { ...rest };
  if (seg && !rest.key && SEGMENT_KEYS.includes(seg)) out.seg = seg;
  if (change.day && DAY.test(change.day)) {
    out.from = change.day;
    out.to = change.day;
  }
  if (change.oow) out.oow = true;
  return dbHref(out);
}

/**
 * The words for a day that lies outside a feed's window (the operator's,
 * D-3338), or `''` inside it or where the feed has no window. The window and
 * its label are the rung table's; nothing is spelled here but the fallback.
 *
 * @param {{feeds: Array<{feed: string, window: {from: string, to: string, label: string} | null}>} | null | undefined} table
 * @param {string} feed
 * @param {string} day `YYYY-MM-DD`
 */
export function outsideWindow(table, feed, day) {
  const w = table?.feeds.find((f) => f.feed === feed)?.window;
  if (!w || !DAY.test(String(day ?? ''))) return '';
  return day < w.from || day > w.to ? w.label : '';
}

// ══ The rung table (/sources.json) ═══════════════════════════════════════════

/** A rung's seconds, for ordering rows finest first; unknown spellings last. @param {unknown} rung */
export function rungSeconds(rung) {
  const m = /^(\d+)(s|min|day)$/.exec(String(rung ?? ''));
  if (!m) return Number.MAX_SAFE_INTEGER;
  return Number(m[1]) * (m[2] === 's' ? 1 : m[2] === 'min' ? 60 : 86_400);
}

/** @type {Record<string, {word: string, tone: Tone}>} */
const STATE_WORDS = Object.freeze({
  held: { word: 'held', tone: 'ok' },
  partial: { word: 'partial', tone: 'warn' },
  uncounted: { word: 'uncounted', tone: 'warn' },
  sessions_unchecked: { word: 'sessions unchecked', tone: 'warn' },
  unexpected: { word: 'unexpected', tone: 'bad' },
  on_request: { word: 'folded on request', tone: 'quiet' },
  absent: { word: 'absent', tone: 'bad' }
});

/**
 * A list of names as a sentence tail that says when it is not the whole list.
 *
 * @param {unknown} named
 * @param {unknown} count the whole count, when the server sent one
 * @param {number} [most] how many to print
 */
export function namedList(named, count, most = 6) {
  const names = Array.isArray(named) ? named.map(String) : [];
  const whole = typeof count === 'number' && Number.isFinite(count) ? count : names.length;
  if (names.length === 0) return { text: '', all: whole === 0, names };
  const shown = names.slice(0, most).join(', ');
  const rest = whole - Math.min(names.length, most);
  return {
    text: rest > 0 ? `${shown} and ${exact(rest)} more` : shown,
    all: names.length >= whole,
    names
  };
}

/**
 * One rung row as the page draws it.
 *
 * @param {any} row a `/sources.json` rung row
 * @param {any} feed the row's feed record from `feeds`, or `undefined`
 * @param {string} key the instrument key, for the in-page timeline link
 */
function rungRow(row, feed, key) {
  const kind = String(row?.kind ?? '');
  const stateCode = String(row?.state ?? '');
  const known = Object.hasOwn(STATE_WORDS, stateCode) ? STATE_WORDS[stateCode] : null;
  /** @type {string[]} */
  const why = [];
  const behind = row?.behind_source;
  if (behind && typeof behind === 'object') {
    const src = behind.source_rung ? `its ${behind.source_rung} source` : 'its source';
    if (Number(behind.sessions_missing) > 0) {
      const list = namedList(behind.sessions_missing_named, behind.sessions_missing);
      why.push(`lacks ${exact(behind.sessions_missing)} session${behind.sessions_missing === 1 ? '' : 's'} ${src} holds${behind.sessions_missing_exact === false ? ' (at least)' : ''}: ${list.text}`);
    }
    if (Number(behind.months_missing) > 0) {
      const list = namedList(behind.months_missing_named, behind.months_missing);
      why.push(`lacks ${exact(behind.months_missing)} month${behind.months_missing === 1 ? '' : 's'} ${src} holds: ${list.text}`);
    }
    if (Number(behind.days_behind) > 0) {
      why.push(`ends ${behind.last_day}, ${exact(behind.days_behind)} day${behind.days_behind === 1 ? '' : 's'} behind ${src} (${behind.source_last_day})`);
    }
  }
  const census = row?.census && typeof row.census === 'object' ? row.census : null;
  if (census && Number(census.months_uncounted) > 0) {
    why.push(`${exact(census.months_uncounted)} month file${census.months_uncounted === 1 ? '' : 's'} on disk the census has not counted${Array.isArray(census.uncounted) && census.uncounted.length ? `: ${namedList(census.uncounted, census.months_uncounted).text}` : ''}`);
  }
  if (stateCode === 'sessions_unchecked') why.push('the calendar gave no session list for this rung or its source, so it is not called whole');
  if (kind === 'fold') why.push(String(row?.fold ?? 'folded on request from the stored seconds; nothing is filed'));

  const sessions = row?.sessions && typeof row.sessions === 'object' ? row.sessions : {};
  const notHeld = typeof sessions.not_held === 'number' ? sessions.not_held : null;
  const holes = namedList(sessions.not_held_named, notHeld);
  const monthHoles = namedList(row?.holes, row?.months_absent);

  const out = row?.outside_window && typeof row.outside_window === 'object' ? row.outside_window : null;
  const windowFrom = feed?.window?.from ? String(feed.window.from) : '';
  const first = String(row?.first_day ?? '');
  const last = String(row?.last_day ?? '');
  return {
    feed: String(row?.feed ?? ''),
    feedName: String(feed?.display ?? row?.feed ?? ''),
    rung: String(row?.rung ?? ''),
    kind,
    tag: String(row?.tag ?? ''),
    tagWord: tagWord({ feed: row?.feed, kind, rung: row?.rung, from: row?.derived_from }),
    stored: row?.stored === true,
    first,
    last,
    /** The part of the span outside the operator's window, shaded (D-3338). */
    outside: out
      ? {
          from: String(out.first ?? ''),
          to: String(out.last ?? ''),
          months: typeof out.months === 'number' ? out.months : null,
          label: String(out.label ?? 'outside your window'),
          db: dbLinkOf(out.db)
        }
      : null,
    /** Where the in-window span starts: the window's first day when the rung starts before it. */
    insideFrom: out && windowFrom && first < windowFrom ? windowFrom : first,
    monthsHeld: typeof row?.months_held === 'number' ? row.months_held : null,
    monthsAbsent: typeof row?.months_absent === 'number' ? row.months_absent : null,
    monthHoles,
    sessions: {
      owed: typeof sessions.owed === 'number' ? sessions.owed : null,
      notHeld,
      holes,
      partly: typeof sessions.partly_held === 'number' ? sessions.partly_held : null,
      calendar: typeof sessions.calendar === 'string' ? sessions.calendar : null,
      why: typeof sessions.why === 'string' ? sessions.why : ''
    },
    state: known
      ? { code: stateCode, word: known.word, tone: known.tone, why }
      : { code: stateCode, word: stateCode ? stateCode.replaceAll('_', ' ') : 'state not sent', tone: /** @type {Tone} */ ('warn'), why },
    basis: basisOf(row?.basis),
    rows: census && typeof census.rows === 'number' ? census.rows : null,
    db: dbLinkOf(row?.db),
    timeline: sourcesHref({ key, rung: String(row?.rung ?? '') })
  };
}

/**
 * The newest day every feed that holds the instrument holds a stored bar of:
 * where the timeline and the day table open when the address names no day,
 * so the first thing on screen has every feed on it. `''` when no feed holds
 * a stored rung.
 *
 * @param {any} body a `/sources.json` answer
 */
export function commonLastDay(body) {
  /** @type {Map<string, string>} */
  const lastOf = new Map();
  for (const row of Array.isArray(body?.rungs) ? body.rungs : []) {
    if (row?.stored !== true || typeof row.last_day !== 'string' || !DAY.test(row.last_day)) continue;
    const feed = String(row.feed);
    const seen = lastOf.get(feed);
    if (seen === undefined || row.last_day > seen) lastOf.set(feed, row.last_day);
  }
  let day = '';
  for (const last of lastOf.values()) if (day === '' || last < day) day = last;
  return day;
}

/**
 * `/sources.json` as the page draws it: one card per feed and one row per
 * feed × rung, finest rung first within a feed, feeds in the answer's order.
 *
 * @param {any} body
 */
export function rungTable(body) {
  const feedsIn = Array.isArray(body?.feeds) ? body.feeds : [];
  const key = String(body?.instrument?.key ?? '');
  /** @type {Map<string, any>} */
  const feedOf = new Map(feedsIn.map((/** @type {any} */ f) => [String(f?.feed), f]));
  const order = new Map(feedsIn.map((/** @type {any} */ f, /** @type {number} */ i) => [String(f?.feed), i]));
  const rows = (Array.isArray(body?.rungs) ? body.rungs : [])
    .map((/** @type {any} */ row) => rungRow(row, feedOf.get(String(row?.feed)), key))
    .sort(
      (/** @type {any} */ a, /** @type {any} */ b) =>
        (order.get(a.feed) ?? 99) - (order.get(b.feed) ?? 99) ||
        Number(b.stored) - Number(a.stored) ||
        rungSeconds(a.rung) - rungSeconds(b.rung)
    );
  const feeds = feedsIn.map((/** @type {any} */ f) => {
    const census = f?.census && typeof f.census === 'object' ? f.census : {};
    const span = (/** @type {any} */ s) => (s && typeof s === 'object' && s.from && s.to ? { from: String(s.from), to: String(s.to), source: String(s.source ?? '') } : null);
    const window = span(f?.window);
    return {
      feed: String(f?.feed ?? ''),
      name: String(f?.display ?? f?.feed ?? ''),
      rungsHeld: typeof f?.rungs_held === 'number' ? f.rungs_held : null,
      archive: span(f?.archive),
      /** The operator's window on this feed, with the server's label for what lies outside it (D-3338). */
      window: window ? { ...window, label: String(f.window.label_outside ?? 'outside your window') } : null,
      census: {
        state: String(census.state ?? 'not sent'),
        tone: /** @type {Tone} */ (census.state === 'held' && !census.degraded ? 'ok' : 'warn'),
        note: String(census.note ?? ''),
        degraded: census.degraded ? String(typeof census.degraded === 'string' ? census.degraded : JSON.stringify(census.degraded)) : '',
        behindDays: typeof census.behind_days === 'number' ? census.behind_days : null,
        behindFrom: census.behind_from ? String(census.behind_from) : ''
      }
    };
  });
  // FOLDS OF ONE FEED ARE ONE LINE THAT NAMES EVERY RUNG. Nine rows that differ
  // only in the rung they fold to bury the three that are stored; a fold files
  // nothing, so its span, months and basis are its source's. Every rung stays
  // named, each with its own timeline link. Rows that differ in anything else
  // stay apart.
  /** @type {Array<ReturnType<typeof rungRow> & {rungs: Array<{rung: string, timeline: string}>}>} */
  const lines = [];
  /** @type {Map<string, number>} */
  const groupAt = new Map();
  for (const row of rows) {
    const one = { rung: row.rung, timeline: row.timeline };
    if (row.kind !== 'fold') {
      lines.push({ ...row, rungs: [one] });
      continue;
    }
    const from = row.tagWord.slice(row.feed.length + 1, row.tagWord.indexOf('→'));
    const id = JSON.stringify([row.feed, from, row.first, row.last, row.monthsHeld, row.state.code, row.state.why, row.basis.price, row.basis.volume, row.basis.close]);
    const at = groupAt.get(id);
    if (at === undefined) {
      groupAt.set(id, lines.length);
      lines.push({ ...row, rungs: [one] });
    } else {
      lines[at].rungs.push(one);
      lines[at].tagWord = `${row.feed} ${from}→ fold`;
    }
  }
  const held = feeds.reduce((/** @type {number} */ n, /** @type {any} */ f) => n + (f.rungsHeld ?? 0), 0);
  const tol = body?.tolerances && typeof body.tolerances === 'object' ? body.tolerances : null;
  return {
    key,
    feeds,
    rows,
    /** What the table draws: `rows`, with one feed's folds on one line. */
    lines,
    notes: Array.isArray(body?.notes) ? body.notes.map(String) : [],
    tolerances: tol,
    /** No feed holds a rung of the instrument: said in words, never an empty table. */
    empty: rows.length === 0 || held === 0 ? `No offered feed holds a rung of ${key || 'this instrument'} in this store.` : '',
    lastDay: commonLastDay(body)
  };
}

// ══ Comparisons, shared by the timeline and the day table ════════════════════

/**
 * One comparison the server made, as a cell. `a` and `b` are the two paisa
 * values compared, read only to say "equal" where the comparison carries no
 * `equal` of its own.
 *
 * @param {any} cmp `{bps, flag, equal?, paisa?, semantics?}` or `{why}` or `{percent, flag}` or `null`
 * @param {unknown} [a]
 * @param {unknown} [b]
 * @returns {{text: string, note: string, tone: Tone, title: string}} `note` is
 *   the label that goes under the number: what kind of difference it is.
 */
export function cmpCell(cmp, a, b) {
  if (cmp === null || cmp === undefined || typeof cmp !== 'object') return { text: '', note: '', tone: 'quiet', title: 'nothing to compare: one of the two holds no value here' };
  const official = cmp.semantics === 'official_vs_last_print';
  if (typeof cmp.why === 'string' && typeof cmp.percent !== 'number' && cmp.flag !== true && cmp.flag !== false) {
    const raw = typeof cmp.bps === 'number' ? ` The raw difference is ${bpsText(cmp.bps)}, which is the basis, not a disagreement.` : '';
    return {
      text: whyOf(cmp.why),
      note: typeof cmp.factor === 'number' ? `×${cmp.factor.toFixed(4)}` : '',
      tone: cmp.why === 'basis_unmeasured' || cmp.why === 'gdfl_first_print_after_open' ? 'warn' : 'info',
      title: `not judged: ${whyOf(cmp.why)}.${raw}`
    };
  }
  if (typeof cmp.percent === 'number') {
    const equal = cmp.equal === true || cmp.percent === 0;
    const text = equal && cmp.equal === true ? '=' : `${cmp.percent < 0 ? '−' : '+'}${Math.abs(cmp.percent).toFixed(2)}%`;
    return {
      text,
      note: '',
      tone: cmp.flag === true ? 'bad' : equal ? 'ok' : 'quiet',
      title: cmp.flag === true ? 'past the volume tolerance' : 'within the volume tolerance'
    };
  }
  if (typeof cmp.bps !== 'number') return { text: '', note: '', tone: 'quiet', title: 'the comparison carries no number' };
  const equal = typeof cmp.equal === 'boolean' ? cmp.equal : Number.isSafeInteger(a) && Number.isSafeInteger(b) ? a === b : cmp.bps === 0;
  const paisa = typeof cmp.paisa === 'number' ? cmp.paisa : Number.isSafeInteger(a) && Number.isSafeInteger(b) ? /** @type {number} */ (a) - /** @type {number} */ (b) : null;
  const by = paisa === null || paisa === 0 ? '' : ` (${paisa < 0 ? '−' : '+'}${rupee(Math.abs(paisa))})`;
  if (equal) {
    return { text: '=', note: '', tone: 'ok', title: official ? 'the official close and the last print are the same price' : 'the two values are the same paisa' };
  }
  if (official) {
    return {
      text: bpsText(cmp.bps),
      note: 'official vs last print',
      tone: 'info',
      title: `official close against last print${by}: labelled, never counted as a disagreement (D-3324)`
    };
  }
  if (cmp.flag === true) return { text: bpsText(cmp.bps), note: 'disagrees', tone: 'bad', title: `disagrees: past the price tolerance${by}` };
  return { text: bpsText(cmp.bps), note: '', tone: 'quiet', title: `differs within the price tolerance${by}` };
}

/**
 * A factor as a cell: `×2.0982`, and what pair it was read on.
 *
 * @param {unknown} factor
 * @param {unknown} of the server's words for the pair it was read on
 * @param {{differs?: boolean, step?: boolean}} [marks]
 */
export function factorCell(factor, of, marks = {}) {
  const how = typeof of === 'string' ? of : '';
  if (typeof factor !== 'number' || !Number.isFinite(factor)) {
    return { text: how ? 'not measured' : '', tone: /** @type {Tone} */ (how ? 'warn' : 'quiet'), title: how || 'no factor on this row' };
  }
  return {
    text: `×${factor.toFixed(4)}`,
    tone: /** @type {Tone} */ (marks.step ? 'warn' : marks.differs ? 'info' : 'quiet'),
    title: `${how || 'GDFL ÷ Zerodha'}${marks.step ? ' · stepped since the last day measured the same way' : ''}`
  };
}

// ══ The combined timeline (/sources/bars.json) ═══════════════════════════════

/**
 * Unix seconds as an IST wall-clock instant, `YYYY-MM-DD HH:MM:SS`, or `''`
 * outside the years a day can be written in. IST is UTC+05:30 with no
 * daylight saving, so this is integer arithmetic on the seconds.
 *
 * @param {number} seconds
 */
export function istOfUnix(seconds) {
  if (!Number.isSafeInteger(seconds)) return '';
  const ms = (seconds + 19_800) * 1000;
  if (ms < Date.UTC(0, 0, 1) || ms > Date.UTC(9999, 11, 31, 23, 59, 59)) return '';
  const at = new Date(ms);
  if (at.getUTCFullYear() < 1000) return '';
  return at.toISOString().slice(0, 19).replace('T', ' ');
}

/**
 * One feed's cell of one bucket as a source row.
 *
 * @param {any} cell the server's cell, or `null` where the feed holds no bar
 * @param {any} source the answer's `sources` entry for the feed
 * @param {string} rung
 * @param {string|null} db the feed's own stored rows of the bucket's day on /db;
 *   dropped where the feed holds no bar, so no link promises rows that are not there
 */
function timelineRow(cell, source, rung, db) {
  const feed = String(source?.feed ?? '');
  const tag = tagWord({ feed, kind: source?.kind, rung, from: source?.read_rung });
  if (!cell || typeof cell !== 'object') {
    return { feed, tag, db: null, absent: 'no bar in this bucket', close: '', o: '', h: '', l: '', c: '', v: '', oi: '', from: '', count: '' };
  }
  return {
    feed,
    tag,
    db,
    absent: '',
    close: cell.close === 'official' ? 'official close' : cell.close === 'last_print' ? 'last print' : String(cell.close ?? ''),
    o: priceText(cell.o),
    h: priceText(cell.h),
    l: priceText(cell.l),
    c: priceText(cell.c),
    v: countText(cell.v),
    oi: typeof cell.oi === 'number' ? countText(cell.oi) : cell.oi_why ? whyOf(cell.oi_why) : '—',
    from: typeof cell.first_at === 'string' ? cell.first_at : '',
    count: typeof cell.seconds === 'number' ? `${exact(cell.seconds)} s folded` : ''
  };
}

/**
 * `/sources/bars.json` as the page draws it: one group per bucket, one tagged
 * row per feed in it, and the server's comparison under them.
 *
 * @param {any} body
 * @param {ReturnType<typeof rungTable> | null} [rungs] the rung table, for each source's own /db link
 */
export function timeline(body, rungs = null) {
  const sources = Array.isArray(body?.sources) ? body.sources : [];
  const rung = String(body?.rung ?? '');
  const key = String(body?.instrument?.key ?? '');
  /** The stored rung each source reads, as the server's own /db link for it. */
  const links = sources.map(
    (/** @type {any} */ s) => rungs?.rows.find((/** @type {any} */ r) => r.stored && r.feed === String(s?.feed) && r.rung === String(s?.read_rung))?.db ?? null
  );
  /**
   * A source's own rows of one day on /db. A page is one IST day, so this is
   * one link built per source and remembered, not one per row; a day outside
   * the feed's window asks /db to show it (D-3338).
   *
   * @type {Map<string, string | null>}
   */
  const built = new Map();
  /** @param {number} i @param {string} feed @param {string} day */
  const linkOn = (i, feed, day) => {
    const id = `${i} ${day}`;
    let href = built.get(id);
    if (href === undefined) {
      href = dbLinkOf(links[i], { day, oow: outsideWindow(rungs, feed, day) !== '' });
      built.set(id, href);
    }
    return href;
  };
  const bases = new Set(sources.map((/** @type {any} */ s) => String(s?.basis?.price ?? '')));
  let anyFactor = bases.size > 1;
  const groups = (Array.isArray(body?.rows) ? body.rows : []).map((/** @type {any} */ row) => {
    const cells = row?.cells && typeof row.cells === 'object' ? row.cells : {};
    const cmp = row?.cmp && typeof row.cmp === 'object' ? row.cmp : null;
    const a = cmp?.base?.against ? cells[cmp.base.against] : null;
    const b = cmp?.base?.of ? cells[cmp.base.of] : null;
    if (cmp && (cmp.basis !== 'same' || typeof cmp.factor === 'number')) anyFactor = true;
    return {
      id: String(row?.t ?? row?.ist ?? ''),
      label: String(row?.ist ?? ''),
      rows: sources.map((/** @type {any} */ s, /** @type {number} */ i) =>
        timelineRow(cells[String(s?.feed)], s, rung, linkOn(i, String(s?.feed), String(row?.ist ?? '').slice(0, 10)))
      ),
      only: row?.only ? String(row.only) : '',
      cmp: cmp
        ? {
            label: `${cmp.base?.against ?? '?'} against ${cmp.base?.of ?? '?'}`,
            o: cmpCell(cmp.o, a?.o, b?.o),
            h: cmpCell(cmp.h, a?.h, b?.h),
            l: cmpCell(cmp.l, a?.l, b?.l),
            c: cmpCell(cmp.c, a?.c, b?.c),
            v: cmpCell(cmp.v, a?.v, b?.v),
            factor: factorCell(cmp.factor, cmp.factor_of, { differs: cmp.basis !== 'same' })
          }
        : null,
      flags: (Array.isArray(row?.flags) ? row.flags : []).map(flagOf)
    };
  });
  /** The cursor the server says it started from (unix seconds), or `null` on a first page. */
  const at = typeof body?.cursor === 'number' && Number.isFinite(body.cursor) ? body.cursor : null;
  const where = `${body?.range?.from ?? 'this range'}${body?.range?.to && body.range.to !== body.range.from ? ` to ${body.range.to}` : ''}`;
  const s = body?.summary && typeof body.summary === 'object' ? body.summary : null;
  const eq = s?.exactly_equal && typeof s.exactly_equal === 'object' ? s.exactly_equal : null;
  /** @type {string[]} */
  const summary = [];
  if (s) {
    summary.push(`${exact(s.rows)} bucket${s.rows === 1 ? '' : 's'} on this page: ${exact(s.both_feeds)} held by both feeds, ${exact(s.one_feed)} by one`);
    if (s.compared_on_one_basis > 0) {
      summary.push(`${exact(s.prices_within_tolerance)} of ${exact(s.compared_on_one_basis)} compared on one basis are within tolerance, ${exact(s.prices_disagree)} disagree`);
      if (eq) summary.push(`exactly equal: open ${exact(eq.o)}, high ${exact(eq.h)}, low ${exact(eq.l)}, close ${exact(eq.c)} of ${exact(s.compared_on_one_basis)}`);
    }
    if (s.basis_differs > 0) summary.push(`${exact(s.basis_differs)} on a different price basis: the factor is shown instead of a disagreement`);
    if (s.basis_unmeasured > 0) summary.push(`${exact(s.basis_unmeasured)} with no same-instant pair: basis not measured, nothing judged`);
  }
  return {
    key,
    rung,
    dir: body?.dir === 'asc' ? 'asc' : 'desc',
    range: { from: String(body?.range?.from ?? ''), to: String(body?.range?.to ?? '') },
    sources: sources.map((/** @type {any} */ src) => ({
      feed: String(src?.feed ?? ''),
      tag: tagWord({ feed: src?.feed, kind: src?.kind, rung, from: src?.read_rung }),
      basis: basisOf(src?.basis),
      exhausted: src?.exhausted === true
    })),
    groups,
    showFactor: anyFactor,
    /** Whether any cell on the page carries an open interest: the column is drawn only then. */
    showOi: (Array.isArray(body?.rows) ? body.rows : []).some((/** @type {any} */ row) =>
      Object.values(row?.cells && typeof row.cells === 'object' ? row.cells : {}).some((/** @type {any} */ cell) => typeof cell?.oi === 'number')
    ),
    summary,
    notes: Array.isArray(body?.notes) ? body.notes.map(String) : [],
    nextCursor: typeof body?.next_cursor === 'number' ? String(body.next_cursor) : '',
    complete: body?.complete === true,
    recordsRead: typeof body?.records_read === 'number' ? body.records_read : null,
    /** The page was reached with a cursor and holds nothing: the cursor is past the end, which says nothing about the day. */
    beyondCursor: groups.length === 0 && at !== null,
    empty:
      groups.length > 0
        ? ''
        : at !== null
          ? `Nothing lies ${body?.dir === 'asc' ? 'after' : 'before'} this address's cursor (${at}${istOfUnix(at) ? `, which is ${istOfUnix(at)} IST` : ''}) among the ${rung || 'bar'} buckets of ${key || 'this instrument'} in ${where}: the cursor is past the end of that range. That says nothing about what the sources hold there; the first page shows it.`
          : `No source holds a ${rung || 'bar'} bucket of ${key || 'this instrument'} in ${where}.`
  };
}

// ══ The day reconciliation (/sources/day.json) ═══════════════════════════════

/**
 * What a day's marks say, as lines: the calendar's kind when it is not an
 * ordinary session, the server's own sentences, and anything degraded.
 *
 * @param {any} marks a day's `marks` (`/dayflags.json`'s record)
 * @returns {Array<{tone: Tone, text: string}>}
 */
export function marksOf(marks) {
  if (!marks || typeof marks !== 'object') return [];
  /** @type {Array<{tone: Tone, text: string}>} */
  const lines = [];
  const cal = marks.calendar && typeof marks.calendar === 'object' ? marks.calendar : null;
  if (cal && cal.kind && cal.kind !== 'regular') {
    const name = [cal.holiday, cal.special].filter((x) => typeof x === 'string' && x).join(', ');
    lines.push({ tone: 'info', text: `${String(cal.kind).replaceAll('_', ' ')}${name ? `: ${name}` : ''}` });
  }
  if (cal && Number(cal.halted_minutes) > 0) lines.push({ tone: 'warn', text: `halted for ${exact(cal.halted_minutes)} minute${cal.halted_minutes === 1 ? '' : 's'}` });
  if (cal && Number(cal.unverified_minutes) > 0) lines.push({ tone: 'warn', text: `${exact(cal.unverified_minutes)} minute${cal.unverified_minutes === 1 ? '' : 's'} the calendar cannot verify` });
  for (const said of Array.isArray(marks.says) ? marks.says : []) lines.push({ tone: 'quiet', text: String(said) });
  for (const d of Array.isArray(marks.degraded) ? marks.degraded : []) lines.push({ tone: 'warn', text: `degraded: ${typeof d === 'string' ? d : JSON.stringify(d)}` });
  return lines;
}

/**
 * One source's fold of one day as a row, or the reason it holds none.
 *
 * @param {{feed: string, tag: string, close: string, db: string|null, outside: string}} src
 *   `outside` is the label of a day outside the feed's window, or `''`
 * @param {any} fold `{o,h,l,c,v,first_at?,last_at?,minutes?,seconds?}` or `null`
 * @param {unknown} absent the day's `absent` reason for this source
 */
function dayRow(src, fold, absent) {
  if (!fold || typeof fold !== 'object') {
    // A source that holds nothing of the day carries no /db link: it would open on no rows.
    return { ...src, db: null, outside: '', absent: absent ? whyOf(absent) : 'no record of this day', o: '', h: '', l: '', c: '', v: '', from: '', to: '', count: '' };
  }
  return {
    ...src,
    absent: '',
    o: priceText(fold.o),
    h: priceText(fold.h),
    l: priceText(fold.l),
    c: priceText(fold.c),
    v: countText(fold.v),
    from: typeof fold.first_at === 'string' ? fold.first_at : '',
    to: typeof fold.last_at === 'string' ? fold.last_at : '',
    count: typeof fold.minutes === 'number' ? `${exact(fold.minutes)} min` : typeof fold.seconds === 'number' ? `${exact(fold.seconds)} s` : ''
  };
}

/**
 * A day NO source holds is one row that names every source's reason, not
 * three rows of the same sentence: a weekend is one line, and a day two
 * sources lack for different reasons still names both.
 *
 * @param {Array<ReturnType<typeof dayRow>>} rows
 */
function foldRows(rows) {
  if (rows.length === 0 || rows.some((r) => !r.absent)) return rows;
  /** @type {Map<string, string[]>} */
  const byWhy = new Map();
  for (const r of rows) byWhy.set(r.absent, [...(byWhy.get(r.absent) ?? []), r.tag]);
  const absent = byWhy.size === 1 ? `no source holds this day: ${rows[0].absent}` : [...byWhy].map(([why, tags]) => `${tags.join(', ')}: ${why}`).join('; ');
  return [{ ...rows[0], feed: '', tag: 'every source', close: '', db: null, outside: '', absent }];
}

/**
 * `/sources/day.json` as the page draws it: per day, Zerodha's official day
 * bar, the fold of its minutes and the fold of GDFL's seconds as three tagged
 * rows, then the server's two comparisons (GDFL against the day bar; the
 * minutes against the day bar), the factor, the flags and the day's marks.
 *
 * @param {any} body
 * @param {ReturnType<typeof rungTable> | null} [rungs] the rung table, for each source's own /db link
 */
export function dayTable(body, rungs = null) {
  const src = body?.sources && typeof body.sources === 'object' ? body.sources : {};
  const key = String(body?.instrument?.key ?? '');
  /** @param {any} s @param {boolean} folded */
  const describe = (s, folded) => {
    const feed = String(s?.feed ?? '');
    const rung = String(s?.rung ?? '');
    const link = rungs?.rows.find((/** @type {any} */ r) => r.feed === feed && r.rung === rung && r.stored)?.db ?? null;
    return {
      feed,
      tag: s ? `${feed} ${rung}${folded ? '→1day fold' : ' stored'}` : 'source not sent',
      close: basisOf(s?.basis).close,
      link
    };
  };
  /**
   * One source on one day: its tag, its own stored rows of that day on /db,
   * and the label of a day outside its feed's window (D-3338, the operator's
   * answer 6). /db hides such a month by default, so the link asks for it.
   *
   * @param {ReturnType<typeof describe>} one @param {string} day
   */
  const sourceOn = (one, day) => {
    const outside = outsideWindow(rungs, one.feed, day);
    return { feed: one.feed, tag: one.tag, close: one.close, db: dbLinkOf(one.link, { day, oow: outside !== '' }), outside };
  };
  const zd = describe(src.zerodha_day, false);
  const zm = describe(src.zerodha_minutes, true);
  const gs = describe(src.gdfl_seconds, true);
  let anyFactor = false;
  const days = (Array.isArray(body?.days) ? body.days : []).map((/** @type {any} */ d) => {
    const day = String(d?.day ?? '');
    const absent = d?.absent && typeof d.absent === 'object' ? d.absent : {};
    const basis = d?.basis && typeof d.basis === 'object' ? d.basis : null;
    if (basis) anyFactor = true;
    const open = d?.open ?? {};
    const high = d?.high ?? {};
    const low = d?.low ?? {};
    const close = d?.close ?? {};
    const vol = d?.volume ?? {};
    const cal = d?.marks?.calendar;
    /** @param {boolean|undefined} outside @param {string} word */
    const extreme = (outside, word) =>
      outside === true
        ? { text: word, note: 'the day range', tone: /** @type {Tone} */ ('warn'), title: `Zerodha's minutes go ${word} its own day bar: the day bar is not a fold of its minutes (DB-G41)` }
        : outside === false
          ? { text: 'inside', note: '', tone: /** @type {Tone} */ ('ok'), title: "Zerodha's minute extreme is inside its day bar's range" }
          : { text: '', note: '', tone: /** @type {Tone} */ ('quiet'), title: 'not compared' };
    const ratio = typeof vol.minute_over_day === 'number' ? vol.minute_over_day : null;
    const minuteVol = vol.minute_vs_day ? cmpCell(vol.minute_vs_day) : { text: '', note: '', tone: /** @type {Tone} */ ('quiet'), title: 'not compared' };
    return {
      id: day,
      label: day,
      sub: typeof cal?.weekday === 'string' ? cal.weekday : '',
      rows: foldRows([
        dayRow(sourceOn(zd, day), d?.zerodha_day, absent.zerodha_day),
        dayRow(sourceOn(zm, day), d?.zerodha_minutes_fold, absent.zerodha_minutes),
        dayRow(sourceOn(gs, day), d?.gdfl_seconds_fold, absent.gdfl_seconds)
      ]),
      cmps: [
        {
          label: `${gs.tag} against ${zd.tag}`,
          o: cmpCell(open.gdfl_vs_zerodha_day, open.gdfl_first_print, open.zerodha_day),
          h: cmpCell(high.gdfl_vs_zerodha_day, high.gdfl_seconds, high.zerodha_day),
          l: cmpCell(low.gdfl_vs_zerodha_day, low.gdfl_seconds, low.zerodha_day),
          c: cmpCell(close.official_vs_gdfl_last_print, close.gdfl_last_print, close.official),
          v: cmpCell(vol.gdfl_vs_zerodha_day ?? (typeof vol.why === 'string' && d?.gdfl_seconds_fold && d?.zerodha_day ? { why: vol.why } : null), vol.gdfl_sum, vol.zerodha_day)
        },
        {
          label: `${zm.tag} against ${zd.tag}`,
          o: cmpCell(open.zerodha_minute_vs_day, open.zerodha_first_minute, open.zerodha_day),
          h: extreme(high.minute_above_day, 'above'),
          l: extreme(low.minute_below_day, 'below'),
          c: cmpCell(close.official_vs_zerodha_last_minute, close.zerodha_last_minute, close.official),
          v: ratio === null ? minuteVol : { ...minuteVol, text: `÷ day ${ratio.toFixed(4)}`, note: minuteVol.text }
        }
      ].filter((c) => [c.o, c.h, c.l, c.c, c.v].some((cell) => cell.text !== '')),
      factor: basis
        ? {
            ...factorCell(basis.factor, basis.factor_of, { differs: basis.basis_differs === true, step: basis.basis_step === true }),
            volume: typeof basis.volume_factor === 'number' ? `volume ×${basis.volume_factor.toFixed(4)}${basis.volume_not_scaled_with_price ? ' · not scaled with price' : ''}` : '',
            minute: typeof basis.minute_factor === 'number' ? `minutes ×${basis.minute_factor.toFixed(4)}${basis.minute_factor_at ? ` at ${basis.minute_factor_at}` : ''}` : ''
          }
        : null,
      flags: (Array.isArray(d?.flags) ? d.flags : []).map(flagOf),
      marks: marksOf(d?.marks),
      auction: d?.auction && typeof d.auction === 'object' && typeof d.auction.says === 'string' ? String(d.auction.says) : '',
      closeEqual: close.equal === true,
      /** The three closes at a glance, for the "closes equal at" line. */
      closes: { official: priceText(close.official), minute: priceText(close.zerodha_last_minute), gdfl: priceText(close.gdfl_last_print) },
      timeline: sourcesHref({ key, rung: SOURCES_DEFAULTS.rung, day })
    };
  });
  const s = body?.summary && typeof body.summary === 'object' ? body.summary : null;
  /** @type {string[]} */
  const summary = [];
  if (s) {
    const both = s.both_feeds;
    summary.push(`${exact(s.days)} day${s.days === 1 ? '' : 's'} on this page: ${exact(both)} held by both feeds, ${exact(s.zerodha_only)} by Zerodha only, ${exact(s.gdfl_only)} by GDFL only, ${exact(s.neither)} by neither`);
    if (both > 0 && s.basis_differs < both) {
      summary.push(`open equal on ${exact(s.open_equal)} of ${exact(both)}, high on ${exact(s.high_equal)}, low on ${exact(s.low_equal)}`);
    }
    if (both > 0 && (s.close_equal > 0 || s.close_differs > 0)) {
      summary.push(`close equal on ${exact(s.close_equal)} of ${exact(both)} and different on ${exact(s.close_differs)} — ${String(s.close_compared_as ?? 'official against last print')}`);
    }
    if (s.basis_differs > 0) summary.push(`${exact(s.basis_differs)} day${s.basis_differs === 1 ? '' : 's'} on a different price basis: the factor is shown instead of a disagreement${s.basis_steps > 0 ? `; the factor stepped on ${exact(s.basis_steps)}` : ''}`);
    if (s.minute_extremes_outside_day > 0) summary.push(`Zerodha's minutes go outside its own day range on ${exact(s.minute_extremes_outside_day)} day${s.minute_extremes_outside_day === 1 ? '' : 's'}`);
    if (s.minute_volume_off_day > 0) summary.push(`Zerodha's minute volume is off its day volume on ${exact(s.minute_volume_off_day)} day${s.minute_volume_off_day === 1 ? '' : 's'}`);
    if (s.gdfl_first_print_after_open > 0) summary.push(`GDFL starts after the opening minute on ${exact(s.gdfl_first_print_after_open)} day${s.gdfl_first_print_after_open === 1 ? '' : 's'}`);
    if (s.volume_compared > 0) summary.push(`volume within tolerance on ${exact(s.volume_within_tolerance)} of ${exact(s.volume_compared)} compared`);
  }
  return {
    key,
    days,
    showFactor: anyFactor,
    summary,
    notes: Array.isArray(body?.notes) ? body.notes.map(String) : [],
    page: typeof body?.page === 'number' ? body.page : 0,
    pages: typeof body?.pages === 'number' ? body.pages : 1,
    from: String(body?.page_from ?? body?.asked?.from ?? ''),
    to: String(body?.page_to ?? body?.asked?.to ?? ''),
    recordsRead: typeof body?.records_read === 'number' ? body.records_read : null,
    fromCache: typeof s?.days_from_cache === 'number' ? s.days_from_cache : null,
    empty: days.length === 0 ? `The day table holds no day of ${key || 'this instrument'} in ${body?.asked?.from ?? 'this range'}.` : ''
  };
}

/**
 * The tolerances the server named, as one line (D-3339: chosen, not sourced).
 *
 * @param {any} tol the answer's `tolerances`
 */
export function toleranceLine(tol) {
  if (!tol || typeof tol !== 'object') return '';
  const part = (/** @type {unknown} */ v, /** @type {string} */ unit) => (typeof v === 'number' ? `${v}${unit}` : 'not sent');
  return `Tolerances (chosen, not sourced; D-3339): a price disagrees past ${part(tol.price_bps, ' bp')}, a volume past ${part(tol.volume_percent, '%')}, a share's factor differs past ${part(tol.factor_percent, '%')}.`;
}
