// READ-02 (R4-05): the reasons dictionary as a page reads it. Every fixture
// is invented: the rows below are shaped like /reasons.json's, and the codes
// `zz_*` exist in no source.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  REASONS_ROUTE, REASONS_SCHEMA, actionFor, actionWordsFor, cutTitle, forgetReasons, loadReasons,
  loadedReasons, readReasons, rowFor, titleFor, titleText
} from '../src/lib/reasons.js';
import { reasonWords } from '../src/lib/ingest-months.js';

/** One served row, as `pull::reasons::body` writes it. */
const row = (/** @type {string} */ code, /** @type {string} */ sentence, /** @type {string} */ control, action = 'an invented action') => ({
  code, sentence, action, control, control_kind: 'operator', step: null, title: `${code}: ${sentence}; ${control}`
});

const ROWS = [
  row('needs_month_generation', 'the month already passed this day', 'press Repair', 'run the import with a repair reason for this month'),
  row('before_listing', 'the bar falls before the share was listed on NSE', 'no action', 'nothing to do'),
  row('zz_long', `an invented sentence ${'that goes on '.repeat(20)}to the end`, 'operator: read it')
];

/** A whole `/reasons.json` body. @param {Record<string, unknown>} over */
const body = (over = {}) => ({
  schema: REASONS_SCHEMA,
  build: 'unstamped',
  count: ROWS.length,
  title_cut: Math.max(...ROWS.map((r) => Array.from(r.title).length)),
  rows: ROWS,
  ...over
});

/** A request stub that answers `answer` and counts its calls. @param {() => Promise<any>} answer */
const stub = (answer) => {
  const calls = /** @type {string[]} */ ([]);
  const request = async (/** @type {string} */ url) => {
    calls.push(url);
    return answer();
  };
  return { calls, request: /** @type {any} */ (request) };
};

const ok = (/** @type {any} */ json) => async () => ({ ok: true, status: 200, json: async () => json });

test('a table read from its body titles each code with the server words', () => {
  const table = readReasons(body());
  assert.equal(table.error, null);
  assert.equal(table.rows.size, 3);
  assert.equal(titleFor('needs_month_generation', table), 'the month already passed this day');
  assert.equal(actionFor('needs_month_generation', table), 'press Repair');
  assert.equal(actionWordsFor('needs_month_generation', table), 'run the import with a repair reason for this month');
  assert.equal(titleText('needs_month_generation', table), 'needs_month_generation: the month already passed this day; press Repair');
  // A code with no row: null, never a placeholder sentence (R4-01's contract).
  for (const code of ['zz_unknown', '', 'NEEDS_MONTH_GENERATION']) {
    assert.equal(titleFor(code, table), null, code);
    assert.equal(actionFor(code, table), null, code);
    assert.equal(actionWordsFor(code, table), null, code);
    assert.equal(titleText(code, table), null, code);
    assert.equal(rowFor(code, table), null, code);
  }
});

test('reasons_json_is_fetched_once_per_page', async () => {
  forgetReasons();
  // Before the answer: nothing is claimed, and no word says "loading".
  assert.equal(loadedReasons(), null);
  assert.equal(titleFor('needs_month_generation'), null);
  assert.equal(actionFor('needs_month_generation'), null);
  const { calls, request } = stub(ok(body()));
  const first = loadReasons(request);
  const again = loadReasons(request);
  assert.equal(first, again, 'one promise for every caller');
  const table = await first;
  await loadReasons(request);
  assert.deepEqual(calls, [REASONS_ROUTE], 'one request per page');
  assert.equal(loadedReasons(), table);
  // The loaded table is the default for every lookup.
  assert.equal(titleFor('before_listing'), 'the bar falls before the share was listed on NSE');
  assert.equal(actionFor('before_listing'), 'no action');
  assert.equal(titleText('before_listing'), 'before_listing: the bar falls before the share was listed on NSE; no action');
  // R4-01's reasonWords takes this titleFor as it is.
  assert.equal(reasonWords({ code: 'before_listing' }, titleFor).title, 'the bar falls before the share was listed on NSE');
  forgetReasons();
  assert.equal(titleFor('before_listing'), null, 'forgotten until asked again');
});

test('a refused, thrown or foreign answer is a table with its reason, never a throw or a second request', async () => {
  const cases = [
    { answer: async () => ({ ok: false, status: 503, json: async () => ({}) }), error: '/reasons.json: HTTP 503' },
    { answer: async () => { throw new Error('the network is down'); }, error: '/reasons.json: the network is down' },
    { answer: async () => { throw 'a bare string'; }, error: '/reasons.json: a bare string' },
    { answer: async () => ({ ok: true, status: 200, json: async () => { throw new SyntaxError('Unexpected token <'); } }), error: '/reasons.json: Unexpected token <' },
    { answer: ok({ ...body(), schema: 'reasons-v2' }), error: 'unknown reasons schema "reasons-v2", page older than server' },
    { answer: ok({ rows: ROWS }), error: 'unknown reasons schema null, page older than server' },
    { answer: ok(null), error: 'the reasons table answered no JSON object' },
    { answer: ok('a string'), error: 'the reasons table answered no JSON object' }
  ];
  for (const { answer, error } of cases) {
    forgetReasons();
    const { calls, request } = stub(answer);
    const table = await loadReasons(request);
    await loadReasons(request);
    assert.equal(table.error, error);
    assert.equal(table.rows.size, 0, error);
    assert.equal(calls.length, 1, error);
    assert.equal(titleFor('needs_month_generation'), null, error);
  }
  forgetReasons();
});

test('a body that holds fewer usable rows than it counts says so and keeps the good ones', () => {
  // COMBINATION: garbage rows, a duplicate code and a count that disagrees.
  const table = readReasons(body({
    count: 5,
    rows: [ROWS[0], null, { code: 7 }, { code: 'zz_no_title', sentence: 's' }, ROWS[0], ROWS[1]]
  }));
  assert.equal(table.rows.size, 2);
  assert.equal(table.error, 'the reasons table says 5 rows and holds 2');
  assert.equal(titleFor('needs_month_generation', table), 'the month already passed this day');
  // Missing optional fields read as empty, never as invented words.
  const bare = readReasons(body({ count: 1, title_cut: null, rows: [{ code: 'zz_bare', sentence: 'an invented sentence', title: 'zz_bare: an invented sentence; ' }] }));
  assert.deepEqual(rowFor('zz_bare', bare), {
    code: 'zz_bare', sentence: 'an invented sentence', action: '', control: '', control_kind: '', step: null,
    title: 'zz_bare: an invented sentence; '
  });
  assert.equal(bare.cut, 0, 'no title_cut served: nothing is cut');
  assert.equal(readReasons(body({ rows: 'not a list', count: 0 })).rows.size, 0);
});

test('a title longer than the table holds is cut at a word, and the sentence stays whole', () => {
  // The served cut is the table's own longest title: none of its titles is cut.
  const table = readReasons(body());
  for (const r of ROWS) assert.equal(titleText(r.code, table), r.title, r.code);
  // A server whose row is longer than its own cut (an inconsistent answer,
  // or a sentence a later package wrote too long): cut at a word, with an
  // ellipsis, in the title only.
  const tight = readReasons(body({ title_cut: 40 }));
  const cut = /** @type {string} */ (titleText('zz_long', tight));
  assert.ok(Array.from(cut).length <= 40, cut);
  assert.ok(cut.endsWith('…'), cut);
  assert.ok(!cut.slice(0, -1).endsWith(' '), cut);
  assert.ok(ROWS[2].title.startsWith(cut.slice(0, -1)), 'a prefix of the title, cut at a word');
  assert.equal(titleFor('zz_long', tight), ROWS[2].sentence, 'whole');
});

test('cutTitle cuts at a word, counts characters, and never empties a title', () => {
  assert.equal(cutTitle('short', 10), 'short');
  assert.equal(cutTitle('exactly ten', 11), 'exactly ten');
  assert.equal(cutTitle('one two three four', 12), 'one two…');
  // A first word longer than the bound is cut inside it.
  assert.equal(cutTitle('abcdefghijklmnop', 5), 'abcd…');
  // Characters, not UTF-16 units: a rupee sign and an emoji count one each.
  assert.equal(cutTitle('₹₹₹ 😀😀😀 tail', 8), '₹₹₹…');
  assert.equal(cutTitle('😀😀😀😀😀😀', 4), '😀😀😀…');
  // No bound, a zero, a negative or a non-integer bound: whole.
  for (const max of [0, -1, Number.NaN, 2.5, Number.POSITIVE_INFINITY]) {
    assert.equal(cutTitle('one two three', max), 'one two three', String(max));
  }
  assert.equal(cutTitle('', 3), '');
});

test('titleFor is one Map lookup at any table size', () => {
  // MAXIMUM: 100,000 rows; p50/p99/max over n = 200 sequential lookups,
  // half known and half not.
  const many = Array.from({ length: 100_000 }, (_, i) => row(`zz_code_${i}`, `an invented sentence ${i}`, 'no action'));
  const table = readReasons(body({ rows: many, count: many.length }));
  assert.equal(table.rows.size, 100_000);
  const took = [];
  for (let n = 0; n < 200; n += 1) {
    const code = n % 2 === 0 ? `zz_code_${(n * 499) % 100_000}` : `zz_absent_${n}`;
    const start = performance.now();
    const found = titleFor(code, table);
    took.push(performance.now() - start);
    assert.equal(found === null, n % 2 === 1, code);
  }
  took.sort((a, b) => a - b);
  const [p50, p99, max] = [took[99], took[197], took[199]];
  console.log(`titleFor n=200 over 100,000 rows: p50=${p50.toFixed(4)}ms p99=${p99.toFixed(4)}ms max=${max.toFixed(4)}ms`);
  assert.ok(p99 < 1, `p99 ${p99} ms`);
});

test('the module writes no sentence of its own', () => {
  // The words are the server's: this file holds no row, and its lookups
  // are Map reads.
  const src = readFileSync(new URL('../src/lib/reasons.js', import.meta.url), 'utf8');
  assert.doesNotMatch(src, /the month already passed this day/);
  assert.match(src, /table\?\.rows\.get\(code\)/);
});
