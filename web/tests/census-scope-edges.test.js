// R3-12 merge-back (25 Oct 2026): THE EDGES OF THE SCOPE CODE, one test per
// branch node's coverage named uncovered on the merged tree (`node --test
// --experimental-test-coverage`, lcov BRDA count 0): a listing row or a root
// row missing a count, a listing out of order with a repeated day, a chain
// read at one expiry and another asked, a held scope missing its fields, a
// server list with counts that are not whole numbers, every way a roll-up
// can be refused or thrown, an unread chain named with and without its day,
// and a segment key that is not a string. Every name and number is invented.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readScope, viewSegments, scopeNeed, scopeCovers, scopeUnderlyings, readRollups, censusLines, facetsUrl, rowsUrl }
  from '../src/lib/store-census.js';
import { listedOf } from '../src/lib/db/selection-model.js';
import { rootBody, listingBody, HEADERS } from './store-module.js';

const FEED = 'example';
const ROOT = facetsUrl({ feed: FEED, segment: /** @type {any} */ (null) });

/** A request over scripted answers `[status, body-or-text]`; a string body is sent as it is. */
const scripted = (/** @type {Record<string, [number, any]>} */ answers) => {
  /** @type {string[]} */
  const asked = [];
  const request = async (/** @type {string} */ url) => {
    asked.push(url);
    const hit = answers[url];
    if (!hit) return new Response(JSON.stringify({ error: 'not_scripted', why: url }), { status: 500, headers: HEADERS });
    return new Response(typeof hit[1] === 'string' ? hit[1] : JSON.stringify(hit[1]), { status: hit[0], headers: HEADERS });
  };
  return { request: /** @type {any} */ (request), asked };
};

test('a root segment that names no underlying count is placed with none, never refused', () => {
  const plan = viewSegments([{ segment: 'INDEX' }, { segment: 'CASH', underlyings: 0 }, { segment: 'FNO' }], ['spot', 'options']);
  assert.deepEqual(plan, { rows: ['INDEX', 'CASH'], contract: ['FNO'], unplaced: [] });
});

test('a listing\'s expiry rows missing their counts and stamps are kept with empty ones, in day order with a repeated day kept twice', async () => {
  const listing = listingBody(FEED, 'FNO', 'ALPHA', 9, []);
  listing.expiries = /** @type {any} */ ([
    { expiry: '2019-03-28', weekday: 'Thu', contracts: 2, strikes: 1 },
    { expiry: '2019-02-28', weekday: 'Thu', contracts: 3, strikes: 1, future: true, rows: { '1s': 4 }, first_ts: 5, last_ts: 6 },
    { expiry: '2019-03-28', weekday: 'Thu', contracts: 7, strikes: 0, future: true }
  ]);
  const { request } = scripted({ [ROOT]: [200, rootBody(FEED)], [facetsUrl({ feed: FEED, segment: 'FNO', underlying: 'ALPHA' })]: [200, listing] });
  const answer = await readScope(request, FEED, { underlying: 'ALPHA', expiry: null, segments: ['options'] });
  assert.equal(answer.ok, true, JSON.stringify(answer));
  if (!answer.ok) return;
  assert.deepEqual(answer.scope.expiries.map((e) => [e.expiry, e.contracts, e.future, JSON.stringify(e.rows), e.first_ts, e.last_ts]), [
    ['2019-02-28', 3, true, '{"1s":4}', 5, 6],
    ['2019-03-28', 2, false, '{}', null, null],
    ['2019-03-28', 7, true, '{}', null, null]
  ]);
});

test('a chain read at one expiry does not answer another: the scope names the one it read', () => {
  const scope = /** @type {any} */ ({ underlying: 'ALPHA', expiry: '2019-02-28', views: ['options'], offered: [{ segment: 'FNO' }] });
  assert.deepEqual(scopeNeed({ scope, filter: 'ALPHA', kind: 'options', expiry: '2019-03-28' }),
    { why: 'the expiry 2019-03-28 was chosen and only 2019-02-28 had been read for ALPHA', missing: true });
  assert.deepEqual(scopeNeed({ scope: { ...scope, expiry: null }, filter: 'ALPHA', kind: 'options', expiry: '2019-03-28' }),
    { why: 'the expiry 2019-03-28 was chosen and no chain had been read for ALPHA', missing: true });
  assert.equal(scopeNeed({ scope, filter: 'ALPHA', kind: 'options', expiry: '2019-02-28' }), null, 'the chain read answers its own day');
});

test('a held scope covers another only for its underlying, its families and its expiry, whatever fields it lacks', () => {
  const want = { underlying: 'ALPHA', expiry: null, segments: ['spot'] };
  assert.equal(scopeCovers(null, want), false, 'nothing held covers nothing');
  assert.equal(scopeCovers(undefined, want), false);
  assert.equal(scopeCovers({ underlying: 'BETA', expiry: null, segments: [] }, want), false, 'another underlying');
  assert.equal(scopeCovers(/** @type {any} */ ({ underlying: 'ALPHA', expiry: null }), want), true, 'a held scope naming no segment read every one');
  assert.equal(scopeCovers({ underlying: 'ALPHA', expiry: null, segments: ['spot'] }, /** @type {any} */ ({ underlying: 'ALPHA', expiry: null })), false,
    'a want naming no segment asks every one, which a spot scope did not read');
  assert.equal(scopeCovers({ underlying: 'ALPHA', expiry: '2019-02-28', segments: ['options'] }, { underlying: 'ALPHA', expiry: '2019-02-28', segments: ['options-cut'] }), true,
    'a cut key reads what its family reads, at the same expiry');
  assert.equal(scopeCovers({ underlying: 'ALPHA', expiry: '2019-02-28', segments: ['options'] }, { underlying: 'ALPHA', expiry: '2019-03-28', segments: ['options'] }), false,
    'another expiry is another chain');
  assert.equal(scopeCovers({ underlying: 'ALPHA', expiry: '2019-02-28', segments: ['options'] }, { underlying: 'ALPHA', expiry: null, segments: ['options'] }), true,
    'a chain read covers its listing');
});

test('the server\'s list adds only whole counts: a count that is no safe integer, or absent, adds nothing', () => {
  const scope = /** @type {any} */ ({ underlying: 'ALPHA', segments: ['CASH', 'FNO'],
    listed: [{ segment: 'INDEX', underlyings: 1 }, { segment: 'CASH', underlyings: 2 }, { segment: 'FNO', underlyings: 2 }] });
  const { rows, why } = scopeUnderlyings(scope, [
    { ok: true, body: { underlyings: [{ exchange: 'NSE', symbol: 'ALPHA', months: 1.5, rows: { '1min': 10, '5min': 2 ** 60, '1day': 'x' } },
      { exchange: 'NSE', symbol: 'BETA', months: 3 }] } },
    { ok: true, body: { underlyings: [{ exchange: 'NSE', symbol: 'ALPHA', months: 4, expiries: 'many', contracts: Infinity, rows: { '1s': 7 } },
      { exchange: 'BSE', symbol: 'ALPHA', months: 1, expiries: 2, contracts: 9 }] } }
  ]);
  assert.equal(why, null);
  assert.deepEqual(rows.map((r) => [r.key, r.exchanges.join('+'), r.months, r.bars, r.spotMonths, r.contractMonths, r.expiries, r.contracts]), [
    ['ALPHA', 'NSE+BSE', 5, 17, 0, 5, 2, 9],
    ['BETA', 'NSE', 3, 0, 3, 0, 0, 0]
  ]);
  const failed = scopeUnderlyings(scope, [{ ok: false, reason: 'HTTP 503', body: null }, { ok: true, body: { underlyings: 'none' } }]);
  assert.deepEqual(failed.rows, []);
  assert.equal(failed.why, "the server's instrument list could not be read whole (CASH: HTTP 503; FNO: its answer lists no underlyings)");
});

test('every way a roll-up fails is kept by name: a refusal with a reason, with a code only, with no body, a torn answer and a thrown request', async () => {
  const scope = /** @type {any} */ ({ underlying: 'ALPHA', segments: ['INDEX', 'CASH', 'FNO'] });
  const url = (/** @type {string} */ segment, /** @type {string | null} */ underlying) => `/census/rollup.json?feed=${FEED}&segment=${segment}${underlying ? `&underlying=${underlying}` : ''}`;
  const good = (/** @type {string} */ segment, /** @type {string | null} */ underlying, /** @type {any} */ over = {}) =>
    ({ schema: 1, feed: FEED, segment, underlying, rungs: [{ timeframe: '1min', months_held: 2, instrument_months: 2, rows: 9, days_held: 30, days_owed: 40 }], months: ['2019-02', '2019-03'], ...over });
  /** @type {Record<string, [number, any]>} */
  const answers = {
    [url('INDEX', 'ALPHA')]: [200, good('INDEX', 'ALPHA', { basis: { days_owed: 'the calendar of record', days_held: 'the day index' } })],
    [url('INDEX', null)]: [400, { error: 'rollup_too_many', why: 'INDEX names more than 4096 cells' }],
    [url('CASH', 'ALPHA')]: [503, { error: 'busy' }],
    [url('CASH', null)]: [500, 'not json at all'],
    [url('FNO', 'ALPHA')]: [200, 'torn {']
  };
  const request = async (/** @type {string} */ asked) => {
    if (asked === url('FNO', null)) throw new Error('the request was cancelled');
    const hit = answers[asked];
    return new Response(typeof hit[1] === 'string' ? hit[1] : JSON.stringify(hit[1]), { status: hit[0], headers: HEADERS });
  };
  const list = await readRollups(/** @type {any} */ (request), FEED, scope);
  assert.deepEqual(list.map((r) => [r.segment, r.underlying, r.ok, r.why, r.months, r.basis === null]), [
    ['INDEX', 'ALPHA', true, null, 2, false],
    ['INDEX', null, false, '/census/rollup.json answered HTTP 400: rollup_too_many: INDEX names more than 4096 cells', 0, true],
    ['CASH', 'ALPHA', false, '/census/rollup.json answered HTTP 503: busy', 0, true],
    ['CASH', null, false, '/census/rollup.json answered HTTP 500: no reason named', 0, true],
    ['FNO', 'ALPHA', false, list[4].why, 0, true],
    ['FNO', null, false, 'the request was cancelled', 0, true]
  ]);
  assert.match(String(list[4].why), /JSON/, 'a torn answer is named by the parse that refused it');
  const thrownText = await readRollups(/** @type {any} */ (async () => { throw 'a bare refusal'; }), FEED, { ...scope, segments: ['INDEX'] });
  assert.deepEqual(thrownText.map((r) => r.why), ['a bare refusal', 'a bare refusal'], 'a thrown value with no message is said as it is');
  const unbased = await readRollups(/** @type {any} */ (async () => new Response(JSON.stringify(good('INDEX', null)), { status: 200, headers: HEADERS })),
    FEED, /** @type {any} */ ({ underlying: null, segments: ['INDEX'] }));
  assert.equal(unbased[0].basis, null, 'an answer without a basis claims none');
});

test('a chain the server would not list is named with its day; a segment refused whole without one', () => {
  const scope = /** @type {any} */ ({ underlying: 'ALPHA', expiry: '2019-02-28', views: ['options'], segments: ['FNO'], read: [],
    unread: [{ segment: 'FNO', expiry: '2019-02-28', why: 'past the cap' }, { segment: 'CASH', expiry: null, why: 'also past it' }],
    offered: [{ segment: 'FNO', expiries: 2 }], expiries: [], listed: [], unplaced: [], months: 0 });
  const lines = censusLines({ feed: FEED, at: 1, lag: null, worker: null, head: null, scope });
  const read = lines.find((line) => line.key === 'read');
  assert.match(String(read?.text), /Not listed by the server: FNO at 2019-02-28 \(past the cap\); CASH \(also past it\)\./);
});

test('a segment key that is not a string offers what the empty key offers', () => {
  const expiries = [{ expiry: '2019-02-28', strikes: 1, future: false }];
  assert.deepEqual(listedOf(expiries, /** @type {any} */ (null), () => ''), expiries);
  assert.deepEqual(listedOf(expiries, /** @type {any} */ (undefined), () => ''), expiries);
  assert.deepEqual(listedOf(expiries, 'spot', () => 'spot'), [], 'spot lists no expiry');
});

test('a rows url of an underlying with an expiry names it; without one it does not', () => {
  assert.match(rowsUrl({ feed: FEED, segment: 'FNO', underlying: 'ALPHA', expiry: '2019-02-28' }), /&expiry=2019-02-28&rows=/);
  assert.doesNotMatch(rowsUrl({ feed: FEED, segment: 'INDEX', underlying: 'ALPHA' }), /expiry=/);
});

/* ----------------------------------------------------------------------
   The answers' checks and the scope's request edges, unrun on the merged
   tree: every field a check refuses, a 429 that names no wait, a refusal
   whose body is not JSON or names only a code, and two rows answers that
   disagree on a month's fullest row.
   ---------------------------------------------------------------------- */
import { rootFault, facetsFault, rowsFault } from '../src/lib/store-census.js';
import { rowsBody } from './store-module.js';

test('every field the root, facets and rows checks read is refused by name when it is wrong', () => {
  const root = rootBody(FEED);
  assert.equal(rootFault(null, FEED), 'it is not a schema-1 facets answer');
  assert.equal(rootFault({ ...root, segments: [{ ...root.segments[0], rows: null }] }, FEED), 'INDEX names no rows');
  assert.equal(rootFault({ ...root, segments: [{ ...root.segments[0], rows: 'many' }] }, FEED), 'INDEX names no rows');
  const query = { feed: FEED, segment: 'FNO', underlying: 'ALPHA' };
  const listing = listingBody(FEED, 'FNO', 'ALPHA', 9, ['2019-02-28']);
  assert.equal(facetsFault(listing, query), null);
  assert.equal(facetsFault(null, query), 'it is not a schema-1 facets answer');
  assert.equal(facetsFault({ ...listing, schema: 2 }, query), 'it is not a schema-1 facets answer');
  assert.equal(facetsFault({ ...listing, feed: 'other' }, query), 'it answers other/FNO, not example/FNO');
  assert.equal(facetsFault({ ...listing, underlying: undefined }, query), 'it answers another underlying or expiry');
  assert.equal(facetsFault({ ...listing, expiry: '2019-02-28' }, query), 'it answers another underlying or expiry');
  assert.equal(facetsFault({ ...listing, underlying: null }, { feed: FEED, segment: 'FNO' }), null, 'a segment answer names no underlying either way');
  assert.equal(facetsFault({ ...listing, generation: -1 }, query), 'its generation is not a whole number');
  assert.equal(facetsFault({ ...listing, generation: 1.5 }, query), 'its generation is not a whole number');
  assert.equal(facetsFault({ ...listing, strikes: {} }, query), 'its strikes is not a list');
  const rows = rowsBody(FEED, 'INDEX', 'ALPHA', 7, [['NSE-INDEX-ALPHA', '2019-03', '1min', 60]]);
  const rowsQuery = { feed: FEED, segment: 'INDEX', underlying: 'ALPHA' };
  assert.equal(rowsFault(rows, rowsQuery), null);
  assert.equal(rowsFault({ ...rows, underlying: undefined }, rowsQuery), 'it answers another underlying or expiry');
  assert.equal(rowsFault({ ...rows, underlying: null, expiry: undefined }, { feed: FEED, segment: 'INDEX' }), null);
  assert.equal(rowsFault({ ...rows, expiry: '2019-02-28' }, rowsQuery), 'it answers another underlying or expiry');
});

test('a 429 naming no usable wait is asked again after the longest wait; a refusal that is not JSON, or names only its code, is said', async () => {
  const root = rootBody(FEED);
  /** @type {number[]} */
  const waits = [];
  let calls = 0;
  const request = async (/** @type {string} */ url) => {
    calls += 1;
    if (url === ROOT && calls === 1) return new Response('{}', { status: 429, headers: { ...HEADERS, 'retry-after': 'soon' } });
    if (url === ROOT) return new Response(JSON.stringify(root), { status: 200, headers: HEADERS });
    if (url === rowsUrl({ feed: FEED, segment: 'INDEX', underlying: 'ALPHA' })) return new Response('<html>busy</html>', { status: 502, headers: HEADERS });
    return new Response(JSON.stringify({ error: 'segment_gone' }), { status: 404, headers: HEADERS });
  };
  const answer = await readScope(/** @type {any} */ (request), FEED, { underlying: 'ALPHA', expiry: null, segments: ['spot'] },
    { pause: async (/** @type {number} */ ms) => { waits.push(ms); } });
  assert.equal(answer.ok, false);
  assert.equal(waits.length, 1);
  assert.ok(waits[0] > 0, 'a wait that is not a number waits the longest the scope allows');
  if (answer.ok) return;
  assert.equal(answer.reason, 'ALPHA in INDEX: HTTP 502 and no reason named');
  // A refusal naming only its code is said by its code.
  const coded = async (/** @type {string} */ url) => (url === ROOT
    ? new Response(JSON.stringify(root), { status: 200, headers: HEADERS })
    : new Response(JSON.stringify({ error: 'segment_gone' }), { status: 404, headers: HEADERS }));
  const second = await readScope(/** @type {any} */ (coded), FEED, { underlying: 'ALPHA', expiry: null, segments: ['spot'] });
  assert.equal(second.ok, false);
  if (!second.ok) assert.equal(second.reason, 'ALPHA in INDEX: segment_gone');
});

test('two rows answers that disagree on a month\'s fullest row refuse the scope by name', async () => {
  const index = rowsBody(FEED, 'INDEX', 'ALPHA', 7, [['NSE-INDEX-ALPHA', '2019-03', '1min', 60]]);
  const cash = rowsBody(FEED, 'CASH', 'ALPHA', 5, [['NSE-CASH-ALPHA', '2019-03', '1min', 50]]);
  // Both name 2019-03 at 1min; the cash answer states another fullest row.
  const { request } = scripted({ [ROOT]: [200, rootBody(FEED)], [rowsUrl({ feed: FEED, segment: 'INDEX', underlying: 'ALPHA' })]: [200, index],
    [rowsUrl({ feed: FEED, segment: 'CASH', underlying: 'ALPHA' })]: [200, cash] });
  const answer = await readScope(request, FEED, { underlying: 'ALPHA', expiry: null, segments: ['spot'] });
  assert.equal(answer.ok, false);
  if (!answer.ok) assert.equal(answer.reason, 'two answers disagree on the fullest row of 2019-03 at 1min');
  // The same fullest row and support in both is one census, read whole.
  const agreeing = { ...cash, denominators: index.denominators };
  const { request: same } = scripted({ [ROOT]: [200, rootBody(FEED)], [rowsUrl({ feed: FEED, segment: 'INDEX', underlying: 'ALPHA' })]: [200, index],
    [rowsUrl({ feed: FEED, segment: 'CASH', underlying: 'ALPHA' })]: [200, agreeing] });
  const whole = await readScope(same, FEED, { underlying: 'ALPHA', expiry: null, segments: ['spot'] });
  assert.equal(whole.ok, true, JSON.stringify(whole));
});

/* ----------------------------------------------------------------------
   The page's sentences, unrun on the merged tree: each refresh kind with
   and without its reason, a clock that is no time, a filter that is no
   text, a storage that throws a bare value, a census state with a reason
   and no count, and the moved lines' every refusal and partial answer.
   ---------------------------------------------------------------------- */
import { refreshText, searchRefreshes, createRefreshLog, movedLines, censusLag } from '../src/lib/store-census.js';

test('every refresh kind is said with and without its reason, and a clock that is no time is a dash', () => {
  const base = { at: Number.NaN, feed: FEED, since: null, rows: 2, added: 1, replaced: 1, generation: null, held: null, worker: null };
  const said = (/** @type {any} */ over) => refreshText(/** @type {any} */ ({ ...base, why: null, ...over }));
  assert.equal(said({ kind: 'scoped' }), `— · ${FEED} · read only the census rows of the instrument on screen from /census/facets.json (no instrument named; 2 row(s)), not the whole census; generation not named.`);
  assert.match(said({ kind: 'first', why: 'a new tab' }), /the first read of this feed on this page; a new tab\)/);
  assert.match(said({ kind: 'delta' }), /changed since the generation held \(1 new, 1 changed in place\)/);
  assert.match(said({ kind: 'unchanged' }), /found nothing changed since the generation held/);
  assert.match(said({ kind: 'unchanged', why: 'the delta was refused' }), /asked for the whole census \(the delta was refused\)/);
  assert.match(said({ kind: 'failed' }), /failed: no reason named\.$/);
  assert.match(said({ kind: 'whole', why: 'a restart' }), /read the whole census again: a restart; generation not named/);
  assert.match(said({ kind: 'whole', held: 4, generation: 9, worker: 'decoded on the page' }), /read the whole census again; generation 9, 4 row\(s\) held \(decoded on the page\)\.$/);
  const entries = /** @type {any[]} */ ([{ ...base, kind: 'scoped', why: null }, { ...base, kind: 'failed', why: 'x' }]);
  assert.equal(searchRefreshes(entries, /** @type {any} */ (null)).length, 2, 'a filter that is no text is every entry');
});

test('a storage that throws a bare value on read and on save is said, and the log keeps what this tab read', () => {
  const storage = { getItem() { throw 'locked'; }, setItem() { throw 'full'; } };
  const log = createRefreshLog(storage);
  assert.equal(log.fault, 'the refresh log saved in this browser could not be read (locked)');
  log.note(/** @type {any} */ ({ at: 1, feed: FEED, kind: 'scoped', why: null }));
  assert.equal(log.fault, 'the refresh log could not be saved (full), so it holds only what this tab read');
  assert.equal(log.list().length, 1);
});

test('a census state with a reason and no count is said with its reason', () => {
  const lag = censusLag(new Headers({ 'x-brutex-census-state': 'held', 'x-brutex-census-reason': 'a part was re-read', 'x-brutex-census-generation': '3' }));
  const lines = censusLines({ feed: FEED, at: 1, lag, worker: null, head: null });
  assert.deepEqual(lines.filter((line) => line.key === 'state').map((line) => line.text), [`The ${FEED} census is held: a part was re-read`]);
});

test('the moved lines say every refused delta in its own words, a partial answer without a stamp, and a head never checked', () => {
  const moved = { from: 1, to: 2, segments: [] };
  const query = (/** @type {string} */ segment) => ({ feed: FEED, segment, underlying: 'ALPHA' });
  const lines = movedLines(moved, null, Number.NaN, 'ALPHA', [
    { ok: false, status: 410, query: query('INDEX'), body: null, reason: null },
    { ok: false, status: 409, query: query('CASH'), body: null, reason: null },
    { ok: false, status: 503, query: query('FNO'), body: null, reason: null },
    { ok: true, status: 200, query: query('BSEFNO'), body: { months: [], expiries: ['2019-02-28'], underlyings: [{ symbol: 'ALPHA' }] }, reason: null }
  ]);
  assert.equal(lines[0].text, 'The store moved after this page read it: generation 1 → 2, seen at —. This page still shows the snapshot read at —.');
  assert.deepEqual(lines.slice(1).map((line) => line.text), [
    'ALPHA INDEX: the server no longer holds the changes since this snapshot; Refresh reads it whole.',
    "ALPHA CASH: the server's census for it stands below this snapshot's generation (read whole since); Refresh reads it whole.",
    'ALPHA FNO: the delta could not be read; Refresh reads it whole.',
    'ALPHA BSEFNO: 0 month(s) and 1 expiry(ies) changed since this snapshot.'
  ]);
  assert.equal('through' in lines[4], false, 'no stamp, no marker');
});

/* ----------------------------------------------------------------------
   The facets loader and the census lines, unrun on the merged tree: an
   answer held is served again, the oldest is let go past the cap, a refresh
   of nothing held is a load, a refused delta and a refused reload are the
   server's answers, a merge of two answers naming no order names none, a
   moved census with no part generations, a roll-up refusal with a reason
   and no code, a refresh said as a delta, and a moved month with no expiry.
   ---------------------------------------------------------------------- */
import { createFacetsLoader, mergeFacets, censusMoved } from '../src/lib/store-census.js';

test('the facets loader serves a held answer again, lets the oldest go past its cap, and refreshes nothing held as a load', async () => {
  /** @type {string[]} */
  const urls = [];
  const loader = createFacetsLoader(/** @type {any} */ (async (/** @type {string} */ url) => {
    urls.push(url);
    const q = new URL(url, 'http://x').searchParams;
    return new Response(JSON.stringify(listingBody(FEED, 'FNO', q.get('underlying'), 9, ['2019-02-28'])), { status: 200, headers: HEADERS });
  }), { keep: 1 });
  const alpha = { feed: FEED, segment: 'FNO', underlying: 'ALPHA' };
  const beta = { feed: FEED, segment: 'FNO', underlying: 'BETA' };
  assert.equal((await loader.refresh(alpha)).ok, true, 'a refresh of nothing held is a cold load');
  assert.equal((await loader.load(alpha)).ok, true);
  assert.equal(urls.length, 1, 'the held answer is served again, not asked');
  await loader.load(beta);
  await loader.load(alpha);
  assert.equal(urls.length, 3, 'past a cap of one, the oldest answer was let go and asked again');
});

test('a refused delta, and a refused reload after a 410, are the server\'s answers, never a patch', async () => {
  const alpha = { feed: FEED, segment: 'FNO', underlying: 'ALPHA' };
  /** @type {Array<() => Response>} */
  const answers = [
    () => new Response(JSON.stringify(listingBody(FEED, 'FNO', 'ALPHA', 9, ['2019-02-28'])), { status: 200, headers: HEADERS }),
    () => new Response(JSON.stringify({ error: 'busy', why: 'every census slot is held' }), { status: 503, headers: HEADERS }),
    () => new Response(JSON.stringify({ error: 'since_too_old' }), { status: 410, headers: HEADERS }),
    () => new Response('', { status: 500, headers: { ...HEADERS, 'x-brutex-census-note': '' } })
  ];
  const loader = createFacetsLoader(/** @type {any} */ (async () => /** @type {() => Response} */ (answers.shift())()));
  assert.equal((await loader.load(alpha)).ok, true);
  const refused = await loader.refresh(alpha);
  assert.deepEqual([refused.ok, refused.status, refused.reason], [false, 503, 'busy'], 'the server\'s own word for the refusal');
  const reload = await loader.refresh(alpha);
  assert.deepEqual([reload.ok, reload.status, reload.reloaded, reload.reason], [false, 500, false, '/census/facets.json answered HTTP 500 and named no reason']);
});

test('two facets answers that name no order merge into one that names none', () => {
  const base = { ...listingBody(FEED, 'FNO', 'ALPHA', 9, ['2019-02-28']), order: undefined };
  const delta = { ...listingBody(FEED, 'FNO', 'ALPHA', 10, []), order: undefined, since: 9, underlyings: [],
    strikes: [{ strike: 1150000, ce: null, pe: { contracts: 1 } }] };
  const merged = mergeFacets({ ...base, strikes: [{ strike: 1150000, ce: { contracts: 2 }, pe: null }] }, delta);
  assert.equal(merged.order, undefined, 'no order is invented');
  assert.deepEqual(merged.strikes, [{ strike: 1150000, ce: { contracts: 2 }, pe: { contracts: 1 } }], 'a side the delta leaves null is the base\'s');
  // THE BASE'S ORDER WHEN THE DELTA NAMES NONE, and the other side left null by a delta.
  const ordered = mergeFacets({ ...base, order: { exchanges: ['NSE'], segments: ['FNO'] }, strikes: [{ strike: 1150000, ce: null, pe: { contracts: 3 } }] },
    { ...delta, strikes: [{ strike: 1150000, ce: { contracts: 4 }, pe: null }] });
  assert.deepEqual(ordered.order, { exchanges: ['NSE'], segments: ['FNO'] });
  assert.deepEqual(ordered.strikes, [{ strike: 1150000, ce: { contracts: 4 }, pe: { contracts: 3 } }]);
});

test('a moved census with no part generations names no segment; a roll-up refusal with a reason and no code is "refused"', async () => {
  const lag = (/** @type {number} */ generation) => ({ generation, parts: undefined });
  assert.deepEqual(censusMoved(/** @type {any} */ ({ feed: FEED, lag: lag(1) }), /** @type {any} */ ({ feed: FEED, lag: lag(2), error: null, at: 1 })),
    { from: 1, to: 2, segments: [] });
  const list = await readRollups(/** @type {any} */ (async () => new Response(JSON.stringify({ why: 'the census is being re-read' }), { status: 503, headers: HEADERS })),
    FEED, /** @type {any} */ ({ underlying: null, segments: ['INDEX'] }));
  assert.equal(list[0].why, '/census/rollup.json answered HTTP 503: refused: the census is being re-read');
});

test('a refresh read as a delta is said as one; a moved month with no expiry names none', () => {
  const lines = censusLines({ feed: FEED, at: 1, lag: censusLag(new Headers({ 'x-brutex-census-state': 'held', 'x-brutex-census-generation': '4' })),
    worker: null, head: null, how: /** @type {any} */ ({ kind: 'delta', rows: 3, added: 2, replaced: 1, why: null }) });
  assert.match(String(lines.find((line) => line.key === 'read')?.text), /^Census refreshed at .* by reading only the 3 row\(s\) changed since the generation this page held \(2 new, 1 changed in place\), now generation 4/);
  const moved = movedLines({ from: 1, to: 2, segments: ['INDEX'] }, null, 1, 'ALPHA', [
    { ok: true, status: 200, query: { feed: FEED, segment: 'INDEX', underlying: 'ALPHA' }, body: { months: [{ month: '2019-03' }], expiries: [], underlyings: [{ symbol: 'ALPHA', last_ts: 1_553_000_000_000_000 }] }, reason: null }
  ]);
  assert.match(String(moved[1].text), /^ALPHA INDEX: import or pull in progress through .*: 1 month\(s\) changed since this snapshot; its newest bar is now /);
  assert.equal(moved[1].through, 1_553_000_000_000_000);
});

/* ----------------------------------------------------------------------
   R3-12 merge-back (26 Oct 2026): A WARMING INDEX IS WAITED OUT. On the
   real store the first /db open 3 s after a start drew "503 index_warming:
   the gdfl FNO facet index is being built over the census (begun 2048 ms
   ago ...)" as the page's error until Refresh; before R3-12 the whole census
   was read in its place.
   ---------------------------------------------------------------------- */
import { SCOPE_WARM_TRIES, SCOPE_RETRY_MAX_MS } from '../src/lib/store-census.js';

test('a_warming_index_is_waited_out_a_bounded_number_of_times', async () => {
  const warming = () => new Response(JSON.stringify({ error: 'index_warming', why: 'the example INDEX facet index is being built over the census; ask again in a moment' }),
    { status: 503, headers: { ...HEADERS, 'retry-after': '1' } });
  const spot = rowsUrl({ feed: FEED, segment: 'INDEX', underlying: 'ALPHA' });
  const cash = rowsUrl({ feed: FEED, segment: 'CASH', underlying: 'ALPHA' });
  const body = (/** @type {string} */ url) => url === ROOT ? rootBody(FEED)
    : url === spot ? rowsBody(FEED, 'INDEX', 'ALPHA', 7, [['NSE-INDEX-ALPHA', '2019-03', '1min', 60]])
      : rowsBody(FEED, 'CASH', 'ALPHA', 5, []);
  const run = async (/** @type {number} */ warm, /** @type {boolean} */ named = true) => {
    /** @type {Map<string, number>} */
    const asked = new Map();
    /** @type {number[]} */
    const waits = [];
    const request = async (/** @type {string} */ url) => {
      const n = (asked.get(url) ?? 0) + 1;
      asked.set(url, n);
      if (url === ROOT && n <= warm) return named ? warming() : new Response('{"error":"down"}', { status: 503, headers: HEADERS });
      return new Response(JSON.stringify(body(url)), { status: 200, headers: HEADERS });
    };
    const answer = await readScope(/** @type {any} */ (request), FEED, { underlying: 'ALPHA', expiry: null, segments: ['spot'] },
      { pause: async (/** @type {number} */ ms) => { waits.push(ms); } });
    return { answer, waits, rootAsked: asked.get(ROOT), cashAsked: asked.get(cash) };
  };
  // WARMING FOR THREE ANSWERS: waited out, read, and counted as retries.
  const three = await run(3);
  assert.equal(three.answer.ok, true, JSON.stringify(three.answer));
  assert.deepEqual(three.waits, [1000, 1000, 1000], 'each wait the server named');
  assert.equal(three.rootAsked, 4);
  if (three.answer.ok) assert.equal(three.answer.retried, 3);
  // WARMING PAST THE BOUND: asked 1 + SCOPE_WARM_TRIES times, then the server's sentence.
  const long = await run(100);
  assert.equal(long.answer.ok, false);
  assert.equal(long.rootAsked, 1 + SCOPE_WARM_TRIES);
  assert.equal(long.waits.length, SCOPE_WARM_TRIES);
  if (!long.answer.ok) assert.match(long.answer.reason, /answered HTTP 503: the example INDEX facet index is being built/);
  // A 503 THAT NAMES NO WAIT is not a warming index: refused at once, asked once.
  const down = await run(1, false);
  assert.equal(down.answer.ok, false);
  assert.equal(down.rootAsked, 1);
  assert.deepEqual(down.waits, []);
  assert.ok(SCOPE_WARM_TRIES * SCOPE_RETRY_MAX_MS <= 12_000, 'the longest a warming index is waited is bounded');
});

test('an_instrument_not_held_in_a_segment_is_counted_as_none_not_refused', async () => {
  // R3-12 merge-back: on the real store NIFTY's Coverage asked CASH's
  // roll-up for NIFTY and drew "not counted: 404 not_held" in red.
  const notHeld = (/** @type {string} */ segment) => new Response(JSON.stringify({ error: 'not_held', why: `ALPHA is not an underlying the example census holds in ${segment}` }), { status: 404, headers: HEADERS });
  const list = await readRollups(/** @type {any} */ (async (/** @type {string} */ url) => notHeld(new URL(url, 'http://x').searchParams.get('segment') ?? '')),
    FEED, /** @type {any} */ ({ underlying: 'ALPHA', segments: ['CASH'] }));
  assert.deepEqual(list.map((r) => [r.underlying, r.ok, r.why, r.rungs.length]), [
    ['ALPHA', true, null, 0],
    [null, false, '/census/rollup.json answered HTTP 404: not_held: ALPHA is not an underlying the example census holds in CASH', 0]
  ], 'the instrument\'s is none held; a whole segment\'s 404 is still a refusal');
});

import { refreshEntry } from '../src/lib/store-census.js';

test('a refresh entry is filed under its kind on every path, never by timing', () => {
  const at = 1;
  assert.equal(refreshEntry({ feed: FEED, at, error: 'x' }).kind, 'failed');
  assert.equal(refreshEntry({ feed: FEED, at, first: true }).kind, 'first');
  assert.equal(refreshEntry({ feed: FEED, at, first: true, how: /** @type {any} */ ({ kind: 'whole' }) }).kind, 'first');
  assert.equal(refreshEntry({ feed: FEED, at, first: true, how: /** @type {any} */ ({ kind: 'delta' }) }).kind, 'delta');
  assert.equal(refreshEntry({ feed: FEED, at }).kind, 'whole', 'no read named, not the first: a whole read');
  const scoped = refreshEntry({ feed: FEED, at, how: /** @type {any} */ ({ kind: 'scoped', why: 'ALPHA', rows: 3, since: 's', added: 1, replaced: 2 }), lag: /** @type {any} */ ({ generation: 9 }), held: 3 });
  assert.deepEqual(scoped, { at, feed: FEED, kind: 'scoped', why: 'ALPHA', since: 's', rows: 3, added: 1, replaced: 2, generation: 9, held: 3, worker: null });
});
