// R3-12: `$lib/store.svelte.js`, compiled by the Svelte compiler and loaded in
// node, so its read path (`syncStore`, `refreshStore`) is driven over a fake
// server rather than read as source. And the invented census answers the
// fake server gives. Every number here is invented; nothing is a vendor row.
import { readFileSync } from 'node:fs';
import { rowsUrl, facetsUrl } from '../src/lib/store-census.js';

/** The client entry of Svelte (node resolves `svelte` to its server entry, whose effects never run). */
export const SVELTE_CLIENT = import.meta.resolve('svelte').replace(/index-server\.js$/, 'index-client.js');
const LIB = new URL('../src/lib/', import.meta.url);

/**
 * A FRESH COPY of the store module (its state is module-level, so each call
 * compiles and imports it again under a new URL). Its `$lib/` and relative
 * imports are resolved to the files on disk.
 * @returns {Promise<any>}
 */
export async function storeModule() {
  const { compileModule } = await import('svelte/compiler');
  const file = new URL('store.svelte.js', LIB);
  const { js } = compileModule(readFileSync(file, 'utf8'), { filename: 'store.svelte.js', generate: 'client' });
  const resolve = (/** @type {string} */ path) => (path === 'svelte' ? SVELTE_CLIENT
    : path.startsWith('$lib/') ? new URL(path.slice(5), LIB).href
      : path.startsWith('.') ? new URL(path, file).href
        : import.meta.resolve(path));
  const code = js.code.replace(/from (["'])([^"']+)\1/g, (_m, q, path) => `from ${q}${resolve(path)}${q}`);
  return import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}#${Math.random()}`);
}

export const ORDER = { exchanges: ['NSE', 'BSE'], segments: ['INDEX', 'CASH', 'FNO'] };
export const HEADERS = { 'content-type': 'application/json', 'x-brutex-census-state': 'held', 'x-brutex-census-generation': '21' };

/** The feed's root answer. @param {string} feed */
export const rootBody = (feed) => ({ schema: 1, feed, segment: null, state: 'held', degraded: null, order: ORDER,
  segments: [{ segment: 'INDEX', generation: 7, underlyings: 2, rows: { '1min': 60 }, first_ts: 1, last_ts: 2 },
    { segment: 'CASH', generation: 5, underlyings: 1, rows: { '1min': 9 }, first_ts: 1, last_ts: 2 },
    { segment: 'FNO', generation: 9, underlyings: 1, rows: { '1s': 30 }, first_ts: 1, last_ts: 2 }] });

/** A tuples body from expanded rows `[instrument, month, rung, rows]`. @param {string} feed @param {Array<[string, string, string, number]>} list */
export const tuples = (feed, list) => {
  const dict = (/** @type {number} */ at) => [...new Set(list.map((row) => row[at]))];
  const instruments = dict(0), months = dict(1), timeframes = dict(2);
  return { schema: 1, encoding: 'census-tuples-v1', feed, instruments, months, timeframes, reasons: ['spot_only'],
    rows: list.map((row) => [instruments.indexOf(row[0]), months.indexOf(row[1]), timeframes.indexOf(row[2]), row[3], 1000, 2000, null, 0, null, 0]) };
};

/**
 * A rows answer for `underlying` in `segment`.
 * @param {string} feed @param {string} segment @param {string} underlying @param {number} generation
 * @param {Array<[string, string, string, number]>} list @param {string | null} [expiry]
 */
export const rowsBody = (feed, segment, underlying, generation, list, expiry = null) => ({ schema: 1, feed, segment, underlying,
  exchange: list.length ? 'NSE' : null, expiry, generation, state: 'held', degraded: null, order: ORDER, cap: 4096,
  held: { months: list.length, rows: list.reduce((n, row) => n + row[3], 0) },
  denominators: [...new Map(list.map((row) => [`${row[1]}|${row[2]}`, [row[1], row[2], row[3] + 5, 3]])).values()],
  census: tuples(feed, list) });

/**
 * The facets answer of `underlying` in `segment` (or of the segment, its underlyings).
 * @param {string} feed @param {string} segment @param {string | null} underlying @param {number} generation @param {string[]} expiries
 */
export const listingBody = (feed, segment, underlying, generation, expiries) => ({ schema: 1, feed, segment, underlying, expiry: null,
  generation, since: null, state: 'held', degraded: null, order: ORDER,
  underlyings: [{ exchange: 'NSE', symbol: underlying ?? 'ALPHA', months: 2, expiries: expiries.length, contracts: 4 * expiries.length, unread_contracts: 0,
    rows: { '1s': 30 }, first_ts: 1, last_ts: 2 }],
  months: [], strikes: [], futures: [],
  expiries: expiries.map((expiry) => ({ expiry, weekday: 'Thu', contracts: 4, strikes: 2, future: true, rows: { '1s': 15 }, first_ts: 1, last_ts: 2 })) });

export const TOO_MANY = { error: 'rows_too_many', why: 'the chain names more than 4096 census rows: narrow it with expiry=', cap: 4096, reached: 4097 };

/**
 * A fake server for one feed and its underlyings: the root, each spot
 * segment's rows, each contract segment's listing and its chain at any
 * expiry, and anything else (the whole census included) answered 503 and
 * RECORDED, so a test can say what was asked.
 * @param {{feed?: string, refuse?: Record<string, [number, any]>}} [options]
 */
export function fakeServer({ feed = 'example', refuse = {} } = {}) {
  /** @type {string[]} */
  const asked = [];
  const answer = (/** @type {string} */ url) => {
    if (refuse[url]) return refuse[url];
    if (url === facetsUrl({ feed, segment: /** @type {any} */ (null) })) return [200, rootBody(feed)];
    const query = new URL(url, 'http://fake').searchParams;
    const segment = query.get('segment');
    const underlying = query.get('underlying');
    const expiry = query.get('expiry');
    if (url.startsWith('/census/facets.json') && segment && underlying) {
      const spot = segment !== 'FNO';
      const key = `NSE-${segment}-${underlying}`;
      if (url === rowsUrl({ feed, segment, underlying, ...(expiry ? { expiry } : {}) })) {
        if (spot) return [200, rowsBody(feed, segment, underlying, segment === 'INDEX' ? 7 : 5, [[key, '2019-03', '1min', 60], [key, '2019-02', '1min', 55]])];
        if (!expiry) return [400, TOO_MANY];
        return [200, rowsBody(feed, segment, underlying, 9, [[`${key}-${expiry}-1150000-CE`, expiry.slice(0, 7), '1s', 10]], expiry)];
      }
      if (url === facetsUrl({ feed, segment, underlying })) return [200, listingBody(feed, segment, underlying, 9, ['2019-02-28', '2019-03-28'])];
    }
    if (url.startsWith('/census/facets.json') && segment && !underlying) {
      return [200, { ...listingBody(feed, segment, null, segment === 'INDEX' ? 7 : segment === 'CASH' ? 5 : 9, []), underlying: null }];
    }
    return [503, { error: 'not_here', why: `the fake server answers no ${url.split('?')[0]}` }];
  };
  /** @param {any} url */
  const fetch = async (url) => {
    const text = String(url);
    asked.push(text);
    const [status, body] = answer(text);
    return new Response(JSON.stringify(body), { status, headers: HEADERS });
  };
  return { fetch, asked };
}

/** Wait until the store is no longer reading (each read is a few promises over the fake server). @param {any} store */
export async function settled(store) {
  for (let i = 0; i < 400 && store.state === 'reading'; i++) await new Promise((done) => setTimeout(done, 1));
  return store.state;
}
