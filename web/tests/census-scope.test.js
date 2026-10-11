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

/* ----------------------------------------------------------------------
   THE SECOND CHECK'S BLOCKING 2 AND ITS FOLLOW-UPS (verify-WP-56b-r1)
   ---------------------------------------------------------------------- */
import { readFileSync } from 'node:fs';
import { scopeNeed, scopeWanted, SCOPE_MS, FACETS_MS, SCOPE_RETRY_MAX_MS } from '../src/lib/store-census.js';

const SCOPE = { underlying: 'ALPHA', expiry: null, months: 3, read: [{ segment: 'INDEX', expiry: null, months: 3, rows: 118, generation: 7 }], unread: [] };
const ON_SCREEN = { scope: SCOPE, view: 'bars', defaultView: 'bars', filter: 'ALPHA', kind: 'spot', reconciling: false, matched: 3 };

test('a scope that does not hold the selection on screen is a reading state, never an empty selection', () => {
  // The selection is in the rows read: the scope answers the page.
  assert.equal(scopeNeed(ON_SCREEN), null);
  // NO SCOPE (the whole census, or nothing read yet): nothing is missing
  // from a scope, whatever the selection matches. An empty selection over
  // the whole census is the store's own answer.
  assert.equal(scopeNeed({ ...ON_SCREEN, scope: null, matched: 0 }), null);
  // THE BLOCKING CASE: one instrument's rows, and the selection on screen
  // matches none of them (an option whose chain was not listed). It is
  // MISSING, with the reason the whole census is then read for.
  assert.deepEqual(scopeNeed({ ...ON_SCREEN, kind: 'options', matched: 0 }),
    { why: 'the rows read for ALPHA hold nothing for the selection on screen', missing: true });
  // Another instrument than the one read: missing too, before any row of it
  // could match.
  assert.deepEqual(scopeNeed({ ...ON_SCREEN, filter: 'BETA', matched: 0 }),
    { why: 'BETA was chosen and only ALPHA had been read', missing: true });
  // WHILE THE SELECTION IS STILL BEING RECONCILED, or before a kind is
  // seeded, an empty match is not yet a verdict either way.
  assert.equal(scopeNeed({ ...ON_SCREEN, matched: 0, reconciling: true }), null);
  assert.equal(scopeNeed({ ...ON_SCREEN, matched: 0, kind: '' }), null);
  assert.equal(scopeNeed({ ...ON_SCREEN, matched: 0, filter: '' }), null);
  // The Coverage grid needs every row and holds its own rows meanwhile.
  assert.deepEqual(scopeNeed({ ...ON_SCREEN, view: 'census' }), { why: 'the Coverage grid lists every instrument', missing: false });
});

test('the page draws its reading state, not the grid, while the selection is missing from the scope', () => {
  // SOURCE SHAPE: the three places that must agree. The banner derives
  // `scopeMissing` from `scopeNeed` (not from a copy of its rules) and keeps
  // it while the whole read is in flight (no 'ready' gate); the page's
  // loading branch reads it; and the grid, drawn only in the branch after,
  // is the one place 'No instrument-month matches' is written.
  const banner = readFileSync(new URL('../src/routes/db/DbCensusBanner.svelte', import.meta.url), 'utf8');
  const page = readFileSync(new URL('../src/routes/db/+page.svelte', import.meta.url), 'utf8');
  const grid = readFileSync(new URL('../src/routes/db/DbBarsGrid.svelte', import.meta.url), 'utf8');
  // THE SCOPE OF THE ROWS ON SCREEN (`shownScope`), which outlives
  // `store.scope` while the whole census is prepared behind them.
  assert.match(banner, /const need = \$derived\(scopeNeed\(\{ scope: shownScope,/);
  assert.match(banner, /const scopeMissing = \$derived\(shownScope !== null && need\?\.missing === true\);/);
  assert.match(banner, /const shownScope = \$derived\(behind \? prepared\.scope : store\.feed === feeds\.active \? store\.scope : null\);/);
  assert.match(banner, /scopeMissing: \{ get: \(\) => scopeMissing \}/);
  assert.match(banner, /const why = need\?\.why \?\? null;/, 'the whole read is asked for by the same verdict');
  const loading = page.indexOf('{:else if (db.loading && (db.rows.length === 0 || db.preparing)) || db.scopeMissing}');
  const drawn = page.indexOf('<DbBarsGrid {db} />');
  assert.ok(loading > 0, 'the loading branch reads db.scopeMissing');
  assert.ok(drawn > loading, 'the grid is drawn in a later branch of the same chain');
  assert.equal(page.slice(loading, drawn).includes('\n  {:else}\n'), true, 'the grid is in the chain\'s last branch');
  assert.equal(grid.split('No instrument-month matches this query').length, 2, 'said in one place');
  for (const source of [banner, page]) assert.equal(source.includes('<h3>No instrument-month matches'), false);
});

test('a chain refused at the expiry on screen yields no scope at all: the whole census is read in the same flight', async () => {
  // The link names ALPHA 2019-03-28; FNO is refused whole AND at that
  // expiry. A scope of the INDEX rows alone would be drawn without the
  // selection it was read for.
  const { request, asked } = server({
    [ROOT]: [200, root()],
    [Q('INDEX')]: [200, rowsBody('INDEX', 7, INDEX_ROWS)],
    [Q('CASH')]: [200, rowsBody('CASH', 5, [])],
    [Q('FNO')]: [400, TOO_MANY],
    [Q('FNO', '2019-03-28')]: [400, { ...TOO_MANY, why: 'ALPHA in FNO holds 5000 census rows at this expiry, more than the 4096 one rows answer carries: read the census whole from /store.json', reached: 5000, counted: 'census rows' }]
  });
  const answer = await readScope(request, 'example', { underlying: 'ALPHA', expiry: '2019-03-28' });
  assert.equal(answer.ok, false);
  if (answer.ok) return;
  assert.match(answer.reason, /^ALPHA in FNO at 2019-03-28: ALPHA in FNO holds 5000 census rows at this expiry/);
  assert.equal(asked.includes(Q('FNO', '2019-03-28')), true);
  // WITH NO EXPIRY ON SCREEN the same refusal leaves a scope of what was
  // read, with FNO named as unread (the first test above).
});

test('a feed chosen later is read for the instrument on screen, not the page\'s opening one', () => {
  const page = { view: 'bars', defaultView: 'bars', filter: 'BETA', expiry: '2019-03-28', fallback: 'ALPHA' };
  assert.deepEqual(scopeWanted(page), { underlying: 'BETA', expiry: '2019-03-28' });
  assert.deepEqual(scopeWanted({ ...page, expiry: '' }), { underlying: 'BETA', expiry: null });
  assert.deepEqual(scopeWanted({ ...page, expiry: undefined }), { underlying: 'BETA', expiry: null });
  // No instrument chosen: the page's opening one.
  assert.deepEqual(scopeWanted({ ...page, filter: '', expiry: null }), { underlying: 'ALPHA', expiry: null });
  // The Coverage grid needs every row.
  assert.equal(scopeWanted({ ...page, view: 'census' }), null);
  const banner = readFileSync(new URL('../src/routes/db/DbCensusBanner.svelte', import.meta.url), 'utf8');
  assert.match(banner, /else if \(f && firstFeed && f !== firstFeed\) \{\s+first = untrack\(\(\) => scopeWanted\(/);
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
  assert.match(banner, /\{#if db\.scopeMissing\}[\s\S]{0,400}<p class="inline-note" role="status" data-scope-missing>Reading the whole census of/);
});

test('a saturated server is asked once more after the wait it names; a second 429 gives the scope up, said', async () => {
  // THE FIRST ANSWER OF EVERY URL IS 429 with retry-after 1; the second is
  // the census's. The scope is read, and says how many requests it asked twice.
  /** @type {Record<string, [number, any]>} */
  const answers = { [ROOT]: [200, root()], [Q('INDEX')]: [200, rowsBody('INDEX', 7, INDEX_ROWS)], [Q('CASH')]: [200, rowsBody('CASH', 5, [])], [Q('FNO')]: [400, TOO_MANY] };
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
  const calm = await readScope(server({ [ROOT]: [200, root()], [Q('INDEX')]: [200, rowsBody('INDEX', 7, INDEX_ROWS)], [Q('CASH')]: [200, rowsBody('CASH', 5, [])], [Q('FNO')]: [400, TOO_MANY] }).request,
    'example', { underlying: 'ALPHA' });
  assert.equal(calm.ok && calm.retried, 0);
});
