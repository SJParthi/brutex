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

const NAME = /^([A-Z]+)-([A-Z]+)-(.+)$/;
/** `<symbol>-<YYYY-MM-DD>-FUT` or `<symbol>-<YYYY-MM-DD>-<strike paisa>-CE|PE`. */
const CONTRACT = /^(.+)-(\d{4}-\d{2}-\d{2}-(?:FUT|\d+-(?:CE|PE)))$/;

/* ----------------------------------------------------------------------
   THE ORDER OF EXCHANGES AND SEGMENTS IS LEARNED FROM THE CENSUS (WP-56b;
   the WP-56 second agent's follow-up 6)
   ----------------------------------------------------------------------
   This file used to carry `NSE, BSE` and `INDEX, CASH, FNO` as lists of its
   own. A census holding an exchange or a segment not in them could not be
   ordered here, so every refresh of it read the whole census until the
   lists were edited. The census the server sends IS in the server's order,
   so the order is read off it: wherever two neighbouring rows of one month
   and one contract differ by exchange, the first row's exchange comes
   first; where they share the exchange and differ by segment, the first
   row's segment comes first. Which segments name contracts is read off the
   names too (a segment in which any name ends in a contract; a name of such
   a segment that ends in none cannot be placed). A census whose neighbours
   contradict each other is in no single order, and is said so.

   A name the order was never learned for (a delta's row on an exchange or
   a segment the census held has none of) cannot be placed: that refresh
   reads the census whole, says why, and the whole census teaches the order.
   ---------------------------------------------------------------------- */

/**
 * The census's order of exchanges and segments, and the segments whose
 * names end in a contract.
 * @typedef {{exchanges: string[], segments: string[], contracts: string[]}} CensusRanks
 */

/**
 * @typedef {{contract: string | null, exchange: number, segment: number, symbol: string}} SeriesOrder
 */

/**
 * An instrument name as the parts the server orders a series by, in the
 * order `ranks` learned, or `null` for a name it cannot place (an exchange
 * or a segment `ranks` does not hold, a contract segment's name that ends in
 * no contract).
 * @param {string} instrument `NSE-INDEX-NIFTY`, `NSE-FNO-NIFTY-2019-03-28-1150000-CE`
 * @param {CensusRanks} ranks
 * @returns {SeriesOrder | null}
 */
export function seriesOrder(instrument, ranks) {
  const m = NAME.exec(instrument);
  if (!m) return null;
  const exchange = ranks.exchanges.indexOf(m[1]), segment = ranks.segments.indexOf(m[2]);
  if (exchange < 0 || segment < 0) return null;
  if (!ranks.contracts.includes(m[2])) return { contract: null, exchange, segment, symbol: m[3] };
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
 * Words in one order that honours every `first|second` pair of `before`, or
 * the pair that cannot be honoured. Words no pair names keep the order they
 * were first seen in. The sets are an exchange list or a segment list: a
 * handful of words, so the quadratic pass is a few comparisons.
 * @param {string[]} seen in order of first appearance
 * @param {Set<string>} before
 * @returns {{order: string[]} | {cycle: string}}
 */
function inOneOrder(seen, before) {
  const left = [...seen];
  /** @type {string[]} */
  const order = [];
  while (left.length > 0) {
    const at = left.findIndex((word) => !left.some((other) => other !== word && before.has(`${other}|${word}`)));
    if (at < 0) {
      const [a, b] = left;
      return { cycle: `${a} and ${b}` };
    }
    order.push(...left.splice(at, 1));
  }
  return { order };
}

/**
 * LEARNS THE ORDER, pair of neighbours by pair. `pair` is called for every
 * row after the first with the row before it; `done` answers the ranks, or
 * the sentence naming why the census is in no single order.
 */
/**
 * The segments in which a name ends in a contract. @param {Iterable<string>} instruments
 * @param {Set<string>} [into]
 */
function contractSegments(instruments, into = new Set()) {
  for (const instrument of instruments) {
    const m = NAME.exec(instrument);
    if (m && !into.has(m[2]) && CONTRACT.test(m[3])) into.add(m[2]);
  }
  return into;
}

/** @param {Set<string>} contracts the segments whose names end in a contract (`contractSegments`) */
function orderLearner(contracts) {
  /** @type {Map<string, {exchange: string, segment: string, contract: string | null} | null>} */
  const parsed = new Map();
  /** @type {string[]} */ const exchanges = [];
  /** @type {string[]} */ const segments = [];
  /** @type {Set<string>} */ const exchangeBefore = new Set();
  /** @type {Set<string>} */ const segmentBefore = new Set();
  /** @type {string | null} */
  let fault = null;
  /** The first row (0-based) whose place contradicts an earlier pair of neighbours, or -1. */
  let contradicts = -1;

  /** @param {string} instrument */
  function parts(instrument) {
    let held = parsed.get(instrument);
    if (held !== undefined) return held;
    const m = NAME.exec(instrument);
    if (!m) {
      held = null;
    } else {
      if (!exchanges.includes(m[1])) exchanges.push(m[1]);
      if (!segments.includes(m[2])) segments.push(m[2]);
      const c = contracts.has(m[2]) ? CONTRACT.exec(m[3]) : null;
      held = contracts.has(m[2]) && c === null ? null : { exchange: m[1], segment: m[2], contract: c ? c[2] : null };
    }
    if (held === null && fault === null) fault = `${JSON.stringify(instrument)} is not a name this page can place in the census's order`;
    parsed.set(instrument, held);
    return held;
  }

  return {
    /** @param {string} instrument */
    first(instrument) { parts(instrument); },
    /** @param {string} beforeMonth @param {string} beforeName @param {string} month @param {string} name @param {number} at the later row, 0-based */
    pair(beforeMonth, beforeName, month, name, at) {
      const b = parts(name);
      if (beforeMonth !== month || beforeName === name) return;
      const a = parts(beforeName);
      if (!a || !b || a.contract !== b.contract) return;
      const [set, first, second] = a.exchange !== b.exchange
        ? [exchangeBefore, a.exchange, b.exchange]
        : [segmentBefore, a.segment, b.segment];
      if (first === second) return;
      // THE OTHER WAY ROUND WAS SEEN FIRST: this row is the one out of the
      // order the rows before it are in, and it is named.
      if (set.has(`${second}|${first}`)) {
        if (contradicts < 0) contradicts = at;
        return;
      }
      set.add(`${first}|${second}`);
    },
    /**
     * @param {(at: number) => string} nameOf a row's `month instrument timeframe`
     * @returns {{ranks: CensusRanks} | {fault: string}}
     */
    done(nameOf) {
      if (fault !== null) return { fault };
      if (contradicts >= 0) {
        return { fault: `row ${contradicts + 1} (${nameOf(contradicts)}) does not follow row ${contradicts} (${nameOf(contradicts - 1)}) in the order this page places a refreshed row by` };
      }
      const byExchange = inOneOrder(exchanges, exchangeBefore), bySegment = inOneOrder(segments, segmentBefore);
      if ('cycle' in byExchange) return { fault: `the census lists the exchanges ${byExchange.cycle} in no single order` };
      if ('cycle' in bySegment) return { fault: `the census lists the segments ${bySegment.cycle} in no single order` };
      return { ranks: { exchanges: byExchange.order, segments: bySegment.order,
        contracts: segments.filter((word) => contracts.has(word)) } };
    }
  };
}

/**
 * A comparator over census rows in the census's order (`ranks`, learned from
 * it): negative when `a` is listed before `b`, zero when they are the same
 * key. Parses each instrument name once (`memo`). Throws for a row it cannot
 * order: an exchange or a segment the census held has none of, which is a
 * refresh that reads the census whole.
 * @param {CensusRanks} ranks
 * @param {Map<string, SeriesOrder | null>} [memo]
 * @returns {(a: {instrument: string, month: string, timeframe: string}, b: {instrument: string, month: string, timeframe: string}) => number}
 */
export function censusOrder(ranks, memo = new Map()) {
  /** @param {string} instrument */
  const parts = (instrument) => {
    let held = memo.get(instrument);
    if (held === undefined) memo.set(instrument, (held = seriesOrder(instrument, ranks)));
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
 * THE ORDER OF A COLUMNAR CENSUS: the ranks learned from it, and `fault`
 * `null` when every row follows the one before it in that order, else the
 * sentence naming the first that does not (or the first name or rung that
 * cannot be ordered at all, or the two words listed in no single order).
 * Two passes, in the census worker; each instrument name is parsed once.
 * @param {CensusColumns} cols
 * @returns {{fault: string | null, ranks: CensusRanks | null}}
 */
export function columnsOrder(cols) {
  const { instruments, months, timeframes } = cols;
  const k = instruments.length;
  const name = (/** @type {number} */ at) => `${months[cols.month[at]]} ${instruments[cols.inst[at]]} ${timeframes[cols.tf[at]]}`;
  const unplaced = (/** @type {number} */ at) => `${JSON.stringify(instruments[at])} is not a name this page can place in the census's order`;
  const follows = (/** @type {number} */ i) => `row ${i + 1} (${name(i)}) does not follow row ${i} (${name(i - 1)}) in the order this page places a refreshed row by`;
  // ONE PARSE PER NAME A ROW USES (a dictionary entry no row names says
  // nothing about the census): the same reading `orderLearner` makes, kept
  // by dictionary index so the two passes over the rows compare integers.
  const used = new Uint8Array(k);
  for (let i = 0; i < cols.n; i++) used[cols.inst[i]] = 1;
  /** @type {(string | null)[]} */ const exchange = new Array(k).fill(null);
  /** @type {string[]} */ const segment = new Array(k);
  /** @type {string[]} */ const tail = new Array(k);
  /** @type {(RegExpExecArray | null)[]} */ const ends = new Array(k).fill(null);
  /** @type {Set<string>} */ const contracts = new Set();
  for (let at = 0; at < k; at++) {
    if (used[at] === 0) continue;
    const m = NAME.exec(instruments[at]);
    if (!m) continue;
    exchange[at] = m[1];
    segment[at] = m[2];
    tail[at] = m[3];
    ends[at] = CONTRACT.exec(m[3]);
    if (ends[at] !== null) contracts.add(m[2]);
  }
  /** The contract a name ends in, where its segment names contracts. @type {(string | null)[]} */
  const contract = new Array(k).fill(null);
  /** @type {string[]} */ const symbol = new Array(k);
  /** @type {string[]} */ const exchanges = [];
  /** @type {string[]} */ const segments = [];
  const seen = (/** @type {number} */ at) => {
    if (exchange[at] === null) return;
    if (!exchanges.includes(/** @type {string} */ (exchange[at]))) exchanges.push(/** @type {string} */ (exchange[at]));
    if (!segments.includes(segment[at])) segments.push(segment[at]);
  };
  for (let at = 0; at < k; at++) {
    if (used[at] === 0 || exchange[at] === null) continue;
    const c = contracts.has(segment[at]) ? ends[at] : null;
    if (contracts.has(segment[at]) && c === null) { exchange[at] = null; continue; }
    contract[at] = c ? c[2] : null;
    symbol[at] = c ? c[1] : tail[at];
  }
  // LEARN: the first name that cannot be placed, in row order, is the fault;
  // else every pair of neighbours that settles two exchanges or two segments.
  /** @type {Set<string>} */ const exchangeBefore = new Set();
  /** @type {Set<string>} */ const segmentBefore = new Set();
  let contradicts = -1;
  for (let i = 0; i < cols.n; i++) {
    const b = cols.inst[i];
    if (exchange[b] === null) return { fault: unplaced(b), ranks: null };
    if (i === 0) { seen(b); continue; }
    const a = cols.inst[i - 1];
    if (a === b) continue;
    seen(b);
    if (cols.month[i - 1] !== cols.month[i] || contract[a] !== contract[b]) continue;
    const byExchange = exchange[a] !== exchange[b];
    if (!byExchange && segment[a] === segment[b]) continue;
    const set = byExchange ? exchangeBefore : segmentBefore;
    const first = /** @type {string} */ (byExchange ? exchange[a] : segment[a]), second = /** @type {string} */ (byExchange ? exchange[b] : segment[b]);
    if (set.has(`${second}|${first}`)) { if (contradicts < 0) contradicts = i; } else set.add(`${first}|${second}`);
  }
  if (contradicts >= 0) return { fault: follows(contradicts), ranks: null };
  const byExchange = inOneOrder(exchanges, exchangeBefore), bySegment = inOneOrder(segments, segmentBefore);
  if ('cycle' in byExchange) return { fault: `the census lists the exchanges ${byExchange.cycle} in no single order`, ranks: null };
  if ('cycle' in bySegment) return { fault: `the census lists the segments ${bySegment.cycle} in no single order`, ranks: null };
  /** @type {CensusRanks} */
  const ranks = { exchanges: byExchange.order, segments: bySegment.order, contracts: segments.filter((word) => contracts.has(word)) };
  // CHECK: every row follows the one before it in that order.
  const exchangeRank = new Int32Array(k), segmentRank = new Int32Array(k);
  for (let at = 0; at < k; at++) {
    if (used[at] === 0) continue;
    exchangeRank[at] = ranks.exchanges.indexOf(/** @type {string} */ (exchange[at]));
    segmentRank[at] = ranks.segments.indexOf(segment[at]);
  }
  const rungs = timeframes.map(rungOrder);
  const bad = rungs.findIndex((seconds) => Number.isNaN(seconds));
  if (bad >= 0) return { fault: `${JSON.stringify(timeframes[bad])} is not a rung this page can place in the census's order`, ranks };
  for (let i = 1; i < cols.n; i++) {
    const ma = cols.month[i - 1], mb = cols.month[i];
    let by = 0;
    if (ma !== mb) by = months[ma] > months[mb] ? -1 : months[ma] < months[mb] ? 1 : 0;
    const a = cols.inst[i - 1], b = cols.inst[i];
    if (by === 0 && a !== b) {
      const ca = contract[a], cb = contract[b];
      if (ca !== cb) by = ca === null ? -1 : cb === null ? 1 : ca < cb ? -1 : 1;
      else if (exchangeRank[a] !== exchangeRank[b]) by = exchangeRank[a] - exchangeRank[b];
      else if (segmentRank[a] !== segmentRank[b]) by = segmentRank[a] - segmentRank[b];
      else if (symbol[a] !== symbol[b]) by = symbol[a] < symbol[b] ? -1 : 1;
    }
    if (by === 0) by = rungs[cols.tf[i - 1]] - rungs[cols.tf[i]];
    if (by >= 0) return { fault: follows(i), ranks };
  }
  return { fault: null, ranks };
}

/** `columnsOrder`'s fault alone. @param {CensusColumns} cols @returns {string | null} */
export const columnsOrderFault = (cols) => columnsOrder(cols).fault;

/**
 * `columnsOrder` over row objects (a census this page decoded itself), and
 * the bars they hold, in slices: one pass learns the order, one checks it.
 * @param {any[]} rows
 * @param {(n: number, work: (start: number, end: number) => void, signal?: AbortSignal) => Promise<void>} slices
 * @param {AbortSignal} [signal]
 * @returns {Promise<{order: string | null, bars: number, ranks: CensusRanks | null}>}
 */
export async function measureCensus(rows, slices, signal) {
  /** @type {Set<string>} */
  const contracts = new Set();
  await slices(rows.length, (start, end) => {
    for (let i = start; i < end; i++) contractSegments([String(rows[i]?.instrument)], contracts);
  }, signal);
  const learner = orderLearner(contracts);
  let bars = 0;
  await slices(rows.length, (start, end) => {
    for (let i = start; i < end; i++) {
      const row = rows[i];
      bars += Number(row?.rows) || 0;
      if (i === 0) learner.first(String(row?.instrument));
      else learner.pair(String(rows[i - 1]?.month), String(rows[i - 1]?.instrument), String(row?.month), String(row?.instrument), i);
    }
  }, signal);
  const learned = learner.done((at) => rowName(rows[at]));
  if ('fault' in learned) return { order: learned.fault, bars, ranks: null };
  const before = censusOrder(learned.ranks);
  /** @type {string | null} */
  let order = null;
  await slices(rows.length, (start, end) => {
    for (let i = Math.max(1, start); i < end && order === null; i++) {
      try {
        if (before(rows[i - 1], rows[i]) >= 0) {
          order = `row ${i + 1} (${rowName(rows[i])}) does not follow row ${i} (${rowName(rows[i - 1])}) in the order this page places a refreshed row by`;
        }
      } catch (why) {
        order = String(/** @type {any} */ (why)?.message ?? why);
      }
    }
  }, signal);
  return { order, bars, ranks: learned.ranks };
}

/**
 * WHAT A PAGE KNOWS ABOUT A CENSUS ARRAY IT HOLDS, beside the array: whether
 * its rows are in the order above (`order` is `null`, or the sentence naming
 * what is not) and the bars they sum to. Kept against the array itself, so a
 * census decoded in the worker carries its own measure and one merged from a
 * delta the measure of its merge; an array nobody measured has none.
 * `ranks` is the order of exchanges and segments learned from it, which a
 * merge into it places rows by.
 * @type {WeakMap<object, {order: string | null, bars: number, ranks?: CensusRanks | null}>}
 */
const measures = new WeakMap();
/** @param {any[]} rows @param {{order: string | null, bars: number, ranks?: CensusRanks | null}} measure */
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
 * @param {CensusRanks | null} [ranks] the order learned from `held`; its own measure's when omitted
 * @returns {Promise<{rows: any[], added: number, replaced: number, bars: number}>} `bars` is the CHANGE in bars held
 */
export async function mergeCensus(held, delta, slices, signal, ranks = censusMeasureOf(held)?.ranks ?? null) {
  if (!ranks) throw new Error('the order of the census held was never learned, so a row cannot be placed in it');
  const before = censusOrder(ranks);
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
