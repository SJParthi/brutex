import { yieldToPage } from './database-preparation.js';

export const CENSUS_ENCODING = 'census-tuples-v1';

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
  const invalid = () => new Error('The body of /store.json is not a JSON array or a valid complete census-tuples-v1 inventory');
  if (!value || typeof value !== 'object' || value.schema !== 1 || value.encoding !== CENSUS_ENCODING || value.feed !== feed || !Array.isArray(value.rows)) throw invalid();
  for (const key of ['instruments', 'months', 'timeframes', 'reasons']) {
    if (!Array.isArray(value[key]) || !value[key].every((/** @type {unknown} */ token) => typeof token === 'string' && token.length > 0)) throw invalid();
  }
  /** @param {number} index @param {string[]} dictionary */
  const token = (index, dictionary) => {
    if (!Number.isSafeInteger(index) || index < 0 || index >= dictionary.length) throw invalid();
    return dictionary[index];
  };
  /** @param {number|null} bps @param {number|null} index */
  const reason = (bps, index) => {
    if (bps !== null) {
      if (!Number.isSafeInteger(bps) || index !== null) throw invalid();
      return null;
    }
    return token(/** @type {number} */ (index), value.reasons);
  };
  /** @type {any[]} */
  const rows = new Array(value.rows.length);
  for (let start = 0; start < value.rows.length; start += 2048) {
    signal.throwIfAborted();
    const end = Math.min(start + 2048, value.rows.length);
    for (let index = start; index < end; index++) {
      const row = value.rows[index];
      if (!Array.isArray(row) || row.length !== 10 || !Number.isSafeInteger(row[3]) || row[3] < 0 ||
          !Number.isSafeInteger(row[4]) || !Number.isSafeInteger(row[5])) throw invalid();
      rows[index] = { feed, instrument: token(row[0], value.instruments), month: token(row[1], value.months),
        timeframe: token(row[2], value.timeframes), rows: row[3], first_ts: row[4], last_ts: row[5],
        chg_bps: row[6], chg_why: reason(row[6], row[7]), prev_chg_bps: row[8], prev_chg_why: reason(row[8], row[9]) };
    }
    await yieldControl();
  }
  signal.throwIfAborted();
  return rows;
}
