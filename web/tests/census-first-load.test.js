// WP-56b: the order the server states, the root of the selection tree, a
// delta that never rides a cache, the banner's words for a 304 after a
// refused delta, and the refresh log.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createCensusLoader, createFacetsLoader, createRefreshLog, facetsOrder, facetsUrl, mergeFacets, rootFault,
  censusLines, censusLag, refreshEntry, refreshText, searchRefreshes, REFRESH_LOG_KEY } from '../src/lib/store-census.js';
import { CENSUS_ENCODING } from '../src/lib/store-census-wire.js';

const ORDER = { exchanges: ['NSE', 'BSE'], segments: ['INDEX', 'CASH', 'FNO'] };
const under = (/** @type {string} */ exchange, /** @type {string} */ symbol) =>
  ({ exchange, symbol, months: 1, expiries: 0, rows: { '1min': 1 }, first_ts: 1, last_ts: 2 });
const facets = (/** @type {any} */ over = {}) => ({ schema: 1, feed: 'example', segment: 'INDEX', underlying: null, expiry: null,
  generation: 5, since: null, state: 'held', degraded: null, order: ORDER, underlyings: [], months: [], expiries: [], strikes: [], futures: [], ...over });
const root = (/** @type {any} */ over = {}) => ({ schema: 1, feed: 'example', segment: null, state: 'held', degraded: null, order: ORDER,
  segments: [{ segment: 'INDEX', generation: 7, underlyings: 2, rows: { '1min': 60 }, first_ts: 1, last_ts: 2 },
    { segment: 'FNO', generation: 9, underlyings: 1, rows: { '1s': 30 }, first_ts: 1, last_ts: 2 }], ...over });

test('underlyings merge in the order the server states, not one this page keeps', () => {
  // The server lists BSE before NSE in this answer: the merge follows it.
  const turned = { exchanges: ['BSE', 'NSE'], segments: ORDER.segments };
  const base = facets({ order: turned, underlyings: [under('BSE', 'SENSEX'), under('NSE', 'NIFTY')] });
  const merged = mergeFacets(base, facets({ order: turned, generation: 6, underlyings: [under('NSE', 'BANKNIFTY'), under('BSE', 'BANKEX')] }));
  assert.deepEqual(merged.underlyings.map((u) => `${u.exchange}-${u.symbol}`), ['BSE-BANKEX', 'BSE-SENSEX', 'NSE-BANKNIFTY', 'NSE-NIFTY']);
  assert.equal('orderWhy' in merged, false, 'nothing to say when every exchange is named');
  // AN EXCHANGE NO LIST IN THIS REPOSITORY NAMES, stated by the server: placed where it says.
  const wider = { exchanges: ['NSE', 'MCX', 'BSE'], segments: ORDER.segments };
  const withMcx = mergeFacets(facets({ order: wider, underlyings: [under('NSE', 'NIFTY'), under('BSE', 'SENSEX')] }),
    facets({ order: wider, generation: 6, underlyings: [under('MCX', 'COMDEX')] }));
  assert.deepEqual(withMcx.underlyings.map((u) => u.exchange), ['NSE', 'MCX', 'BSE']);
  // NOT NAMED by the order: placed after the named ones, and said.
  const unnamed = mergeFacets(facets({ underlyings: [under('NSE', 'NIFTY'), under('BSE', 'SENSEX')] }), facets({ generation: 6, underlyings: [under('MCX', 'COMDEX')] }));
  assert.deepEqual(unnamed.underlyings.map((u) => u.exchange), ['NSE', 'BSE', 'MCX']);
  assert.equal(unnamed.orderWhy, 'the order this API states does not name MCX');
  // AN API OLDER THAN THE STATEMENT: by text, and said.
  const { order: _gone, ...old } = facets({ underlyings: [under('NSE', 'NIFTY')] });
  const { order: _gone2, ...oldDelta } = facets({ generation: 6, underlyings: [under('BSE', 'SENSEX')] });
  const older = mergeFacets(old, oldDelta);
  assert.deepEqual(older.underlyings.map((u) => u.exchange), ['BSE', 'NSE']);
  assert.match(older.orderWhy, /states no order/);
  assert.deepEqual(facetsOrder({ order: { exchanges: ['NSE', 'NSE'], segments: [] } }),
    { exchanges: [], segments: [], why: 'the order this API states is not two lists of distinct names' });
});

test('the root answer is checked: segments in the order stated, each with a generation and a count', () => {
  assert.equal(rootFault(root(), 'example'), null);
  assert.equal(rootFault(root({ segments: [] }), 'example'), null, 'a feed that holds nothing lists nothing');
  assert.match(String(rootFault(root(), 'other')), /not the root of other/);
  assert.match(String(rootFault(facets(), 'example')), /not the root/);
  assert.match(String(rootFault({ ...root(), segments: undefined }, 'example')), /lists no segments/);
  assert.match(String(rootFault(root({ order: undefined }), 'example')), /states no order/);
  const turned = root();
  turned.segments.reverse();
  assert.match(String(rootFault(turned, 'example')), /not in the order it states \(INDEX\)/);
  assert.match(String(rootFault(root({ segments: [{ segment: 'BOND', generation: 1, underlyings: 1, rows: {} }] }), 'example')), /"BOND" is not in the order it states/);
  assert.match(String(rootFault(root({ segments: [{ segment: 'INDEX', generation: -1, underlyings: 1, rows: {} }] }), 'example')), /generation of INDEX/);
  assert.match(String(rootFault(root({ segments: [{ segment: 'INDEX', generation: 1, underlyings: 0, rows: {} }] }), 'example')), /counts no underlying/);
  assert.match(String(rootFault(null, 'example')), /not a schema-1/);
});

test('the loader reads the root with one request that names no segment, shares a flight, and keeps nothing', async () => {
  /** @type {{url: string, options: any}[]} */
  const asked = [];
  let body = root();
  const loader = createFacetsLoader(async (url, options) => {
    asked.push({ url: String(url), options });
    await new Promise((done) => setImmediate(done));
    return Response.json(body, { headers: { 'x-brutex-census-state': 'held' } });
  });
  assert.equal(facetsUrl({ feed: 'a b' }), '/census/facets.json?feed=a%20b');
  const [one, two] = await Promise.all([loader.root('example'), loader.root('example')]);
  assert.equal(asked.length, 1, 'two callers in one turn share one request');
  assert.equal(asked[0].url, '/census/facets.json?feed=example');
  assert.equal(asked[0].options.cache, 'no-store');
  assert.equal(one.ok && two.ok, true);
  assert.deepEqual(one.body.segments.map((/** @type {any} */ row) => row.segment), ['INDEX', 'FNO']);
  // NOT KEPT: the next ask is a new request, and sees what the feed holds now.
  body = root({ segments: [] });
  const later = await loader.root('example');
  assert.equal(asked.length, 2);
  assert.deepEqual(later.body.segments, []);
  // A body that is not a root is refused with its reason, never used.
  body = /** @type {any} */ (facets());
  const wrong = await loader.root('example');
  assert.equal(wrong.ok, false);
  assert.match(wrong.reason ?? '', /not the root of example/);
});

const wire = (/** @type {string[]} */ lines, /** @type {any} */ more = {}) => {
  const rows = lines.map((text) => text.split(' '));
  const uniq = (/** @type {number} */ at) => [...new Set(rows.map((r) => r[at]))];
  const instruments = uniq(1), months = uniq(0), timeframes = uniq(2);
  return { schema: 1, encoding: CENSUS_ENCODING, feed: 'example', fields: ['instrument', 'month', 'timeframe', 'rows', 'first_ts', 'last_ts', 'chg_bps', 'chg_why', 'prev_chg_bps', 'prev_chg_why'],
    instruments, months, timeframes, reasons: ['previous_not_stored'],
    rows: rows.map((r) => [instruments.indexOf(r[1]), months.indexOf(r[0]), timeframes.indexOf(r[2]), 2, 1, 2, null, 0, null, 0]), ...more };
};
const HELD = ['2019-03 NSE-INDEX-NIFTY 1min', '2019-02 NSE-INDEX-NIFTY 1min'];
const headers = (/** @type {Record<string, string>} */ more = {}) => ({ 'x-brutex-census-state': 'held', 'x-brutex-census-degraded': '',
  'x-brutex-census-generation': '40', 'x-brutex-census-part-generations': '', etag: '"g40"', ...more });

test('a delta is asked with cache no-store; a refused delta whose whole read is a 304 says no body was read', async () => {
  /** @type {{url: string, options: any}[]} */
  const asked = [];
  let wholeReads = 0;
  const loader = createCensusLoader(async (url, options) => {
    asked.push({ url: String(url), options });
    if (String(url).includes('since=')) {
      return Response.json({ error: 'since_too_old', why: 'read whole since' }, { status: 410, headers: headers({ 'x-brutex-census-since': '40' }) });
    }
    wholeReads += 1;
    if (new Headers(options?.headers).get('if-none-match') === '"g40"') return new Response(null, { status: 304, headers: headers() });
    return Response.json(wire(HELD), { headers: headers() });
  });
  const first = await loader.load('example', 0);
  assert.equal(first.ok, true);
  const again = await loader.load('example', 1);
  const delta = asked.find(({ url }) => url.includes('since='));
  assert.equal(delta?.options.cache, 'no-store', 'the delta never rides a browser cache');
  assert.equal(again.status, 304);
  assert.equal(again.how?.kind, 'unchanged');
  assert.match(again.how?.why ?? '', /the server refused a delta since 40 \(HTTP 410/);
  assert.equal(again.body, first.body, 'the rows held are kept: no body was read');
  assert.equal(wholeReads, 2);
  const lines = censusLines({ feed: 'example', at: 1_760_000_000_000, lag: censusLag(again.headers), worker: null, head: null, how: again.how });
  const said = lines.find((line) => line.key === 'whole');
  assert.equal(said?.tone, 'info', 'nothing was read whole, so it is not a warning');
  assert.match(said?.text ?? '', /^This refresh could not ask for only what changed \(the server refused a delta since 40 .*\), so it asked for the whole census; the server answered that the census this page holds is current, and no body was read\.$/);
  assert.equal(lines.some((line) => /read the whole census again/.test(line.text)), false, 'never "read the whole census again" beside "unchanged"');
  // A whole body that WAS read keeps the warning.
  const whole = censusLines({ feed: 'example', at: 1, lag: censusLag(again.headers), worker: null, head: null,
    how: { kind: 'whole', since: null, rows: 0, added: 0, replaced: 0, why: 'the delta did not add up' } });
  assert.deepEqual(whole.find((line) => line.key === 'whole'), { key: 'whole', tone: 'warn', text: 'This refresh read the whole census again, not only what changed: the delta did not add up.' });
});

/** A storage that keeps strings, or throws on demand. */
function fakeStorage() {
  /** @type {Map<string, string>} */
  const kept = new Map();
  return { kept, full: false, getItem(/** @type {string} */ key) { return kept.get(key) ?? null; },
    setItem(/** @type {string} */ key, /** @type {string} */ value) { if (this.full) throw new Error('quota exceeded'); kept.set(key, value); } };
}

test('every read of the census leaves a log entry that is saved, kept across loads, bounded and searchable', () => {
  const storage = fakeStorage();
  const log = createRefreshLog(storage, { keep: 3 });
  assert.deepEqual(log.list(), []);
  assert.equal(log.fault, null);
  const lag = /** @type {any} */ ({ generation: 7053829 });
  log.note(refreshEntry({ feed: 'gdfl', at: 1_760_000_000_000, how: { kind: 'whole', since: null, rows: 0, added: 0, replaced: 0, why: null }, lag, held: 385442, first: true }));
  log.note(refreshEntry({ feed: 'gdfl', at: 1_760_000_005_000, how: { kind: 'delta', since: 'INDEX=1,CASH=2,FNO=3', rows: 3, added: 1, replaced: 2, why: null }, lag, held: 385443 }));
  log.note(refreshEntry({ feed: 'gdfl', at: 1_760_000_010_000, how: { kind: 'whole', since: null, rows: 0, added: 0, replaced: 0, why: 'the server refused a delta since 9 (HTTP 410: since_too_old)' }, lag, held: 385443 }));
  log.note(refreshEntry({ feed: 'zerodha', at: 1_760_000_015_000, error: '/store.json answered HTTP 503' }));
  assert.deepEqual(log.list().map((e) => e.kind), ['delta', 'whole', 'failed'], 'the newest three are kept, oldest first');
  // SAVED: another load of the page reads the same log.
  const reloaded = createRefreshLog(storage, { keep: 3 });
  assert.deepEqual(reloaded.list(), log.list());
  assert.equal(JSON.parse(storage.kept.get(REFRESH_LOG_KEY) ?? '[]').length, 3);
  // THE SENTENCES.
  const [delta, whole, failed] = log.list();
  assert.match(refreshText(delta), / · gdfl · read only the 3 row\(s\) changed since INDEX=1,CASH=2,FNO=3 \(1 new, 2 changed in place\); generation 70,53,829, 3,85,443 row\(s\) held\.$/);
  assert.match(refreshText(whole), /read the whole census again: the server refused a delta since 9 \(HTTP 410: since_too_old\); generation/);
  assert.match(refreshText(failed), / · zerodha · failed: \/store\.json answered HTTP 503\.$/);
  assert.match(refreshText(refreshEntry({ feed: 'gdfl', at: 1, how: null, first: true })), /the first read of this feed on this page/);
  assert.match(refreshText(refreshEntry({ feed: 'gdfl', at: 1, how: { kind: 'unchanged', since: '40', rows: 0, added: 0, replaced: 0, why: null } })), /found nothing changed since 40/);
  assert.match(refreshText(refreshEntry({ feed: 'gdfl', at: 1, how: { kind: 'unchanged', since: null, rows: 0, added: 0, replaced: 0, why: 'no epoch' } })), /asked for the whole census \(no epoch\) and the server answered that the census held is current; no body was read/);
  // SEARCHABLE: every word must be in the entry's sentence; newest first.
  assert.deepEqual(searchRefreshes(log.list(), '').map((e) => e.kind), ['failed', 'whole', 'delta']);
  assert.deepEqual(searchRefreshes(log.list(), 'since_too_old').map((e) => e.kind), ['whole']);
  assert.deepEqual(searchRefreshes(log.list(), 'GDFL whole').map((e) => e.kind), ['whole']);
  assert.deepEqual(searchRefreshes(log.list(), 'zerodha 503').map((e) => e.kind), ['failed']);
  assert.deepEqual(searchRefreshes(log.list(), 'nothing-matches-this'), []);
  // TWO TABS: an entry another tab saved meanwhile is kept, not overwritten.
  const other = createRefreshLog(storage, { keep: 10 });
  other.note(refreshEntry({ feed: 'gdfl', at: 1_760_000_020_000, how: { kind: 'unchanged', since: '41', rows: 0, added: 0, replaced: 0, why: null } }));
  const wide = createRefreshLog(storage, { keep: 10 });
  wide.note(refreshEntry({ feed: 'gdfl', at: 1_760_000_025_000, error: 'offline' }));
  assert.deepEqual(wide.list().map((e) => e.at), [1_760_000_005_000, 1_760_000_010_000, 1_760_000_015_000, 1_760_000_020_000, 1_760_000_025_000]);
});

test('a refresh log that cannot be saved or read says so and still holds this tab\'s reads', () => {
  // STORAGE FULL: the write is refused; the entry is kept in the tab and the fault named.
  const storage = fakeStorage();
  storage.full = true;
  const log = createRefreshLog(storage);
  log.note(refreshEntry({ feed: 'gdfl', at: 5, error: 'x' }));
  assert.equal(log.list().length, 1);
  assert.match(log.fault ?? '', /could not be saved \(quota exceeded\), so it holds only what this tab read/);
  // NO STORAGE AT ALL (a private window that offers none).
  const none = createRefreshLog(null);
  none.note(refreshEntry({ feed: 'gdfl', at: 6, error: 'y' }));
  assert.equal(none.list().length, 1);
  assert.match(none.fault ?? '', /offers no storage/);
  // GARBAGE SAVED BY SOMETHING ELSE: not read as a log, said, and replaced by the next read.
  for (const garbage of ['not json', '{"a":1}', '[{"at":"soon"}]', '[null]']) {
    const dirty = fakeStorage();
    dirty.kept.set(REFRESH_LOG_KEY, garbage);
    const read = createRefreshLog(dirty);
    assert.deepEqual(read.list(), [], garbage);
    assert.match(read.fault ?? '', /could not be read|is not a list of entries/, garbage);
    read.note(refreshEntry({ feed: 'gdfl', at: 7, error: 'z' }));
    assert.equal(JSON.parse(dirty.kept.get(REFRESH_LOG_KEY) ?? '[]').length, 1, garbage);
  }
  // A storage whose read throws (blocked site data).
  const blocked = createRefreshLog({ getItem() { throw new Error('access denied'); }, setItem() { throw new Error('access denied'); } });
  assert.match(blocked.fault ?? '', /could not be read \(access denied\)/);
  blocked.note(refreshEntry({ feed: 'gdfl', at: 8, error: 'w' }));
  assert.equal(blocked.list().length, 1);
});
