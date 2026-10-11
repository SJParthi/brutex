// WP-56b: the first paint of /db from ONE INSTRUMENT'S census rows
// (`/census/facets.json?...&rows=census-tuples-v1`), never the whole census:
// what is read, what is refused and said, and which read the shared store
// makes for whom. Every number below is invented.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readScope, rowsFault, rowsUrl, planRead, scopeText, censusLines, refreshEntry, refreshText, searchRefreshes } from '../src/lib/store-census.js';
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
const INDEX_ROWS = /** @type {Array<[string, string, string, number]>} */ ([
  ['NSE-INDEX-ALPHA', '2019-03', '1min', 60], ['NSE-INDEX-ALPHA', '2019-03', '1day', 3], ['NSE-INDEX-ALPHA', '2019-02', '1min', 55]]);
const TOO_MANY = { error: 'rows_too_many', why: 'ALPHA in FNO names more than 4096 census rows: narrow it with expiry=, or read the census whole from /store.json', cap: 4096, reached: 4097 };

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
const ROOT = '/census/facets.json?feed=example';

test('a first paint reads the root and one instrument\'s rows per segment, never /store.json', async () => {
  const { request, asked } = server({
    [ROOT]: [200, root()],
    [Q('INDEX')]: [200, rowsBody('INDEX', 7, INDEX_ROWS)],
    [Q('CASH')]: [200, rowsBody('CASH', 5, [])],
    [Q('FNO')]: [400, TOO_MANY]
  });
  const answer = await readScope(request, 'example', { underlying: 'ALPHA' }, { now: () => 1234 });
  assert.equal(answer.ok, true, JSON.stringify(answer));
  if (!answer.ok) return;
  assert.deepEqual(asked.sort(), [ROOT, Q('CASH'), Q('FNO'), Q('INDEX')].sort());
  assert.equal(asked.some((url) => url.includes('/store.json')), false);
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
  assert.deepEqual(answer.scope, { underlying: 'ALPHA', expiry: null, months: 3,
    read: [{ segment: 'INDEX', expiry: null, months: 3, rows: 118, generation: 7 }, { segment: 'CASH', expiry: null, months: 0, rows: 0, generation: 5 }],
    unread: [{ segment: 'FNO', why: TOO_MANY.why }] });
  assert.equal(scopeText(answer.scope), 'INDEX 3 row(s)');
});

test('a chain too long to list whole is asked at the expiry on screen, and merged in the census\'s order', async () => {
  const chain = /** @type {Array<[string, string, string, number]>} */ ([
    ['NSE-FNO-ALPHA-2019-03-28-1150000-CE', '2019-03', '1s', 10], ['NSE-FNO-ALPHA-2019-03-28-1150000-PE', '2019-03', '1s', 20]]);
  const { request, asked } = server({
    [ROOT]: [200, root()],
    [Q('INDEX')]: [200, rowsBody('INDEX', 7, INDEX_ROWS)],
    [Q('CASH')]: [200, rowsBody('CASH', 5, [])],
    [Q('FNO')]: [400, TOO_MANY],
    [Q('FNO', '2019-03-28')]: [200, rowsBody('FNO', 9, chain, { expiry: '2019-03-28' })]
  });
  const answer = await readScope(request, 'example', { underlying: 'ALPHA', expiry: '2019-03-28' });
  assert.equal(answer.ok, true, JSON.stringify(answer));
  if (!answer.ok) return;
  assert.equal(asked.includes(Q('FNO', '2019-03-28')), true);
  assert.deepEqual(answer.rows.map((row) => `${row.month} ${row.instrument} ${row.timeframe}`), [
    '2019-03 NSE-INDEX-ALPHA 1min', '2019-03 NSE-INDEX-ALPHA 1day',
    '2019-03 NSE-FNO-ALPHA-2019-03-28-1150000-CE 1s', '2019-03 NSE-FNO-ALPHA-2019-03-28-1150000-PE 1s',
    '2019-02 NSE-INDEX-ALPHA 1min']);
  assert.deepEqual(answer.scope.unread, []);
  assert.equal(scopeText(answer.scope), 'INDEX 3 row(s), FNO at 2019-03-28 2 row(s)');
});

test('a scope that cannot be trusted is refused with its reason, never a short census', async () => {
  const base = { [ROOT]: [200, root()], [Q('INDEX')]: [200, rowsBody('INDEX', 7, INDEX_ROWS)], [Q('CASH')]: [200, rowsBody('CASH', 5, [])], [Q('FNO')]: [400, TOO_MANY] };
  /** @type {Array<[string, Record<string, any>, RegExp]>} */
  const cases = [
    ['the root is refused', { [ROOT]: [503, { error: 'census_unreadable', why: 'the manifest is torn' }] }, /answered HTTP 503: the manifest is torn/],
    ['the root is not a root', { [ROOT]: [200, { schema: 1, feed: 'example', segment: 'INDEX' }] }, /not the root of example/],
    ['a degraded census', { [ROOT]: [200, root({ degraded: 'recovered generation 20' })] }, /degraded \(recovered generation 20\)/],
    ['an empty feed', { [ROOT]: [200, root({ segments: [] })] }, /holds no census row/],
    ['a part that moved between the answers', { [Q('INDEX')]: [200, rowsBody('INDEX', 8, INDEX_ROWS)] }, /census moved between the answers \(INDEX: generation 7, then 8\)/],
    ['a census that moved between the answers', { [Q('INDEX')]: [200, rowsBody('INDEX', 7, INDEX_ROWS), { 'x-brutex-census-generation': '22' }] }, /census moved between the answers/],
    ['counts that do not add up', { [Q('INDEX')]: [200, rowsBody('INDEX', 7, INDEX_ROWS, { held: { months: 4, rows: 118 } })] }, /counts 4 row\(s\) of 118 bars and carries 3 of 118/],
    ['a row of another instrument', { [Q('INDEX')]: [200, rowsBody('INDEX', 7, [...INDEX_ROWS, ['NSE-INDEX-ALPHABET', '2019-03', '1min', 1]])] }, /a row of NSE-INDEX-ALPHABET, which is not ALPHA/],
    ['a row with no denominator', { [Q('INDEX')]: [200, rowsBody('INDEX', 7, INDEX_ROWS, { denominators: [['2019-03', '1min', 65, 3]] })] }, /names no fullest row for 2019-03 at 1day/],
    ['a fullest row smaller than the row', { [Q('INDEX')]: [200, rowsBody('INDEX', 7, INDEX_ROWS, { denominators: [['2019-03', '1min', 59, 3], ['2019-03', '1day', 8, 3], ['2019-02', '1min', 60, 3]] })] }, /names no fullest row for 2019-03 at 1min that holds its own/],
    ['a malformed rows answer', { [Q('INDEX')]: [200, { schema: 1, feed: 'example', segment: 'INDEX', underlying: 'ALPHA', expiry: null, generation: 7 }] }, /ALPHA in INDEX: it does not count the rows it holds/],
    ['a torn tuples body', { [Q('INDEX')]: [200, rowsBody('INDEX', 7, INDEX_ROWS, { census: { schema: 1, encoding: 'census-tuples-v1', feed: 'example', rows: [[0]] } })] }, /not a JSON array or a valid complete census-tuples-v1/],
    ['another refusal', { [Q('INDEX')]: [409, { error: 'rows_incomplete', why: 'the rows this index can name do not sum' }] }, /ALPHA in INDEX: the rows this index can name do not sum/],
    ['an instrument the feed does not hold', { [Q('INDEX')]: [200, rowsBody('INDEX', 7, [])], [Q('FNO')]: [200, rowsBody('FNO', 9, [])] }, /example holds no census row of ALPHA/],
    ['only a chain too long, and no expiry on screen', { [Q('INDEX')]: [200, rowsBody('INDEX', 7, [])] }, /ALPHA's rows could not be listed \(FNO: ALPHA in FNO names more than 4096/]
  ];
  for (const [name, over, why] of cases) {
    const { request } = server({ ...base, ...over });
    const answer = await readScope(request, 'example', { underlying: 'ALPHA' }).catch((thrown) => ({ ok: false, reason: String(thrown?.message ?? thrown) }));
    assert.equal(answer.ok, false, name);
    if (!answer.ok) assert.match(answer.reason, why, name);
  }
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

test('the shared store reads a scope only for a cold page that named an instrument, and never hands one to another page', () => {
  const first = { underlying: 'ALPHA', expiry: null };
  const cold = planRead({ feed: 'example', generation: 0, first, valueFeed: null, valueScoped: false, wholeAsked: false });
  assert.deepEqual(cold, { scoped: true, wholeKey: 'example#0', key: 'example#0#scope:ALPHA:', clear: true });
  // A refresh of a scope re-reads the scope, and keeps the rows on screen.
  assert.deepEqual(planRead({ feed: 'example', generation: 1, first, valueFeed: 'example', valueScoped: true, wholeAsked: false }),
    { scoped: true, wholeKey: 'example#1', key: 'example#1#scope:ALPHA:', clear: false });
  // ANOTHER PAGE (no instrument named) over a scoped value: the whole
  // census, a different request, and the instrument's rows are dropped
  // before it leaves.
  const other = planRead({ feed: 'example', generation: 0, first: null, valueFeed: 'example', valueScoped: true, wholeAsked: false });
  assert.deepEqual(other, { scoped: false, wholeKey: 'example#0', key: 'example#0', clear: true });
  assert.notEqual(other.key, cold.key);
  // The page that drew the scope asks for the whole census and keeps its rows meanwhile.
  assert.equal(planRead({ feed: 'example', generation: 0, first, valueFeed: 'example', valueScoped: true, wholeAsked: true, keep: true }).clear, false);
  assert.equal(planRead({ feed: 'example', generation: 0, first, valueFeed: 'example', valueScoped: true, wholeAsked: true, keep: true }).scoped, false);
  // Once any subscriber asked the whole census of a feed, it is never scoped again, even cold.
  assert.equal(planRead({ feed: 'example', generation: 3, first, valueFeed: null, valueScoped: false, wholeAsked: true }).scoped, false);
  // A whole census already held is not replaced by a scope.
  assert.deepEqual(planRead({ feed: 'example', generation: 0, first, valueFeed: 'example', valueScoped: false, wholeAsked: false }),
    { scoped: false, wholeKey: 'example#0', key: 'example#0', clear: false });
  // A feed change clears, scoped or not; an expiry is part of what was asked.
  assert.equal(planRead({ feed: 'other', generation: 0, first: null, valueFeed: 'example', valueScoped: false, wholeAsked: false }).clear, true);
  assert.equal(planRead({ feed: 'example', generation: 0, first: { underlying: 'ALPHA', expiry: '2019-03-28' }, valueFeed: null, valueScoped: false, wholeAsked: false }).key,
    'example#0#scope:ALPHA:2019-03-28');
});

test('the banner says the page holds one instrument\'s rows, what was not read, and why a whole census was read instead', () => {
  const scope = { underlying: 'ALPHA', expiry: null, months: 3,
    read: [{ segment: 'INDEX', expiry: null, months: 3, rows: 118, generation: 7 }, { segment: 'CASH', expiry: null, months: 0, rows: 0, generation: 5 }],
    unread: [{ segment: 'FNO', why: 'too many rows' }] };
  const lag = /** @type {any} */ ({ state: 'held', generation: 21, parts: null, faults: [], reason: null });
  const [read] = censusLines({ feed: 'example', at: Date.UTC(2026, 9, 11, 4, 30), lag, worker: null, head: null, how: null, scope });
  assert.equal(read.key, 'read');
  assert.match(read.text, /generation 21: only the census rows of ALPHA \(INDEX 3 row\(s\)\) from \/census\/facets\.json, not the whole census\./);
  assert.match(read.text, /covers ALPHA alone until the whole census is read, which happens when a control outside the bar rows is used\./);
  assert.match(read.text, /Not read yet: FNO \(too many rows\)\./);
  assert.doesNotMatch(read.text, /every list, total and offset on this page is that snapshot/);
  // The scope's own description is not a reason a whole census was read: no such line.
  const how = /** @type {any} */ ({ kind: 'scoped', since: null, rows: 3, added: 0, replaced: 0, why: 'ALPHA: INDEX 3 row(s)' });
  assert.deepEqual(censusLines({ feed: 'example', at: 1, lag, worker: null, head: null, how, scope }).map((line) => line.key), ['read']);
  const [reading] = censusLines({ feed: 'example', at: 1, lag, worker: null, head: null, how: null, scope, reading: true });
  assert.match(reading.text, /until the whole census is read, which is being read now\./);
  // The whole census, read because the scope could not be: said, with the reason.
  const whole = censusLines({ feed: 'example', at: 1, lag, worker: null, head: null, how: null, scopeWhy: 'example holds no census row of ALPHA' });
  assert.match(whole[0].text, /every list, total and offset on this page is that snapshot/);
  assert.deepEqual(whole.find((line) => line.key === 'scope'),
    { key: 'scope', tone: 'info', text: 'This page read the whole census before its first paint, not only the instrument on screen: example holds no census row of ALPHA.' });
  assert.equal(censusLines({ feed: 'example', at: 1, lag, worker: null, head: null, how: null }).some((line) => line.key === 'scope'), false);
});

test('the refresh log names a scoped read, and a first whole read that a failed scope caused', () => {
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
