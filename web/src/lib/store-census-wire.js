import { yieldToPage } from './database-preparation.js';

export const CENSUS_ENCODING = 'census-tuples-v1';

/**
 * The sentence every refusal below throws. One spelling, so a caller (and a
 * test) can tell a malformed census from any other failure.
 */
const invalid = () => new Error('The body of /store.json is not a JSON array or a valid complete census-tuples-v1 inventory');

/**
 * THE INVENTORY'S HEADER, CHECKED ONCE. Throws the refusal above for anything
 * that is not a complete `census-tuples-v1` body for `feed`.
 * @param {any} value
 * @param {string} feed
 */
function checkHeader(value, feed) {
  if (!value || typeof value !== 'object' || value.schema !== 1 || value.encoding !== CENSUS_ENCODING || value.feed !== feed || !Array.isArray(value.rows)) throw invalid();
  for (const key of ['instruments', 'months', 'timeframes', 'reasons']) {
    if (!Array.isArray(value[key]) || !value[key].every((/** @type {unknown} */ token) => typeof token === 'string' && token.length > 0)) throw invalid();
  }
}

/** @param {unknown} index @param {string[]} dictionary @returns {number} */
const tokenIndex = (index, dictionary) => {
  if (!Number.isSafeInteger(index) || /** @type {number} */ (index) < 0 || /** @type {number} */ (index) >= dictionary.length) throw invalid();
  return /** @type {number} */ (index);
};

/**
 * A change and its reason: exactly one of the two is present. Returns the
 * reason's dictionary index, or -1 when the change is a number.
 * @param {unknown} bps @param {unknown} index @param {string[]} reasons
 */
const reasonIndex = (bps, index, reasons) => {
  if (bps !== null) {
    if (!Number.isSafeInteger(bps) || index !== null) throw invalid();
    return -1;
  }
  return tokenIndex(index, reasons);
};

/** One wire tuple, checked; throws the refusal for anything malformed. @param {unknown} row */
function checkTuple(row) {
  if (!Array.isArray(row) || row.length !== 10 || !Number.isSafeInteger(row[3]) || row[3] < 0 ||
      !Number.isSafeInteger(row[4]) || !Number.isSafeInteger(row[5])) throw invalid();
  return row;
}

/** Decode the server's lossless compact inventory. All stored months remain
 * present; dictionaries remove repeated labels and per-field JSON names.
 * A first decode is O(rows), in cancellable bounded slices. Old APIs which
 * ignore the encoding query may answer their original array unchanged.
 * @param {unknown} body
 * @param {string} feed
 * @param {AbortSignal} signal
 * @param {()=>Promise<unknown>} [yieldControl]
 * @returns {Promise<any[]>}
 */
export async function decodeCensus(body, feed, signal, yieldControl = yieldToPage) {
  signal.throwIfAborted();
  if (Array.isArray(body)) return body;
  const value = /** @type {any} */ (body);
  checkHeader(value, feed);
  /** @type {any[]} */
  const rows = new Array(value.rows.length);
  for (let start = 0; start < value.rows.length; start += 2048) {
    signal.throwIfAborted();
    const end = Math.min(start + 2048, value.rows.length);
    for (let index = start; index < end; index++) {
      const row = checkTuple(value.rows[index]);
      const why = reasonIndex(row[6], row[7], value.reasons);
      const prevWhy = reasonIndex(row[8], row[9], value.reasons);
      rows[index] = { feed, instrument: value.instruments[tokenIndex(row[0], value.instruments)],
        month: value.months[tokenIndex(row[1], value.months)],
        timeframe: value.timeframes[tokenIndex(row[2], value.timeframes)], rows: row[3], first_ts: row[4], last_ts: row[5],
        chg_bps: row[6], chg_why: why < 0 ? null : value.reasons[why],
        prev_chg_bps: row[8], prev_chg_why: prevWhy < 0 ? null : value.reasons[prevWhy] };
    }
    await yieldControl();
  }
  signal.throwIfAborted();
  return rows;
}

/**
 * THE SAME INVENTORY AS COLUMNS — what the census worker decodes into and
 * hands the page (WP-56, D-3327; DB-G84).
 *
 * `decodeCensus` builds one object per row, which is what every page reads;
 * built in a worker and posted as objects, 385,442 rows cost the page about
 * 1.2 s to deserialise (measured with `structuredClone`, 10 Oct 2026). As
 * columns the numbers are typed arrays the page receives without a copy, and
 * the labels stay the four dictionaries the wire already sends. The page then
 * builds its row objects in bounded slices (`rowsOfColumns`).
 *
 * Lossless and checked exactly as `decodeCensus` checks: the same refusal for
 * the same malformed body, so the two paths cannot disagree about which
 * census is valid. A change is a safe integer or `null`; `null` is stored as
 * NaN, which no safe integer is, and its reason as a dictionary index, -1
 * where the change is a number.
 *
 * Returns `null` for a legacy array body (an API that ignores `encoding`):
 * that body is already the rows.
 *
 * @param {unknown} body
 * @param {string} feed
 * @returns {CensusColumns | null}
 */
export function decodeCensusColumns(body, feed) {
  if (Array.isArray(body)) return null;
  const value = /** @type {any} */ (body);
  checkHeader(value, feed);
  const n = value.rows.length;
  const cols = {
    feed, n,
    instruments: value.instruments, months: value.months, timeframes: value.timeframes, reasons: value.reasons,
    inst: new Int32Array(n), month: new Int32Array(n), tf: new Int32Array(n),
    rows: new Float64Array(n), first: new Float64Array(n), last: new Float64Array(n),
    chg: new Float64Array(n), chgWhy: new Int32Array(n), prev: new Float64Array(n), prevWhy: new Int32Array(n)
  };
  for (let index = 0; index < n; index++) {
    const row = checkTuple(value.rows[index]);
    cols.inst[index] = tokenIndex(row[0], value.instruments);
    cols.month[index] = tokenIndex(row[1], value.months);
    cols.tf[index] = tokenIndex(row[2], value.timeframes);
    cols.rows[index] = row[3];
    cols.first[index] = row[4];
    cols.last[index] = row[5];
    cols.chgWhy[index] = reasonIndex(row[6], row[7], value.reasons);
    cols.chg[index] = row[6] === null ? NaN : row[6];
    cols.prevWhy[index] = reasonIndex(row[8], row[9], value.reasons);
    cols.prev[index] = row[8] === null ? NaN : row[8];
  }
  return cols;
}

/**
 * @typedef {{feed: string, n: number, instruments: string[], months: string[],
 *   timeframes: string[], reasons: string[], inst: Int32Array, month: Int32Array,
 *   tf: Int32Array, rows: Float64Array, first: Float64Array, last: Float64Array,
 *   chg: Float64Array, chgWhy: Int32Array, prev: Float64Array, prevWhy: Int32Array}} CensusColumns
 */

/**
 * The typed arrays of a columnar census, for a transfer list. Each column is
 * made by `decodeCensusColumns` over its own `ArrayBuffer`, never a shared one.
 * @param {CensusColumns} cols
 * @returns {ArrayBuffer[]}
 */
export function columnBuffers(cols) {
  return [cols.inst, cols.month, cols.tf, cols.rows, cols.first, cols.last, cols.chg, cols.chgWhy, cols.prev, cols.prevWhy]
    .map((array) => /** @type {ArrayBuffer} */ (array.buffer));
}

/**
 * Rows `start..end` of a columnar census as the objects `decodeCensus` builds,
 * written into `out` at the same indexes. Field for field the same values,
 * including `null` for a change the wire sent as `null`.
 * @param {CensusColumns} cols
 * @param {number} start
 * @param {number} end
 * @param {any[]} out
 */
export function rowsOfColumns(cols, start, end, out) {
  const { feed, instruments, months, timeframes, reasons } = cols;
  for (let i = start; i < end; i++) {
    const why = cols.chgWhy[i], prevWhy = cols.prevWhy[i];
    out[i] = { feed, instrument: instruments[cols.inst[i]], month: months[cols.month[i]],
      timeframe: timeframes[cols.tf[i]], rows: cols.rows[i], first_ts: cols.first[i], last_ts: cols.last[i],
      chg_bps: why < 0 ? cols.chg[i] : null, chg_why: why < 0 ? null : reasons[why],
      prev_chg_bps: prevWhy < 0 ? cols.prev[i] : null, prev_chg_why: prevWhy < 0 ? null : reasons[prevWhy] };
  }
  return out;
}

/* ======================================================================
   THE CENSUS'S ORDER, AND A DELTA MERGED INTO IT (WP-56, D-3327 item 3;
   DB-022, DB-060)
   ----------------------------------------------------------------------
   `/store.json` lists its rows newest month first and, inside a month, in
   the series' own order (`crates/api/src/census.rs`, `held_entries` and
   `Series`): a spot series before every contract; contracts by their own
   text (`2019-03-28-1150000-CE`); then exchange (NSE, BSE), segment (INDEX,
   CASH, FNO), symbol and the rung's seconds. A refresh is answered with
   only the rows changed since the generation this page holds
   (`/store.json?...&since=`), and they are merged INTO THAT ORDER, so the
   census a page holds after a refresh is, row for row and position for
   position, the census a whole read at the new generation answers
   (`store-census-wire.test.js` holds the two equal).

   THE ORDER IS THE SERVER'S RULE, WRITTEN HERE A SECOND TIME, and a second
   copy of a rule drifts. So it is never trusted unchecked: every whole
   census is checked against it as it is decoded (`columnsOrderFault`,
   `rowsOrderFault`), row after row, and a census the rule does not describe
   is never patched: its refresh reads it whole and the page says why.
   `store_route.rs` and `store-census-wire.test.js` pin both sides to one
   example list.
   ====================================================================== */
const RUNG = /^([1-9]\d*)(s|min|day)$/;
const RUNG_UNIT = { s: 1, min: 60, day: 86_400 };

/**
 * Seconds one bar of a rung covers, read off its own spelling (`1s`,
 * `15min`, `1day`), or NaN for a spelling that is none of those. Used to
 * ORDER rows only; a NaN is a census this page cannot order, said as such.
 * @param {string} timeframe
 */
export function rungOrder(timeframe) {
  const m = RUNG.exec(timeframe);
  return m ? Number(m[1]) * RUNG_UNIT[/** @type {'s' | 'min' | 'day'} */ (m[2])] : NaN;
}

const EXCHANGE_ORDER = new Map([['NSE', 0], ['BSE', 1]]);
const SEGMENT_ORDER = new Map([['INDEX', 0], ['CASH', 1], ['FNO', 2]]);
const NAME = /^([A-Z]+)-([A-Z]+)-(.+)$/;
/** `<symbol>-<YYYY-MM-DD>-FUT` or `<symbol>-<YYYY-MM-DD>-<strike paisa>-CE|PE`. */
const CONTRACT = /^(.+)-(\d{4}-\d{2}-\d{2}-(?:FUT|\d+-(?:CE|PE)))$/;

/**
 * @typedef {{contract: string | null, exchange: number, segment: number, symbol: string}} SeriesOrder
 */

/**
 * An instrument name as the parts the server orders a series by, or `null`
 * for a name this page cannot take apart (an exchange or a segment it does
 * not know, an F&O name that ends in no contract).
 * @param {string} instrument `NSE-INDEX-NIFTY`, `NSE-FNO-NIFTY-2019-03-28-1150000-CE`
 * @returns {SeriesOrder | null}
 */
export function seriesOrder(instrument) {
  const m = NAME.exec(instrument);
  if (!m) return null;
  const exchange = EXCHANGE_ORDER.get(m[1]), segment = SEGMENT_ORDER.get(m[2]);
  if (exchange === undefined || segment === undefined) return null;
  if (segment !== 2) return { contract: null, exchange, segment, symbol: m[3] };
  const c = CONTRACT.exec(m[3]);
  return c ? { contract: c[2], exchange, segment, symbol: c[1] } : null;
}

/** @param {SeriesOrder} a @param {SeriesOrder} b */
function compareSeries(a, b) {
  if (a.contract !== b.contract) {
    if (a.contract === null) return -1;
    if (b.contract === null) return 1;
    return a.contract < b.contract ? -1 : 1;
  }
  if (a.exchange !== b.exchange) return a.exchange - b.exchange;
  if (a.segment !== b.segment) return a.segment - b.segment;
  if (a.symbol !== b.symbol) return a.symbol < b.symbol ? -1 : 1;
  return 0;
}

/**
 * A comparator over census rows in the server's order: negative when `a` is
 * listed before `b`, zero when they are the same key. Parses each instrument
 * name once (`memo`). Throws for a row it cannot order; callers that merge
 * have checked the census first, so that is a census that changed shape.
 * @param {Map<string, SeriesOrder | null>} [memo]
 * @returns {(a: {instrument: string, month: string, timeframe: string}, b: {instrument: string, month: string, timeframe: string}) => number}
 */
export function censusOrder(memo = new Map()) {
  /** @param {string} instrument */
  const parts = (instrument) => {
    let held = memo.get(instrument);
    if (held === undefined) memo.set(instrument, (held = seriesOrder(instrument)));
    if (held === null) throw new Error(`${JSON.stringify(instrument)} is not a name this page can place in the census's order`);
    return held;
  };
  return (a, b) => {
    if (a.month !== b.month) return a.month > b.month ? -1 : 1;
    if (a.instrument !== b.instrument) {
      const by = compareSeries(parts(a.instrument), parts(b.instrument));
      if (by !== 0) return by;
    }
    if (a.timeframe === b.timeframe) return 0;
    const ra = rungOrder(a.timeframe), rb = rungOrder(b.timeframe);
    if (Number.isNaN(ra) || Number.isNaN(rb)) throw new Error(`${JSON.stringify(Number.isNaN(ra) ? a.timeframe : b.timeframe)} is not a rung this page can place in the census's order`);
    return ra - rb;
  };
}

/** @param {{instrument: string, month: string, timeframe: string}} row */
const rowName = (row) => `${row.month} ${row.instrument} ${row.timeframe}`;

/**
 * WHETHER A COLUMNAR CENSUS IS IN THE ORDER THIS PAGE KNOWS: `null` when
 * every row follows the one before it, else the sentence naming the first
 * that does not (or the first name or rung that cannot be ordered at all).
 * One pass, in the census worker; each instrument name is parsed once.
 * @param {CensusColumns} cols
 * @returns {string | null}
 */
export function columnsOrderFault(cols) {
  const { instruments, months, timeframes } = cols;
  /** @type {(SeriesOrder | null)[]} */
  const series = new Array(instruments.length);
  for (let i = 0; i < instruments.length; i++) {
    series[i] = seriesOrder(instruments[i]);
    if (series[i] === null) return `${JSON.stringify(instruments[i])} is not a name this page can place in the census's order`;
  }
  const rungs = timeframes.map(rungOrder);
  const bad = rungs.findIndex((seconds) => Number.isNaN(seconds));
  if (bad >= 0) return `${JSON.stringify(timeframes[bad])} is not a rung this page can place in the census's order`;
  for (let i = 1; i < cols.n; i++) {
    const ma = cols.month[i - 1], mb = cols.month[i];
    let by = 0;
    if (ma !== mb) by = months[ma] > months[mb] ? -1 : months[ma] < months[mb] ? 1 : 0;
    if (by === 0 && cols.inst[i - 1] !== cols.inst[i]) {
      by = compareSeries(/** @type {SeriesOrder} */ (series[cols.inst[i - 1]]), /** @type {SeriesOrder} */ (series[cols.inst[i]]));
    }
    if (by === 0) by = rungs[cols.tf[i - 1]] - rungs[cols.tf[i]];
    if (by >= 0) {
      const name = (/** @type {number} */ at) => `${months[cols.month[at]]} ${instruments[cols.inst[at]]} ${timeframes[cols.tf[at]]}`;
      return `row ${i + 1} (${name(i)}) does not follow row ${i} (${name(i - 1)}) in the order this page places a refreshed row by`;
    }
  }
  return null;
}

/**
 * `columnsOrderFault` over row objects (a census this page decoded itself),
 * and the bars they hold, in slices.
 * @param {any[]} rows
 * @param {(n: number, work: (start: number, end: number) => void, signal?: AbortSignal) => Promise<void>} slices
 * @param {AbortSignal} [signal]
 * @returns {Promise<{order: string | null, bars: number}>}
 */
export async function measureCensus(rows, slices, signal) {
  const before = censusOrder();
  /** @type {string | null} */
  let order = null;
  let bars = 0;
  await slices(rows.length, (start, end) => {
    for (let i = start; i < end; i++) {
      const row = rows[i];
      bars += Number(row?.rows) || 0;
      if (order !== null || i === 0) continue;
      try {
        if (before(rows[i - 1], row) >= 0) {
          order = `row ${i + 1} (${rowName(row)}) does not follow row ${i} (${rowName(rows[i - 1])}) in the order this page places a refreshed row by`;
        }
      } catch (why) {
        order = String(/** @type {any} */ (why)?.message ?? why);
      }
    }
  }, signal);
  return { order, bars };
}

/**
 * WHAT A PAGE KNOWS ABOUT A CENSUS ARRAY IT HOLDS, beside the array: whether
 * its rows are in the order above (`order` is `null`, or the sentence naming
 * what is not) and the bars they sum to. Kept against the array itself, so a
 * census decoded in the worker carries its own measure and one merged from a
 * delta the measure of its merge; an array nobody measured has none.
 * @type {WeakMap<object, {order: string | null, bars: number}>}
 */
const measures = new WeakMap();
/** @param {any[]} rows @param {{order: string | null, bars: number}} measure */
export const noteCensusMeasure = (rows, measure) => { measures.set(rows, measure); };
/** @param {any[]} rows */
export const censusMeasureOf = (rows) => measures.get(rows) ?? null;

/**
 * A DELTA MERGED INTO THE CENSUS THE PAGE HOLDS.
 *
 * `held` is a census in the order above (checked); `delta` the rows the
 * server answered a `since` with, in the same order (checked here, and a
 * delta that is not is refused). A delta row whose key is held REPLACES that
 * row in place; one whose key is not held is INSERTED where the order puts
 * it. `held` is not changed: the answer is a new array, in which every row
 * the delta did not name is the same object it was.
 *
 * Each delta row is placed by a binary search that starts where the row
 * before it landed, so the cost is O(delta × log census) comparisons and one
 * copy of the array, in slices.
 *
 * @param {any[]} held
 * @param {any[]} delta
 * @param {(n: number, work: (start: number, end: number) => void, signal?: AbortSignal) => Promise<void>} slices
 * @param {AbortSignal} [signal]
 * @returns {Promise<{rows: any[], added: number, replaced: number, bars: number}>} `bars` is the CHANGE in bars held
 */
export async function mergeCensus(held, delta, slices, signal) {
  const before = censusOrder();
  const n = held.length, k = delta.length;
  /** Where each delta row goes in `held`, and whether it replaces the row there. */
  const at = new Int32Array(k);
  const exact = new Uint8Array(k);
  let lo = 0, added = 0, replaced = 0, bars = 0;
  for (let j = 0; j < k; j++) {
    const row = delta[j];
    if (j > 0 && before(delta[j - 1], row) >= 0) {
      throw new Error(`the delta's row ${j + 1} (${rowName(row)}) does not follow its row ${j} (${rowName(delta[j - 1])})`);
    }
    let low = lo, high = n;
    while (low < high) {
      const mid = (low + high) >>> 1;
      if (before(held[mid], row) < 0) low = mid + 1;
      else high = mid;
    }
    at[j] = low;
    bars += row.rows;
    if (low < n && before(held[low], row) === 0) {
      exact[j] = 1;
      replaced += 1;
      bars -= held[low].rows;
      lo = low + 1;
    } else {
      added += 1;
      lo = low;
    }
  }
  /** @type {any[]} */
  const rows = new Array(n + added);
  let out = 0, j = 0;
  await slices(n, (start, end) => {
    for (let p = start; p < end; p++) {
      while (j < k && at[j] === p && exact[j] === 0) rows[out++] = delta[j++];
      if (j < k && at[j] === p) rows[out++] = delta[j++];
      else rows[out++] = held[p];
    }
  }, signal);
  while (j < k) rows[out++] = delta[j++];
  return { rows, added, replaced, bars };
}
