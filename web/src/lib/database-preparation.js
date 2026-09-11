import { denominators } from './completeness.js';

/** Yield the main thread between bounded preparation slices. */
export function yieldToPage() {
  const scheduler = /** @type {{scheduler?:{yield?:()=>Promise<void>}}} */ (globalThis).scheduler;
  return scheduler?.yield ? scheduler.yield() : new Promise(resolve => setTimeout(resolve, 0));
}

/** Prepare a complete census without monopolising the navigation task.
 * The same denominator implementation serves every chunk; maps merge before
 * any row is decorated. No partial totals or rows are returned.
 * @template {import('./completeness.js').Row} T
 * @template R
 * @param {T[]} rows
 * @param {(row:T, observed:ReturnType<typeof denominators>)=>R} decorate
 * @param {{signal:AbortSignal,chunkSize?:number,yieldControl?:()=>Promise<unknown>,tokens?:(row:R)=>string[]}} options
 * @returns {Promise<{rows:R[],observed:ReturnType<typeof denominators>,prefix:Map<string,R[]>}>}
 */
export async function prepareDatabase(rows, decorate, options) {
  const { signal, chunkSize = 2048, yieldControl = yieldToPage } = options;
  if (!Number.isSafeInteger(chunkSize) || chunkSize < 1 || chunkSize > 8192) {
    throw new Error('Database preparation chunk must be from 1 through 8192 rows.');
  }
  signal.throwIfAborted();
  await yieldControl();
  const observed = denominators([]);
  for (let start = 0; start < rows.length; start += chunkSize) {
    signal.throwIfAborted();
    const part = denominators(rows.slice(start, start + chunkSize));
    for (const [key, maximum] of part.fullest) {
      observed.fullest.set(key, Math.max(observed.fullest.get(key) ?? maximum, maximum));
    }
    for (const [key, count] of part.support) {
      observed.support.set(key, (observed.support.get(key) ?? 0) + count);
    }
    await yieldControl();
  }
  /** @type {R[]} */
  const decorated = new Array(rows.length);
  /** @type {Map<string,R[]>} */
  const prefix = new Map();
  for (let start = 0; start < rows.length; start += chunkSize) {
    signal.throwIfAborted();
    const end = Math.min(start + chunkSize, rows.length);
    for (let index = start; index < end; index++) {
      const row = decorate(rows[index], observed);
      decorated[index] = row;
      const seen = new Set();
      for (const raw of options.tokens?.(row) ?? []) {
        const token = raw.toUpperCase();
        for (let length = 1; length <= Math.min(4, token.length); length++) {
          const key = token.slice(0, length);
          if (seen.has(key)) continue;
          seen.add(key);
          let bucket = prefix.get(key);
          if (!bucket) prefix.set(key, (bucket = []));
          bucket.push(row);
        }
      }
    }
    await yieldControl();
  }
  signal.throwIfAborted();
  return { rows: decorated, observed, prefix };
}

/** One name parse per instrument in this immutable census generation.
 * @template R
 * @param {(name:string)=>R} parse
 * @returns {(name:string)=>R}
 */
export function instrumentMemo(parse) {
  /** @type {Map<string,R>} */
  const names = new Map();
  return name => {
    if (names.has(name)) return /** @type {R} */ (names.get(name));
    const value = parse(name);
    names.set(name, value);
    return value;
  };
}

/** Keep one completed immutable snapshot for navigation back to the DB page.
 * A changed input invalidates it. Failed or cancelled work is never retained.
 * This saves repeated O(rows) preparation; initial preparation still scales
 * with the inventory and stores O(rows) references.
 */
export function createDatabasePreparation() {
  /** @type {{input:unknown[],body:any}|null} */
  let completed = null;
  let revision = 0;
  /** @type {typeof prepareDatabase} */
  const read = async (rows, decorate, options) => {
    options.signal.throwIfAborted();
    if (completed?.input === rows) return completed.body;
    completed = null;
    const mine = ++revision;
    const body = await prepareDatabase(rows, decorate, options);
    if (mine === revision && !options.signal.aborted) completed = { input: rows, body };
    return body;
  };
  return { read };
}

export const databasePreparation = createDatabasePreparation();
