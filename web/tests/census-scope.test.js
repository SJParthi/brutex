// WP-56b: the first paint of /db from ONE INSTRUMENT'S census rows
// (`/census/facets.json?...&rows=census-tuples-v1`), never the whole census:
// what is read, what is refused and said, and which read the shared store
// makes for whom. R3-12 (FM-621): the scope names the segments its view reads,
// a contract view offers its expiries from the facets answer, and no /db view
// reads the whole census in a scope's place. Every number below is invented.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readScope, rowsFault, rowsUrl, facetsUrl, planRead, scopeText, censusLines, refreshEntry, refreshText, searchRefreshes,
  viewSegments, viewFamily, unknownViews, scopeCovers } from '../src/lib/store-census.js';
import { denominators, denomKey } from '../src/lib/completeness.js';

const ORDER = { exchanges: ['NSE', 'BSE'], segments: ['INDEX', 'CASH', 'FNO'] };
const HEADERS = { 'content-type': 'application/json', 'x-brutex-census-state': 'held', 'x-brutex-census-generation': '21' };
const root = (/** @type {any} */ over = {}) => ({ schema: 1, feed: 'example', segment: null, state: 'held', degraded: null, order: ORDER,
  segments: [{ segment: 'INDEX', generation: 7, underlyings: 2, rows: { '1min': 60 }, first_ts: 1, last_ts: 2 },
    { segment: 'CASH', generation: 5, underlyings: 1, rows: { '1min': 9 }, first_ts: 1, last_ts: 2 },
    { segment: 'FNO', generation: 9, underlyings: 1, rows: { '1s': 30 }, first_ts: 1, last_ts: 2 }], ...over });
/** A tuples body from expanded rows `[instrument, month, rung, rows]`. */
const tuples = (/** @type {Array<[string, string, string, number]>} */ list) => {
  const dict = (/** @type {number} */ at) => [...new Set(list.map((row) => row[at]))];
  const instruments = dict(0), months = dict(1), timeframes = dict(2);
  return { schema: 1, encoding: 'census-tuples-v1', feed: 'example', instruments, months, timeframes, reasons: ['spot_only'],
    rows: list.map((row) => [instruments.indexOf(row[0]), months.indexOf(row[1]), timeframes.indexOf(row[2]), row[3], 1000, 2000, null, 0, null, 0]) };
};
const rowsBody = (/** @type {string} */ segment, /** @type {number} */ generation, /** @type {Array<[string, string, string, number]>} */ list,
  /** @type {any} */ over = {}) => ({ schema: 1, feed: 'example', segment, underlying: 'ALPHA', exchange: list.length ? 'NSE' : null, expiry: null,
  generation, state: 'held', degraded: null, order: ORDER, cap: 4096,
  held: { months: list.length, rows: list.reduce((n, row) => n + row[3], 0) },
  denominators: [...new Map(list.map((row) => [`${row[1]}|${row[2]}`, [row[1], row[2], row[3] + 5, 3]])).values()],
  census: tuples(list), ...over });
/** The facets answer of ALPHA in a contract segment: its expiries, and no rows. */
const listingBody = (/** @type {string} */ segment, /** @type {number} */ generation, /** @type {string[]} */ expiries, /** @type {any} */ over = {}) => ({
  schema: 1, feed: 'example', segment, underlying: 'ALPHA', expiry: null, generation, since: null, state: 'held', degraded: null, order: ORDER,
  underlyings: expiries.length ? [{ exchange: 'NSE', symbol: 'ALPHA', months: 2, expiries: expiries.length, contracts: 40, unread_contracts: 0,
    rows: { '1s': 30 }, first_ts: 1, last_ts: 2 }] : [],
  months: [], strikes: [], futures: [],
  expiries: expiries.map((expiry, n) => ({ expiry, weekday: 'Thu', contracts: 20 + n, strikes: 10, future: n % 2 === 0, rows: { '1s': 15 }, first_ts: 1, last_ts: 2 })),
  ...over });
const INDEX_ROWS = /** @type {Array<[string, string, string, number]>} */ ([
  ['NSE-INDEX-ALPHA', '2019-03', '1min', 60], ['NSE-INDEX-ALPHA', '2019-03', '1day', 3], ['NSE-INDEX-ALPHA', '2019-02', '1min', 55]]);
const TOO_MANY = { error: 'rows_too_many', why: 'ALPHA in FNO names more than 4096 census rows: narrow it with expiry=, or read the census whole from /store.json', cap: 4096, reached: 4097 };
const EXPIRIES = ['2019-02-28', '2019-03-28'];

/** A server: `answers` maps a URL to `[status, body, headers?]`; every URL asked is recorded. */
const server = (/** @type {Record<string, [number, any, Record<string, string>?]>} */ answers) => {
  /** @type {string[]} */
  const asked = [];
  /** @type {any} */
  const request = async (/** @type {string} */ url, /** @type {any} */ options) => {
    asked.push(url);
    assert.equal(options?.cache, 'no-store', `${url} must not ride a cache`);
    const hit = answers[url];
    if (!hit) return new Response(JSON.stringify({ error: 'not_scripted', why: `no answer scripted for ${url}` }), { status: 500, headers: HEADERS });
    return new Response(JSON.stringify(hit[1]), { status: hit[0], headers: { ...HEADERS, ...(hit[2] ?? {}) } });
  };
  return { request, asked };
};
const Q = (/** @type {string} */ segment, /** @type {string | null} */ expiry = null) =>
  rowsUrl({ feed: 'example', segment, underlying: 'ALPHA', ...(expiry ? { expiry } : {}) });
/** The listing (facets answer, no rows) of ALPHA in a segment. */
const L = (/** @type {string} */ segment) => facetsUrl({ feed: 'example', segment, underlying: 'ALPHA' });
const ROOT = '/census/facets.json?feed=example';
/** Every answer a scope of every segment can ask. */
const EVERY = () => ({ [ROOT]: /** @type {[number, any]} */ ([200, root()]), [Q('INDEX')]: /** @type {[number, any]} */ ([200, rowsBody('INDEX', 7, INDEX_ROWS)]),
  [Q('CASH')]: /** @type {[number, any]} */ ([200, rowsBody('CASH', 5, [])]), [L('FNO')]: /** @type {[number, any]} */ ([200, listingBody('FNO', 9, EXPIRIES)]) });

test('a first paint reads the root and one instrument\'s rows per spot segment and the contract listing, never /store.json', async () => {
  const { request, asked } = server(EVERY());
  const answer = await readScope(request, 'example', { underlying: 'ALPHA' }, { now: () => 1234 });
  assert.equal(answer.ok, true, JSON.stringify(answer));
  if (!answer.ok) return;
  // NO SEGMENT NAMED IS EVERY SEGMENT; the contract segment is LISTED, its
  // rows are not asked without an expiry (R3-12).
  assert.deepEqual(asked.sort(), [ROOT, Q('CASH'), Q('INDEX'), L('FNO')].sort());
  assert.equal(asked.some((url) => url.includes('/store.json')), false);
  assert.equal(asked.includes(Q('FNO')), false);
  assert.equal(Q('INDEX'), '/census/facets.json?feed=example&segment=INDEX&underlying=ALPHA&rows=census-tuples-v1');
  // THE ROWS ARE THE CENSUS'S, in its order (newest month first), decoded by
  // the decoder the whole census uses.
  assert.deepEqual(answer.rows.map((row) => `${row.instrument}|${row.month}|${row.timeframe}|${row.rows}`),
    ['NSE-INDEX-ALPHA|2019-03|1min|60', 'NSE-INDEX-ALPHA|2019-03|1day|3', 'NSE-INDEX-ALPHA|2019-02|1min|55']);
  assert.equal(answer.rows[0].first_ts, 1000);
  // THE DENOMINATORS ARE THE SERVER'S (the feed's), not a recount of these
  // rows: recounting one instrument would call its own row the fullest.
  assert.equal(answer.observed.fullest.get(denomKey('2019-03', '1min')), 65);
  assert.equal(answer.observed.support.get(denomKey('2019-03', '1min')), 3);
  assert.equal(denominators(answer.rows).fullest.get(denomKey('2019-03', '1min')), 60, 'what a recount of the scope alone would say');
  assert.equal(answer.lag.generation, 21);
  assert.equal(answer.readAt, 1234);
  assert.deepEqual(answer.scope.read, [{ segment: 'INDEX', expiry: null, months: 3, rows: 118, generation: 7 }, { segment: 'CASH', expiry: null, months: 0, rows: 0, generation: 5 }]);
  assert.deepEqual(answer.scope.unread, []);
  assert.deepEqual(answer.scope.views, []);
  assert.deepEqual(answer.scope.segments, ['INDEX', 'CASH', 'FNO']);
  assert.deepEqual(answer.scope.offered, [{ segment: 'FNO', generation: 9, expiries: 2, contracts: 40 }]);
  assert.deepEqual(answer.scope.expiries.map((row) => `${row.segment} ${row.expiry} ${row.contracts} ${row.future}`), ['FNO 2019-02-28 20 true', 'FNO 2019-03-28 21 false']);
  assert.deepEqual(answer.scope.listed.map((row) => `${row.segment} ${row.underlyings} ${row.read}`), ['INDEX 2 true', 'CASH 1 true', 'FNO 1 true']);
  assert.deepEqual(answer.scope.unplaced, []);
  assert.equal(answer.scope.expiry, null);
  assert.equal(scopeText(answer.scope), 'INDEX 3 row(s), FNO 2 expiry(ies) offered');
});

test('a contract segment is read at the expiry on screen, beside its listing, and merged in the census\'s order', async () => {
  const chain = /** @type {Array<[string, string, string, number]>} */ ([
    ['NSE-FNO-ALPHA-2019-03-28-1150000-CE', '2019-03', '1s', 10], ['NSE-FNO-ALPHA-2019-03-28-1150000-PE', '2019-03', '1s', 20]]);
  const { request, asked } = server({ ...EVERY(), [Q('FNO', '2019-03-28')]: [200, rowsBody('FNO', 9, chain, { expiry: '2019-03-28' })] });
  const answer = await readScope(request, 'example', { underlying: 'ALPHA', expiry: '2019-03-28' });
  assert.equal(answer.ok, true, JSON.stringify(answer));
  if (!answer.ok) return;
  assert.equal(asked.includes(Q('FNO', '2019-03-28')), true);
  assert.equal(asked.includes(Q('FNO')), false, 'never the chain unnarrowed');
  assert.deepEqual(answer.rows.map((row) => `${row.month} ${row.instrument} ${row.timeframe}`), [
    '2019-03 NSE-INDEX-ALPHA 1min', '2019-03 NSE-INDEX-ALPHA 1day',
    '2019-03 NSE-FNO-ALPHA-2019-03-28-1150000-CE 1s', '2019-03 NSE-FNO-ALPHA-2019-03-28-1150000-PE 1s',
    '2019-02 NSE-INDEX-ALPHA 1min']);
  assert.deepEqual(answer.scope.unread, []);
  assert.equal(answer.scope.expiry, '2019-03-28');
  assert.equal(answer.scope.expiries.length, 2, 'the other expiries stay offered');
  assert.equal(scopeText(answer.scope), 'INDEX 3 row(s), FNO at 2019-03-28 2 row(s), FNO 2 expiry(ies) offered');
});

test('a scope that cannot be trusted is refused with its reason, never a short census', async () => {
  const base = EVERY();
  /** @type {Array<[string, Record<string, any>, RegExp]>} */
  const cases = [
    ['the root is refused', { [ROOT]: [503, { error: 'census_unreadable', why: 'the manifest is torn' }] }, /answered HTTP 503: the manifest is torn/],
    ['the root is not a root', { [ROOT]: [200, { schema: 1, feed: 'example', segment: 'INDEX' }] }, /not the root of example/],
    ['a degraded census', { [ROOT]: [200, root({ degraded: 'recovered generation 20' })] }, /degraded \(recovered generation 20\)/],
    ['an empty feed', { [ROOT]: [200, root({ segments: [] })] }, /holds no census row/],
    ['a part that moved between the answers', { [Q('INDEX')]: [200, rowsBody('INDEX', 8, INDEX_ROWS)] }, /census moved between the answers \(INDEX: generation 7, then 8\)/],
    ['a census that moved between the answers', { [Q('INDEX')]: [200, rowsBody('INDEX', 7, INDEX_ROWS), { 'x-brutex-census-generation': '22' }] }, /census moved between the answers/],
    ['a listing that moved between the answers', { [L('FNO')]: [200, listingBody('FNO', 10, EXPIRIES)] }, /census moved between the answers \(FNO: generation 9, then 10\)/],
    ['counts that do not add up', { [Q('INDEX')]: [200, rowsBody('INDEX', 7, INDEX_ROWS, { held: { months: 4, rows: 118 } })] }, /counts 4 row\(s\) of 118 bars and carries 3 of 118/],
    ['a row of another instrument', { [Q('INDEX')]: [200, rowsBody('INDEX', 7, [...INDEX_ROWS, ['NSE-INDEX-ALPHABET', '2019-03', '1min', 1]])] }, /a row of NSE-INDEX-ALPHABET, which is not ALPHA/],
    ['a row with no denominator', { [Q('INDEX')]: [200, rowsBody('INDEX', 7, INDEX_ROWS, { denominators: [['2019-03', '1min', 65, 3]] })] }, /names no fullest row for 2019-03 at 1day/],
    ['a fullest row smaller than the row', { [Q('INDEX')]: [200, rowsBody('INDEX', 7, INDEX_ROWS, { denominators: [['2019-03', '1min', 59, 3], ['2019-03', '1day', 8, 3], ['2019-02', '1min', 60, 3]] })] }, /names no fullest row for 2019-03 at 1min that holds its own/],
    ['a malformed rows answer', { [Q('INDEX')]: [200, { schema: 1, feed: 'example', segment: 'INDEX', underlying: 'ALPHA', expiry: null, generation: 7 }] }, /ALPHA in INDEX: it does not count the rows it holds/],
    ['a torn tuples body', { [Q('INDEX')]: [200, rowsBody('INDEX', 7, INDEX_ROWS, { census: { schema: 1, encoding: 'census-tuples-v1', feed: 'example', rows: [[0]] } })] }, /not a JSON array or a valid complete census-tuples-v1/],
    ['another refusal', { [Q('INDEX')]: [409, { error: 'rows_incomplete', why: 'the rows this index can name do not sum' }] }, /ALPHA in INDEX: the rows this index can name do not sum/],
    ['a listing refused', { [L('FNO')]: [400, { error: 'ambiguous_underlying', why: 'ALPHA is held on NSE, BSE; name one with exchange=' }] }, /ALPHA in FNO: ALPHA is held on NSE, BSE/],
    ['a listing of another underlying', { [L('FNO')]: [200, listingBody('FNO', 9, EXPIRIES, { underlying: 'BETA' })] }, /ALPHA in FNO: it answers another underlying or expiry/],
    ['a listing whose expiries are not a list', { [L('FNO')]: [200, listingBody('FNO', 9, EXPIRIES, { expiries: null })] }, /its expiries is not a list/],
    ['a segment this page does not know, before any request', {}, /segment "index" is not one this page knows \(it reads spot, futures, options\); nothing was read for it/]
  ];
  for (const [name, over, why] of cases) {
    const { request, asked } = server({ ...base, ...over });
    const segments = name.startsWith('a segment this page does not know') ? ['index'] : null;
    const answer = await readScope(request, 'example', { underlying: 'ALPHA', segments }).catch((thrown) => ({ ok: false, reason: String(thrown?.message ?? thrown) }));
    assert.equal(answer.ok, false, name);
    if (!answer.ok) assert.match(answer.reason, why, name);
    if (segments) assert.deepEqual(asked, [], 'refused before any request left');
  }
});

test('an instrument a segment holds none of is the store\'s own answer, not a failure (R3-12)', async () => {
  // CHANGED (R3-12): this was a refusal, so that the whole census could be
  // read in its place. A rows answer lists every census row of its
  // underlying in its segment, so an empty one is the store holding none;
  // with no whole-census read behind it, it is shown as that.
  const { request } = server({ ...EVERY(), [Q('INDEX')]: [200, rowsBody('INDEX', 7, [])], [L('FNO')]: [200, listingBody('FNO', 9, [])] });
  const answer = await readScope(request, 'example', { underlying: 'ALPHA' });
  assert.equal(answer.ok, true, JSON.stringify(answer));
  if (!answer.ok) return;
  assert.deepEqual(answer.rows, []);
  assert.deepEqual(answer.scope.offered, [{ segment: 'FNO', generation: 9, expiries: 0, contracts: 0 }]);
  assert.equal(scopeText(answer.scope), 'FNO 0 expiry(ies) offered');
  const spot = await readScope(server({ [ROOT]: [200, root()], [Q('INDEX')]: [200, rowsBody('INDEX', 7, [])], [Q('CASH')]: [200, rowsBody('CASH', 5, [])] }).request,
    'example', { underlying: 'ALPHA', segments: ['spot'] });
  assert.equal(spot.ok && scopeText(spot.scope), 'no row held');
});

test('a rows answer is checked field by field', () => {
  const query = { feed: 'example', segment: 'INDEX', underlying: 'ALPHA' };
  assert.equal(rowsFault(rowsBody('INDEX', 7, INDEX_ROWS), query), null);
  assert.match(String(rowsFault(null, query)), /not a schema-1/);
  assert.match(String(rowsFault(rowsBody('CASH', 7, INDEX_ROWS), query)), /answers example\/CASH/);
  assert.match(String(rowsFault(rowsBody('INDEX', 7, INDEX_ROWS, { underlying: 'BETA' }), query)), /another underlying or expiry/);
  assert.match(String(rowsFault(rowsBody('INDEX', 7, INDEX_ROWS), { ...query, expiry: '2019-03-28' })), /another underlying or expiry/);
  assert.match(String(rowsFault(rowsBody('INDEX', -1, INDEX_ROWS), query)), /generation is not a whole number/);
  assert.match(String(rowsFault(rowsBody('INDEX', 7, INDEX_ROWS, { census: undefined }), query)), /carries no census rows/);
  assert.match(String(rowsFault(rowsBody('INDEX', 7, INDEX_ROWS, { denominators: null }), query)), /carries no denominators/);
  assert.match(String(rowsFault(rowsBody('INDEX', 7, INDEX_ROWS, { denominators: [['2019-03', '1min', 60, 0]] }), query)), /not \[month, rung, fullest, keys\]/);
});

test('the shared store reads a scope for every page that names an instrument, and never hands one to another page', () => {
  const first = { underlying: 'ALPHA', expiry: null, segments: ['spot'] };
  const cold = planRead({ feed: 'example', generation: 0, first, valueFeed: null, valueScoped: false });
  assert.deepEqual(cold, { scoped: true, wholeKey: 'example#0', key: 'example#0#scope:ALPHA::spot', clear: true });
  // A refresh of a scope re-reads the scope, and keeps the rows on screen.
  assert.deepEqual(planRead({ feed: 'example', generation: 1, first, valueFeed: 'example', valueScoped: true }),
    { scoped: true, wholeKey: 'example#1', key: 'example#1#scope:ALPHA::spot', clear: false });
  // ANOTHER PAGE (no instrument named) over a scoped value: the whole
  // census, a different request, and the instrument's rows are dropped
  // before it leaves.
  const other = planRead({ feed: 'example', generation: 0, first: null, valueFeed: 'example', valueScoped: true });
  assert.deepEqual(other, { scoped: false, wholeKey: 'example#0', key: 'example#0', clear: true });
  assert.notEqual(other.key, cold.key);
  // CHANGED (R3-12): a whole census another page read before is no reason
  // for /db to read the whole census: a page that names an instrument reads
  // its scope, and the whole value is dropped before the scope lands.
  assert.deepEqual(planRead({ feed: 'example', generation: 3, first, valueFeed: 'example', valueScoped: false }),
    { scoped: true, wholeKey: 'example#3', key: 'example#3#scope:ALPHA::spot', clear: true });
  // A feed change clears, scoped or not; an expiry and the segments are part of what was asked.
  assert.equal(planRead({ feed: 'other', generation: 0, first: null, valueFeed: 'example', valueScoped: false }).clear, true);
  assert.equal(planRead({ feed: 'example', generation: 0, first: { underlying: 'ALPHA', expiry: '2019-03-28', segments: ['options-cut'] }, valueFeed: null, valueScoped: false }).key,
    'example#0#scope:ALPHA:2019-03-28:options');
  assert.equal(planRead({ feed: 'example', generation: 0, first: { underlying: 'ALPHA' }, valueFeed: null, valueScoped: false }).key, 'example#0#scope:ALPHA::');
});

test('the banner says the page holds one instrument\'s rows, what the server would not list and what no segment shows', () => {
  const scope = { underlying: 'ALPHA', expiry: '2019-03-28', months: 3, views: [], segments: ['INDEX', 'CASH', 'FNO'],
    read: [{ segment: 'INDEX', expiry: null, months: 3, rows: 118, generation: 7 }, { segment: 'CASH', expiry: null, months: 0, rows: 0, generation: 5 }],
    unread: [{ segment: 'FNO', expiry: '2019-03-28', why: 'too many rows' }], offered: [{ segment: 'FNO', generation: 9, expiries: 2, contracts: 40 }],
    expiries: [], listed: [], unplaced: ['BOND'] };
  const lag = /** @type {any} */ ({ state: 'held', generation: 21, parts: null, faults: [], reason: null });
  const [read] = censusLines({ feed: 'example', at: Date.UTC(2026, 9, 11, 4, 30), lag, worker: null, head: null, how: null, scope });
  assert.equal(read.key, 'read');
  assert.match(read.text, /generation 21: only the census rows of ALPHA \(INDEX 3 row\(s\), FNO 2 expiry\(ies\) offered\) from \/census\/facets\.json, not the whole census\./);
  assert.match(read.text, /covers ALPHA alone; this page never reads the whole census\./);
  assert.match(read.text, /Not listed by the server: FNO at 2019-03-28 \(too many rows\)\./);
  assert.match(read.text, /Held and not read, because no segment of this page shows them: BOND\./);
  assert.doesNotMatch(read.text, /every list, total and offset on this page is that snapshot/);
  // The scope's own description is not a reason a whole census was read: no such line.
  const how = /** @type {any} */ ({ kind: 'scoped', since: null, rows: 3, added: 0, replaced: 0, why: 'ALPHA: INDEX 3 row(s)' });
  assert.deepEqual(censusLines({ feed: 'example', at: 1, lag, worker: null, head: null, how, scope }).map((line) => line.key), ['read']);
  const [reading] = censusLines({ feed: 'example', at: 1, lag, worker: null, head: null, how: null, scope, reading: true });
  assert.match(reading.text, /never reads the whole census, and the scope of the selection now on screen is being read\./);
  // CHANGED (R3-12): there is no "read the whole census before its first
  // paint" line any more; no /db read is whole. Another page's whole read
  // says it is that snapshot.
  const whole = censusLines({ feed: 'example', at: 1, lag, worker: null, head: null, how: null });
  assert.match(whole[0].text, /every list, total and offset on this page is that snapshot/);
  assert.equal(whole.some((line) => line.key === 'scope'), false);
});

test('the refresh log names a scoped read, and a first whole read and its reason', () => {
  const lag = /** @type {any} */ ({ generation: 21 });
  const scoped = refreshEntry({ feed: 'example', at: 1, lag, held: 3,
    how: { kind: 'scoped', since: null, rows: 3, added: 0, replaced: 0, why: 'ALPHA: INDEX 3 row(s)' } });
  assert.equal(scoped.kind, 'scoped');
  assert.match(refreshText(scoped), /example · read only the census rows of the instrument on screen from \/census\/facets\.json \(ALPHA: INDEX 3 row\(s\); 3 row\(s\)\), not the whole census; generation 21, 3 row\(s\) held\./);
  const fallen = refreshEntry({ feed: 'example', at: 1, lag, held: 9, first: true,
    how: { kind: 'whole', since: null, rows: 9, added: 0, replaced: 0, why: 'the instrument on screen could not be read alone from /census/facets.json (HTTP 503)' } });
  assert.equal(fallen.kind, 'first');
  assert.match(refreshText(fallen), /read the whole census \(the first read of this feed on this page; the instrument on screen could not be read alone from \/census\/facets\.json \(HTTP 503\)\)/);
  assert.deepEqual(searchRefreshes([scoped, fallen], 'scoped alpha').map((entry) => entry.kind), ['scoped']);
  assert.deepEqual(searchRefreshes([scoped, fallen], '503').map((entry) => entry.kind), ['first']);
});

/* ----------------------------------------------------------------------
   THE SECOND CHECK'S BLOCKING 2 AND ITS FOLLOW-UPS (verify-WP-56b-r1)
   ---------------------------------------------------------------------- */
import { readFileSync } from 'node:fs';
import { scopeNeed, scopeWanted, SCOPE_MS, FACETS_MS, SCOPE_RETRY_MAX_MS } from '../src/lib/store-census.js';

const SCOPE = { underlying: 'ALPHA', expiry: null, months: 3, views: ['spot'], segments: ['INDEX', 'CASH'],
  read: [{ segment: 'INDEX', expiry: null, months: 3, rows: 118, generation: 7 }], unread: [], offered: [], expiries: [], listed: [], unplaced: [] };
const ON_SCREEN = { scope: SCOPE, filter: 'ALPHA', kind: 'spot', expiry: '' };

test('a scope that does not hold the selection on screen is a reading state, never an empty selection', () => {
  // The selection is in the rows read: the scope answers the page.
  assert.equal(scopeNeed(ON_SCREEN), null);
  // NO SCOPE (another page's whole census, or nothing read yet): nothing is
  // missing from a scope.
  assert.equal(scopeNeed({ ...ON_SCREEN, scope: null }), null);
  // Another instrument than the one read: missing, before any row of it
  // could match, until its own scope lands.
  assert.deepEqual(scopeNeed({ ...ON_SCREEN, filter: 'BETA' }),
    { why: 'BETA was chosen and only ALPHA had been read', missing: true });
  // A SEGMENT THE SCOPE DID NOT READ (R3-12): missing, never drawn from the
  // spot rows. A key in the same family is the same read.
  assert.deepEqual(scopeNeed({ ...ON_SCREEN, kind: 'options' }),
    { why: 'options was chosen and only spot had been read for ALPHA', missing: true });
  assert.deepEqual(scopeNeed({ ...ON_SCREEN, kind: '' }),
    { why: 'every segment was chosen and only spot had been read for ALPHA', missing: true });
  const contract = { ...SCOPE, views: ['options'], segments: ['FNO'], offered: [{ segment: 'FNO', generation: 9, expiries: 2, contracts: 40 }] };
  assert.equal(scopeNeed({ scope: contract, filter: 'ALPHA', kind: 'options-cut', expiry: '' }), null);
  // An expiry the scope did not read is missing; one it read is not.
  assert.deepEqual(scopeNeed({ scope: contract, filter: 'ALPHA', kind: 'options', expiry: '2019-03-28' }),
    { why: 'the expiry 2019-03-28 was chosen and no chain had been read for ALPHA', missing: true });
  assert.equal(scopeNeed({ scope: { ...contract, expiry: '2019-03-28' }, filter: 'ALPHA', kind: 'options', expiry: '2019-03-28' }), null);
  // CHANGED (R3-12): an empty match WITHIN what was read is the store's own
  // answer, not a reason to read the whole census.
  assert.equal(scopeNeed({ ...ON_SCREEN, filter: '' }), null);
  // An expiry on a spot view, or where no contract segment was read, waits for nothing.
  assert.equal(scopeNeed({ ...ON_SCREEN, expiry: '2019-03-28' }), null);
  assert.equal(scopeNeed({ scope: { ...SCOPE, views: [] }, filter: 'ALPHA', kind: '', expiry: '2019-03-28' }), null);
});

test('the page draws its reading state, not the grid, while the selection is missing from the scope', () => {
  // SOURCE SHAPE: the three places that must agree. The banner derives
  // `scopeMissing` from `scopeNeed` (not from a copy of its rules); the
  // page's loading branch reads it; and the grid, drawn only in the branch
  // after, is the one place 'No instrument-month matches' is written.
  const banner = readFileSync(new URL('../src/routes/db/DbCensusBanner.svelte', import.meta.url), 'utf8');
  const page = readFileSync(new URL('../src/routes/db/+page.svelte', import.meta.url), 'utf8');
  const grid = readFileSync(new URL('../src/routes/db/DbBarsGrid.svelte', import.meta.url), 'utf8');
  assert.match(banner, /const need = \$derived\(scopeNeed\(\{ scope: shownScope,/);
  assert.match(banner, /const scopeMissing = \$derived\(shownScope !== null && need\?\.missing === true\);/);
  assert.match(banner, /const shownScope = \$derived\(store\.feed === feeds\.active \? store\.scope : null\);/);
  assert.match(banner, /scopeMissing: \{ get: \(\) => scopeMissing \}/);
  const loading = page.indexOf('{:else if (db.loading && (db.rows.length === 0 || db.preparing)) || db.scopeMissing}');
  const drawn = page.indexOf('<DbBarsGrid {db} />');
  assert.ok(loading > 0, 'the loading branch reads db.scopeMissing');
  assert.ok(drawn > loading, 'the grid is drawn in a later branch of the same chain');
  assert.equal(page.slice(loading, drawn).includes('\n  {:else}\n'), true, 'the grid is in the chain\'s last branch');
  assert.equal(grid.split('No instrument-month matches this query').length, 2, 'said in one place');
  for (const source of [banner, page]) assert.equal(source.includes('<h3>No instrument-month matches'), false);
});

test('a chain the server will not list at the expiry on screen is named unread, and no whole census is read for it', async () => {
  // CHANGED (R3-12): this read the whole census in the same flight. The
  // link names ALPHA 2019-03-28 and FNO holds more rows at that expiry than
  // one rows answer carries: the scope keeps what it read, names the chain
  // with the server's sentence, and nothing reads /store.json.
  const { request, asked } = server({ ...EVERY(),
    [Q('FNO', '2019-03-28')]: [400, { ...TOO_MANY, why: 'ALPHA in FNO holds 5000 census rows at this expiry, more than the 4096 one rows answer carries: read the census whole from /store.json', reached: 5000, counted: 'census rows' }]
  });
  const answer = await readScope(request, 'example', { underlying: 'ALPHA', expiry: '2019-03-28', segments: ['options'] });
  assert.equal(answer.ok, true, JSON.stringify(answer));
  if (!answer.ok) return;
  assert.deepEqual(answer.scope.unread.map((part) => `${part.segment} ${part.expiry}`), ['FNO 2019-03-28']);
  assert.match(answer.scope.unread[0].why, /^ALPHA in FNO holds 5000 census rows at this expiry/);
  assert.deepEqual(asked.sort(), [ROOT, L('FNO'), Q('FNO', '2019-03-28')].sort());
  assert.equal(asked.some((url) => url.includes('/store.json')), false);
});

test('a feed chosen later is read for the instrument on screen, not the page\'s opening one', () => {
  const page = { filter: 'BETA', kind: 'options', expiry: '2019-03-28', fallback: 'ALPHA' };
  assert.deepEqual(scopeWanted(page), { underlying: 'BETA', expiry: '2019-03-28', segments: ['options'] });
  assert.deepEqual(scopeWanted({ ...page, expiry: '' }), { underlying: 'BETA', expiry: null, segments: ['options'] });
  assert.deepEqual(scopeWanted({ ...page, expiry: undefined }), { underlying: 'BETA', expiry: null, segments: ['options'] });
  // No instrument chosen: the page's opening one. No segment chosen: every one.
  assert.deepEqual(scopeWanted({ ...page, filter: '', kind: '', expiry: null }), { underlying: 'ALPHA', expiry: null, segments: [] });
  // A spot view reads no chain, whatever expiry is left in the address.
  assert.deepEqual(scopeWanted({ ...page, kind: 'spot' }), { underlying: 'BETA', expiry: null, segments: ['spot'] });
  // The family is what is read: a cut or live key reads its family.
  assert.deepEqual(scopeWanted({ ...page, kind: 'futures-live' }).segments, ['futures']);
  const banner = readFileSync(new URL('../src/routes/db/DbCensusBanner.svelte', import.meta.url), 'utf8');
  assert.match(banner, /const want = db\.filter !== '' \? scopeWanted\(\{ filter: db\.filter, kind: db\.kind, expiry: db\.expiry, fallback: opening\(\) \}\) : untrack\(firstScope\);/);
  assert.match(banner, /if \(f !== firstFeed \|\| !scopeCovers\(first, want\)\) first = want;/);
});

test('a scope request that never answers is given up after its own short ceiling', async () => {
  assert.equal(SCOPE_MS, 3_000);
  assert.ok(SCOPE_MS < FACETS_MS);
  /** @type {number[]} */
  const ceilings = [];
  /** @type {any} */
  const request = async (/** @type {string} */ url, /** @type {any} */ options) => {
    ceilings.push(options.ms);
    // What `ask` does at its ceiling: an answer that is not ok, in words.
    return new Response(JSON.stringify({ error: 'timeout', why: `${url} did not answer within ${options.ms} ms` }), { status: 504, headers: HEADERS });
  };
  const answer = await readScope(request, 'example', { underlying: 'ALPHA' });
  assert.deepEqual(ceilings, [SCOPE_MS]);
  assert.equal(answer.ok, false);
  if (!answer.ok) assert.match(answer.reason, /did not answer within 3000 ms/);
});

test('a page restored from the back-forward cache re-reads its census; a fresh load does not read twice', () => {
  const page = readFileSync(new URL('../src/routes/db/+page.svelte', import.meta.url), 'utf8');
  assert.match(page, /onpageshow=\{\(event\) => \{ if \(event\.persisted\) void refreshStore\(\); \}\}/);
  // The reading state under a missing selection says why it is reading.
  const banner = readFileSync(new URL('../src/routes/db/DbCensusBanner.svelte', import.meta.url), 'utf8');
  assert.match(banner, /\{#if db\.scopeMissing\}[\s\S]{0,400}<p class="inline-note" role="status" data-scope-missing>Reading the census rows of the selection on screen/);
});

test('a saturated server is asked once more after the wait it names; a second 429 gives the scope up, said', async () => {
  // THE FIRST ANSWER OF EVERY URL IS 429 with retry-after 1; the second is
  // the census's. The scope is read, and says how many requests it asked twice.
  const answers = EVERY();
  /** @type {Map<string, number>} */
  const asked = new Map();
  /** @type {number[]} */
  const waits = [];
  const saturated = (/** @type {number} */ refusals) => /** @type {any} */ (async (/** @type {string} */ url) => {
    const n = (asked.get(url) ?? 0) + 1;
    asked.set(url, n);
    if (n <= refusals) return new Response(JSON.stringify({ error: 'census_read_unavailable', why: 'Saturated' }), { status: 429, headers: { ...HEADERS, 'retry-after': '1' } });
    return new Response(JSON.stringify(answers[url][1]), { status: answers[url][0], headers: HEADERS });
  });
  const pause = async (/** @type {number} */ ms) => { waits.push(ms); };
  const answer = await readScope(saturated(1), 'example', { underlying: 'ALPHA' }, { pause });
  assert.equal(answer.ok, true, JSON.stringify(answer));
  if (!answer.ok) return;
  assert.equal(answer.retried, 4);
  assert.deepEqual(waits, [1000, 1000, 1000, 1000], 'the wait the server named');
  assert.deepEqual([...asked.values()], [2, 2, 2, 2], 'each asked exactly twice, never a third time');
  assert.equal(answer.scope.months, 3);

  // TWICE SATURATED: no third request, no scope, and the reason is the server's.
  asked.clear();
  waits.length = 0;
  const refused = await readScope(saturated(2), 'example', { underlying: 'ALPHA' }, { pause });
  assert.equal(refused.ok, false);
  if (!refused.ok) assert.match(refused.reason, /answered HTTP 429: Saturated/);
  assert.deepEqual([...asked.values()], [2]);

  // A retry-after the server did not name, or a silly one, waits the cap and no longer.
  asked.clear();
  waits.length = 0;
  /** @type {any} */
  const unnamed = async (/** @type {string} */ url) => {
    const n = (asked.get(url) ?? 0) + 1;
    asked.set(url, n);
    if (n === 1 && url === ROOT) return new Response('{}', { status: 429, headers: { ...HEADERS, 'retry-after': '86400' } });
    return new Response(JSON.stringify(answers[url][1]), { status: answers[url][0], headers: HEADERS });
  };
  const late = await readScope(unnamed, 'example', { underlying: 'ALPHA' }, { pause });
  assert.equal(late.ok, true);
  assert.deepEqual(waits, [SCOPE_RETRY_MAX_MS]);
  assert.equal(SCOPE_RETRY_MAX_MS, 1_500);
  // An ordinary scope asked nothing twice.
  const calm = await readScope(server(EVERY()).request, 'example', { underlying: 'ALPHA' });
  assert.equal(calm.ok && calm.retried, 0);
});

/* ----------------------------------------------------------------------
   R3-12 (FM-621, provisional id): A SPOT VIEW NEVER ASKS FOR OPTION ROWS
   ---------------------------------------------------------------------- */

test('a_spot_view_never_requests_fno_facet_rows', async () => {
  // /db?feed=gdfl&seg=spot&sym=NIFTY asked /census/facets.json?...segment=FNO&
  // underlying=NIFTY&rows=census-tuples-v1 and drew 400 rows_too_many on
  // nearly every open (147,331 > 4,096 on :8080, 18 Oct). The fake server
  // answers FNO with that refusal, so asking it would show.
  const { request, asked } = server({ ...EVERY(), [Q('FNO')]: [400, TOO_MANY] });
  const wanted = scopeWanted({ filter: 'ALPHA', kind: 'spot', expiry: '', fallback: 'ALPHA' });
  const answer = await readScope(request, 'example', wanted);
  assert.equal(answer.ok, true, JSON.stringify(answer));
  if (!answer.ok) return;
  assert.deepEqual(asked.filter((url) => url.includes('segment=FNO')), [], 'no request names FNO at all');
  assert.deepEqual(asked.sort(), [ROOT, Q('CASH'), Q('INDEX')].sort());
  assert.deepEqual(answer.scope.unread, [], 'nothing unread: FNO was never asked');
  assert.deepEqual(answer.scope.segments, ['INDEX', 'CASH']);
  assert.deepEqual(answer.scope.offered, []);
  // THE CHIPS KEEP THE ROOT'S OWN COUNTS, the unread segment's included.
  assert.deepEqual(answer.scope.listed, [{ segment: 'INDEX', underlyings: 2, rows: { '1min': 60 }, read: true },
    { segment: 'CASH', underlyings: 1, rows: { '1min': 9 }, read: true }, { segment: 'FNO', underlyings: 1, rows: { '1s': 30 }, read: false }]);
  assert.equal(scopeNeed({ scope: answer.scope, filter: 'ALPHA', kind: 'spot', expiry: '' }), null);
});

test('an_options_view_with_no_expiry_offers_expiries_from_the_facets_answer', async () => {
  // The 12 Oct acceptance case: NIFTY options, no expiry. The server refuses
  // the unnarrowed chain (as on the real store, 146,978 > 4,096); the page
  // must not ask it, and must not read the whole census to list expiries.
  const many = Array.from({ length: 404 }, (_, n) => new Date(Date.UTC(2018, 8, 6) + n * 7 * 86_400_000).toISOString().slice(0, 10));
  const { request, asked } = server({ [ROOT]: [200, root()], [Q('FNO')]: [400, TOO_MANY], [L('FNO')]: [200, listingBody('FNO', 9, many)] });
  const answer = await readScope(request, 'example', scopeWanted({ filter: 'ALPHA', kind: 'options', expiry: '', fallback: 'ALPHA' }));
  assert.equal(answer.ok, true, JSON.stringify(answer));
  if (!answer.ok) return;
  assert.equal(asked.some((url) => url.includes('segment=FNO') && url.includes('rows=census-tuples-v1')), false, 'no FNO rows request without an expiry');
  assert.equal(asked.some((url) => url.includes('/store.json')), false, 'no whole-list download');
  assert.deepEqual(asked.sort(), [ROOT, L('FNO')].sort(), 'the options view asks FNO only, and only its listing');
  assert.deepEqual(answer.scope.expiries.map((row) => row.expiry), many, 'the expiry list is the facets answer\'s');
  assert.equal(answer.scope.expiries.length, 404);
  assert.deepEqual(answer.rows, [], 'no row of a contract is read before its expiry is chosen');
  assert.deepEqual(answer.scope.unread, []);
  assert.equal(scopeText(answer.scope), 'FNO 404 expiry(ies) offered');
});

test('a_segment_switch_reads_the_new_segments_scope', async () => {
  const { request, asked } = server(EVERY());
  const spotWanted = scopeWanted({ filter: 'ALPHA', kind: 'spot', expiry: '', fallback: 'ALPHA' });
  const spot = await readScope(request, 'example', spotWanted);
  assert.equal(spot.ok, true);
  if (!spot.ok) return;
  // THE SWITCH: Options is picked over a spot scope. The grid does not draw
  // Options from rows that never read it (`missing`), and the new view's
  // scope is a different request (one read of FNO's listing).
  const optionsWanted = scopeWanted({ filter: 'ALPHA', kind: 'options', expiry: '', fallback: 'ALPHA' });
  assert.deepEqual(scopeNeed({ scope: spot.scope, filter: 'ALPHA', kind: 'options', expiry: '' }),
    { why: 'options was chosen and only spot had been read for ALPHA', missing: true });
  assert.equal(scopeCovers(spotWanted, optionsWanted), false);
  const before = planRead({ feed: 'example', generation: 0, first: spotWanted, valueFeed: null, valueScoped: false });
  const after = planRead({ feed: 'example', generation: 0, first: optionsWanted, valueFeed: 'example', valueScoped: true });
  assert.notEqual(after.key, before.key);
  assert.equal(after.clear, false, 'the spot rows stay on screen under the reading state meanwhile');
  asked.length = 0;
  const options = await readScope(request, 'example', optionsWanted);
  assert.equal(options.ok, true);
  if (!options.ok) return;
  assert.deepEqual(asked.sort(), [ROOT, L('FNO')].sort(), 'one request for the new segment, after the root');
  assert.equal(scopeNeed({ scope: options.scope, filter: 'ALPHA', kind: 'options', expiry: '' }), null);
  // AND BACK: Spot over the options scope is missing again until read.
  assert.equal(scopeNeed({ scope: options.scope, filter: 'ALPHA', kind: 'spot', expiry: '' })?.missing, true);
  // A scope of every segment answers either without a request.
  assert.equal(scopeCovers({ underlying: 'ALPHA', expiry: null, segments: [] }, optionsWanted), true);
  assert.equal(scopeCovers({ underlying: 'ALPHA', expiry: null, segments: [] }, spotWanted), true);
  // SOURCE SHAPE: the banner reads the scope of the selection on screen, so
  // a segment switch reaches `syncStore` as a new scope, never `wholeStore`.
  const banner = readFileSync(new URL('../src/routes/db/DbCensusBanner.svelte', import.meta.url), 'utf8');
  assert.match(banner, /syncStore\(f, first\);/);
  assert.equal(banner.includes('wholeStore'), false, 'no /db view reads the whole census');
});

test('the store segments a view reads come from the root\'s own list, and an unknown view or segment is named', () => {
  const listed = root().segments;
  assert.deepEqual(viewSegments(listed, ['spot']), { rows: ['INDEX', 'CASH'], contract: [], unplaced: [] });
  assert.deepEqual(viewSegments(listed, ['options']), { rows: [], contract: ['FNO'], unplaced: [] });
  assert.deepEqual(viewSegments(listed, ['futures-cut']), { rows: [], contract: ['FNO'], unplaced: [] });
  assert.deepEqual(viewSegments(listed, []), { rows: ['INDEX', 'CASH'], contract: ['FNO'], unplaced: [] });
  assert.deepEqual(viewSegments(listed, null), viewSegments(listed, []));
  // OUT OF THE BOX: the server lists a segment this page has never seen. It
  // is neither read as spot nor as a contract: it is named, unread.
  const more = [...listed, { segment: 'BOND', generation: 1, underlyings: 4, rows: {}, first_ts: 1, last_ts: 2 }];
  assert.deepEqual(viewSegments(more, []), { rows: ['INDEX', 'CASH'], contract: ['FNO'], unplaced: ['BOND'] });
  // A feed with no spot segment: a spot view reads nothing, and says so as empty.
  assert.deepEqual(viewSegments([listed[2]], ['spot']), { rows: [], contract: [], unplaced: [] });
  // THE ADDRESS'S seg=index: refused by name, it places no segment.
  assert.deepEqual(unknownViews(['index']), ['index']);
  assert.deepEqual(unknownViews(['spot', 'options-live', 'futures']), []);
  assert.deepEqual(unknownViews(null), []);
  assert.equal(viewFamily('options-live'), 'options');
  assert.equal(viewFamily(undefined), '');
});

/* ----------------------------------------------------------------------
   R3-12 (NEW-07, RA-029): THE COVERAGE ROLL-UP COMES FROM THE SERVER
   ---------------------------------------------------------------------- */
import { readRollups, rollupUrl, rollupFault, scopeUnderlyings, scopeFailure } from '../src/lib/store-census.js';

const rollupBody = (/** @type {string} */ segment, /** @type {string | null} */ underlying, /** @type {any} */ over = {}) => ({ schema: 1, feed: 'example',
  segment, exchange: underlying ? 'NSE' : null, underlying, state: 'held', degraded: null, generation: 7, order: 'months ascending',
  basis: { days_owed: 'sessions in the calendar of record', days_held: 'first to last bar' },
  rungs: [{ timeframe: '1min', months_held: 2, instrument_months: underlying ? 2 : 40, rows: underlying ? 115 : 4000, first_ts: 1, last_ts: 2, days_held: 40, days_owed: 41 }],
  months: [{ month: '2019-02', timeframe: '1min' }, { month: '2019-03', timeframe: '1min' }], ...over });

test('the Coverage roll-up is the server\'s, per segment read: the instrument\'s and the whole segment\'s, a refusal kept by name', async () => {
  const scope = /** @type {any} */ ({ underlying: 'ALPHA', segments: ['INDEX', 'FNO'] });
  const R = (/** @type {string} */ segment, /** @type {string | null} */ underlying) => rollupUrl({ feed: 'example', segment, underlying });
  assert.equal(R('INDEX', 'ALPHA'), '/census/rollup.json?feed=example&segment=INDEX&underlying=ALPHA');
  assert.equal(R('INDEX', null), '/census/rollup.json?feed=example&segment=INDEX');
  const { request, asked } = server({
    [R('INDEX', 'ALPHA')]: [200, rollupBody('INDEX', 'ALPHA')],
    [R('INDEX', null)]: [200, rollupBody('INDEX', null)],
    [R('FNO', 'ALPHA')]: [200, rollupBody('FNO', 'ALPHA')],
    [R('FNO', null)]: [400, { error: 'rollup_too_many', why: 'FNO names more than 4096 (month, rung) cells' }]
  });
  const list = await readRollups(request, 'example', scope);
  assert.deepEqual(asked.sort(), [R('FNO', 'ALPHA'), R('FNO', null), R('INDEX', 'ALPHA'), R('INDEX', null)].sort());
  assert.equal(asked.some((url) => url.includes('/store.json')), false);
  assert.deepEqual(list.map((roll) => `${roll.segment}|${roll.underlying}|${roll.ok}|${roll.months}`),
    ['INDEX|ALPHA|true|2', 'INDEX|null|true|2', 'FNO|ALPHA|true|2', 'FNO|null|false|0']);
  assert.equal(list[1].rungs[0].instrument_months, 40);
  assert.match(String(list[3].why), /answered HTTP 400: rollup_too_many: FNO names more than 4096 \(month, rung\) cells/);
  // A body that is not the answer asked is refused, field by field.
  const query = { feed: 'example', segment: 'INDEX', underlying: 'ALPHA' };
  assert.equal(rollupFault(rollupBody('INDEX', 'ALPHA'), query), null);
  assert.match(String(rollupFault(null, query)), /not a schema-1 roll-up/);
  assert.match(String(rollupFault(rollupBody('CASH', 'ALPHA'), query)), /answers example\/CASH/);
  assert.match(String(rollupFault(rollupBody('INDEX', 'BETA'), query)), /another underlying/);
  assert.match(String(rollupFault(rollupBody('INDEX', 'ALPHA', { rungs: null }), query)), /not lists/);
  assert.match(String(rollupFault(rollupBody('INDEX', 'ALPHA', { rungs: [{ timeframe: '1min' }] }), query)), /does not count/);
  const torn = await readRollups(server({ [R('INDEX', 'ALPHA')]: [200, { schema: 1 }], [R('INDEX', null)]: [200, rollupBody('INDEX', null)] }).request,
    'example', /** @type {any} */ ({ underlying: 'ALPHA', segments: ['INDEX'] }));
  assert.match(String(torn[0].why), /^\/census\/rollup\.json: it answers undefined\/undefined/);
  // SOURCE SHAPE: the grid reads it while Coverage is open, and draws its refusals.
  const grid = readFileSync(new URL('../src/routes/db/DbCoverageGrid.svelte', import.meta.url), 'utf8');
  assert.match(grid, /if \(db\.view !== 'census' \|\| !scope \|\| !feed\) return;/);
  assert.match(grid, /const list = await readRollups\(ask, feed, scope\);/);
  assert.match(grid, /<span class="risk">not counted: \{roll\.why\}<\/span>/);
});

test('the instrument list is the server\'s underlyings of the segments read, and a list that failed is named', () => {
  const scope = /** @type {any} */ ({ underlying: 'ALPHA', segments: ['INDEX', 'CASH', 'FNO'], listed: root().segments });
  const under = (/** @type {string} */ symbol, /** @type {number} */ months, /** @type {any} */ more = {}) =>
    ({ exchange: 'NSE', symbol, months, expiries: 0, contracts: 0, rows: { '1min': 10 * months }, ...more });
  const { rows, why } = scopeUnderlyings(scope, [
    { ok: true, body: { underlyings: [under('ALPHA', 3), under('GAMMA', 1)] } },
    { ok: true, body: { underlyings: [under('BETA', 2)] } },
    { ok: true, body: { underlyings: [under('ALPHA', 2, { expiries: 5, contracts: 40, rows: { '1s': 7 } }), under('DELTA', 1, { expiries: 1, contracts: 2 })] } }
  ]);
  assert.equal(why, null);
  assert.deepEqual(rows.map((row) => `${row.key} ${row.spotMonths}/${row.contractMonths} ${row.expiries} ${row.bars}`),
    ['ALPHA 3/2 5 37', 'BETA 2/0 0 20', 'DELTA 0/1 1 10', 'GAMMA 1/0 0 10']);
  // A SEGMENT WHOSE LIST COULD NOT BE READ: the others are offered, and the gap is named.
  const partial = scopeUnderlyings(scope, [{ ok: true, body: { underlyings: [under('ALPHA', 1)] } }, { ok: false, reason: 'HTTP 503', body: null },
    { ok: true, body: {} }]);
  assert.deepEqual(partial.rows.map((row) => row.key), ['ALPHA']);
  assert.equal(partial.why, 'the server\'s instrument list could not be read whole (CASH: HTTP 503; FNO: its answer lists no underlyings)');
  // An empty list is an empty list, said as none.
  assert.deepEqual(scopeUnderlyings({ ...scope, segments: [] }, []), { rows: [], why: null });
  assert.equal(scopeFailure({ underlying: 'ALPHA', expiry: '2019-03-28', segments: ['options-live'] }, 'HTTP 503'),
    'the census rows of ALPHA (options at 2019-03-28) could not be read from /census/facets.json: HTTP 503. This page never reads the whole census in their place; Refresh asks again');
  assert.match(scopeFailure({ underlying: 'ALPHA' }, 'x'), /^the census rows of ALPHA \(every segment\) could not/);
});

/* ----------------------------------------------------------------------
   R3-12: EXTREME CASES (the operator's rule 4). One record, the most an
   answer carries, boundaries of the day and the year, out-of-order and
   torn answers, garbage views, a symbol no list names, and combinations.
   ---------------------------------------------------------------------- */
import { listedOf, segmentOf } from '../src/lib/db/selection-model.js';

test('extreme: one record everywhere: one spot row, one listed expiry', async () => {
  const one = /** @type {Array<[string, string, string, number]>} */ ([['NSE-INDEX-ALPHA', '2019-03', '1day', 1]]);
  const { request } = server({ ...EVERY(), [Q('INDEX')]: [200, rowsBody('INDEX', 7, one)], [L('FNO')]: [200, listingBody('FNO', 9, ['2019-03-28'])] });
  const answer = await readScope(request, 'example', { underlying: 'ALPHA' });
  assert.equal(answer.ok, true, JSON.stringify(answer));
  if (!answer.ok) return;
  assert.deepEqual(answer.rows.map((row) => `${row.instrument}|${row.rows}`), ['NSE-INDEX-ALPHA|1']);
  assert.deepEqual(answer.scope.expiries.map((row) => row.expiry), ['2019-03-28']);
  assert.equal(scopeText(answer.scope), 'INDEX 1 row(s), FNO 1 expiry(ies) offered');
});

test('extreme: the most one rows answer carries (4,096) is read whole; one past it is named, never read short', async () => {
  const full = /** @type {Array<[string, string, string, number]>} */ (Array.from({ length: 4096 }, (_, n) =>
    [`NSE-FNO-ALPHA-2019-03-28-${1_000_000 + n * 100}-${n % 2 ? 'PE' : 'CE'}`, '2019-03', '1s', 1]));
  const atCap = await readScope(server({ ...EVERY(), [Q('FNO', '2019-03-28')]: [200, rowsBody('FNO', 9, full, { expiry: '2019-03-28' })] }).request,
    'example', { underlying: 'ALPHA', expiry: '2019-03-28', segments: ['options'] });
  assert.equal(atCap.ok && atCap.rows.length, 4096);
  const past = await readScope(server({ ...EVERY(), [Q('FNO', '2019-03-28')]: [400, { ...TOO_MANY, reached: 4097 }] }).request,
    'example', { underlying: 'ALPHA', expiry: '2019-03-28', segments: ['options'] });
  assert.equal(past.ok && past.rows.length, 0);
  assert.equal(past.ok && past.scope.unread.length, 1);
});

test('extreme: the day and the year: an expiry today is live, at the archive end expired, one day past it cut, and a year turns in order', () => {
  const today = '2026-10-24';
  const archiveTo = '2026-09-24';
  const listing = ['2026-10-24', '2026-09-24', '2026-09-25', '2019-12-26', '2020-01-02', '2026-10-29'].map((expiry) => ({ expiry, strikes: 10, future: true }));
  const filed = (/** @type {any} */ e, /** @type {string} */ family) => segmentOf({ expiry: e.expiry, instrument: family === 'futures' ? 'x-FUT' : 'x-CE' }, today, archiveTo);
  assert.deepEqual(listedOf(listing, 'options-live', filed).map((e) => e.expiry), ['2026-10-24', '2026-10-29'], 'today is not settled');
  assert.deepEqual(listedOf(listing, 'options', filed).map((e) => e.expiry), ['2026-09-24', '2019-12-26', '2020-01-02'], 'the archive end is inside it');
  assert.deepEqual(listedOf(listing, 'options-cut', filed).map((e) => e.expiry), ['2026-09-25'], 'one day past the archive end is cut');
  assert.deepEqual(listedOf(listing, 'spot', filed), [], 'spot lists no expiry');
  assert.equal(listedOf(listing, '', filed).length, 6, 'every segment lists every one');
  // A futures key keeps only expiries with a future, an options key only those with strikes.
  const mixed = [{ expiry: '2019-03-28', strikes: 0, future: true }, { expiry: '2019-04-25', strikes: 4, future: false }];
  assert.deepEqual(listedOf(mixed, 'futures', filed).map((e) => e.expiry), ['2019-03-28']);
  assert.deepEqual(listedOf(mixed, 'options', filed).map((e) => e.expiry), ['2019-04-25']);
});

test('extreme: a listing out of order comes back in order; a torn body and a torn root are named, never thrown', async () => {
  const turned = ['2020-01-02', '2019-12-26', '2019-03-28'];
  const sorted = await readScope(server({ ...EVERY(), [L('FNO')]: [200, listingBody('FNO', 9, turned)] }).request, 'example', { underlying: 'ALPHA', segments: ['options'] });
  assert.deepEqual(sorted.ok && sorted.scope.expiries.map((row) => row.expiry), ['2019-03-28', '2019-12-26', '2020-01-02']);
  /** @type {any} */
  const tornAt = (/** @type {string} */ torn) => async (/** @type {string} */ url) => url === torn
    ? new Response('{"schema":1,"feed":"exa', { status: 200, headers: HEADERS })
    : server(EVERY()).request(url, { cache: 'no-store' });
  for (const [torn, why] of /** @type {Array<[string, RegExp]>} */ ([[ROOT, /^\/census\/facets\.json\?feed=example: its body is not JSON$/],
    [Q('INDEX'), /^ALPHA in INDEX: its body is not JSON$/], [L('FNO'), /^ALPHA in FNO: its body is not JSON$/]])) {
    const answer = await readScope(tornAt(torn), 'example', { underlying: 'ALPHA' });
    assert.equal(answer.ok, false, torn);
    if (!answer.ok) assert.match(answer.reason, why);
  }
});

test('extreme: garbage views are refused before any request, mixed with good ones or not', async () => {
  for (const segments of [['options', 'index'], [''], ['SPOT'], ['spot-cut-live'], ['__proto__']]) {
    const { request, asked } = server(EVERY());
    const answer = await readScope(request, 'example', { underlying: 'ALPHA', segments });
    assert.equal(answer.ok, false, JSON.stringify(segments));
    if (!answer.ok) assert.match(answer.reason, /is not one this page knows \(it reads spot, futures, options\); nothing was read for it/);
    assert.deepEqual(asked, [], JSON.stringify(segments));
  }
});

test('extreme, out of the box: a symbol with a hyphen and an ampersand is asked encoded, and a longer name sharing its prefix is not its row', async () => {
  // NSE lists BAJAJ-AUTO and M&M: a name no list in this repository spells.
  const ask = (/** @type {string} */ underlying) => rowsUrl({ feed: 'example', segment: 'CASH', underlying });
  assert.equal(ask('M&M'), '/census/facets.json?feed=example&segment=CASH&underlying=M%26M&rows=census-tuples-v1');
  const rows = /** @type {Array<[string, string, string, number]>} */ ([['NSE-CASH-BAJAJ-AUTO', '2019-03', '1min', 7]]);
  const body = { ...rowsBody('CASH', 5, rows), underlying: 'BAJAJ' };
  const answers = { [ROOT]: /** @type {[number, any]} */ ([200, root()]), [rowsUrl({ feed: 'example', segment: 'INDEX', underlying: 'BAJAJ' })]: /** @type {[number, any]} */ ([200, { ...rowsBody('INDEX', 7, []), underlying: 'BAJAJ' }]),
    [rowsUrl({ feed: 'example', segment: 'CASH', underlying: 'BAJAJ' })]: /** @type {[number, any]} */ ([200, body]) };
  const answer = await readScope(server(answers).request, 'example', { underlying: 'BAJAJ', segments: ['spot'] });
  assert.equal(answer.ok, false);
  if (!answer.ok) assert.equal(answer.reason, 'CASH answered a row of NSE-CASH-BAJAJ-AUTO, which is not BAJAJ');
  // A chain's rows carry the expiry asked, or they are refused.
  const chain = await readScope(server({ ...EVERY(), [Q('FNO', '2019-03-28')]: [200, rowsBody('FNO', 9, [['NSE-FNO-ALPHA-2019-04-25-1150000-CE', '2019-04', '1s', 3]], { expiry: '2019-03-28' })] }).request,
    'example', { underlying: 'ALPHA', expiry: '2019-03-28', segments: ['options'] });
  assert.equal(!chain.ok && chain.reason, 'FNO answered a row of NSE-FNO-ALPHA-2019-04-25-1150000-CE, which is not ALPHA at 2019-03-28');
});

test('extreme, combined: a saturated server AND a census that moves between the answers: asked twice at most, then refused as moved', async () => {
  /** @type {Map<string, number>} */
  const asked = new Map();
  const answers = EVERY();
  /** @type {any} */
  const request = async (/** @type {string} */ url) => {
    const n = (asked.get(url) ?? 0) + 1;
    asked.set(url, n);
    if (n === 1) return new Response('{}', { status: 429, headers: { ...HEADERS, 'retry-after': '1' } });
    // THE SECOND ANSWER OF THE LISTING IS A NEWER CENSUS.
    if (url === L('FNO')) return new Response(JSON.stringify(listingBody('FNO', 9, EXPIRIES)), { status: 200, headers: { ...HEADERS, 'x-brutex-census-generation': '22' } });
    return new Response(JSON.stringify(answers[url][1]), { status: answers[url][0], headers: HEADERS });
  };
  const answer = await readScope(request, 'example', { underlying: 'ALPHA' }, { pause: async () => {} });
  assert.equal(answer.ok, false);
  if (!answer.ok) assert.match(answer.reason, /census moved between the answers \(the feed's generation 21, then 22, while FNO was read\)/);
  assert.equal([...asked.values()].every((n) => n <= 2), true, 'never a third request');
});

test('extreme, combined: ten times the expiries AND the chosen chain past the cap AND every segment: offered, named, nothing whole', async () => {
  const many = Array.from({ length: 4040 }, (_, n) => new Date(Date.UTC(2018, 8, 6) + n * 7 * 86_400_000).toISOString().slice(0, 10));
  const chosen = many[2020];
  const { request, asked } = server({ ...EVERY(), [L('FNO')]: [200, listingBody('FNO', 9, many)], [Q('FNO', chosen)]: [400, { ...TOO_MANY, reached: 9000 }] });
  const answer = await readScope(request, 'example', { underlying: 'ALPHA', expiry: chosen, segments: [] });
  assert.equal(answer.ok, true, JSON.stringify(answer));
  if (!answer.ok) return;
  assert.equal(answer.scope.expiries.length, 4040);
  assert.deepEqual(answer.scope.unread.map((part) => `${part.segment} ${part.expiry}`), [`FNO ${chosen}`]);
  assert.equal(answer.rows.length, 3, 'the spot rows stay');
  assert.equal(asked.some((url) => url.includes('/store.json')), false);
  // The page then refuses that chain by name rather than drawing it empty.
  assert.equal(scopeNeed({ scope: answer.scope, filter: 'ALPHA', kind: 'options', expiry: chosen }), null, 'not missing: read and refused');
});

/* ----------------------------------------------------------------------
   R3-12, DB-B47 (the page half): AN INDEX'S MISSING NEWEST MONTHS ARE HOLES
   ---------------------------------------------------------------------- */
import { owedTo } from '../src/lib/db/selection-model.js';
import { owedKind, absentMonths } from '../src/lib/completeness.js';

test('an_index_owes_its_months_to_the_feeds_span_end', () => {
  const archived = { archive: { from: '2018-09-03', to: '2026-09-24' } };
  const windowed = { window: { from: '2018-01-01', to: '2026-09-30' } };
  // GDFL INDIAVIX 1s held to 2025-10 inside an archive to 2026-09: eleven holes, never "outside life".
  assert.equal(owedTo('2025-10', true, archived), '2026-09');
  assert.equal(absentMonths('2025-08', owedTo('2025-10', true, archived), new Set(['2025-08', '2025-09', '2025-10'])).length, 11);
  assert.equal(owedTo('2026-09', true, windowed), '2026-09', 'the window when there is no archive');
  assert.equal(owedTo('2025-10', true, windowed), '2026-09', 'a window feed owes to its window end');
  assert.equal(owedTo('2025-10', true, { ...archived, window: { to: '2026-10-31' } }), '2026-10', 'the wider of the archive and the window, as the route\'s served_end');
  assert.equal(owedTo('2025-10', true, { archive: { to: '2026-11-02' }, window: { to: '2026-09-30' } }), '2026-11', 'the wider either way');
  assert.equal(owedTo('2025-10', true, { archive: { to: 20260924 }, window: null }), '2025-10', 'a span that is not text claims nothing');
  assert.equal(owedTo('2026-10', true, archived), '2026-10', 'never moved back');
  assert.equal(owedTo('2023-07', false, archived), '2023-07', 'a share that stopped (HDFC) keeps its held life');
  assert.equal(owedTo('2025-10', true, {}), '2025-10', 'no span, no claim');
  assert.equal(owedTo('2025-10', true, { archive: { to: 'garbage' } }), '2025-10', 'a span that is not a day claims nothing');
  // WHICH SERIES NEVER EXPIRES is the owed-kind rule's, not a word typed in the grid.
  assert.equal(owedKind('INDEX', false, null).owed, 'index');
  assert.notEqual(owedKind('CASH', false, null).owed, 'index');
  const grid = readFileSync(new URL('../src/routes/db/DbCoverageGrid.svelte', import.meta.url), 'utf8');
  assert.match(grid, /last = owedTo\(last, s\.like\?\.expiry == null && owedKind\(segmentOfKey\(instrument\) \?\? '', false, null\)\.owed === 'index', db\.activeFeed\);/);
});

/* ----------------------------------------------------------------------
   R3-12 repair 1 (the second agent's blocking 2): THE IMPORT JOURNAL IS
   READ OVER ONE DAY. To stop typing a probe day (RA-258) the Coverage
   drawer of an imported contract asked `/imports.json?feed=<feed>` with no
   range; the route then walks every journal row of the feed's span, one
   evidence probe each (`crates/api/src/imports.rs` `body()`: no asked ends
   is the journal's first and last day), for a `kinds` summary that is the
   same for any range. Real store, 26 Oct 2026: 11,508 rows and 253,074 B
   in 12.4 ms a contract opened, against 1 row and 9,997 B in 0.9 ms for a
   one-day read, and the whole walk grows with every import day.
   ---------------------------------------------------------------------- */
import { importDayUrl } from '../src/lib/db/selection-model.js';
import { DB_PAGE_FILES, dbSource } from './database-page-fixture.js';

test('a_contract_reads_the_import_journal_over_one_day', () => {
  // THE ROUTE'S RANGE RULE, as `body()` applies it: the asked ends, else the
  // journal's own first and last day; a day outside the journal walks nothing.
  const journal = { first: '2018-09-03', last: '2026-09-30' };
  const dayOf = (/** @type {string} */ iso) => Date.UTC(Number(iso.slice(0, 4)), Number(iso.slice(5, 7)) - 1, Number(iso.slice(8, 10))) / 86_400_000;
  const walked = (/** @type {string} */ url) => {
    const asked = new URL(url, 'http://route.invalid').searchParams;
    const from = Math.max(dayOf(asked.get('from') ?? journal.first), dayOf(journal.first));
    const to = Math.min(dayOf(asked.get('to') ?? journal.last), dayOf(journal.last));
    return Math.max(0, to - from + 1);
  };
  assert.equal(walked('/imports.json?feed=example'), dayOf(journal.last) - dayOf(journal.first) + 1, 'no range is the whole journal');
  // BOTH ENDS, ONE DAY, THE CONTRACT'S OWN EXPIRY.
  const url = importDayUrl('example', '2019-03-28');
  assert.equal(url, '/imports.json?feed=example&from=2019-03-28&to=2019-03-28');
  assert.equal(walked(String(url)), 1);
  // An expiry past the journal's last day (a contract the import has not
  // reached) walks nothing and still answers the kinds; the feed is encoded.
  assert.equal(walked(String(importDayUrl('example', '2026-12-29'))), 0);
  assert.equal(importDayUrl('a b&c', '2019-03-28'), '/imports.json?feed=a%20b%26c&from=2019-03-28&to=2019-03-28');
  // NOT A DAY: nothing is asked (never the unbounded request in its place),
  // and a value that is not text is not read as a day even when it prints as one.
  for (const bad of [null, undefined, '', '2019-03', '2019-3-28', '28-03-2019', 20190328, '2019-03-28T09:15', ' 2019-03-28', {},
    { toString: () => '2019-03-28' }]) {
    assert.equal(importDayUrl('example', bad), null, String(bad));
  }
  // SOURCE SHAPE: the drawer asks through it, and no /db page file builds an
  // `/imports.json` request of its own (comments are not requests).
  const grid = dbSource('../src/routes/db/DbCoverageGrid.svelte');
  assert.match(grid, /const url = importDayUrl\(feed, o\.expiry\);\n\s*if \(url === null\) \{\n\s*importedTo = '';\n\s*return;\n\s*\}/);
  assert.match(grid, /const r = await ask\(url, \{ cache: 'no-store', signal: ticket\.signal \}\);/);
  const code = (/** @type {string} */ text) => text.replace(/<!--[\s\S]*?-->/g, '').replace(/\/\*[\s\S]*?\*\//g, '').replace(/(^|[^:'"`\\])\/\/[^\n]*/g, '$1');
  const asks = DB_PAGE_FILES.flatMap((file) => code(dbSource(file)).split('\n').filter((line) => line.includes('/imports.json')).map((line) => `${file}: ${line.trim()}`));
  assert.deepEqual(asks, ['../src/lib/db/selection-model.js: return `/imports.json?feed=${encodeURIComponent(feed)}&from=${day}&to=${day}`;']);
});

/* ----------------------------------------------------------------------
   R3-12 merge-back (26 Oct 2026): A SCOPE THAT READ NO ROW DRAWS THE BOARD.
   On the real store the 12 Oct acceptance case (NIFTY options, no expiry)
   read the expiry listing and no contract row, and the page's empty-feed
   branch drew "Nothing stored for Global Datafeeds · This feed has never
   landed a bar in this store" over 422 listed expiries; the board's rung,
   strike, side and grid sentences then said "nothing has been read for
   Global Datafeeds yet ... an unread store". The browser proof is the
   Mac's (M6: the Expiry picker offers 405 expired-option expiries).
   ---------------------------------------------------------------------- */
test('a_scope_with_no_row_draws_the_board_and_says_what_was_read', () => {
  const read = (/** @type {string} */ path) => readFileSync(new URL(`../src/${path}`, import.meta.url), 'utf8');
  const page = read('routes/db/+page.svelte');
  assert.equal(page.split('{:else if db.rows.length === 0 && !db.censusScope}').length, 2, 'the empty-feed branch is guarded by the scope');
  assert.equal(page.includes('{:else if db.rows.length === 0}\n'), false, 'no unguarded empty-feed branch is left');
  const selection = read('lib/db/selection.svelte.js');
  assert.match(selection, /const scopeEmptyWhy = \$derived\.by\(\(\) => \{/);
  assert.equal(selection.split('if (db.deco.length === 0 && scopeEmptyWhy) return scopeEmptyWhy;').length, 3, 'the rung and side refusals say what was read');
  assert.match(selection, /if \(c\.size === 0\) for \(const e of listedExpiries\) for \(const tf of Object\.keys\(e\.rows \?\? \{\}\)\)/);
  assert.match(selection, /scopeEmptyWhy: \{ get: \(\) => scopeEmptyWhy \}/);
  assert.match(read('routes/db/DbMoneyness.svelte'), /if \(db\.deco\.length === 0 && db\.scopeEmptyWhy\) return db\.scopeEmptyWhy;/);
  const grid = read('routes/db/DbBarsGrid.svelte');
  assert.match(grid, /\? db\.deco\.length === 0 && db\.scopeEmptyWhy/);
  assert.match(grid, /: `\$\{db\.feedName\} holds \$\{fmt\(db\.deco\.length\)\} instrument-month\(s\) in total\.`\}/, 'the whole-store sentence is kept for a page with no scope');
  const controls = read('routes/db/DbControls.svelte');
  assert.match(controls, /db\.contractHere === 0 && db\.expiryListed\.size > 0/);
  assert.match(controls, /detail: db\.censusScope\n\s+\? `\$\{fmt\(r\.months\)\} month\(s\) in \$\{db\.censusScope\.segments\.join\(', '\) \|\| 'no segment read'\}`/);
});

/* ----------------------------------------------------------------------
   THE EMPTY SCOPE'S SENTENCE, RUN (R3-12 merge-back, 26 Oct 2026). The
   source-shape test above holds where `scopeEmptyWhy` is read; the hand
   mutation probe found that it could not see the sentence itself turned off
   (`scope !== null` for `scope === null` left every test green). This runs
   the declaration out of `selection.svelte.js`, compiled by Svelte, over an
   invented scope: no scope says nothing, a contract view with listed
   expiries says its chain waits for an expiry, and a scope that read no row
   says it is the store's answer, never an unread store.
   ---------------------------------------------------------------------- */
import { parse as parseSvelte, compileModule as compileSvelte } from 'svelte/compiler';

/** The empty-scope sentence over `scope` and `listed`, from the page's own declaration. @param {any} scope @param {any[]} listed */
async function emptyScopeWhy(scope, listed) {
  const state = readFileSync(new URL('../src/lib/db/selection.svelte.js', import.meta.url), 'utf8');
  const wrapped = '<script module>' + state + '</script>';
  const program = /** @type {any} */ (parseSvelte(wrapped)).module?.content;
  const factory = program.body.find((/** @type {any} */ n) => n.type === 'ExportNamedDeclaration' && n.declaration?.id?.name === 'selectionState')?.declaration;
  const decl = (factory?.body?.body ?? []).find((/** @type {any} */ n) => n.type === 'VariableDeclaration' && n.declarations[0]?.id?.name === 'scopeEmptyWhy');
  if (!decl) throw new Error('The empty-scope seam changed: no scopeEmptyWhy in selectionState.');
  const lib = (/** @type {string} */ p) => new URL(`../src/lib/${p}`, import.meta.url).href;
  const source = `import { fmt } from '${lib('db/notes-labels.js')}';
export function mount(scope, listed) {
  const db = $state({ censusScope: scope });
  const listedExpiries = $state(listed);
  /** @type {any} */
  let read = null;
  const stop = $effect.root(() => {
    ${wrapped.slice(decl.start, decl.end)}
    read = () => scopeEmptyWhy;
  });
  return { read, stop };
}`;
  const { js } = compileSvelte(source, { filename: 'empty-scope.svelte.js', generate: 'client' });
  const client = import.meta.resolve('svelte').replace(/index-server\.js$/, 'index-client.js');
  const code = js.code.replace(/from (["'])([^"']+)\1/g, (_m, q, p) => `from ${q}${p === 'svelte' ? client : import.meta.resolve(p)}${q}`);
  const { mount } = await import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}#${Math.random()}`);
  const page = mount(scope, listed);
  try { return page.read(); } finally { page.stop(); }
}

test('the_empty_scope_sentence_says_what_was_read', async () => {
  assert.equal(await emptyScopeWhy(null, []), null, 'no scope on screen: no sentence, the whole-store words stand');
  const listed = [{ expiry: '2031-01-28' }, { expiry: '2031-02-25' }, { expiry: '2031-03-25' }];
  assert.equal(await emptyScopeWhy({ underlying: 'SAMPLE', views: ['options'] }, listed),
    'no contract row of SAMPLE is read until its expiry is chosen: 3 expiries are listed by the server (/census/facets.json) for options');
  const none = await emptyScopeWhy({ underlying: 'SAMPLE', views: [] }, []);
  assert.equal(none, 'the census rows of SAMPLE read for every segment from /census/facets.json hold no row here — the store\'s own answer for this selection, not an unread store');
  assert.doesNotMatch(String(none), /never landed|unread store, not a store without/, 'never the unread-store sentence');
});
