import { ask } from './ask.js';
import { parseKey } from './instrument.js';

/** Read only the displayed rows from the already addressed month files.
 * The census has selected these files and their global base; each native read
 * seeks directly to its local offset. No extrema request triggers a history
 * scan. Transfer and bar work are bounded by the page size (plus one native
 * predecessor read per file). Finding the months in the census remains O(files).
 * @param {string} feed
 * @param {any[]} files Files in displayed time order.
 * @param {number} first Global zero-based page offset.
 * @param {number} limit Number of displayed rows.
 * @param {number} base Global offset of the first file in files.
 * @param {boolean} desc
 * @param {AbortSignal} signal
 * @param {typeof ask} [request]
 */
export async function readDatabasePage(feed, files, first, limit, base, desc, signal, request = ask) {
  if (![first, limit, base].every(Number.isSafeInteger) || first < 0 || base < 0 || base > first || limit < 1 || limit > 1000) {
    throw new Error('The database page must have exact bounded row offsets and a size from 1 through 1000.');
  }
  signal.throwIfAborted();
  const end = first + limit;
  if (!Number.isSafeInteger(end)) throw new Error('The database page end exceeds exact row addressing.');
  /** @type {{row:any,offset:number,limit:number}[]} */
  const pages = [];
  let seen = base;
  for (const row of files) {
    if (!Number.isSafeInteger(row.rows) || row.rows < 0 || !Number.isSafeInteger(seen + row.rows)) {
      throw new Error('The stored month has an invalid row count for page addressing.');
    }
    const offset = Math.max(0, first - seen);
    const take = Math.max(0, Math.min(row.rows, end - seen) - offset);
    if (take > 0) pages.push({ row, offset, limit: take });
    seen += row.rows;
    if (seen >= end) break;
  }
  /** @type {any[]} */
  const out = new Array(pages.length);
  let next = 0;
  const worker = async () => {
    while (next < pages.length) {
      signal.throwIfAborted();
      const index = next++;
      const { row, offset, limit: take } = pages[index];
      const parsed = parseKey(row.instrument);
      const prefix = `${parsed.exchange}-${parsed.segment}-${parsed.underlying}`;
      const result = { key: row.key, row, bars: [], faults: null, error: null, paged: true, held: null };
      try {
        if (parsed.why || !parsed.exchange || !parsed.segment || !parsed.underlying) throw new Error(parsed.why ?? 'The stored month has no address.');
        const query = new URLSearchParams({ feed, exchange: parsed.exchange, segment: parsed.segment,
          symbol: parsed.underlying, contract: parsed.kind === 'spot' ? '' : parsed.key.slice(prefix.length + 1),
          timeframe: row.timeframe, from: row.month, to: row.month, sort: 'ts',
          dir: desc ? 'desc' : 'asc', offset: String(offset), limit: String(take), extremes: '0' });
        const response = await request(`/bars/window.json?${query}`, { signal });
        const body = await response.json();
        signal.throwIfAborted();
        if (!response.ok) throw new Error(body?.error ?? `HTTP ${response.status}`);
        if (body?.months_read !== 1 || body?.months_missing !== 0) {
          throw new Error(`The stored month ${row.instrument} ${row.month} could not be opened for this page.`);
        }
        if (!body || !Array.isArray(body.bars) || body.bars.length > take ||
            !Number.isSafeInteger(body.total) || body.total < 0 || body.scanned !== false ||
            (body.faults !== null && typeof body.faults !== 'string')) {
          throw new Error('The server did not return the bounded stored bar page that was requested.');
        }
        if (body.faults === null && body.bars.length !== Math.min(take, Math.max(0, body.total - offset))) {
          throw new Error('The stored bar page is incomplete and the server supplied no fault reason.');
        }
        out[index] = { ...result, bars: body.bars, faults: body.faults, held: body.total };
      } catch (why) {
        signal.throwIfAborted();
        out[index] = { ...result, error: String(why instanceof Error ? why.message : why) };
      }
    }
  };
  await Promise.all(Array.from({ length: Math.min(6, pages.length) }, worker));
  signal.throwIfAborted();
  return out;
}
