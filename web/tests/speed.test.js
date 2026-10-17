// THE SPEED PAGE, THE SPEED SHEET, THE BROWSER HARNESS, THE PAGE BUDGET AND
// THE SHARED REPORTER (R3-15).
//
// Before R3-15 none of this existed: `/speed.json` had no reader, the speed
// sheet was a hand-run script that typed its own URLs, no page's size was
// budgeted, the header's probe pulled the whole `/feeds.json` body every 8 s
// on every tab, and a page's failure never reached `/page-report`. Each named
// test below fails on that tree (its module or its rule is absent) and passes
// on this one.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, mkdtempSync, mkdirSync, writeFileSync, readdirSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, relative, sep } from 'node:path';
import http from 'node:http';

import {
  PAGE_TARGETS, MEASURED_PAGES, SPEED_ROUTE, WINDOWS, SORTS, GARBLED, BUNDLE_BUDGETS,
  readSpeedReport, microsText, speedRows, sortRows, speedTile, summary, ignoredFields,
  garbledUrl, budgetOf, builtPages, budgetVerdict, pageSelf
} from '../src/lib/speed.js';
import {
  REPORT_ROUTE, MAX_REPORT_BYTES, MAX_PAGE_BYTES, REPORTS_PER_MINUTE, MINUTE_MS, FALLBACK_EVENT,
  reportBody, outcomeOf, createPageReporter, installPageReport, reportFallback
} from '../src/lib/page-report.js';
import { upCheck, apiTitle, UP_ROUTE, UP_METHOD, UP_CHECKING } from '../src/lib/api-status.js';
import * as sheet from '../ci/speed-sheet.mjs';
import * as harness from '../ci/browser-harness.mjs';
import { windowQuery } from '../src/lib/db/bars-plan.js';
import { dayflagsUrl } from '../src/lib/db/dayflags.js';
import { rungsUrl } from '../src/lib/sources.js';

const web = new URL('../', import.meta.url);
/** @param {string} path */
const source = (path) => readFileSync(new URL(path, web), 'utf8');
const flush = () => new Promise((resolve) => setTimeout(resolve, 0));

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/** @param {Partial<import('../src/lib/speed.js').Figures>} [f] */
const figures = (f = {}) => ({ n: 0, answered: 0, refused: 0, busy: 0, failed: 0, cancelled: 0, p50_micros: 0, p99_micros: 0, max_micros: 0, ...f });

/** @param {string} route @param {object} [o] */
const row = (route, o = {}) => ({
  route,
  kind: 'route',
  since_start: figures(),
  last_minute: figures(),
  last_hour: figures(),
  last_hour_minutes: 0,
  first_after_start_micros: null,
  ...o
});

/** @param {any[]} rows */
const report = (rows) => ({ started_ms: 1_791_600_000_000, now_ms: 1_791_600_060_000, rows, over_cap: 0, resolution: 'buckets' });

const CTX = { feed: 'zerodha', instrument: 'NSE-INDEX-NIFTY', timeframe: '1min', month: '2026-09', day: '2026-09-30', from: '2026-09-01', to: '2026-09-30' };

// ---------------------------------------------------------------------------
// The four named tests
// ---------------------------------------------------------------------------

test('the_speed_sheet_uses_the_pages_own_requests', async () => {
  const sheetSource = source('ci/speed-sheet.mjs');
  const catalogue = sheetSource.slice(sheetSource.indexOf('const ENTRIES = ['), sheetSource.indexOf('export const CATALOGUE'));
  assert.ok(catalogue.length > 1_000, 'the catalogue was found');
  // NO URL IS TYPED BY HAND: no query string is spelled in the catalogue.
  assert.doesNotMatch(catalogue, /\.json\?[a-z_]+=/, 'the catalogue spells a query instead of calling a builder');
  for (const e of sheet.CATALOGUE) {
    const url = e.make(CTX);
    assert.ok(url !== null, `${e.route} built nothing for a held selection`);
    assert.equal(new URL(url, 'http://x').pathname, e.route, `${e.route}'s line builds ${url}`);
    const user = source(e.usedBy);
    if (e.builder === null) {
      assert.ok(!url.includes('?'), `${e.route} carries a query without a builder`);
      assert.ok(user.includes(`${e.route}'`) || user.includes(`${e.route}\``) || user.includes(`UP_ROUTE`) || user.includes('SPEED_ROUTE'), `${e.usedBy} does not ask ${e.route}`);
      continue;
    }
    // THE SAME FUNCTION THE PAGE CALLS, from the page's own library...
    assert.ok(e.from, `${e.route} names no library`);
    const lib = await import(new URL(e.from, web).href);
    assert.equal(lib[e.builder.name], e.builder, `${e.builder.name} is not ${e.from}'s own export`);
    assert.match(sheetSource, new RegExp(`import \\{[^}]*\\b${e.builder.name}\\b[^}]*\\} from '\\.\\./${e.from.replace(/[.$]/g, '\\$&')}'`));
    // ...and the page file really calls it.
    assert.ok(user.includes(`${e.builder.name}(`), `${e.usedBy} never calls ${e.builder.name}`);
  }
  // Byte for byte what the page's builder answers for the same selection.
  const plan = { keys: [CTX.instrument], timeframe: CTX.timeframe, from: CTX.month, to: CTX.month };
  const { requests, skipped } = sheet.requestsFor(CTX);
  assert.deepEqual(skipped, []);
  assert.ok(requests.some((r) => r.url === `/bars/window.json?${windowQuery(CTX.feed, plan, { sort: 'ts', desc: true, offset: 0 })}`));
  assert.ok(requests.some((r) => r.url === dayflagsUrl(CTX.feed, CTX.instrument, CTX.from, CTX.to)));
  assert.ok(requests.some((r) => r.url === rungsUrl(CTX.instrument)));
  assert.ok(requests.some((r) => r.method === UP_METHOD && r.url === UP_ROUTE), 'the header-only up check is timed as the page sends it');
  // A page that builds its URL inline is replayed VERBATIM from the harness's capture.
  const capture = JSON.stringify({ requests: [{ page: '/gaps', method: 'GET', url: '/gaps.json?feed=zerodha&symbol=NIFTY' }, { page: '/db', method: 'GET', url: '/feeds.json' }] });
  const withCapture = sheet.requestsFor(CTX, sheet.readCapture(capture));
  const replay = withCapture.requests.filter((r) => r.builder === '(the page, captured)');
  assert.deepEqual(replay.map((r) => [r.page, r.url]), [['/gaps', '/gaps.json?feed=zerodha&symbol=NIFTY']], 'replayed verbatim, a request already built once is not timed twice');
  assert.throws(() => sheet.readCapture(JSON.stringify({ requests: [{ page: '/ingest', method: 'POST', url: '/pull' }] })), /replays reads only/);
  assert.throws(() => sheet.readCapture(JSON.stringify({ requests: [{ page: '/x', method: 'GET', url: 'http://elsewhere/x.json' }] })), /not a path of this server/);
  assert.throws(() => sheet.readCapture(JSON.stringify({ requests: [{ page: '/x', method: 'GET', url: '//elsewhere/x.json' }] })), /not a path of this server/);
  assert.throws(() => sheet.readCapture(JSON.stringify({ requests: [{ url: '/x' }] })), /method-less/);
  assert.throws(() => sheet.readCapture('{}'), /no requests list/);
  assert.equal(sheet.readCapture(JSON.stringify({ requests: [{ method: 'head', url: '/x' }] }))[0].page, '?');
  // A builder that answers null for a selection is said, never dropped in silence.
  const none = sheet.requestsFor({ ...CTX, instrument: 'not a key' });
  assert.ok(none.skipped.some((s) => s.includes('rungsUrl built no request')), none.skipped.join('\n'));
});

test('a_request_with_an_ignored_field_fails_the_run', async () => {
  // A stand-in server that reads every field but `limit` on /bars/window.json:
  // a garbled `limit` is answered with the same bytes as the real one.
  const server = standIn({ ignore: new Set(['limit']) });
  const result = await sheet.runSheet(argsFor(), { send: server.send, log: () => {} });
  assert.equal(result.ok, false);
  const ignoredLimit = result.failures.filter((f) => f.includes('field limit was ignored (garbled'));
  assert.ok(ignoredLimit.length > 0, result.failures.join('\n'));
  assert.ok(ignoredLimit.some((f) => f.includes('/bars/window.json?')), 'the /db grid read that sent it is named');
  assert.ok(ignoredLimit.every((f) => /[?&]limit=/.test(f)), 'only a request that sent the field is named');
  // The same run against a server that refuses every value it cannot read passes.
  const strict = standIn({ ignore: new Set() });
  const clean = await sheet.runSheet(argsFor(), { send: strict.send, log: () => {} });
  assert.deepEqual(clean.failures, []);
  assert.equal(clean.ok, true);
  // The server's own word is enough on its own: an `ignored` list, or a 400 that says it does not read a field.
  assert.deepEqual(ignoredFields({ url: '/logs.json?level=eror', status: 200, body: JSON.stringify({ ignored: [{ param: 'level', asked: 'eror' }] }) }).map((i) => [i.field, i.how]), [['level', 'named']]);
  assert.deepEqual(ignoredFields({ url: '/imports.json?feed=gdfl&bogus=1', status: 400, body: JSON.stringify({ error: '/imports.json does not read bogus: refused' }) }).map((i) => [i.field, i.how]), [['bogus', 'refused']]);
});

test('a_failed_fetch_is_found_in_the_logs', async () => {
  // The browser's side: a page whose read of /bars.json gets no answer posts
  // the report through the original fetch, and a stand-in for the server's
  // route keeps it in a log the test then reads back through /logs.json.
  const log = /** @type {string[]} */ ([]);
  const win = fakeWindow(async (input, init) => {
    const url = String(input);
    if (url === REPORT_ROUTE && init?.method === 'POST') {
      const body = String(init.body);
      assert.equal(init.headers['content-type'], 'application/x-www-form-urlencoded');
      assert.ok(new TextEncoder().encode(body).length <= MAX_REPORT_BYTES);
      const form = new URLSearchParams(body);
      log.push(JSON.stringify({ target: 'api.page.access', message: 'page reported a failure', page: form.get('page'), what: form.get('what') }));
      return response(202);
    }
    if (url.startsWith('/logs.json')) return response(200, JSON.stringify({ lines: log }));
    throw new TypeError('Failed to fetch');
  });
  installPageReport(win);
  await assert.rejects(win.fetch('/bars.json?feed=zerodha&month=2026-09'), /Failed to fetch/);
  await flush();
  const read = await (await win.fetch('/logs.json?target=api.page.access')).json();
  const found = read.lines.map((/** @type {string} */ l) => JSON.parse(l)).find((/** @type {any} */ l) => l.what.includes('/bars.json?feed=zerodha&month=2026-09'));
  assert.ok(found, `the failed fetch is not in the log: ${JSON.stringify(read)}`);
  assert.equal(found.page, '/db');
  assert.equal(found.what, 'request: GET /bars.json?feed=zerodha&month=2026-09 got no answer: Failed to fetch');
  // The harness checks the same thing against the real server, end to end.
  const h = source('ci/browser-harness.mjs');
  assert.match(h, /Fetch\.failRequest/);
  assert.match(h, /\/logs\.json\?target=api\.page\.access/);
  // And the layout installs the reporter for every page with one line.
  assert.match(source('src/routes/+layout.svelte'), /^\s*import '\$lib\/page-report\.js';/m);
});

test('the_bundle_is_inside_its_budget', async () => {
  // The rule, on a built tree it can read: the committed web/build.
  const verdict = budgetVerdict(builtPages(readBuild(new URL('build/', web).pathname)));
  assert.ok(verdict.length >= 10, 'every page of the build is weighed');
  for (const v of verdict) assert.equal(v.over, false, v.line);
  for (const v of verdict.filter((v) => v.overTarget)) assert.ok(v.line.includes('over the 400.0 KB wire target'), 'an over-target page says so');
  // A page that grows past its budget fails the BUILD, through the plugin
  // vite.config.js runs over the client output.
  const config = (await import('../vite.config.js')).default;
  const plugin = /** @type {any} */ (config.plugins.find((/** @type {any} */ p) => p?.name === 'brutex-page-budget'));
  assert.ok(plugin, 'vite.config.js runs the page budget');
  const big = synthBuild({ pageBytes: BUNDLE_BUDGETS.default + 1 });
  const ctx = { error: (/** @type {string} */ m) => { throw new Error(m); }, warn: () => {} };
  const quiet = console.log;
  console.log = () => {};
  try {
    assert.throws(() => plugin.writeBundle.call(ctx, { dir: big }), /1 page\(s\) past their budget.*\/gaps: 400\.0 KB on first load.*budget 400\.0 KB — OVER ITS BUDGET/);
    assert.doesNotThrow(() => plugin.writeBundle.call(ctx, { dir: synthBuild({ pageBytes: 1_000 }) }));
    // The server build carries no client entry and is not weighed; neither is a build with no directory.
    assert.doesNotThrow(() => plugin.writeBundle.call(ctx, { dir: mkdtempSync(join(tmpdir(), 'r315-server-')) }));
    assert.doesNotThrow(() => plugin.writeBundle.call(ctx, {}));
  } finally {
    console.log = quiet;
  }
});

// ---------------------------------------------------------------------------
// $lib/speed.js
// ---------------------------------------------------------------------------

test('the targets and pages are the work package\'s', () => {
  assert.deepEqual(PAGE_TARGETS, { firstContentP99Ms: 300, longestPauseMs: 50, heapBytes: 100_000_000, wireBytes: 400_000 });
  for (const p of ['/db', '/ingest', '/gaps', '/audit', '/markets', '/sources', '/speed']) assert.ok(MEASURED_PAGES.includes(p));
  assert.equal(SPEED_ROUTE, '/speed.json');
  assert.deepEqual(WINDOWS.map((w) => w.key), ['last_hour', 'last_minute', 'since_start']);
});

test('a speed report is read field by field and a missing figure is refused by name', () => {
  const good = report([row('/bars.json', { since_start: figures({ n: 3, answered: 3, p50_micros: 10, p99_micros: 20, max_micros: 20 }), first_after_start_micros: 900 })]);
  assert.equal(readSpeedReport(good).rows[0].route, '/bars.json');
  /** @type {[unknown, RegExp][]} */
  const bad = [
    [null, /not a report object/],
    [[], /not a report object/],
    [{ ...good, started_ms: -1 }, /whole started_ms/],
    [{ ...good, now_ms: 1.5 }, /whole now_ms/],
    [{ ...good, resolution: '' }, /resolution sentence/],
    [{ ...good, rows: {} }, /rows list/],
    [report([null]), /row 0 has no route name/],
    [report([['/a']]), /row 0 has no route name/],
    [report([row('')]), /row 0 has no route name/],
    [report([row('/a', { kind: 'Route' })]), /kind "Route", not route or stage/],
    [report([row('/a', { last_minute: null })]), /\/a has no last_minute window/],
    [report([row('/a', { last_hour: figures({ p99_micros: undefined }) })]), /\/a last_hour has no whole p99_micros/],
    [report([row('/a', { last_hour_minutes: '3' })]), /no whole last_hour_minutes/],
    [report([row('/a', { first_after_start_micros: -2 })]), /neither null nor whole/]
  ];
  for (const [body, why] of bad) assert.throws(() => readSpeedReport(body), why);
});

test('a time is printed in its unit, and a window that holds nothing is a dash, never 0 µs', () => {
  assert.equal(microsText(0, 0), '—');
  assert.equal(microsText(Number.NaN), '—');
  assert.equal(microsText(999), '999 µs');
  assert.equal(microsText(1_000), '1.00 ms', 'a millisecond is said in ms');
  assert.equal(microsText(1_234), '1.23 ms');
  assert.equal(microsText(9_999), '10.00 ms');
  assert.equal(microsText(10_000), '10.0 ms', 'from ten ms, one decimal');
  assert.equal(microsText(45_678), '45.7 ms');
  assert.equal(microsText(999_999), '1000.0 ms');
  assert.equal(microsText(1_000_000), '1.00 s', 'a second is said in s');
  assert.equal(microsText(2_500_000), '2.50 s');
});

test('the table keeps the server\'s order, filters by kind, marks a p99 past the target and sorts stably', () => {
  const r = report([
    row('/a', { last_hour: figures({ n: 2, failed: 1, p50_micros: 5, p99_micros: 400_000, max_micros: 500_000 }), last_hour_minutes: 12 }),
    row('stage:vendor scrub', { kind: 'stage', last_hour: figures({ n: 9, p99_micros: 10 }) }),
    row('/b', { last_hour: figures({ n: 9, p99_micros: 300_000 }) }),
    row('/c'),
    row('/d', { last_hour: figures({ n: 0, p99_micros: 400_000 }) })
  ]);
  assert.equal(speedRows(r, 'last_hour').at(-1)?.slow, false, 'a window with no request is never slow, whatever its figures');
  r.rows.pop();
  const all = speedRows(r, 'last_hour');
  assert.deepEqual(all.map((x) => x.route), ['/a', 'stage:vendor scrub', '/b', '/c']);
  assert.deepEqual(all.map((x) => x.slow), [true, false, false, false], 'past 300 ms only; exactly 300 ms is inside, an empty window is never slow');
  assert.equal(all[0].minutes, 12);
  assert.deepEqual(speedRows(r, 'last_hour', 'stage').map((x) => x.route), ['stage:vendor scrub']);
  assert.deepEqual(speedRows(r, 'last_hour', 'route').map((x) => x.route), ['/a', '/b', '/c']);
  assert.throws(() => speedRows(r, 'yesterday'), /no speed window is named "yesterday"/);
  assert.deepEqual(sortRows(all, 'p99').map((x) => x.route), ['/a', '/b', 'stage:vendor scrub', '/c']);
  assert.deepEqual(sortRows(all, 'n').map((x) => x.route), ['stage:vendor scrub', '/b', '/a', '/c']);
  assert.deepEqual(sortRows([all[2], all[0], all[3], all[0]], 'route').map((x) => x.route), ['/a', '/a', '/b', '/c']);
  const twins = [{ ...all[0], order: 7 }, { ...all[0], order: 3 }, { ...all[0], order: 5 }];
  assert.deepEqual(sortRows(twins, 'route').map((x) => x.order), [3, 5, 7], 'equal names keep the order first asked');
  assert.deepEqual(sortRows(all, 'order').map((x) => x.route), all.map((x) => x.route));
  assert.deepEqual(SORTS.map((s) => s.key), ['order', 'p99', 'n', 'route']);
});

test('the speed tile names the slowest route and every failure, over the last hour or since the start', () => {
  assert.equal(speedTile(report([])).state, 'empty');
  assert.match(speedTile(report([row('/a')])).line, /No route has been asked/);
  const early = speedTile(report([row('/a', { since_start: figures({ n: 4, p99_micros: 900 }) })]));
  assert.equal(early.window, 'since_start', 'no minute frozen yet: since the start');
  assert.equal(early.state, 'ok');
  assert.equal(early.line, '1 routes, 4 requests (since the start); slowest p99 /a 900 µs');
  const slow = speedTile(report([row('/a', { last_hour: figures({ n: 4, p99_micros: 2_000 }) }), row('/b', { last_hour: figures({ n: 1, p99_micros: 301_000 }) }), row('stage:x', { kind: 'stage', last_hour: figures({ n: 1, p99_micros: 9_000_000 }) })]));
  assert.equal(slow.state, 'slow');
  assert.deepEqual(slow.worst, { route: '/b', p99: 301_000 }, 'a stage is not a page\'s wait');
  const mixed = speedTile(report([row('/a', { last_hour: figures({ n: 4, p99_micros: 10 }) }), row('/b', { since_start: figures({ n: 9, p99_micros: 99 }) })]));
  assert.equal(mixed.window, 'last_hour', 'one route asked in the last hour is enough for the hour');
  assert.equal(mixed.routes, 1);
  const failing = speedTile(report([row('/a', { last_hour: figures({ n: 4, failed: 2, p99_micros: 10 }) })]));
  assert.equal(failing.state, 'failing');
  assert.match(failing.line, /; 2 failed$/);
});

test('a summary is nearest rank, so every figure is a time that was measured', () => {
  assert.deepEqual(summary([]), { n: 0, p50: 0, p99: 0, max: 0 });
  assert.deepEqual(summary([7]), { n: 1, p50: 7, p99: 7, max: 7 });
  const hundred = Array.from({ length: 100 }, (_, i) => 100 - i);
  assert.deepEqual(summary(hundred), { n: 100, p50: 50, p99: 99, max: 100 });
  assert.deepEqual(summary([3, 1, 2]), { n: 3, p50: 2, p99: 3, max: 3 });
});

test('a garbled field is the field only, every occurrence, and the same bytes alone are evidence', () => {
  const g = new URLSearchParams({ g: GARBLED }).toString().slice(2);
  assert.equal(garbledUrl('/x.json?a=1&b=2&b=3', 'b'), `/x.json?a=1&b=${g}&b=${g}`);
  assert.equal(garbledUrl('/x.json?a=1', 'z'), `/x.json?a=1&z=${g}`);
  assert.equal(new URL(garbledUrl('/x.json?a=1', 'a'), 'http://x').searchParams.get('a'), GARBLED, 'the server decodes it back to the garbled word');
  const base = { url: '/x.json?a=1&b=2', status: 200, body: '{"rows":[]}' };
  assert.deepEqual(ignoredFields({ ...base, garbled: { a: { status: 400, body: 'no' }, b: { status: 200, body: '{"rows":[]}' } } }).map((i) => [i.field, i.how]), [['b', 'garbled']]);
  assert.deepEqual(ignoredFields({ ...base, garbled: { b: { status: 200, body: '{"rows":[1]}' } } }), [], 'a different answer read the field');
  assert.deepEqual(ignoredFields({ ...base, status: 404, garbled: { b: { status: 404, body: '{"rows":[]}' } } }), [], 'a refused original says nothing about a field');
  assert.deepEqual(ignoredFields({ ...base, status: 404, garbled: { b: { status: 200, body: '{"rows":[]}' } } }), [], 'nor does a refused original beside an answered probe');
  assert.deepEqual(ignoredFields({ ...base, status: 300, garbled: { b: { status: 300, body: '{"rows":[]}' } } }), [], 'a 300 is not an answer');
  assert.deepEqual(ignoredFields({ ...base, garbled: { b: { status: 300, body: '{"rows":[]}' } } }), [], 'nor is a probe answered 300');
  assert.deepEqual(ignoredFields({ ...base, status: 299, garbled: { b: { status: 299, body: '{"rows":[]}' } } }).map((i) => i.how), ['garbled'], '299 is 2xx');
  assert.deepEqual(ignoredFields({ url: '/x.json?a=1', status: 400, body: '{"error":7,"why":"a is not a parameter"}' }).map((i) => i.how), ['refused'], 'an error that is not a sentence leaves the body to read');
  assert.deepEqual(ignoredFields({ url: '/x.json?a=1', status: 200, body: 'not json' }), []);
  assert.deepEqual(ignoredFields({ url: '/x.json?a=1', status: 200, body: '{"ignored":["a",{"field":"b"},{}]}' }).map((i) => i.field), ['a', 'b', '?']);
  assert.deepEqual(ignoredFields({ url: '/x.json?a=1', status: 400, body: 'a is not a parameter here' }).map((i) => i.how), ['refused']);
  assert.deepEqual(ignoredFields({ url: '/x.json?a=1', status: 400, body: '{"error":"a must be a day"}' }), [], 'a refused value is a read field');
});

test('each page has a budget, and the over-target pages are named', () => {
  assert.equal(budgetOf('/gaps'), 400_000);
  assert.equal(budgetOf('/db'), 750_000);
  const v = budgetVerdict([{ route: '/db', node: 6, js: 600_000, css: 100_000, bytes: 700_000, files: [] }, { route: '/gaps', node: 7, js: 1, css: 1, bytes: 400_001, files: [] }, { route: '/audit', node: 3, js: 1, css: 1, bytes: 400_000, files: [] }]);
  assert.deepEqual(v.map((x) => [x.over, x.overTarget]), [[false, true], [true, true], [false, false]], 'exactly the budget, and exactly the target, are inside');
  assert.match(v[1].line, /— OVER ITS BUDGET — over the 400\.0 KB wire target/);
});

test('the budget refuses a build it cannot read, naming what was missing', () => {
  /** @param {Record<string, string | number>} entries */
  const files = (entries) => new Map(Object.entries(entries).map(([k, v]) => [k, typeof v === 'number' ? { size: v } : { size: v.length, text: v }]));
  assert.throws(() => builtPages(files({})), /no _app\/immutable\/entry\/app/);
  assert.throws(() => builtPages(files({ '_app/immutable/entry/app.A.js': 'x' })), /no __vite__mapDeps/);
  const deps = (/** @type {string} */ rest) => `const __vite__mapDeps=(i,m=__vite__mapDeps,d=(m.f||(m.f=["/_app/immutable/nodes/0.A.js","/_app/immutable/nodes/7.A.js"])))=>i.map(i=>d[i]);${rest}`;
  assert.throws(() => builtPages(files({ '_app/immutable/entry/app.A.js': deps('') })), /no root layout node/);
  assert.throws(() => builtPages(files({ '_app/immutable/entry/app.A.js': deps('import(`../nodes/0.A.js`),__vite__mapDeps([0])') })), /no route table/);
  assert.throws(() => builtPages(files({ '_app/immutable/entry/app.A.js': deps('import(`../nodes/0.A.js`),__vite__mapDeps([0]);const d={"/gaps":[7]}') })), /no preload list for \/gaps/);
  assert.throws(() => builtPages(files({ '_app/immutable/entry/app.A.js': deps('import(`../nodes/0.A.js`),__vite__mapDeps([0]);import(`../nodes/7.A.js`),__vite__mapDeps([1]);const d={"/gaps":[7]}') })), /could not find _app\/immutable\/nodes\/0\.A\.js, which \/gaps loads/);
  const ok = builtPages(files({
    '_app/immutable/entry/app.A.js': deps('import"../chunks/c.js";import(`../nodes/0.A.js`),__vite__mapDeps([0]);import(`../nodes/7.A.js`),__vite__mapDeps([1]);import(`../nodes/8.A.js`),__vite__mapDeps([]);const d={"/gaps":[7],"/e":[~8]}'),
    '_app/immutable/entry/start.A.js': 'import{a}from"./app.A.js";',
    '_app/immutable/chunks/c.js': 'import"./c.js";import"./d.js";',
    '_app/immutable/chunks/d.js': 5,
    '_app/immutable/nodes/0.A.js': 10,
    '_app/immutable/nodes/7.A.js': 20
  }));
  assert.deepEqual(ok.map((p) => p.route), ['/gaps', '/e']);
  assert.equal(ok[0].css, 0);
  assert.ok(ok[0].files.includes('_app/immutable/chunks/d.js'), 'a chunk imported by a chunk is on the first load');
  assert.equal(ok[1].bytes, ok[0].bytes - 20, 'a page with an empty preload list ships the shared base alone');
});

test('a page reads its own load from the performance timeline, and says null for what the browser did not give', () => {
  assert.deepEqual(pageSelf(null), { firstContentMs: null, wireBytes: null, resources: 0, heapBytes: null });
  /** @param {Record<string, any[]>} by @param {any} [memory] */
  const perf = (by, memory) => ({ getEntriesByType: (/** @type {string} */ t) => by[t] ?? [], memory });
  assert.deepEqual(
    pageSelf(perf({ paint: [{ name: 'first-paint', startTime: 1 }, { name: 'first-contentful-paint', startTime: 120.4 }], navigation: [{ transferSize: 1_000 }], resource: [{ transferSize: 500 }, { transferSize: undefined }, { transferSize: Number.NaN }] }, { usedJSHeapSize: 4_000_000 })),
    { firstContentMs: 120, wireBytes: 1_500, resources: 4, heapBytes: 4_000_000 }
  );
  assert.deepEqual(pageSelf(perf({ paint: [{ name: 'first-contentful-paint', startTime: Number.NaN }] })), { firstContentMs: null, wireBytes: null, resources: 0, heapBytes: null });
  assert.equal(pageSelf(perf({}, { usedJSHeapSize: Number.NaN })).heapBytes, null, 'a heap that is not a number is not reported');
});

// ---------------------------------------------------------------------------
// $lib/page-report.js
// ---------------------------------------------------------------------------

test('a report body fits the server\'s 1 KiB, is cut at a character, and a report the server would refuse is never sent', () => {
  assert.equal(reportBody('/db', 'error: x'), 'page=%2Fdb&what=error%3A%20x');
  const long = reportBody('/db', `${'é'.repeat(2_000)}`);
  assert.ok(long !== null && new TextEncoder().encode(long).length <= MAX_REPORT_BYTES);
  assert.ok(decodeURIComponent(String(long).split('what=')[1]).endsWith('é…'));
  assert.equal(reportBody('db', 'x'), null, 'not a path');
  assert.equal(reportBody(`/${'a'.repeat(MAX_PAGE_BYTES)}`, 'x'), null, 'past 256 bytes');
  assert.equal(reportBody('/db', '   '), null, 'nothing said');
  assert.equal(reportBody('/d\u0000b', 'a\tb\nc\u0007d\u0085e'), `page=${encodeURIComponent('/d b')}&what=${encodeURIComponent('a\tb\nc d e')}`);
  assert.equal(reportBody(/** @type {any} */ (null), /** @type {any} */ (undefined)), null);
  assert.equal(reportBody('/d\nb', 'a\u007fb\u00a0c'), `page=${encodeURIComponent('/d b')}&what=${encodeURIComponent('a b\u00a0c')}`, 'DEL is a control character, a no-break space is not, and a page keeps no newline');
  const edge = `/${'a'.repeat(MAX_PAGE_BYTES - 1)}`;
  assert.ok(reportBody(edge, 'x') !== null, 'a page of exactly 256 bytes is a page');
  const room = MAX_REPORT_BYTES - 'page=%2Fdb&what='.length;
  assert.equal(reportBody('/db', 'a'.repeat(room)), `page=%2Fdb&what=${'a'.repeat(room)}`, 'a report that fits exactly is sent whole');
  assert.equal(decodeURIComponent(String(reportBody('/db', 'a'.repeat(2_000))).split('what=')[1]), `${'a'.repeat(room - 9)}…`, 'the longest prefix whose ellipsis fits exactly');
});

test('an outcome is the server\'s word for it', () => {
  assert.equal(outcomeOf({ status: 200 }), 'answered');
  assert.equal(outcomeOf({ status: 304 }), 'answered');
  assert.equal(outcomeOf({ status: 404 }), 'refused');
  assert.equal(outcomeOf({ status: 399 }), 'answered');
  assert.equal(outcomeOf({ status: 400 }), 'refused');
  assert.equal(outcomeOf({ status: 429 }), 'busy');
  assert.equal(outcomeOf({ status: 503, retryAfter: true }), 'busy');
  assert.equal(outcomeOf({ status: 503 }), 'failed');
  assert.equal(outcomeOf({ status: 500 }), 'failed');
  assert.equal(outcomeOf({ error: new DOMException('stop', 'AbortError') }), 'cancelled');
  assert.equal(outcomeOf({ error: new DOMException('late', 'TimeoutError') }), 'failed');
  assert.equal(outcomeOf({ error: new TypeError('Failed to fetch') }), 'failed');
  assert.equal(outcomeOf({ error: null }), 'failed');
});

test('one report per distinct failure a minute, at most ten a minute, and what was held back is carried', async () => {
  let now = 0;
  /** @type {string[]} */
  const sent = [];
  const r = createPageReporter({ send: async (b) => void sent.push(b), page: () => '/db', now: () => now });
  assert.equal(r.report('error', 'boom'), true);
  assert.equal(r.report('error', 'boom'), false, 'the same failure again inside the minute');
  for (let i = 0; i < REPORTS_PER_MINUTE + 3; i += 1) r.report('request', `GET /x${i}`);
  await flush();
  assert.equal(sent.length, REPORTS_PER_MINUTE);
  assert.deepEqual(r.counts(), { sent: REPORTS_PER_MINUTE, heldBack: 4, sendFailures: 0 });
  now = MINUTE_MS;
  assert.equal(r.report('error', 'boom'), true, 'a new minute, the same failure is said again');
  await flush();
  assert.match(decodeURIComponent(sent.at(-1) ?? ''), /error: boom \[4 more held back by this tab's ration before this one\]/);
  // An entry kept at a roll is one minute old at the next report, exactly: it is said again.
  let t = 0;
  /** @type {string[]} */
  const again = [];
  const edgeR = createPageReporter({ send: async (b) => void again.push(b), page: () => '/db', now: () => t });
  edgeR.report('error', 'x');
  t = 30_000;
  edgeR.report('error', 'y');
  t = 60_000;
  edgeR.report('error', 'z');
  t = 90_000;
  assert.equal(edgeR.report('error', 'y'), true, 'y, sent at 30 s, is a minute old at 90 s');
  t = 90_001;
  assert.equal(edgeR.report('error', 'y'), false);
  const none = createPageReporter({ send: async () => {}, page: () => '/db', perMinute: 0 });
  assert.equal(none.report('error', 'x'), false, 'a ration of nothing sends nothing');
  const pathless = createPageReporter({ send: async (b) => void sent.push(b), page: () => 'no-slash', now: () => now });
  assert.equal(pathless.report('error', 'x'), false, 'a report the server would refuse is not sent');
  assert.equal(pathless.counts().sent, 0);
});

test('a report that cannot be sent is said on the console once a minute, never looped', async () => {
  let now = 0;
  /** @type {string[]} */
  const warned = [];
  const r = createPageReporter({ send: async () => { throw new Error('HTTP 503'); }, page: () => '/db', now: () => now, warn: (l) => warned.push(l) });
  r.report('error', 'a');
  r.report('error', 'b');
  await flush();
  assert.equal(warned.length, 1);
  assert.match(warned[0], /\/page-report could not take the report "error: a" \(HTTP 503\); 1 reports unsent/);
  now = MINUTE_MS;
  r.report('error', 'c');
  await flush();
  assert.equal(warned.length, 2);
  assert.equal(r.counts().sendFailures, 3);
  const quiet = console.error;
  /** @type {string[]} */
  const consoled = [];
  console.error = (/** @type {string} */ l) => void consoled.push(l);
  try {
    createPageReporter({ send: async () => { throw 'down'; }, page: () => '/db' }).report('error', 'z');
    createPageReporter({ send: async () => { throw new Error(''); }, page: () => '/db' }).report('error', 'e');
    await flush();
  } finally {
    console.error = quiet;
  }
  assert.match(consoled[0], /\(down\)/);
  assert.ok(consoled[1].includes('"error: e" ()'), 'an error with an empty message is said as empty, not as its type');
});

test('installed, fetch reports a failure and no ordinary answer, never itself, and uninstalls cleanly', async () => {
  /** @type {{url: string, init: any}[]} */
  const calls = [];
  /** @type {(input: any, init?: any) => Promise<any>} */
  const network = async (input, init) => {
    const url = typeof input === 'string' ? input : input instanceof URL ? input.pathname : String(input.url);
    calls.push({ url, init });
    if (url === REPORT_ROUTE) return response(url.includes('x') ? 500 : 202);
    if (url.includes('fail')) return response(500);
    if (url.includes('busy')) return response(503, '', { 'retry-after': '1' });
    if (url.includes('gone')) return response(404);
    if (url.includes('abort')) throw new DOMException('the page left', 'AbortError');
    if (url.includes('net')) throw new TypeError('Failed to fetch');
    if (url.includes('blank')) throw new Error('');
    return response(200);
  };
  const win = fakeWindow(network);
  const original = win.fetch;
  const off = installPageReport(win, { perMinute: 50 });
  assert.notEqual(win.fetch, original);
  assert.equal(installPageReport(win)(), undefined, 'a second install adds nothing');
  await win.fetch('/ok.json');
  await win.fetch('/busy.json');
  await win.fetch('/gone.json');
  await assert.rejects(win.fetch('/abort.json'));
  await win.fetch('/fail.json', { method: 'post' });
  await win.fetch(/** @type {any} */ ({ url: '/fail-request.json', method: 'PUT' }));
  await win.fetch(new URL('http://localhost/fail-url.json'));
  await assert.rejects(win.fetch('/net.json'), /Failed to fetch/);
  await assert.rejects(win.fetch('/blank.json'));
  await win.fetch(REPORT_ROUTE, { method: 'POST', body: 'page=/x&what=y' });
  await flush();
  const reports = calls.filter((c) => c.url === REPORT_ROUTE && c.init?.keepalive === true).map((c) => new URLSearchParams(c.init.body).get('what'));
  assert.deepEqual(reports, [
    'request: POST /fail.json answered HTTP 500',
    'request: PUT /fail-request.json answered HTTP 500',
    'request: GET http://localhost/fail-url.json answered HTTP 500',
    'request: GET /net.json got no answer: Failed to fetch',
    'request: GET /blank.json got no answer:'
  ]);
  win.emit('error', { message: 'x is undefined', filename: '/_app/a.js', lineno: 3, colno: 9 });
  win.emit('error', { error: 'bare' });
  win.emit('error', {});
  win.emit('unhandledrejection', { reason: new Error('lost') });
  win.emit('unhandledrejection', { reason: 'plain' });
  win.emit('unhandledrejection', { reason: new Error('') });
  win.emit('error', { message: 'at zero', filename: '/z.js', lineno: 0, colno: 0 });
  win.emit(FALLBACK_EVENT, { detail: '' });
  win.emit(FALLBACK_EVENT, { detail: 'the census answered from its prefix sums' });
  win.emit(FALLBACK_EVENT, {});
  await flush();
  const more = calls.filter((c) => c.url === REPORT_ROUTE && c.init?.keepalive === true).map((c) => new URLSearchParams(c.init.body).get('what')).slice(5);
  assert.deepEqual(more, [
    'error: x is undefined at /_app/a.js:3:9',
    'error: bare',
    'error: an uncaught error',
    'error: unhandled rejection: lost',
    'error: unhandled rejection: plain',
    'error: unhandled rejection:',
    'error: at zero at /z.js:0:0',
    'fallback:',
    'fallback: the census answered from its prefix sums',
    'fallback: a fallback with no description'
  ]);
  win.emit('error', { message: 'no line', filename: '/b.js' });
  await flush();
  off();
  assert.equal(win.fetch, original);
  assert.equal(win.listeners.size, 0);
  off();
  assert.deepEqual(installPageReport(null)(), undefined);
  assert.deepEqual(installPageReport({ fetch: 1 })(), undefined);
});

test('a report the server did not keep is a console line, and a 429 is the server\'s own ration', async () => {
  for (const [status, warns] of /** @type {const} */ ([[202, 0], [429, 0], [503, 1]])) {
    /** @type {string[]} */
    const warned = [];
    const win = fakeWindow(async (input) => (String(input) === REPORT_ROUTE ? response(status) : response(500)));
    installPageReport(win, { warn: (l) => warned.push(l) });
    await win.fetch('/x.json');
    await flush();
    await flush();
    assert.equal(warned.length, warns, `HTTP ${status}`);
  }
  const bare = fakeWindow(async () => response(500));
  delete bare.location;
  /** @type {string[]} */
  const where = [];
  installPageReport(bare, { warn: () => {} });
  bare.fetch = ((inner) => async (/** @type {any} */ i, /** @type {any} */ o) => { where.push(String(i)); return inner(i, o); })(bare.fetch);
  await bare.fetch('/y.json');
  await flush();
  assert.deepEqual(where, ['/y.json'], 'a window with no location reports as /');
});

test('a fallback is reported through the event, and only where a reporter listens', () => {
  assert.equal(reportFallback('x'), false, 'no window under node');
  assert.equal(reportFallback('x', { dispatchEvent: () => true }), false, 'no reporter installed');
  assert.equal(reportFallback('x', { [Symbol.for('brutex.page-report')]: {}, dispatchEvent: 1 }), false, 'a window that cannot dispatch is not asked to');
  const win = fakeWindow(async () => response(200));
  installPageReport(win);
  assert.equal(reportFallback('the grid fell back to the census', win), true);
  assert.equal(win.dispatched.at(-1)?.type, FALLBACK_EVENT);
  assert.equal(win.dispatched.at(-1)?.detail, 'the grid fell back to the census');
});

// ---------------------------------------------------------------------------
// $lib/api-status.js — the header-only up check
// ---------------------------------------------------------------------------

test('the up check asks for headers only, from a constant route, and an idle tab asks for nothing else', async () => {
  /** @type {{url: string, init: any}[]} */
  const asked = [];
  const ticket = { signal: new AbortController().signal, current: () => true };
  const time = { elapsed: (() => { let t = 0; return () => (t += 5); })(), clock: () => 1_791_600_000_000 };
  const up = await upCheck(async (url, init) => { asked.push({ url, init }); return response(200, '', { 'content-type': 'application/json; charset=utf-8', 'x-brutex-build': 'a'.repeat(40) }); }, ticket, time);
  assert.deepEqual(asked.map((a) => [a.url, a.init.method, a.init.cache]), [[UP_ROUTE, 'HEAD', 'no-store']]);
  assert.equal(asked[0].init.signal, ticket.signal);
  assert.deepEqual(up, { state: 'up', ms: 5, why: null, at: 1_791_600_000_000, route: UP_ROUTE, method: UP_METHOD, build: 'a'.repeat(40) });
  const html = await upCheck(async () => response(200, '', { 'content-type': 'text/html' }), ticket, time);
  assert.match(String(html?.why), /answered text\/html, not JSON — the API is not behind this route/);
  const none = await upCheck(async () => response(200, '', {}), ticket, time);
  assert.match(String(none?.why), /answered no content-type/);
  const down = await upCheck(async () => response(502), ticket, time);
  assert.equal(down?.state, 'down');
  assert.equal(down?.why, 'HTTP 502');
  assert.equal((await upCheck(async () => { throw 'refused'; }, ticket, time))?.why, 'refused');
  assert.equal((await upCheck(async () => { throw new Error(''); }, ticket, time))?.why, '', 'an empty message is said empty, not as the error\'s type');
  const revoked = { signal: ticket.signal, current: () => false };
  assert.equal(await upCheck(async () => response(200, '', { 'content-type': 'application/json' }), revoked, time), null);
  assert.equal(await upCheck(async () => { throw new Error('x'); }, revoked, time), null);
  const real = await upCheck(async () => response(204, '', { 'content-type': 'application/json' }), ticket);
  assert.equal(real?.state, 'up');
  // The layout asks through it and no longer reads /feeds.json for the button.
  const layout = source('src/routes/+layout.svelte');
  assert.match(layout, /const next = await upCheck\(ask, \{ signal: ticket\.signal, current: \(\) => ticket\.current\(\) \}\);/);
  assert.doesNotMatch(layout, /ask\('\/feeds\.json', \{ cache: 'no-store', signal: ticket\.signal \}\)/);
  assert.match(layout, /let api = \$state\(\{ \.\.\.UP_CHECKING \}\);/);
});

test('the button\'s title names what was asked, and how', () => {
  const stamp = (/** @type {number} */ ms) => `stamp(${ms})`;
  assert.equal(apiTitle(UP_CHECKING, stamp), 'Header-only round trip (HEAD) to /inspection.json, not answered yet. Click to check now.');
  assert.equal(apiTitle({ ...UP_CHECKING, state: 'up', at: 5 }, stamp), 'Header-only round trip (HEAD) to /inspection.json. Checked stamp(5). Click to re-check.');
  assert.equal(apiTitle({ ...UP_CHECKING, state: 'down', why: 'HTTP 502', at: 5 }, stamp), 'Probing /inspection.json failed: HTTP 502');
  assert.equal(apiTitle({ state: 'up', why: null, at: 5, route: '/x.json', method: 'GET' }, stamp), 'Round trip to /x.json. Checked stamp(5). Click to re-check.');
});

// ---------------------------------------------------------------------------
// web/ci/speed-sheet.mjs
// ---------------------------------------------------------------------------

test('the sheet\'s command line is read and every misuse is named', () => {
  const a = sheet.parseArgs(['--base', 'http://127.0.0.1:8080']);
  assert.deepEqual([a.n, a.clients, a.restarts, a.instrument, a.allowUnstamped, a.writer], [200, [1, 10, 60], 20, 'NSE-INDEX-NIFTY', false, null]);
  const b = sheet.parseArgs(['--base', 'http://h:1', '--n', '5', '--clients', '2,3', '--writer', 'w', '--restart', 'r', '--restarts', '2', '--from-pages', 'c', '--feed', 'gdfl', '--instrument', 'NSE-INDEX-BANKNIFTY', '--timeframe', '1s', '--month', '2026-09', '--day', '2026-09-30', '--out', 'o.md', '--allow-unstamped', '--timeout-ms', '9']);
  assert.deepEqual([b.n, b.clients, b.writer, b.restart, b.restarts, b.fromPages, b.feed, b.timeframe, b.month, b.day, b.out, b.allowUnstamped, b.timeoutMs], [5, [2, 3], 'w', 'r', 2, 'c', 'gdfl', '1s', '2026-09', '2026-09-30', 'o.md', true, 9]);
  /** @type {[string[], RegExp][]} */
  const bad = [
    [['x'], /unexpected argument/],
    [['--base'], /needs a value/],
    [['--base', '--n'], /needs a value/],
    [['--bogus'], /unknown option --bogus/],
    [[], /--base must be an origin/],
    [['--base', 'http://h:1/path'], /no path/],
    [['--base', 'http://h:1', '--n', '0'], /--n must be a whole number of at least 1/],
    [['--base', 'http://h:1', '--clients', '0'], /--clients is a comma list/],
    [['--base', 'http://h:1', '--day', '30-09-2026'], /--day is YYYY-MM-DD/],
    [['--base', 'http://h:1', '--month', '2026-9'], /--month is YYYY-MM/]
  ];
  for (const [argv, why] of bad) assert.throws(() => sheet.parseArgs(argv), why);
  assert.deepEqual(sheet.monthDays('2024-02'), { from: '2024-02-01', to: '2024-02-29' });
});

test('the sheet times over the network, and a request with no answer is said, never thrown', async () => {
  const server = http.createServer((req, res) => {
    if (req.url === '/slow') return void setTimeout(() => res.end('late'), 200);
    if (req.url === '/broken') return void res.socket?.destroy();
    if (req.url === '/partial') {
      res.writeHead(200, { 'content-type': 'application/json', 'content-length': '100' });
      res.write('{"cut');
      return void setTimeout(() => res.socket?.destroy(), 20);
    }
    if (req.url === '/plain') return void res.end('{}');
    res.writeHead(200, { 'content-type': 'application/json', 'x-brutex-build': 'b'.repeat(40) });
    res.end('{"ok":true}');
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', () => resolve(undefined)));
  const address = /** @type {import('node:net').AddressInfo} */ (server.address());
  const base = `http://127.0.0.1:${address.port}`;
  try {
    const a = await sheet.timed(base, '/x');
    assert.deepEqual([a.status, a.body, a.bytes, a.build], [200, '{"ok":true}', 11, 'b'.repeat(40)]);
    assert.ok(a.ms > 0);
    const late = await sheet.timed(base, '/slow', { timeoutMs: 20 });
    assert.equal(late.status, 0);
    assert.match(/** @type {any} */ (late).error, /no answer within 20 ms/);
    assert.equal((await sheet.timed(base, '/broken')).status, 0);
    const cut = await sheet.timed(base, '/partial');
    assert.equal(cut.status, 0, 'headers with a body cut off are no answer');
    assert.match(/** @type {any} */ (cut).error, /the answer was cut off/);
    assert.equal((await sheet.timed(base, '/plain')).build, null, 'a server that names no build is said as null');
    const agent = new http.Agent({ keepAlive: true });
    const row = await sheet.measure((p, o) => sheet.timed(base, p, o), { page: '/p', route: '/x', label: '', method: 'GET', url: '/x', builder: 'b' }, { n: 6, clients: 2, phase: 'warm', agentOf: () => agent, timeoutMs: 1_000 });
    assert.deepEqual([row.n, row.classes.answered, row.builds], [6, 6, ['b'.repeat(40)]]);
    assert.equal((await sheet.timed('https://127.0.0.1:1', '/x', { timeoutMs: 500 })).status, 0, 'https is spoken, and a closed port is no answer');
  } finally {
    server.close();
  }
  assert.equal(sheet.classOf({ status: 0, error: 'x', ms: 0, build: null, body: '', bytes: 0, headers: {} }), 'no answer');
  assert.equal(sheet.classOf(answer(204)), 'answered');
  assert.equal(sheet.classOf(answer(429)), 'busy');
  assert.equal(sheet.classOf(answer(503, '', { 'retry-after': '1' })), 'busy');
  assert.equal(sheet.classOf(answer(400)), 'refused');
  assert.equal(sheet.classOf(answer(500)), 'failed');
});

test('the sheet discovers the selection /db opens on, from the server, or refuses naming what is missing', async () => {
  const args = sheet.parseArgs(['--base', 'http://h:1']);
  /** @param {Record<string, any>} answers */
  const get = (answers) => async (/** @type {string} */ p) => (p in answers ? answer(200, JSON.stringify(answers[p])) : answer(404));
  const rungs = { rungs: [{ feed: 'zerodha', rung: '1day', stored: true, last_month: '2026-09', last_day: '2026-09-30' }, { feed: 'zerodha', rung: '1min', stored: true, last_month: '2026-08', last_day: '2026-08-31' }, { feed: 'gdfl', rung: '1s', stored: true, last_month: '2026-07' }, { feed: 'zerodha', rung: '3min', stored: false, last_month: '2026-09' }] };
  const feeds = [{ wire: 'dhan', ready: false }, { wire: 'zerodha', ready: true }];
  const url = String(rungsUrl('NSE-INDEX-NIFTY'));
  const ctx = await sheet.discover(get({ '/feeds.json': feeds, [url]: rungs }), args);
  assert.deepEqual(ctx, { feed: 'zerodha', instrument: 'NSE-INDEX-NIFTY', timeframe: '1min', month: '2026-08', day: '2026-08-31', from: '2026-08-01', to: '2026-08-31' });
  const gdfl = await sheet.discover(get({ [url]: rungs }), { ...args, feed: 'gdfl' });
  assert.deepEqual([gdfl.timeframe, gdfl.day], ['1s', '2026-07-01'], 'a rung with no last day starts at its month');
  const given = await sheet.discover(get({}), { ...args, feed: 'zerodha', timeframe: '1min', month: '2026-01', day: '2026-01-02' });
  assert.equal(given.to, '2026-01-31');
  const first = await sheet.discover(get({ '/feeds.json': [{ wire: 'groww', ready: false }], [url]: { rungs: [{ feed: 'groww', rung: '1min', stored: true, last_month: '2026-01' }] } }), args);
  assert.equal(first.feed, 'groww', 'no feed ready: the first listed');
  await assert.rejects(sheet.discover(get({}), args), /\/feeds\.json answered HTTP 404/);
  await assert.rejects(sheet.discover(get({ '/feeds.json': {} }), args), /named no feed; pass --feed/);
  await assert.rejects(sheet.discover(async () => ({ status: 0, error: 'refused', ms: 0, build: null, body: '', bytes: 0, headers: {} }), args), /answered refused/);
  await assert.rejects(sheet.discover(get({}), { ...args, feed: 'x', instrument: 'nonsense' }), /not an instrument key rungsUrl can ask about/);
  await assert.rejects(sheet.discover(get({ [url]: {} }), { ...args, feed: 'x' }), /holds no stored rung of x/);
});

test('every phase of the sheet is run or said NOT MEASURED, and every row is stamped with one build', async () => {
  const server = standIn({ ignore: new Set() });
  // A writer that exits at once, against a server slow enough for the exit to land mid-pass.
  const slow = standIn({ ignore: new Set(), delayMs: 3 });
  const done = await sheet.runSheet({ ...argsFor(), writer: 'true', restart: 'true', restarts: 2 }, { send: slow.send, log: () => {} });
  assert.ok(done.failures.some((f) => f.includes('the --writer command ended before the warm+writer pass did')), done.failures.join('\n'));
  assert.ok(done.rows.some((r) => r.phase === 'warm+writer'));
  assert.equal(done.rows.filter((r) => r.phase === 'first after restart')[0].n, 2);
  const text = sheet.renderSheet(done, argsFor());
  assert.match(text, /^# Speed sheet — FAIL/);
  assert.match(text, /\| cccccccccccc \| \/db \| `\/bars\/window\.json` \| the grid, newest first \| cold \| 1 \| 3 \|/);
  assert.match(text, /- NOT MEASURED: the requests \/gaps, \/audit, \/markets/);
  const quiet = await sheet.runSheet(argsFor(), { send: server.send, log: () => {}, capture: JSON.stringify({ requests: [] }) });
  const said = sheet.renderSheet(quiet, argsFor());
  assert.match(said, /NOT MEASURED: warm\+writer: no --writer command/);
  assert.match(said, /NOT MEASURED: first after restart: no --restart command was given \(20 restarts asked\)/);
  assert.doesNotMatch(said, /NOT MEASURED: the requests \/gaps/);
  // A restart command that fails, and a server that does not come back, are failures.
  const bad = await sheet.runSheet({ ...argsFor(), restart: 'false', restarts: 2 }, { send: server.send, log: () => {} });
  assert.ok(bad.failures.includes('restart 1: the --restart command exited 1'));
  // After the restart: down for two checks, then up, and the first /aliases.json fails.
  const state = { restarted: false, down: 0 };
  const flaky = standIn({ ignore: new Set(), up: () => !(state.down > 0 && state.down-- > 0), failFirstOf: '/aliases.json', failWhen: () => state.restarted });
  const restart = async () => {
    state.restarted = true;
    state.down = 2;
    return 0;
  };
  const late = await sheet.runSheet({ ...argsFor(), restart: 'unused', restarts: 1 }, { send: flaky.send, log: () => {}, restart });
  assert.deepEqual(late.failures, ['restart 1: the first /aliases.json answered HTTP 500']);
  assert.equal(late.rows.filter((r) => r.phase === 'first after restart').length, sheet.requestsFor(late.ctx ?? CTX).requests.length);
});

test('the sheet fails a server that is down, unstamped, mixed, refusing or failing — each by name', async () => {
  const down = await sheet.runSheet(argsFor(), { send: async () => ({ status: 0, error: 'ECONNREFUSED', ms: 1, build: null, body: '', bytes: 0, headers: {} }), log: () => {} });
  assert.equal(down.ok, false);
  assert.match(down.failures[0], /is not up: HEAD \/inspection\.json answered ECONNREFUSED/);
  const down502 = await sheet.runSheet(argsFor(), { send: async () => answer(502), log: () => {} });
  assert.match(down502.failures[0], /answered HTTP 502/);
  const unstamped = await sheet.runSheet(argsFor(), { send: standIn({ ignore: new Set(), build: 'unstamped' }).send, log: () => {} });
  assert.ok(unstamped.failures.some((f) => f.includes('names an unstamped build')));
  /** @type {string[]} */
  const logged = [];
  const allowed = await sheet.runSheet({ ...argsFor(), allowUnstamped: true }, { send: standIn({ ignore: new Set(), build: null }).send, log: (l) => logged.push(l) });
  assert.ok(logged.some((l) => l.startsWith('WARNING: the server names no build')));
  assert.equal(allowed.ok, true, allowed.failures.join('\n'));
  let flip = 0;
  const mixed = await sheet.runSheet(argsFor(), { send: standIn({ ignore: new Set(), build: () => ((flip += 1) % 2 ? 'c'.repeat(40) : 'd'.repeat(40)) }).send, log: () => {} });
  assert.ok(mixed.failures.some((f) => /was answered by 2 builds/.test(f)));
  assert.match(sheet.renderSheet(mixed, argsFor()), /\| MIXED \|/);
  let first = true;
  const other = await sheet.runSheet(argsFor(), { send: standIn({ ignore: new Set(), build: () => (first ? ((first = false), 'c'.repeat(40)) : 'e'.repeat(40)) }).send, log: () => {} });
  assert.ok(other.failures.some((f) => /was answered by e{40}, not c{40}/.test(f)));
  const refusing = await sheet.runSheet(argsFor(), { send: standIn({ ignore: new Set(), status: { '/aliases.json': 404 } }).send, log: () => {} });
  assert.ok(refusing.failures.some((f) => /\/db \/aliases\.json \(cold, 1 clients\): 3 refused — HTTP 404/.test(f)), refusing.failures.join('\n'));
  const failing = await sheet.runSheet(argsFor(), { send: standIn({ ignore: new Set(), status: { '/feeds.json?': 500 } }).send, log: () => {} });
  assert.equal(failing.ok, true, 'a status keyed to no request changes nothing');
  const silent = await sheet.runSheet(argsFor(), { send: standIn({ ignore: new Set(), drop: '/speed.json' }).send, log: () => {} });
  assert.ok(silent.failures.some((f) => /\/speed\.json \(cold, 1 clients\): 3 no answer — dropped/.test(f)));
  assert.ok(silent.failures.some((f) => /was answered by no named build/.test(f)));
  const empty = sheet.renderSheet({ ok: false, rows: [{ ...silent.rows[0], builds: [] }], ignored: [], failures: [], notMeasured: [], skipped: ['s'], ctx: null, commit: null }, argsFor());
  assert.match(empty, /Build not named; selection not discovered/);
  assert.match(empty, /\| not named \|/);
  assert.match(empty, /- SKIPPED: s/);
});

// ---------------------------------------------------------------------------
// web/ci/browser-harness.mjs — the parts that need no browser
// ---------------------------------------------------------------------------

test('the harness\'s command line is read and every misuse is named', () => {
  const a = harness.parseArgs(['--base', 'http://127.0.0.1:8080', '--chrome', '/c']);
  assert.deepEqual([a.n, a.pages, a.idleTabs, a.idleMs, a.interactions, a.idle, a.reportCheck, a.hooks, a.chrome], [200, [...MEASURED_PAGES], [1, 10, 20], 3_600_000, true, true, true, [], '/c']);
  const b = harness.parseArgs(['--base', 'http://h:1', '--n', '3', '--pages', '/db', '--idle-tabs', '2', '--idle-ms', '1000', '--no-interactions', '--no-idle', '--no-report-check', '--hook', 'a.mjs', '--hook', 'b.mjs', '--capture', 'c.json', '--out', 'o.md', '--settle-ms', '60']);
  assert.deepEqual([b.n, b.pages, b.idleTabs, b.idleMs, b.interactions, b.idle, b.reportCheck, b.hooks, b.capture, b.out, b.settleMs], [3, ['/db'], [2], 1000, false, false, false, ['a.mjs', 'b.mjs'], 'c.json', 'o.md', 60]);
  /** @type {[string[], RegExp][]} */
  const bad = [
    [['x'], /unexpected argument/],
    [['--hook'], /--hook needs a value/],
    [['--bogus'], /unknown option/],
    [[], /--base must be an origin/],
    [['--base', 'http://h:1', '--n', 'x'], /--n must be a whole number/],
    [['--base', 'http://h:1', '--pages', 'db'], /--pages is a comma list of paths/],
    [['--base', 'http://h:1', '--idle-tabs', '65'], /--idle-tabs is a comma list/]
  ];
  for (const [argv, why] of bad) assert.throws(() => harness.parseArgs(argv), why);
  assert.ok(harness.INTERACTIONS.some((i) => i.name.includes('20,000-row')), 'the sort edge is LOCAL_SORT_BUDGET');
  assert.deepEqual(harness.INTERACTIONS.map((i) => i.name.split(' at ')[0]), ['picker click', 'feed switch', 'page turn of option cells', 'sort']);
});

test('a page is judged against the four targets, each miss by name and figure', () => {
  const load = (/** @type {object} */ o) => ({ fcp: 100, longest: 10, heap: 10e6, wire: 300e3, settled: true, status: 200, ...o });
  const ok = harness.judge('/gaps', 'cold', [load({}), load({ fcp: 120 })]);
  assert.deepEqual(ok.failures, []);
  assert.match(ok.row, /^\| \/gaps \| cold \| 2 \| 100\.0 \/ 120\.0 \/ 120\.0 \| 10\.0 \/ 10\.0 \/ 10\.0 \| 10\.0 \/ 10\.0 \/ 10\.0 \| 300\.0 \/ 300\.0 \/ 300\.0 \| holds \|$/);
  const bad = harness.judge('/db', 'warm', [load({ fcp: 300, longest: 51, heap: 100e6, wire: 400e3, settled: false, status: 503 }), load({ fcp: null, status: 0 })]);
  assert.deepEqual(bad.failures, [
    '/db warm: 2 of 2 loads were not the page (HTTP 503, 0)',
    '/db warm: 1 of 2 loads painted no first content',
    '/db warm: first content p99 300.0 ms, not under 300 ms',
    '/db warm: a pause of 51.0 ms, over 50 ms',
    '/db warm: script heap 100.0 MB, not under 100 MB',
    '/db warm: 400.0 KB on the wire, not under 400 KB',
    '/db warm: 1 loads still had a request in flight after 30 s'
  ]);
  assert.match(harness.judge('/x', 'cold', [load({ fcp: null })]).row, /\| not painted \|.*\| FAILS \|$/);
});

test('an idle tab may ask only for the header-only up check; anything else is listed', () => {
  const base = 'http://127.0.0.1:8080';
  const quiet = harness.idleVerdict([{ method: 'HEAD', url: `${base}/inspection.json` }, { method: 'HEAD', url: `${base}/inspection.json` }], base);
  assert.deepEqual(quiet.other, []);
  assert.equal(quiet.counts.get('HEAD /inspection.json'), 2);
  const noisy = harness.idleVerdict([{ method: 'GET', url: `${base}/feeds.json` }, { method: 'GET', url: `${base}/inspection.json` }, { method: 'HEAD', url: 'http://elsewhere/inspection.json' }, { method: 'GET', url: `${base}/feeds.json` }], base);
  assert.deepEqual(noisy.other, ['GET /feeds.json', 'GET /inspection.json', 'HEAD http://elsewhere/inspection.json']);
});

test('the capture keeps the page\'s own reads, verbatim, and nothing else', () => {
  const base = 'http://127.0.0.1:8080';
  const c = harness.captured('/gaps', [
    { method: 'GET', url: `${base}/gaps` },
    { method: 'GET', url: `${base}/_app/immutable/entry/app.js` },
    { method: 'GET', url: `${base}/favicon.svg` },
    { method: 'GET', url: `${base}/gaps.json?feed=zerodha&month=2026-09` },
    { method: 'HEAD', url: `${base}/inspection.json` },
    { method: 'POST', url: `${base}/page-report` },
    { method: 'GET', url: 'http://elsewhere/x.json' }
  ], base);
  assert.deepEqual(c, [{ page: '/gaps', method: 'GET', url: '/gaps.json?feed=zerodha&month=2026-09' }, { page: '/gaps', method: 'HEAD', url: '/inspection.json' }]);
  assert.doesNotThrow(() => sheet.readCapture(JSON.stringify({ requests: c })), 'what the harness writes, the sheet reads');
});

test('the dev server proxies the two routes the Speed page and the reporter need, and never the page itself', () => {
  const vite = source('vite.config.js');
  assert.match(vite, /'\/speed\.json',/);
  assert.match(vite, /'\/page-report'\n\];/);
  assert.doesNotMatch(vite, /'\/speed',/);
  assert.match(source('src/routes/+layout.svelte'), /\{ href: '\/speed', label: 'Speed'/);
});

test('the Speed page reads /speed.json once, through the checked reader, and never polls', () => {
  const page = source('src/routes/speed/+page.svelte');
  assert.match(page, /readSpeedReport\(body\)/);
  assert.match(page, /buildLabel\(read\.build\)/);
  assert.match(page, /buildOf\(r\.headers\)/);
  assert.doesNotMatch(page, /setInterval|watchVisible/);
  assert.match(page, /microsText\(r\.p99, r\.n\)/);
});

test('the sheet\'s own edges: a capture file, a request with no first answer, a server that never comes back, an unstamped restart, slow rows', async () => {
  const dir = mkdtempSync(join(tmpdir(), 'r315-sheet-'));
  const capture = join(dir, 'capture.json');
  writeFileSync(capture, JSON.stringify({ requests: [{ page: '/gaps', method: 'GET', url: '/gaps.json?feed=zerodha' }] }));
  const fromFile = await sheet.runSheet({ ...argsFor(), fromPages: capture }, { send: standIn({ ignore: new Set() }).send, log: () => {} });
  assert.ok(fromFile.rows.some((r) => r.url === '/gaps.json?feed=zerodha'), 'the capture file is read and replayed');
  const dropped = await sheet.runSheet(argsFor(), { send: standIn({ ignore: new Set(), drop: '/dayflags.json' }).send, log: () => {} });
  assert.ok(dropped.failures.some((f) => f.includes('/dayflags.json') && f.includes('no answer')), 'a read with no answer is failed where it is timed, not in the field check');
  assert.ok(!dropped.failures.some((f) => f.includes('field') && f.includes('/dayflags.json')));
  const never = await sheet.runSheet({ ...argsFor(), restart: 'x', restarts: 1, upWaitMs: 1 }, { send: standIn({ ignore: new Set(), up: (() => { let first = true; return () => (first ? ((first = false), true) : false); })() }).send, log: () => {}, restart: async () => 0 });
  assert.ok(never.failures.includes('restart 1: the server was not up 1 ms after the restart'), never.failures.join('\n'));
  const gone = standIn({ ignore: new Set(), drop: '/aliases.json' });
  const blank = await sheet.runSheet({ ...argsFor(), restart: 'x', restarts: 1, allowUnstamped: true }, { send: standIn({ ignore: new Set(), build: null, drop: '/aliases.json' }).send, log: () => {}, restart: async () => 0 });
  assert.ok(blank.failures.includes('restart 1: the first /aliases.json answered dropped'), blank.failures.join('\n'));
  assert.deepEqual(blank.rows.filter((r) => r.phase === 'first after restart')[0].builds, [], 'an unstamped server stamps no build on a restart row');
  void gone;
  const slow = sheet.renderSheet({ ...blank, rows: [{ ...blank.rows[0], p50: 12.345, p99: 9.5, max: 100 }] }, argsFor());
  assert.match(slow, /\| 12\.3 \| 9\.50 \| 100\.0 \|/);
  // A writer killed at the end of its pass, by a signal, is said as ended by its signal.
  const killed = await sheet.runSheet({ ...argsFor(), writer: 'sleep 5' }, { send: standIn({ ignore: new Set() }).send, log: () => {} });
  assert.ok(!killed.failures.some((f) => f.includes('--writer')), killed.failures.join('\n'));
});

test('the sheet runs over the real network against a server, by default, and as a command', async () => {
  const strict = standIn({ ignore: new Set() });
  const server = http.createServer(async (req, res) => {
    const a = await strict.send('', String(req.url), { method: req.method });
    if (a.status === 0) return void res.socket?.destroy();
    res.writeHead(a.status, { 'content-type': 'application/json', ...(a.build ? { 'x-brutex-build': a.build } : {}) });
    res.end(a.body);
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', () => resolve(undefined)));
  const base = `http://127.0.0.1:${/** @type {import('node:net').AddressInfo} */ (server.address()).port}`;
  const dir = mkdtempSync(join(tmpdir(), 'r315-cli-'));
  try {
    const args = { ...sheet.parseArgs(['--base', base, '--n', '2', '--clients', '1,3']), feed: CTX.feed, timeframe: CTX.timeframe, month: CTX.month, day: CTX.day };
    const quiet = console.error;
    console.error = () => {};
    let real;
    try {
      real = await sheet.runSheet(args);
    } finally {
      console.error = quiet;
    }
    assert.deepEqual(real.failures, []);
    assert.ok(real.rows.every((r) => r.classes.answered === r.n), 'every timed request was answered over the network');
    const { spawnSync } = await import('node:child_process');
    const script = new URL('ci/speed-sheet.mjs', web).pathname;
    const misuse = spawnSync(process.execPath, [script, '--bogus'], { encoding: 'utf8' });
    assert.equal(misuse.status, 2);
    assert.match(misuse.stderr, /speed-sheet: unknown option --bogus/);
    const out = join(dir, 'sheet.md');
    const { execFile } = await import('node:child_process');
    const ran = await new Promise((resolve) => execFile(process.execPath, [script, '--base', base, '--n', '1', '--clients', '1', '--feed', CTX.feed, '--timeframe', CTX.timeframe, '--month', CTX.month, '--day', CTX.day, '--out', out], (error, stdout) => resolve({ code: error ? error.code : 0, stdout })));
    assert.equal(/** @type {any} */ (ran).code, 0, String(/** @type {any} */ (ran).stdout));
    assert.match(readFileSync(out, 'utf8'), /^# Speed sheet — PASS/);
  } finally {
    server.close();
  }
});

test('the harness in a real browser: the four figures, the interactions, idle tabs, the reporter end to end, hooks and the capture', { timeout: 300_000 }, async () => {
  const chrome = harness.parseArgs(['--base', 'http://h:1']).chrome;
  assert.ok(chrome, 'the browser harness test needs Chrome or Chromium: set BRUTEX_CHROME');
  const root = process.getuid?.() === 0 ? ['--no-sandbox'] : [];
  const site = await browserStandIn({});
  const dir = mkdtempSync(join(tmpdir(), 'r315-harness-'));
  const hook = join(dir, 'hook.mjs');
  writeFileSync(hook, 'export default async (h) => ({ rows: [`hook saw ${h.base}`], failures: [`hook failure`] });\n');
  const silent = join(dir, 'silent.mjs');
  writeFileSync(silent, 'export default async () => undefined;\n');
  const capture = join(dir, 'capture.json');
  try {
    const args = harness.parseArgs(['--base', site.base, '--n', '2', '--pages', '/db,/gaps,/blank,/missing', '--idle-tabs', '1,2', '--idle-ms', '1000', '--settle-ms', '100', '--hook', hook, '--hook', silent, '--capture', capture, ...root]);
    const r = await harness.runHarness(args);
    const text = r.lines.join('\n');
    assert.match(text, /^\| \/db \| cold \| 2 \| [\d.]+ \/ [\d.]+ \/ [\d.]+ \|/m);
    assert.match(text, /^\| \/db \| warm \| 2 \|.*\| holds \|$/m);
    assert.match(text, /^\| picker click \| 2 \| [\d.]+ \/ [\d.]+ \/ [\d.]+ \| [\d.]+ \|$/m);
    assert.match(text, /^\| feed switch \| 0 \| NOT MEASURED: \[aria-controls="feed-listbox"\] is not on \/db \| — \|$/m);
    assert.match(text, /^\| 1 \| 1 \| 1 s \| \d+ \| none \|$/m);
    assert.match(text, /^\| 2 \| 2 \| 1 s \| \d+ \| GET \/feeds\.json \|$/m, 'two idle tabs, each in its own window, both visible');
    assert.match(text, /Shared reporter: a failed \/feeds\.json\?harness=\d+ was reported \(202\) and found in \/logs\.json\./);
    assert.match(text, /hook saw http:\/\/127\.0\.0\.1:\d+/);
    assert.ok(r.failures.includes('2 idle tabs asked for more than the header-only up check: GET /feeds.json'), r.failures.join('\n'));
    assert.ok(r.failures.some((f) => f.startsWith('/gaps: an uncaught error: Error: boom')), r.failures.join('\n'));
    assert.ok(r.failures.some((f) => f.startsWith('/gaps: an uncaught error: Uncaught')), 'a thrown string is named by its text');
    assert.ok(r.failures.some((f) => f.startsWith('/blank cold: 2 of 2 loads painted no first content')));
    assert.ok(r.failures.includes('hook failure'));
    assert.ok(r.failures.includes('/missing cold: 2 of 2 loads were not the page (HTTP 404)'), 'a page the server does not serve is not timed as one');
    assert.deepEqual(r.notes, ['feed switch: NOT MEASURED — the selector [aria-controls="feed-listbox"] was not found on /db']);
    assert.equal(r.ok, false);
    const saved = JSON.parse(readFileSync(capture, 'utf8'));
    assert.ok(saved.requests.some((/** @type {any} */ q) => q.page === '/gaps' && q.url === '/feeds.json'));
    assert.doesNotThrow(() => sheet.readCapture(readFileSync(capture, 'utf8')));
    // The reporter's three ways to fail the check, each by name.
    for (const [o, why] of /** @type {[object, RegExp][]} */ ([
      [{ noReporter: true }, /sent no \/page-report for a failed/],
      [{ reportStatus: 503 }, /\/page-report answered 503 for the report of/],
      [{ forget: true }, /was answered 202 and is not in \/logs\.json/]
    ])) {
      const s = await browserStandIn(o);
      try {
        const one = await harness.runHarness(harness.parseArgs(['--base', s.base, '--n', '1', '--pages', '/speed', '--no-interactions', '--no-idle', '--settle-ms', '100', ...root]));
        assert.ok(one.failures.some((f) => why.test(f)), one.failures.join('\n'));
      } finally {
        s.server.close();
      }
    }
    // The protocol's own refusal reaches the caller.
    const b = await harness.launch(chrome, root.length > 0);
    try {
      await assert.rejects(b.cdp.send('Bogus.method'), /Bogus\.method/);
    } finally {
      b.close();
    }
  } finally {
    site.server.close();
  }
});

test('the harness refuses a browser it cannot start or reach, by name, and as a command', async () => {
  await assert.rejects(harness.launch('/nonexistent/chrome'), /no browser at "\/nonexistent\/chrome"/);
  await assert.rejects(harness.launch(''), /no browser at ""/);
  await assert.rejects(harness.launch('/bin/false'), /the browser exited 1 before it listened/);
  const dir = mkdtempSync(join(tmpdir(), 'r315-fake-chrome-'));
  const mute = join(dir, 'mute');
  writeFileSync(mute, '#!/bin/sh\nexec sleep 5\n', { mode: 0o755 });
  await assert.rejects(harness.launch(mute, false, 200), /printed no DevTools address in 200 ms/);
  const liar = join(dir, 'liar');
  writeFileSync(liar, '#!/bin/sh\necho "DevTools listening on ws://127.0.0.1:1/devtools/browser/x" >&2\nexec sleep 5\n', { mode: 0o755 });
  await assert.rejects(harness.launch(liar), /could not connect to ws:\/\/127\.0\.0\.1:1/);
  const { spawnSync } = await import('node:child_process');
  const script = new URL('ci/browser-harness.mjs', web).pathname;
  const misuse = spawnSync(process.execPath, [script, '--bogus'], { encoding: 'utf8' });
  assert.equal(misuse.status, 2);
  assert.match(misuse.stderr, /browser-harness: unknown option --bogus/);
  const nobrowser = spawnSync(process.execPath, [script, '--base', 'http://127.0.0.1:1', '--chrome', '/nonexistent/chrome'], { encoding: 'utf8' });
  assert.equal(nobrowser.status, 2);
  assert.match(nobrowser.stderr, /browser-harness: no browser at/);
});

test('the harness as a command writes its sheet and exits by its verdict', { timeout: 120_000 }, async () => {
  const site = await browserStandIn({});
  const dir = mkdtempSync(join(tmpdir(), 'r315-harness-cli-'));
  const out = join(dir, 'harness.md');
  try {
    const root = process.getuid?.() === 0 ? ['--no-sandbox'] : [];
    const { execFile } = await import('node:child_process');
    const script = new URL('ci/browser-harness.mjs', web).pathname;
    for (const [pages, code] of /** @type {const} */ ([['/speed', 0], ['/blank', 1]])) {
      const ran = await new Promise((resolve) => execFile(process.execPath, [script, '--base', site.base, '--n', '1', '--pages', pages, '--no-interactions', '--no-idle', '--no-report-check', '--settle-ms', '100', '--out', out, ...root], (error, stdout) => resolve({ code: error ? error.code : 0, stdout })));
      assert.equal(/** @type {any} */ (ran).code, code, String(/** @type {any} */ (ran).stdout));
      assert.match(readFileSync(out, 'utf8'), code === 0 ? /^# Browser harness — PASS/ : /^# Browser harness — FAIL[\s\S]*- FAILED: \/blank cold: 1 of 1 loads painted no first content/);
    }
  } finally {
    site.server.close();
  }
});

test('the last edges: which browser, how an exception is named, odd captures, odd fetch arguments, the window\'s own reporter', async () => {
  assert.equal(harness.chromeOf('/given', '/env', () => true), '/given');
  assert.equal(harness.chromeOf(undefined, '/env', () => true), '/env');
  assert.equal(harness.chromeOf(undefined, undefined, (p) => p === harness.CHROME_PATHS[2]), harness.CHROME_PATHS[2]);
  assert.equal(harness.chromeOf(undefined, undefined, () => false), '', 'no browser anywhere is empty, and launch refuses it by name');
  assert.equal(harness.exceptionText({ exception: { description: 'Error: boom' }, text: 'Uncaught' }), 'Error: boom');
  assert.equal(harness.exceptionText({ exception: {}, text: 'Uncaught' }), 'Uncaught');
  assert.equal(harness.exceptionText({}), 'an exception the browser did not describe');
  assert.throws(() => sheet.readCapture(JSON.stringify({ requests: [null] })), /capture request 0 is a method-less/);
  const url = String(rungsUrl('NSE-INDEX-NIFTY'));
  const ctx = await sheet.discover(async (p) => (p === url ? answer(200, JSON.stringify({ rungs: [null, { feed: 'zerodha', rung: '1min', stored: true, last_month: '2026-09', last_day: '2026-09-30' }] })) : answer(404)), { ...sheet.parseArgs(['--base', 'http://h:1']), feed: 'zerodha' });
  assert.equal(ctx.month, '2026-09', 'a null row in the rung table is passed over');
  const { spawnSync } = await import('node:child_process');
  const down = spawnSync(process.execPath, [new URL('ci/speed-sheet.mjs', web).pathname, '--base', 'http://127.0.0.1:1', '--timeout-ms', '500'], { encoding: 'utf8' });
  assert.equal(down.status, 1, 'a server that is not up fails the run');
  assert.match(down.stdout, /- FAILED: the server at http:\/\/127\.0\.0\.1:1 is not up/);
  // fetch's other argument shapes, a URL that cannot be read, a thrown value that is not an Error.
  /** @type {string[]} */
  const said = [];
  const win = fakeWindow(async (input, init) => {
    if (String(input) === REPORT_ROUTE) {
      said.push(String(new URLSearchParams(init.body).get('what')));
      return response(202);
    }
    if (String(input).includes('offline')) throw 'offline';
    return response(500);
  });
  installPageReport(win);
  await win.fetch(/** @type {any} */ ({ toString: () => '/thing.json' }));
  await win.fetch('http://[not a url');
  await assert.rejects(win.fetch('/offline.json'));
  await flush();
  assert.deepEqual(said, ['request: GET /thing.json answered HTTP 500', 'request: GET http://[not a url answered HTTP 500', 'request: GET /offline.json got no answer: offline']);
  // In a browser the module installs itself on `window`, and reportFallback
  // finds it there: a fresh process with a `window`, importing the module.
  const inBrowser = [
    "const seen = new Map(); const posted = [];",
    "globalThis.window = { fetch: async (u, o) => { posted.push(String(o?.body ?? u)); return { status: 202, headers: new Headers() }; }, location: { pathname: '/speed', origin: 'http://localhost' },",
    "  addEventListener: (t, f) => seen.set(t, f), removeEventListener: () => {}, dispatchEvent: (e) => { seen.get(e.type)?.(e); return true; } };",
    `const m = await import(${JSON.stringify(new URL('src/lib/page-report.js', web).href)});`,
    "const said = m.reportFallback('the grid fell back');",
    "await new Promise((r) => setTimeout(r, 10));",
    "console.log(JSON.stringify({ installed: seen.has(m.FALLBACK_EVENT), said, posted }));"
  ].join('\n');
  const child = spawnSync(process.execPath, ['--input-type=module', '-e', inBrowser], { encoding: 'utf8' });
  assert.equal(child.status, 0, child.stderr);
  const seen = JSON.parse(child.stdout);
  assert.equal(seen.installed, true, 'imported in a browser, the reporter is installed on window');
  assert.equal(seen.said, true, 'reportFallback finds window on its own');
  assert.match(decodeURIComponent(seen.posted[0]), /page=\/speed&what=fallback: the grid fell back/);
  const given = await sheet.discover(async (p) => (p === url ? answer(200, JSON.stringify({ rungs: [{ feed: 'zerodha', rung: '1min', stored: true, last_month: '2026-09' }, { feed: 'zerodha', rung: '1day', stored: true, last_month: '2026-08', last_day: '2026-08-31' }] })) : answer(404)), { ...sheet.parseArgs(['--base', 'http://h:1']), feed: 'zerodha', timeframe: '1day' });
  assert.deepEqual([given.timeframe, given.month], ['1day', '2026-08'], 'a timeframe given picks its own rung, not the finest');
  // The run discovers its selection from the server, and a selection it cannot discover is a failed run, said.
  const base = sheet.parseArgs(['--base', 'http://stand.in', '--n', '1', '--clients', '1']);
  const found = await sheet.runSheet({ ...base, feed: 'zerodha' }, { send: standIn({ ignore: new Set() }).send, log: () => {} });
  assert.deepEqual([found.ctx?.timeframe, found.ctx?.month, found.ctx?.day], ['1min', '2026-09', '2026-09-30']);
  const lost = await sheet.runSheet(base, { send: standIn({ ignore: new Set() }).send, log: () => {} });
  assert.deepEqual(lost.failures, ['discovering the selection: /feeds.json named no feed; pass --feed']);
  assert.equal(lost.ok, false);
  assert.equal(harness.renderHarness({ ok: false, lines: ['| t |'], notes: ['n1'], failures: ['f1'] }, { n: 2, base: 'http://h:1' }), '# Browser harness — FAIL\n\nn = 2 cold and 2 warm loads a page against http://h:1.\n\n| t |\n\n- n1\n- FAILED: f1\n');
  // A build whose app entry carries no text is refused like one with no preload list.
  assert.throws(() => builtPages(new Map([['_app/immutable/entry/app.A.js', { size: 9 }]])), /no __vite__mapDeps/);
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * A stand-in for the app in a browser: pages that load the REAL
 * `$lib/page-report.js`, a `/page-report` that keeps what it is sent, and a
 * `/logs.json` that serves it back.
 *
 * @param {{noReporter?: boolean, reportStatus?: number, forget?: boolean}} o
 */
async function browserStandIn(o) {
  const reporter = source('src/lib/page-report.js');
  /** @type {object[]} */
  const log = [];
  const script = o.noReporter ? '' : `<script type="module">import '/lib/page-report.js';</script>`;
  /** @param {string} title @param {string} [extra] */
  const page = (title, extra = '') => `<!doctype html><meta charset="utf-8"><title>${title}</title><body><h1>${title}</h1>${extra}${script}`;
  /** @type {Record<string, string>} */
  const pages = {
    '/db': page('db', `<input aria-controls="db-search-list"><div id="db-search-list"></div><table><tr><th aria-sort="none"><button>ts</button></th></tr></table><button aria-label="Next page">next</button><script>document.querySelector('input').addEventListener('input', () => { document.querySelector('#db-search-list').innerHTML = '<div role="option" onclick="fetch(\\'/feeds.json\\')">NIFTY</div>'; }); setInterval(() => fetch('/inspection.json', { method: 'HEAD' }), 300);</script>`),
    '/gaps': page('gaps', `<script>fetch('/feeds.json'); fetch('/broken').catch(() => null); setInterval(() => fetch('/feeds.json'), 300); setTimeout(() => { throw new Error('boom'); }, 0); setTimeout(() => { throw 'plain'; }, 0);</script>`),
    '/speed': page('speed', `<script>new Worker('/lib/worker.js');</script>`),
    '/blank': `<!doctype html><title>blank</title><body>${script}`
  };
  const server = http.createServer((req, res) => {
    const u = new URL(String(req.url), 'http://x');
    /** @param {number} status @param {string} type @param {string} body @param {Record<string, string>} [extra] */
    const send = (status, type, body, extra = {}) => {
      res.writeHead(status, { 'content-type': type, ...extra });
      res.end(body);
    };
    if (u.pathname === '/lib/page-report.js') return send(200, 'text/javascript', reporter);
    if (u.pathname === '/lib/worker.js') return send(200, 'text/javascript', 'self.onmessage = () => {};', { 'cache-control': 'max-age=3600' });
    if (u.pathname === UP_ROUTE) return send(200, 'application/json', req.method === 'HEAD' ? '' : '{}', { 'x-brutex-build': 'f'.repeat(40) });
    if (u.pathname === REPORT_ROUTE && req.method === 'POST') {
      let body = '';
      req.on('data', (c) => (body += c));
      req.on('end', () => {
        const f = new URLSearchParams(body);
        if (!o.forget) log.push({ target: 'api.page.access', page: f.get('page'), what: f.get('what') });
        send(o.reportStatus ?? 202, 'application/json', '{"written":true}');
      });
      return;
    }
    if (u.pathname === '/logs.json') return send(200, 'application/json', JSON.stringify({ lines: log }));
    if (u.pathname === '/feeds.json') return send(200, 'application/json', '[]');
    if (u.pathname === '/broken') return void res.socket?.destroy();
    if (u.pathname in pages) return send(200, 'text/html; charset=utf-8', pages[u.pathname]);
    send(404, 'text/plain', 'no');
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', () => resolve(undefined)));
  return { server, log, base: `http://127.0.0.1:${/** @type {import('node:net').AddressInfo} */ (server.address()).port}` };
}

/** A `Response` as far as the code under test reads one.
 * @param {number} status @param {string} [body] @param {Record<string, string>} [headers] @returns {any} */
function response(status, body = '', headers = {}) {
  const h = new Headers(headers);
  return { status, ok: status >= 200 && status < 300, headers: h, json: async () => JSON.parse(body), text: async () => body };
}

/** A timed answer for the sheet. @param {number} status @param {string} [body] @param {Record<string, string>} [headers] */
function answer(status, body = '', headers = {}) {
  return { status, headers, body, ms: 1, bytes: body.length, build: 'c'.repeat(40) };
}

/**
 * A window with just what the reporter touches.
 * @param {(input: any, init?: any) => Promise<any>} fetch
 */
function fakeWindow(fetch) {
  /** @type {Map<string, (e: any) => void>} */
  const listeners = new Map();
  /** @type {any[]} */
  const dispatched = [];
  /** @type {any} */
  const win = {
    fetch,
    location: { pathname: '/db', origin: 'http://localhost' },
    listeners,
    dispatched,
    addEventListener: (/** @type {string} */ t, /** @type {(e: any) => void} */ f) => listeners.set(t, f),
    removeEventListener: (/** @type {string} */ t) => listeners.delete(t),
    dispatchEvent: (/** @type {any} */ e) => {
      dispatched.push(e);
      listeners.get(e.type)?.(e);
      return true;
    },
    emit: (/** @type {string} */ t, /** @type {any} */ e) => listeners.get(t)?.(e)
  };
  return win;
}

/** Sheet arguments for a stand-in server: three of each, one and two clients. */
function argsFor() {
  return { ...sheet.parseArgs(['--base', 'http://stand.in', '--n', '3', '--clients', '1,2']), feed: CTX.feed, timeframe: CTX.timeframe, month: CTX.month, day: CTX.day };
}

/**
 * A stand-in for the server: every route answers by its query; a field in
 * `ignore` is not read (its garbled value is answered like the real one), and
 * every other garbled value is refused, as a strict reader would.
 *
 * @param {{ignore: Set<string>, build?: string | null | (() => string), status?: Record<string, number>,
 *   drop?: string, up?: () => boolean, failFirstOf?: string, failWhen?: () => boolean, delayMs?: number}} o
 */
function standIn(o) {
  let served = 0;
  return {
    /** @param {string} _base @param {string} path @param {{method?: string}} [opts] */
    send: async (_base, path, opts = {}) => {
      served += 1;
      if (o.delayMs) await new Promise((resolve) => setTimeout(resolve, o.delayMs));
      const u = new URL(path, 'http://stand.in');
      const build = typeof o.build === 'function' ? o.build() : o.build === undefined ? 'c'.repeat(40) : o.build;
      if (u.pathname === UP_ROUTE && opts.method === 'HEAD') {
        const up = o.up ? o.up() : true;
        return { status: up ? 200 : 503, headers: {}, body: '', ms: 0.1, bytes: 0, build };
      }
      if (o.drop && u.pathname === o.drop) return { status: 0, error: 'dropped', ms: 1, build: null, body: '', bytes: 0, headers: {} };
      if (o.failFirstOf === u.pathname && (o.failWhen ? o.failWhen() : true)) {
        o.failFirstOf = undefined;
        return { status: 500, headers: {}, body: 'cold', ms: 1, bytes: 4, build };
      }
      const status = o.status?.[u.pathname] ?? 200;
      const read = [...u.searchParams].filter(([k]) => !o.ignore.has(k));
      if (read.some(([, v]) => v === GARBLED)) return { status: 400, headers: {}, body: '{"error":"a value this route cannot read"}', ms: 1, bytes: 40, build };
      const body = u.pathname === '/sources.json'
        ? JSON.stringify({ rungs: [{ feed: 'zerodha', rung: '1min', stored: true, last_month: '2026-09', last_day: '2026-09-30', read }] })
        : JSON.stringify({ route: u.pathname, read });
      return { status, headers: {}, body, ms: 1 + (served % 7), bytes: body.length, build };
    }
  };
}

/** Every file under `root`, as the budget reads them. @param {string} root */
function readBuild(root) {
  /** @type {Map<string, {size: number, text?: string}>} */
  const files = new Map();
  /** @param {string} dir */
  const walk = (dir) => {
    for (const name of readdirSync(dir)) {
      const full = join(dir, name);
      const s = statSync(full);
      if (s.isDirectory()) walk(full);
      else {
        const rel = relative(root, full).split(sep).join('/');
        files.set(rel, { size: s.size, text: rel.endsWith('.js') ? readFileSync(full, 'utf8') : undefined });
      }
    }
  };
  walk(root);
  return files;
}

/** A client build on disk with one page, `/gaps`, of about `pageBytes`. @param {{pageBytes: number}} o */
function synthBuild(o) {
  const dir = mkdtempSync(join(tmpdir(), 'r315-build-'));
  mkdirSync(join(dir, '_app/immutable/entry'), { recursive: true });
  mkdirSync(join(dir, '_app/immutable/nodes'), { recursive: true });
  const app = 'const __vite__mapDeps=(i,m=__vite__mapDeps,d=(m.f||(m.f=["/_app/immutable/nodes/0.A.js","/_app/immutable/nodes/7.A.js"])))=>i.map(i=>d[i]);import(`../nodes/0.A.js`),__vite__mapDeps([0]);import(`../nodes/7.A.js`),__vite__mapDeps([1]);const d={"/gaps":[7]};';
  writeFileSync(join(dir, '_app/immutable/entry/app.A.js'), app);
  writeFileSync(join(dir, '_app/immutable/nodes/0.A.js'), 'x');
  writeFileSync(join(dir, '_app/immutable/nodes/7.A.js'), 'x'.repeat(Math.max(0, o.pageBytes - app.length - 1)));
  writeFileSync(join(dir, '_app/immutable/nodes/7.A.js.map'), 'x'.repeat(o.pageBytes));
  mkdirSync(join(dir, '_app/immutable/assets'), { recursive: true });
  writeFileSync(join(dir, '_app/immutable/assets/unused.css'), 'a{}');
  return dir;
}
