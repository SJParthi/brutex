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
import { readFileSync, mkdtempSync, mkdirSync, writeFileSync, readdirSync, statSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, relative, sep } from 'node:path';
import http from 'node:http';

import {
  PAGE_TARGETS, MEASURED_PAGES, SPEED_ROUTE, WINDOWS, SORTS, GARBLED, BUNDLE_BUDGETS,
  readSpeedReport, microsText, speedRows, sortRows, speedTile, summary, ignoredFields,
  garbledUrl, budgetOf, builtPages, budgetVerdict, bytesText, earlierReading, SELF_TYPES, selfStart, selfAdd, selfText, watchSelf
} from '../src/lib/speed.js';
import {
  REPORT_ROUTE, MAX_REPORT_BYTES, MAX_PAGE_BYTES, REPORTS_PER_MINUTE, MINUTE_MS, FALLBACK_EVENT,
  reportBody, outcomeOf, createPageReporter, installPageReport, reportFallback, REPLACEMENT
} from '../src/lib/page-report.js';
import { upCheck, apiTitle, UP_ROUTE, UP_METHOD, UP_CHECKING } from '../src/lib/api-status.js';
// The exports a repair added are read through the module, so a test of one
// fails on the tree before it by name and not the whole file at its import.
import * as speedModule from '../src/lib/speed.js';
import * as sheet from '../ci/speed-sheet.mjs';
import * as harness from '../ci/browser-harness.mjs';
import { windowQuery, LOCAL_SORT_BUDGET } from '../src/lib/db/bars-plan.js';
import { dayflagsUrl } from '../src/lib/db/dayflags.js';
import { rungsUrl, SOURCES_DEFAULTS } from '../src/lib/sources.js';
import { linkOfKey, DEFAULT_INSTRUMENT } from '../src/lib/db/selection-model.js';

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
  assert.ok(requests.some((r) => r.label === 'extremes' && r.url === `/bars/window.json?${windowQuery(CTX.feed, plan, { sort: 'ts', desc: true, offset: 0, limit: 1, extremes: true })}`), 'the extremes read, newest first, as DbExtremes asks it');
  assert.ok(requests.some((r) => r.url === rungsUrl(CTX.instrument)));
  assert.ok(requests.some((r) => r.method === UP_METHOD && r.url === UP_ROUTE), 'the header-only up check is timed as the page sends it');
  // A page that builds its URL inline is replayed VERBATIM from the harness's capture.
  const capture = JSON.stringify({ requests: [{ page: '/gaps', method: 'GET', url: '/gaps.json?feed=zerodha&symbol=NIFTY' }, { page: '/db', method: 'GET', url: '/feeds.json' }, { page: '/gaps', method: 'GET', url: '/gaps.json?feed=zerodha&symbol=NIFTY' }] });
  const withCapture = sheet.requestsFor(CTX, sheet.readCapture(capture));
  const replay = withCapture.requests.filter((r) => r.builder === '(the page, captured)');
  assert.deepEqual(replay.map((r) => [r.page, r.url]), [['/gaps', '/gaps.json?feed=zerodha&symbol=NIFTY']], 'replayed verbatim, a request already built once is not timed twice');
  assert.throws(() => sheet.readCapture(JSON.stringify({ requests: [{ page: '/ingest', method: 'POST', url: '/pull' }] })), /replays reads only/);
  assert.throws(() => sheet.readCapture(JSON.stringify({ requests: [{ page: '/x', method: 'GET', url: 'http://elsewhere/x.json' }] })), /not a path of this server/);
  assert.throws(() => sheet.readCapture(JSON.stringify({ requests: [{ page: '/x', method: 'GET', url: '//elsewhere/x.json' }] })), /not a path of this server/);
  assert.throws(() => sheet.readCapture(JSON.stringify({ requests: [{ url: '/x' }] })), /method-less/);
  assert.throws(() => sheet.readCapture('{}'), /no requests list/);
  assert.equal(sheet.readCapture(JSON.stringify({ requests: [{ method: 'head', url: '/x' }] }))[0].page, '?');
  assert.equal(sheet.readCapture(JSON.stringify({ requests: [{ page: '', method: 'GET', url: '/x' }] }))[0].page, '', 'a page named empty is kept as named');
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
    // A client output without its entry, or with no directory, is REFUSED by
    // name (repair round 1: it passed unweighed). The server build, which
    // ships nothing to a browser, is told apart by its own configuration:
    // 'the page budget refuses a client build it cannot weigh, and a build
    // that weighed nothing'.
    assert.throws(() => plugin.writeBundle.call(ctx, { dir: mkdtempSync(join(tmpdir(), 'r315-server-')) }), /the page budget found no client build to weigh: .*r315-server-.* holds no _app\/immutable\/entry/);
    assert.throws(() => plugin.writeBundle.call(ctx, {}), /the build named no output directory/);
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
  assert.equal(microsText(9_999), '10.0 ms', 'the unit and its decimals follow the rounding (repair round 1: it printed 10.00 ms)');
  assert.equal(microsText(10_000), '10.0 ms', 'from ten ms, one decimal');
  assert.equal(microsText(45_678), '45.7 ms');
  assert.equal(microsText(999_999), '1.00 s', 'never 1000.0 ms (repair round 1)');
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
  assert.deepEqual(ignoredFields({ ...base, status: 300, garbled: { b: { status: 200, body: '{"rows":[]}' } } }), [], 'an original answered 300 says nothing about a field, whatever the probe got');
  assert.deepEqual(ignoredFields({ ...base, status: 299, garbled: { b: { status: 299, body: '{"rows":[]}' } } }).map((i) => i.how), ['garbled'], '299 is 2xx');
  assert.deepEqual(ignoredFields({ url: '/x.json?a=1', status: 400, body: '{"error":7,"why":"a is not a parameter"}' }).map((i) => i.how), ['refused'], 'an error that is not a sentence leaves the body to read');
  assert.deepEqual(ignoredFields({ url: '/x.json?a=1', status: 200, body: 'not json' }), []);
  assert.deepEqual(ignoredFields({ url: '/x.json?a=1', status: 200, body: '{"ignored":["a",{"field":"b"},{}]}' }).map((i) => i.field), ['a', 'b', '?']);
  assert.deepEqual(ignoredFields({ url: '/x.json?a=1', status: 400, body: 'a is not a parameter here' }).map((i) => i.how), ['refused']);
  assert.deepEqual(ignoredFields({ url: '/x.json?a=1', status: 400, body: '{"error":"a must be a day"}' }), [], 'a refused value is a read field');
});

test('each page has a budget, and the over-target pages are named', () => {
  assert.equal(budgetOf('/gaps'), 400_000);
  assert.equal(budgetOf('/db'), 825_000, 'raised for R3-04 on the 20 Oct merged tree (D-5915 item 10)');
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

test('a page reads its own load as the browser delivers it: a late paint and a late fetch are counted, and only what the browser cannot report is said not reported by it', () => {
  const all = ['element', 'paint', 'navigation', 'resource', 'longtask'];
  // A browser with no observer: every figure said unsupported, never a zero.
  const none = selfStart(null);
  assert.deepEqual(
    { paint: none.paint, wire: none.wire, firstContentMs: none.firstContentMs, heapBytes: none.heapBytes, atMs: none.atMs },
    { paint: 'unsupported', wire: 'unsupported', firstContentMs: null, heapBytes: null, atMs: null }
  );
  assert.ok(Object.isFrozen(none));
  assert.equal(selfText(none, '/speed'), 'This page\'s own load, from this browser: first content not reported by this browser · bytes on the wire not reported by this browser · script heap not reported by this browser.');
  assert.equal(selfStart(['paint', 'resource']).wire, 'unsupported', 'a wire without the document is not passed off as the whole load');
  assert.equal(selfStart(['navigation', 'resource']).paint, 'unsupported');
  assert.equal(selfStart(/** @type {any} */ ('paint navigation resource')).paint, 'unsupported', 'only an array is a list of types');
  assert.deepEqual(SELF_TYPES, ['paint', 'navigation', 'resource']);
  // THE DEFECT (the R3-15 re-check, 21 Oct): the page read its timeline once,
  // at mount, before it had painted and before its last fetches ended, and
  // said "not reported by this browser" for a first content Chrome reported.
  let r = selfAdd(selfStart(all), [
    { entryType: 'navigation', name: 'http://h:1/speed?x=1', transferSize: 9_000 },
    { entryType: 'resource', name: 'http://h:1/a.js', transferSize: 248_500 }
  ], { atMs: 40, heapBytes: 2_300_000 });
  assert.equal(selfText(r, '/speed'), 'This page\'s own load, from this browser: first content not reported yet · 257.5 KB on the wire over 2 fetches · script heap 2.3 MB. Read 40.0 ms after the tab opened, with the load still going; redrawn as each entry arrives.');
  // The paint and the later fetches arrive: each counted once, the first first content stands.
  r = selfAdd(r, [{ entryType: 'paint', name: 'first-paint', startTime: 50 }, { entryType: 'paint', name: 'first-contentful-paint', startTime: 84.4 }], { atMs: 90 });
  r = selfAdd(r, [{ entryType: 'resource', transferSize: 151_800 }, { entryType: 'paint', name: 'first-contentful-paint', startTime: 900 }, null, { entryType: 'longtask', duration: 80 }], { atMs: 1_234.5, loaded: true, heapBytes: Number.NaN });
  assert.equal(r.firstContentMs, 84);
  assert.equal(r.wireBytes, 409_300);
  assert.equal(r.fetches, 3);
  assert.equal(r.heapBytes, 2_300_000, 'a heap that is not a number does not replace the last one read');
  assert.equal(selfText(r, '/speed'), 'This page\'s own load, from this browser: first content 84 ms · 409.3 KB on the wire over 3 fetches (not under the 400 KB target) · script heap 2.3 MB. Read 1.23 s after the tab opened, after the load; a later fetch is added when it ends.');
  assert.equal(selfAdd(r, [], { loaded: false }).loaded, true, 'a load that ended stays ended');
  // A first content that is not a number is not one; a slow one and a large heap are said against their targets.
  const odd = selfAdd(selfStart(all), [{ entryType: 'paint', name: 'first-contentful-paint', startTime: Number.NaN }]);
  assert.equal(odd.paint, 'waiting');
  assert.equal(odd.firstContentMs, null);
  const slow = selfAdd(selfStart(all), [{ entryType: 'paint', name: 'first-contentful-paint', startTime: 300 }], { heapBytes: 100_000_000 });
  assert.match(selfText(slow, '/speed'), /first content 300 ms \(not under the 300 ms p99 target\) · 0\.0 KB on the wire over 0 fetches · script heap 100\.0 MB \(not under the 100 MB target\)\.$/);
  assert.match(selfText(selfAdd(selfStart(all), [{ entryType: 'paint', name: 'first-contentful-paint', startTime: 299.4 }]), '/speed'), /first content 299 ms · /);
  // An entry with no size, entries the browser dropped, and a wire the browser cannot give.
  const gaps = selfAdd(selfStart(all), [{ entryType: 'resource' }, { entryType: 'resource', transferSize: Number.POSITIVE_INFINITY }, { entryType: 'resource', transferSize: 399_999 }], { dropped: 7 });
  assert.match(selfText(selfAdd(gaps, [], { dropped: Number.NaN }), '/speed'), /400\.0 KB on the wire over 3 fetches, 2 of them with no size from this browser, and 7 more the browser dropped before this page could read them · /);
  assert.equal(selfAdd(gaps, [], { dropped: -3 }).dropped, 7, 'a count below zero adds nothing');
  const blind = selfAdd(selfStart(['paint']), [{ entryType: 'navigation', name: 'http://h:1/speed', transferSize: 1 }]);
  assert.equal(blind.fetches, 0, 'a wire the browser cannot give is never half counted');
  // A tab that opened on another page and came here by a link: the figures are the tab's.
  const linked = selfAdd(selfStart(all), [{ entryType: 'navigation', name: 'http://h:1/db?feed=zerodha', transferSize: 1_000 }]);
  assert.equal(linked.documentPath, '/db');
  assert.match(selfText(linked, '/speed'), / This tab opened on \/db and came here by a link inside the app, so these are the tab's figures since it opened, not this page's alone\.$/);
  assert.equal(selfAdd(selfStart(all), [{ entryType: 'navigation', name: 'not a url', transferSize: 1 }]).documentPath, null);
  // A /speed document's FIRST watch is this page's own load; a later watch of
  // the same document is a revisit and says the tab's figures (repair 3, the
  // round-3 tests at the end of this file).
  assert.doesNotMatch(selfText(selfAdd(selfStart(all), [{ entryType: 'navigation', name: 'http://h:1/speed', transferSize: 1 }]), '/speed'), /came here by a link|came back|tab's figures/);
  assert.match(selfText(selfAdd(selfStart(all, true), [{ entryType: 'navigation', name: 'http://h:1/speed', transferSize: 1 }]), '/speed'), / This tab opened on \/speed, left it and came back by a link inside the app, so these are the tab's figures since it opened, not this page's alone\.$/);
  // Chrome hands a buffered observer the document's entry twice (measured 21 Oct): it is one fetch, the later size standing.
  const doc = (/** @type {number | undefined} */ transferSize) => ({ entryType: 'navigation', name: 'http://h:1/speed', transferSize });
  const twice = selfAdd(selfAdd(selfStart(all), [doc(877), doc(877), { entryType: 'resource', transferSize: 50_300 }]), [doc(900)]);
  assert.deepEqual([twice.fetches, twice.wireBytes, twice.unsized], [2, 51_200, 0]);
  const sizedLater = selfAdd(selfStart(all), [doc(undefined), doc(700)]);
  assert.deepEqual([sizedLater.fetches, sizedLater.wireBytes, sizedLater.unsized], [1, 700, 0]);
  const sizeLost = selfAdd(selfStart(all), [doc(700), doc(undefined)]);
  assert.deepEqual([sizeLost.fetches, sizeLost.wireBytes, sizeLost.unsized], [1, 0, 1]);
});

test('watching a page\'s own load: one buffered observer per supported type, a reading at once and after each delivery and the load, and a stop that leaves nothing', () => {
  /** @type {any[]} */
  const made = [];
  class Observer {
    static supportedEntryTypes = ['paint', 'navigation', 'resource', 'longtask'];
    /** @param {(list: any, self: any, options?: any) => void} cb */
    constructor(cb) {
      this.cb = cb;
      /** @type {any[]} */
      this.asked = [];
      this.disconnected = false;
      made.push(this);
    }
    /** @param {any} o */
    observe(o) {
      this.asked.push(o);
    }
    disconnect() {
      this.disconnected = true;
    }
    /** @param {any[]} entries @param {number} [dropped] */
    deliver(entries, dropped) {
      this.cb({ getEntries: () => entries }, this, dropped === undefined ? undefined : { droppedEntriesCount: dropped });
    }
  }
  /** @type {Map<string, () => void>} */
  const listeners = new Map();
  let now = 12.5;
  const win = {
    PerformanceObserver: Observer,
    performance: { now: () => now, memory: { usedJSHeapSize: 1_500_000 } },
    document: { readyState: 'interactive' },
    addEventListener: (/** @type {string} */ t, /** @type {() => void} */ f) => listeners.set(t, f),
    removeEventListener: (/** @type {string} */ t) => listeners.delete(t)
  };
  /** @type {any[]} */
  const seen = [];
  const stop = watchSelf(win, (reading) => seen.push(reading));
  assert.equal(made.length, 1, 'one observer');
  assert.deepEqual(made[0].asked, [{ type: 'paint', buffered: true }, { type: 'navigation', buffered: true }, { type: 'resource', buffered: true }]);
  assert.equal(seen.length, 1, 'a reading at once');
  assert.equal(seen[0].atMs, 12.5);
  assert.equal(seen[0].heapBytes, 1_500_000);
  assert.match(selfText(seen[0], '/speed'), /^This page's own load, from this browser: first content not reported yet · 0\.0 KB on the wire over 0 fetches · /);
  now = 60;
  made[0].deliver([{ entryType: 'resource', transferSize: 100 }, { entryType: 'paint', name: 'first-contentful-paint', startTime: 58 }], 2);
  assert.equal(seen.length, 2);
  assert.deepEqual([seen[1].wireBytes, seen[1].fetches, seen[1].dropped, seen[1].firstContentMs, seen[1].loaded], [100, 1, 2, 58, false]);
  now = 500;
  win.document.readyState = 'complete';
  /** @type {() => void} */ (listeners.get('load'))();
  assert.equal(seen.at(-1).loaded, true, 'the load event is a reading of its own');
  made[0].deliver([{ entryType: 'navigation', name: 'http://h:1/speed', transferSize: 900 }]);
  assert.deepEqual([seen.at(-1).wireBytes, seen.at(-1).fetches, seen.at(-1).dropped, seen.at(-1).atMs], [1_000, 2, 2, 500]);
  stop();
  assert.equal(made[0].disconnected, true);
  assert.equal(listeners.size, 0, 'the load listener is removed');
  // A browser that supports only paint is asked only for paint; one with no observer gets one reading, and its stop does nothing.
  const paintOnly = { ...win, PerformanceObserver: class extends Observer { static supportedEntryTypes = ['paint']; } };
  watchSelf(paintOnly, () => {})();
  assert.deepEqual(made.at(-1).asked, [{ type: 'paint', buffered: true }]);
  /** @type {any[]} */
  const bare = [];
  const stopBare = watchSelf({}, (reading) => bare.push(reading));
  stopBare();
  assert.equal(bare.length, 1);
  assert.equal(bare[0].paint, 'unsupported');
  assert.equal(bare[0].atMs, null, 'no clock, no stamp');
  assert.doesNotThrow(() => watchSelf(null, () => {})());
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
  assert.deepEqual(r.counts(), { sent: REPORTS_PER_MINUTE, heldBack: 4, sendFailures: 0, faults: 0 });
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
    [['--base', 'http://h:1', '--clients', '1,0'], /--clients is a comma list/],
    [['--base', 'http://h:1', '--clients', '513'], /--clients is a comma list/],
    [['--base', 'http://h:1', '--day', '30-09-2026'], /--day is YYYY-MM-DD/],
    [['--base', 'http://h:1', '--month', '2026-9'], /--month is YYYY-MM/],
    [['--base', 'http://h:1', '--clients', ''], /--clients is a comma list/],
    [['--base', ''], /--base must be an origin/]
  ];
  for (const [argv, why] of bad) assert.throws(() => sheet.parseArgs(argv), why);
  assert.deepEqual(sheet.parseArgs(['--base', 'http://h:1', '--clients', '1,512']).clients, [1, 512], '512 clients is the largest allowed');
  assert.equal(sheet.parseArgs(['--base', 'http://h:1', '--instrument', '']).instrument, '', 'an empty instrument is kept as given, and refused where it is asked about, never defaulted');
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
    assert.deepEqual([row.n, row.classes.answered, row.builds, row.bytes, row.said], [6, 6, ['b'.repeat(40)], 11, null], 'the largest answer, and nothing said for a clean row');
    let connections = 0;
    server.on('connection', () => (connections += 1));
    await sheet.measure((p, o) => sheet.timed(base, p, o), { page: '/p', route: '/x', label: '', method: 'GET', url: '/x', builder: 'b' }, { n: 4, clients: 1, phase: 'warm', agentOf: sheet.warmAgent, timeoutMs: 1_000 });
    assert.equal(connections, 1, 'a warm client keeps one connection');
    await sheet.measure((p, o) => sheet.timed(base, p, o), { page: '/p', route: '/x', label: '', method: 'GET', url: '/x', builder: 'b' }, { n: 4, clients: 1, phase: 'cold', agentOf: () => false, timeoutMs: 1_000 });
    assert.equal(connections, 5, 'a cold request opens a connection each');
    assert.equal((await sheet.timed('https://127.0.0.1:1', '/x', { timeoutMs: 500 })).status, 0, 'https is spoken, and a closed port is no answer');
  } finally {
    server.close();
  }
  assert.equal(sheet.classOf({ status: 0, error: 'x', ms: 0, build: null, body: '', bytes: 0, headers: {} }), 'no answer');
  assert.equal(sheet.classOf(answer(204)), 'answered');
  assert.equal(sheet.classOf(answer(429)), 'busy');
  assert.equal(sheet.classOf(answer(503, '', { 'retry-after': '1' })), 'busy');
  assert.equal(sheet.classOf(answer(400)), 'refused');
  assert.equal(sheet.classOf(answer(400, '', { 'retry-after': '1' })), 'refused', 'Retry-After makes only a 503 busy');
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
  const noMonth = await sheet.discover(get({ [url]: rungs }), { ...args, feed: 'zerodha', timeframe: '1min', day: '2026-08-03' });
  assert.deepEqual([noMonth.month, noMonth.day], ['2026-08', '2026-08-03'], 'a month not given is discovered though the rung and the day are');
  const noDay = await sheet.discover(get({ [url]: rungs }), { ...args, feed: 'zerodha', timeframe: '1min', month: '2026-08' });
  assert.equal(noDay.day, '2026-08-31', 'a day not given is discovered though the rung and the month are');
  const given = await sheet.discover(get({}), { ...args, feed: 'zerodha', timeframe: '1min', month: '2026-01', day: '2026-01-02' });
  assert.equal(given.to, '2026-01-31');
  const offered = await sheet.discover(get({ '/feeds.json': [{ wire: 'groww', ready: false }, { wire: 'zerodha', ready: true, offered: true }], [url]: rungs }), args);
  assert.equal(offered.feed, 'zerodha', 'a feed offered and ready is chosen over the first listed');
  const first = await sheet.discover(get({ '/feeds.json': [{ wire: 'groww', ready: false }], [url]: { rungs: [{ feed: 'groww', rung: '1min', stored: true, last_month: '2026-01' }] } }), args);
  assert.equal(first.feed, 'groww', 'no feed ready: the first listed');
  await assert.rejects(sheet.discover(get({}), args), /\/feeds\.json answered HTTP 404/);
  await assert.rejects(sheet.discover(get({ '/feeds.json': {} }), args), /named no feed; pass --feed/);
  await assert.rejects(sheet.discover(async () => ({ status: 0, error: 'refused', ms: 0, build: null, body: '', bytes: 0, headers: {} }), args), /answered refused/);
  await assert.rejects(sheet.discover(get({}), { ...args, feed: 'x', instrument: 'nonsense' }), /not an instrument key rungsUrl can ask about/);
  await assert.rejects(sheet.discover(get({ [url]: {} }), { ...args, feed: 'x' }), /holds no stored rung of x/);
});

test('a large answer is timed by one client at a time and kept as its digest, each bound said', async () => {
  // Found on the Mac (20 Oct): the page capture holds /gaps' 184 MB census,
  // which sixty clients at once would hold in the sheet's own memory, and a
  // 591 MB /store.json is past the longest string one process may hold.
  assert.equal(sheet.MAX_TEXT_BYTES, 64_000_000);
  assert.equal(sheet.MAX_ANSWER_BYTES, 16_000_000);
  const a = Buffer.from('{"rows":[1,2,3]}');
  assert.equal(sheet.answerText(a), '{"rows":[1,2,3]}', 'an ordinary answer is its text');
  assert.equal(sheet.answerText(a, a.length), '{"rows":[1,2,3]}', 'exactly the limit is still text');
  const long = sheet.answerText(a, a.length - 1);
  assert.match(long, /^\(an answer of 16 bytes, sha256 [0-9a-f]{64}\)$/);
  assert.equal(sheet.answerText(Buffer.from('{"rows":[1,2,3]}'), 1), long, 'the same bytes, the same digest');
  assert.notEqual(sheet.answerText(Buffer.from('{"rows":[1,2,4]}'), 1), long, 'one byte apart, two digests: the ignored-field check stays exact');
  // The command line.
  assert.equal(sheet.parseArgs(['--base', 'http://h:1']).maxAnswerBytes, 16_000_000);
  assert.equal(sheet.parseArgs(['--base', 'http://h:1', '--max-answer-bytes', '40']).maxAnswerBytes, 40);
  assert.throws(() => sheet.parseArgs(['--base', 'http://h:1', '--max-answer-bytes', '0']), /--max-answer-bytes/);
  // A run: every answer of the stand-in is past 40 bytes but the up check's.
  const server = standIn({ ignore: new Set() });
  const run = await sheet.runSheet({ ...argsFor(), maxAnswerBytes: 40 }, { send: server.send, log: () => {}, capture: JSON.stringify({ requests: [] }) });
  const cold = run.rows.filter((r) => r.phase === 'cold');
  const big = cold.filter((r) => r.bytes > 40);
  assert.ok(big.length > 0 && big.length < cold.length, 'some answers are past the bound, the up check is not');
  assert.equal(run.rows.filter((r) => r.phase === 'warm' && r.clients === 1).length, cold.length, 'one client times every request');
  assert.deepEqual(run.rows.filter((r) => r.phase === 'warm' && r.clients === 2).map((r) => r.url), cold.filter((r) => r.bytes <= 40).map((r) => r.url), 'two at once time only what is inside the bound');
  for (const r of big) {
    assert.ok(run.notMeasured.includes(`warm at 2 clients: ${r.url.startsWith('/inspection') ? 'HEAD' : 'GET'} ${r.url} answered ${r.bytes} bytes, past --max-answer-bytes 40; 2 at once would hold ${2 * r.bytes} bytes in this process, so it is timed at one client only`), r.url);
  }
  assert.match(sheet.renderSheet(run, argsFor()), /NOT MEASURED: warm at 2 clients: GET \/census\/facets\.json\?feed=zerodha answered 59 bytes, past --max-answer-bytes 40; 2 at once would hold 118 bytes in this process/);
  // Inside the bound (the default), nothing is skipped.
  const all = await sheet.runSheet(argsFor(), { send: server.send, log: () => {}, capture: JSON.stringify({ requests: [] }) });
  assert.equal(all.rows.filter((r) => r.phase === 'warm' && r.clients === 2).length, cold.length);
  assert.ok(!all.notMeasured.some((l) => l.includes('--max-answer-bytes')));
});

test('every phase of the sheet is run or said NOT MEASURED, and every row is stamped with one build', async () => {
  const server = standIn({ ignore: new Set() });
  // A writer that exits at once, against a server slow enough for the exit to land mid-pass.
  const slow = standIn({ ignore: new Set(), delayMs: 3 });
  const done = await sheet.runSheet({ ...argsFor(), writer: 'true', restart: 'true', restarts: 2 }, { send: slow.send, log: () => {} });
  assert.ok(done.failures.some((f) => f.includes('the --writer command ended before the warm+writer pass did')), done.failures.join('\n'));
  assert.ok(done.rows.some((r) => r.phase === 'warm+writer'));
  assert.equal(done.rows.filter((r) => r.phase === 'first after restart')[0].n, 2);
  assert.deepEqual(done.rows.filter((r) => r.phase === 'first after restart')[0].classes, { answered: 2 }, 'a restart row counts its answers like every other row');
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
  const b = harness.parseArgs(['--base', 'http://h:1', '--n', '3', '--pages', '/db', '--idle-tabs', '2', '--idle-ms', '1000', '--no-interactions', '--no-idle', '--no-report-check', '--hook', 'a.mjs', '--hook', 'b.mjs', '--capture', 'c.json', '--out', 'o.md', '--settle-ms', '60', '--step-wait-ms', '100']);
  assert.deepEqual([b.n, b.pages, b.idleTabs, b.idleMs, b.interactions, b.idle, b.reportCheck, b.hooks, b.capture, b.out, b.settleMs, b.stepWaitMs], [3, ['/db'], [2], 1000, false, false, false, ['a.mjs', 'b.mjs'], 'c.json', 'o.md', 60, 100]);
  assert.equal(a.stepWaitMs, harness.STEP_WAIT_MS, 'a step waits 30 s for its control by default');
  assert.equal(harness.STEP_WAIT_MS, 30_000);
  /** @type {[string[], RegExp][]} */
  const bad = [
    [['x'], /unexpected argument/],
    [['--hook'], /--hook needs a value/],
    [['--bogus'], /unknown option/],
    [[], /--base must be an origin/],
    [['--base', 'http://h:1', '--n', 'x'], /--n must be a whole number/],
    [['--base', 'http://h:1', '--pages', 'db'], /--pages is a comma list of paths/],
    [['--base', 'http://h:1', '--pages', '/db,gaps'], /--pages is a comma list of paths/],
    [['--base', 'http://h:1', '--idle-tabs', '65'], /--idle-tabs is a comma list/],
    [['--base', 'http://h:1', '--idle-tabs', '1,0'], /--idle-tabs is a comma list/],
    [['--base', 'http://h:1', '--idle-tabs', '1.5'], /--idle-tabs is a comma list/],
    [['--base', 'http://h:1', '--idle-tabs', ''], /--idle-tabs is a comma list/],
    [['--base', 'http://h:1', '--pages', ''], /--pages is a comma list of paths/],
    [['--base', ''], /--base must be an origin/],
    [['--base', 'http://h:1', '--step-wait-ms', '99'], /--step-wait-ms must be a whole number of at least 100/]
  ];
  for (const [argv, why] of bad) assert.throws(() => harness.parseArgs(argv), why);
  assert.deepEqual(harness.parseArgs(['--base', 'http://h:1', '--idle-tabs', '64']).idleTabs, [64], '64 idle tabs is the most allowed');
  assert.equal(harness.parseArgs(['--base', 'http://h:1']).noSandbox, false, 'the sandbox stays on unless asked');
  assert.equal(harness.parseArgs(['--base', 'http://h:1', '--no-sandbox']).noSandbox, true);
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
  assert.deepEqual(harness.judge('/x', 'cold', [load({ status: 300 })]).failures, ['/x cold: 1 of 1 loads were not the page (HTTP 300)'], 'a redirect is not the page');
  assert.deepEqual(harness.judge('/x', 'cold', [load({ status: 299, longest: 50, fcp: 299.9 })]).failures, [], 'a pause of exactly 50 ms is no pause over 50 ms; 299 is 2xx');
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
  const slow = sheet.renderSheet({ ...blank, rows: [{ ...blank.rows[0], p50: 12.345, p99: 9.5, max: 10 }] }, argsFor());
  assert.match(slow, /\| 12\.3 \| 9\.50 \| 10\.0 \|/, 'from ten ms, one decimal');
  assert.match(slow, /\|\n\n- (SKIPPED|NOT MEASURED|FAILED): /, 'a blank line between the table and the list');
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
    for (const name of ['picker click', 'feed switch', 'page turn of option cells', 'sort at the 20,000-row browser sort edge']) {
      assert.match(text, new RegExp(`^\\| ${name} \\| 2 \\| [\\d.]+ / [\\d.]+ / [\\d.]+ \\| [\\d.]+ \\|$`, 'm'), `${name} is measured on a page that has its controls`);
    }
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
    assert.equal(r.notes.length, 1, r.notes.join('\n'));
    assert.equal(r.notes[0], `sort edge: /db?feed=b&seg=INDEX&sym=NIFTY&key=NSE-INDEX-NIFTY&tf=1min&from=${EDGE_FROM}&to=2099-09-30 holds 19875 bars of b 1min, the longest range of days ending 2099-09-30 at or under 20000 (${(await harness.sortEdge(edgeGet)).asked} GETs; passed over a 1s: 22500 bars on 2099-09-30 alone)`);
    assert.equal(r.ok, false);
    const saved = JSON.parse(readFileSync(capture, 'utf8'));
    assert.ok(saved.requests.some((/** @type {any} */ q) => q.page === '/gaps' && q.url === '/feeds.json'));
    assert.doesNotThrow(() => sheet.readCapture(readFileSync(capture, 'utf8')));
    // A /db without the controls, and a server that cannot say where the sort edge is: each NOT MEASURED, by name.
    const bare = await browserStandIn({ bareDb: true });
    try {
      const none = await harness.runHarness(harness.parseArgs(['--base', bare.base, '--n', '1', '--pages', '/speed', '--no-idle', '--no-report-check', '--settle-ms', '100', '--step-wait-ms', '200', ...root]));
      assert.deepEqual(none.notes, [
        'picker click: NOT MEASURED — the selector [aria-controls="db-search-list"] was not found on /db',
        `feed switch: NOT MEASURED — the selector ${harness.FEED_PICKER} was not found on /db`,
        `page turn of option cells: NOT MEASURED — the selector ${harness.EXPIRY_PICKER} was not found on /db?seg=options`,
        'sort at the 20,000-row browser sort edge: NOT MEASURED — finding the sort edge: /sources.json?exchange=NSE&segment=INDEX&symbol=NIFTY answered HTTP 404'
      ]);
      assert.match(none.lines.join('\n'), /^\| sort at the 20,000-row browser sort edge \| 0 \| NOT MEASURED: finding the sort edge: .* answered HTTP 404 \| — \|$/m);
    } finally {
      bare.server.close();
    }
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
      await b.close();
    }
  } finally {
    site.server.close();
  }
});

test('the harness reads a page\'s figures through flush(), so an entry the observer has not delivered yet is still counted', () => {
  // Found on the Mac (18 Oct, load average 62): a /speed cold load that had
  // painted read "painted no first content", because a PerformanceObserver
  // delivers in a later task and the read came first. This observer never
  // delivers at all: every entry waits in takeRecords().
  /** @type {Record<string, object[]>} */
  const queued = {
    paint: [{ name: 'first-paint', startTime: 9 }, { name: 'first-contentful-paint', startTime: 12 }],
    longtask: [{ duration: 70 }, { duration: 55 }]
  };
  class Silent {
    /** @param {unknown} callback */
    constructor(callback) {
      this.callback = callback;
      this.type = '';
    }
    /** @param {{type: string}} o */
    observe(o) {
      this.type = o.type;
    }
    takeRecords() {
      const records = queued[this.type];
      queued[this.type] = [];
      return records;
    }
  }
  /** @type {any} */
  const win = {};
  new Function('window', 'PerformanceObserver', harness.OBSERVER)(win, Silent);
  assert.equal(win.__harness.fcp, null, 'nothing was delivered: the fields alone read "not painted"');
  const read = () => new Function('window', `return ${harness.READ_FIGURES}`)(win);
  assert.deepEqual(read(), { fcp: 12, longest: 70, tasks: 2 });
  assert.deepEqual(read(), { fcp: 12, longest: 70, tasks: 2 }, 'an entry is taken once, never counted twice');
  queued.longtask = [{ duration: 51 }];
  new Function('window', harness.RESET_PAUSE)(win);
  assert.equal(win.__harness.longest, 0, 'a pause queued before the reset is folded, then forgotten');
  assert.equal(win.__harness.tasks, 3);
  queued.longtask = [{ duration: 40 }];
  assert.deepEqual(read(), { fcp: 12, longest: 40, tasks: 4 }, 'only what came after the reset is the pause');
  // A browser that knows neither entry type leaves the figures empty, never invented.
  /** @type {any} */
  const bare = {};
  new Function('window', 'PerformanceObserver', harness.OBSERVER)(bare, class {
    observe() {
      throw new TypeError('unsupported entry type');
    }
  });
  assert.deepEqual(new Function('window', `return ${harness.READ_FIGURES}`)(bare), { fcp: null, longest: 0, tasks: 0 });
  // Every read in the harness goes through it.
  const h = source('ci/browser-harness.mjs');
  assert.doesNotMatch(h, /evaluate\('window\.__harness(\.longest)?'\)/, 'a read of the fields alone is back');
  assert.match(h, /tab\.evaluate\(READ_FIGURES\)/);
});

test('a load waits, bounded, for a paint the browser has not delivered yet, and a page that never paints is still said not painted', async () => {
  // Found on the Mac (20 Oct, the whole web suite running beside other
  // agents' builds): `the harness as a command…` read "/speed cold: 1 of 1
  // loads painted no first content" for a page that paints. Chrome queues
  // the first-contentful-paint entry when the frame's presentation is
  // reported, which under load came after the two frames `settle` waits for,
  // so even `takeRecords()` had nothing yet. The figure itself is the
  // browser's own timestamp, so waiting for it changes no measurement.
  /** @type {Record<string, object[]>} */
  const queued = { paint: [], longtask: [] };
  class Late {
    /** @param {unknown} callback */
    constructor(callback) {
      this.callback = callback;
      this.type = '';
    }
    /** @param {{type: string}} o */
    observe(o) {
      this.type = o.type;
    }
    takeRecords() {
      const records = queued[this.type];
      queued[this.type] = [];
      return records;
    }
  }
  assert.equal(harness.PAINT_WAIT_MS, 2_000);
  /** @type {any} */
  const win = {};
  new Function('window', 'PerformanceObserver', harness.OBSERVER)(win, Late);
  const read = (/** @type {any} */ w) => new Function('window', `return ${harness.READ_FIGURES}`)(w);
  assert.equal(read(win).fcp, null, 'read at once, the paint not yet delivered reads as none');
  setTimeout(() => queued.paint.push({ name: 'first-contentful-paint', startTime: 40 }), 120);
  const t0 = performance.now();
  const waited = await new Function('window', `return ${harness.AWAIT_PAINT}`)(win);
  const took = performance.now() - t0;
  assert.equal(waited.fcp, 40, 'the paint delivered late is the paint read');
  assert.equal(read(win).fcp, 40);
  assert.ok(took >= 100 && took < harness.PAINT_WAIT_MS, `it waited for the paint and no longer: ${took} ms`);
  // A page that never paints is not waited on for ever, and is never given a figure.
  /** @type {any} */
  const blank = {};
  new Function('window', 'PerformanceObserver', harness.OBSERVER)(blank, Late);
  const t1 = performance.now();
  const never = await new Function('window', `return ${harness.AWAIT_PAINT}`)(blank);
  const cap = performance.now() - t1;
  assert.equal(never.fcp, null);
  assert.ok(cap >= harness.PAINT_WAIT_MS - 50 && cap < harness.PAINT_WAIT_MS + 1_500, `bounded by the cap: ${cap} ms`);
  // A page that has painted is not waited on at all.
  const t2 = performance.now();
  await new Function('window', `return ${harness.AWAIT_PAINT}`)(win);
  assert.ok(performance.now() - t2 < 100);
  // Every load waits for its paint before it reads, and that wait is bounded here too, not by the page's clock alone (repair 4).
  assert.match(source('ci/browser-harness.mjs'), /await tab\.within\(AWAIT_PAINT, PAINT_WAIT_MS\);\s*h = await tab\.evaluate\(READ_FIGURES\);/);
});

test('a read-only run sends no write: the page\'s own report is blocked in the browser, named, and never reaches the server', { timeout: 300_000 }, async () => {
  const live = harness.parseArgs(['--base', 'http://127.0.0.1:1', '--read-only']);
  assert.equal(live.readOnly, true);
  assert.equal(live.reportCheck, false, '--read-only never runs the reporter check, which needs its one POST');
  assert.equal(harness.parseArgs(['--base', 'http://127.0.0.1:1']).readOnly, false);
  assert.equal(harness.parseArgs(['--base', 'http://127.0.0.1:1', '--no-report-check']).readOnly, false, '--no-report-check alone is not read only');
  const v = harness.readOnlyVerdict([
    { method: 'POST', url: 'http://h:1/page-report', body: 'page=%2Fgaps&what=request%3A+GET+%2Fbroken+got+no+answer', blocked: true },
    { method: 'POST', url: 'http://h:1/pull?feed=zerodha', body: '', blocked: false },
    { method: 'PUT', url: 'http://elsewhere/x', body: 'raw', blocked: false },
    { method: 'POST', url: 'http://h:1/page-report', body: '', blocked: true }
  ], 'http://h:1');
  assert.deepEqual(v.notes, [
    'read-only: POST /page-report was blocked in the browser and never sent (/gaps: request: GET /broken got no answer)',
    'read-only: POST /page-report was blocked in the browser and never sent'
  ]);
  assert.deepEqual(v.failures, ['read-only: POST /pull?feed=zerodha left the browser', 'read-only: PUT http://elsewhere/x left the browser (raw)']);
  // WHICH request is the report: decided by parsed origin, path and method,
  // never by a pattern that only has to appear somewhere in the address.
  const o = 'http://h:1';
  assert.equal(harness.isReportWrite({ method: 'POST', url: 'http://h:1/page-report' }, o), true);
  assert.equal(harness.isReportWrite({ method: 'PUT', url: 'http://h:1/page-report?x=1' }, o), true, 'a query is still the route');
  assert.equal(harness.isReportWrite({ method: 'GET', url: 'http://h:1/page-report' }, o), false, 'a read is never blocked');
  assert.equal(harness.isReportWrite({ method: 'HEAD', url: 'http://h:1/page-report' }, o), false);
  assert.equal(harness.isReportWrite({ method: 'POST', url: 'http://h:1/lib/page-report.js' }, o), false, 'a script named after the route is not the route');
  assert.equal(harness.isReportWrite({ method: 'POST', url: 'http://h:1/page-reports' }, o), false);
  assert.equal(harness.isReportWrite({ method: 'POST', url: 'http://elsewhere:1/page-report' }, o), false, 'another origin\'s route is not this run\'s');
  assert.equal(harness.isReportWrite({ method: 'POST', url: 'not a url' }, o), false);
  assert.equal(harness.reportPausePattern(), `*${REPORT_ROUTE}*`);
  // In a real browser: /reporting fails a request after its reporter is
  // installed (as every page of the app does: the layout imports the
  // reporter before a page asks for anything), and the reporter posts. Read
  // only, the post never reaches the stand-in while the reporter's own
  // script, whose name contains the route, still loads; without it, it does.
  const site = await browserStandIn({});
  const root = process.getuid?.() === 0 ? ['--no-sandbox'] : [];
  const common = ['--base', site.base, '--n', '1', '--pages', '/reporting', '--no-interactions', '--no-idle', '--settle-ms', '300', ...root];
  try {
    const quiet = await harness.runHarness(harness.parseArgs([...common, '--read-only']));
    assert.ok(site.requests.includes('GET /lib/page-report.js'), `the reporter's own script loaded: ${site.requests.join(', ')}`);
    assert.deepEqual(site.log, [], 'no report reached the server');
    assert.ok(!site.requests.includes(`POST ${REPORT_ROUTE}`), 'no write reached the server');
    assert.ok(quiet.notes.some((n) => /^read-only: POST \/page-report was blocked in the browser and never sent \(\/reporting: request: GET \/broken got no answer/.test(n)), quiet.notes.join('\n'));
    assert.deepEqual(quiet.failures.filter((f) => f.startsWith('read-only:')), []);
    const loud = await harness.runHarness(harness.parseArgs([...common, '--no-report-check']));
    assert.ok(site.log.some((l) => /** @type {any} */ (l).page === '/reporting'), 'without --read-only the page writes its report');
    assert.ok(!loud.notes.some((n) => n.startsWith('read-only:')));
    // A read-only tab with no base cannot know which address to block: refused
    // before the browser is asked anything, never opened unguarded.
    await assert.rejects(harness.openTab(/** @type {any} */ ({}), undefined, { readOnly: true }), /a read-only tab needs the run's base/);
  } finally {
    site.server.close();
  }
});

test('the harness refuses a browser it cannot start or reach, by name, and as a command', async () => {
  await assert.rejects(harness.launch('/nonexistent/chrome'), /no browser at "\/nonexistent\/chrome"/);
  await assert.rejects(harness.launch(''), /no browser at ""/);
  // `false` is /usr/bin/false on macOS and on current Linux images, /bin/false
  // on older ones: the first that exists (it was /bin/false alone, which the
  // Mac does not have, so the test failed there on "no browser at").
  const falseCommand = ['/usr/bin/false', '/bin/false'].find((p) => existsSync(p));
  assert.ok(falseCommand, 'a false command exists on this machine');
  await assert.rejects(harness.launch(String(falseCommand)), /the browser exited 1 before it listened/);
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
  for (const bad of ['', '{"requests":[{"method":"POST","url":"/pull"}]}']) {
    const unread = await sheet.runSheet({ ...base, feed: 'zerodha' }, { send: standIn({ ignore: new Set() }).send, log: () => {}, capture: bad });
    assert.equal(unread.ok, false);
    assert.equal(unread.rows.length, 0, 'nothing is timed from a capture that cannot be read');
    assert.match(unread.failures[0], /^the capture of the pages' own requests could not be read: /);
  }
  const lost = await sheet.runSheet(base, { send: standIn({ ignore: new Set() }).send, log: () => {} });
  assert.deepEqual(lost.failures, ['discovering the selection: /feeds.json named no feed; pass --feed']);
  assert.equal(lost.ok, false);
  // The sheet names the build that answered (repair 4): none, one, or every one of several.
  assert.equal(harness.renderHarness({ ok: false, lines: ['| t |'], notes: ['n1'], failures: ['f1'] }, { n: 2, base: 'http://h:1' }), '# Browser harness — FAIL\n\nn = 2 cold and 2 warm loads a page against http://h:1, answered by no named build.\n\n| t |\n\n- n1\n- FAILED: f1\n');
  assert.match(harness.renderHarness({ ok: true, lines: [], notes: [], failures: [], builds: ['a'.repeat(40)] }, { n: 1, base: 'http://h:1' }), /^n = 1 cold and 1 warm loads a page against http:\/\/h:1, answered by build a{40}\.$/m);
  assert.match(harness.renderHarness({ ok: false, lines: [], notes: [], failures: [], builds: ['a', 'b'] }, { n: 1, base: 'http://h:1' }), /, answered by 2 builds: a, b\.$/m);
  // A build whose app entry carries no text is refused like one with no preload list.
  assert.throws(() => builtPages(new Map([['_app/immutable/entry/app.A.js', { size: 9 }]])), /no __vite__mapDeps/);
});

// ---------------------------------------------------------------------------
// The sort edge: invented rungs in the 2090s, so no figure can be mistaken for
// a real one. A weekday holds 22,500 bars at 1s, 375 at 1min and 1 at 1day.
// ---------------------------------------------------------------------------

const EDGE_SOURCES = {
  feeds: [{ feed: 'a' }, { feed: 'b' }],
  rungs: [
    { feed: 'b', rung: '1day', stored: true, db: '/db?feed=b&seg=INDEX&sym=NIFTY&key=NSE-INDEX-NIFTY&tf=1day', first_day: '2091-01-01', last_day: '2099-09-30' },
    { feed: 'b', rung: '1min', stored: true, db: '/db?feed=b&seg=INDEX&sym=NIFTY&key=NSE-INDEX-NIFTY&tf=1min', first_day: '2091-01-01', last_day: '2099-09-30' },
    { feed: 'a', rung: '1min', stored: false, kind: 'fold', first_day: '2091-01-01', last_day: '2099-09-30' },
    { feed: 'b', rung: '2min', stored: true, first_day: '2091-01-01', last_day: '2099-09-30' },
    { feed: 'a', rung: '1s', stored: true, db: '/db?feed=a&seg=INDEX&sym=NIFTY&key=NSE-INDEX-NIFTY&tf=1s', first_day: '2091-01-01', last_day: '2099-09-30' }
  ]
};

/** Bars a weekday holds at each rung. */
const EDGE_PER_DAY = /** @type {Record<string, number>} */ ({ '1s': 22_500, '1min': 375, '1day': 1 });

/** Weekdays from `from` to `to`, both included. @param {string} from @param {string} to */
function weekdays(from, to) {
  let n = 0;
  for (let t = Date.parse(`${from}T00:00:00Z`); t <= Date.parse(`${to}T00:00:00Z`); t += 86_400_000) {
    const d = new Date(t).getUTCDay();
    if (d !== 0 && d !== 6) n += 1;
  }
  return n;
}

/** What /bars/window.json would count for one question of the sort edge. @param {URLSearchParams} q */
function edgeTotal(q) {
  return weekdays(String(q.get('from_day')), String(q.get('to_day'))) * EDGE_PER_DAY[String(q.get('timeframe'))];
}

/** The day after the 54th weekday back from 2099-09-30: the earliest day 53 weekdays (19,875 1min bars) reach. */
const EDGE_FROM = (() => {
  let t = Date.parse('2099-09-30T00:00:00Z');
  for (let seen = 0; ; t -= 86_400_000) {
    const d = new Date(t).getUTCDay();
    if (d !== 0 && d !== 6) seen += 1;
    if (seen === 54) return new Date(t + 86_400_000).toISOString().slice(0, 10);
  }
})();

/** A GET on a server holding {@link EDGE_SOURCES}. @param {string} path */
async function edgeGet(path) {
  const u = new URL(path, 'http://x');
  if (u.pathname === '/sources.json') return { status: 200, body: JSON.stringify(EDGE_SOURCES) };
  return { status: 200, body: JSON.stringify({ total: edgeTotal(u.searchParams) }) };
}

test('the sort edge is found from the data: the longest range at or under the budget, on the first rung where one day fits, or refused by name', async () => {
  /** @type {string[]} */
  const asked = [];
  /** @param {string} path */
  const get = async (path) => {
    asked.push(path);
    return edgeGet(path);
  };
  const e = await harness.sortEdge(get);
  assert.equal(e.asked, asked.length, 'every GET is counted');
  assert.deepEqual([e.feed, e.rung, e.from, e.to, e.rows], ['b', '1min', EDGE_FROM, '2099-09-30', 19_875]);
  assert.equal(e.address, `/db?feed=b&seg=INDEX&sym=NIFTY&key=NSE-INDEX-NIFTY&tf=1min&from=${EDGE_FROM}&to=2099-09-30`);
  assert.deepEqual(e.passed, ['a 1s: 22500 bars on 2099-09-30 alone'], 'a rung whose one day is past the budget is passed over and said');
  // The edge itself: one day earlier is past the budget.
  const before = new Date(Date.parse(`${EDGE_FROM}T00:00:00Z`) - 86_400_000).toISOString().slice(0, 10);
  assert.ok(weekdays(before, '2099-09-30') * 375 > LOCAL_SORT_BUDGET);
  // Halving: eight years of days in at most about a dozen counts.
  assert.ok(e.asked <= 18, `${e.asked} GETs`);
  // Every count is the page's own windowQuery, one row a question.
  for (const p of asked.filter((x) => x.startsWith('/bars/window.json?'))) {
    const q = new URL(p, 'http://x').searchParams;
    assert.equal(q.get('limit'), '1');
    assert.equal(q.get('key'), 'NSE-INDEX-NIFTY');
  }
  assert.ok(asked[0].startsWith('/sources.json?'), 'the rungs are asked first, through rungsUrl');
  // A history that fits whole is taken whole; exactly the budget fits.
  const only = { feeds: [{ feed: 'b' }], rungs: [EDGE_SOURCES.rungs[1]] };
  const small = await harness.sortEdge(async (p) => (p.startsWith('/sources') ? { status: 200, body: JSON.stringify(only) } : edgeGet(p)), 'NSE-INDEX-NIFTY', 375 * weekdays('2091-01-01', '2099-09-30'));
  assert.deepEqual([small.rung, small.from, small.rows], ['1min', '2091-01-01', 375 * weekdays('2091-01-01', '2099-09-30')]);
  // A budget no day of any rung fits: refused, naming every rung passed over.
  await assert.rejects(harness.sortEdge(get, 'NSE-INDEX-NIFTY', 0), /no stored rung of NSE-INDEX-NIFTY holds a day at or under 0 bars \(a 1s: 22500 bars on 2099-09-30 alone; b 1min: 375 bars on 2099-09-30 alone; b 1day: 1 bars on 2099-09-30 alone\)$/);
  // A server that refuses, an answer with no whole total, garbage rungs and a key rungsUrl cannot ask about.
  await assert.rejects(harness.sortEdge(async () => ({ status: 503, body: '' })), /\/sources\.json\?exchange=NSE&segment=INDEX&symbol=NIFTY answered HTTP 503/);
  await assert.rejects(harness.sortEdge(async (p) => (p.startsWith('/sources') ? { status: 200, body: JSON.stringify(EDGE_SOURCES) } : { status: 200, body: '{"total":-1}' })), /answered no whole total/);
  await assert.rejects(harness.sortEdge(async (p) => (p.startsWith('/sources') ? { status: 200, body: JSON.stringify(EDGE_SOURCES) } : { status: 200, body: '{}' })), /answered no whole total/);
  await assert.rejects(harness.sortEdge(async () => ({ status: 200, body: JSON.stringify({ feeds: 'x', rungs: [{ stored: true, db: '/db', first_day: 'soon', last_day: '2099-09-30' }, null] }) })), /no stored rung of NSE-INDEX-NIFTY holds a day at or under 20000 bars$/);
  await assert.rejects(harness.sortEdge(get, 'not a key'), /not a key is not an instrument key rungsUrl can ask about/);
  // A feed the server does not list sorts after those it does.
  const late = await harness.sortEdge(async (p) => (p.startsWith('/sources') ? { status: 200, body: JSON.stringify({ feeds: [{ feed: 'b' }], rungs: [EDGE_SOURCES.rungs[4], EDGE_SOURCES.rungs[1]] }) } : get(p)));
  assert.deepEqual([late.feed, late.passed], ['b', []]);
});

test('the Speed page watches its own load and never reads it once in a timer', () => {
  // Before: `setTimeout(() => (self = pageSelf(performance)), 0)` inside
  // onMount read the timeline before the page had painted and before its last
  // fetches ended (the R3-15 re-check, 21 Oct).
  const page = source('src/routes/speed/+page.svelte');
  assert.match(page, /const stopSelf = watchSelf\(typeof window === 'undefined' \? null : window, \(reading\) => \(self = reading\)\);/);
  assert.match(page, /stopSelf\(\);/, 'the watch stops with the page');
  assert.match(page, />\{selfText\(self, here\)\}<\/p>/);
  assert.match(page, /data-wire-bytes=\{self\.wire === 'observed' \? self\.wireBytes : undefined\}/);
  assert.match(page, /data-first-content-ms=\{self\.firstContentMs \?\? undefined\}/);
  assert.doesNotMatch(page, /setTimeout|getEntriesByType|pageSelf/);
});

test('the Speed page\'s own line in a real browser: the tab\'s own first content and its whole wire, after the load', { timeout: 120_000 }, async () => {
  const chrome = harness.parseArgs(['--base', 'http://h:1']).chrome;
  assert.ok(chrome, 'this test needs Chrome or Chromium: set BRUTEX_CHROME');
  const site = await browserStandIn({});
  const b = await harness.launch(chrome, process.getuid?.() === 0);
  try {
    for (let i = 0; i < 3; i += 1) {
      const { browserContextId } = await b.cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
      const tab = await harness.openTab(b.cdp, browserContextId, { readOnly: true, base: site.base });
      await harness.loadOnce(tab, b.cdp, `${site.base}/self`, 300);
      const v = await tab.evaluate(`(() => {
        const fcp = performance.getEntriesByType('paint').find((e) => e.name === 'first-contentful-paint');
        const loads = [...performance.getEntriesByType('navigation'), ...performance.getEntriesByType('resource')];
        const line = document.querySelector('[data-speed-self]');
        return {
          text: line.textContent,
          drawnFirstContent: line.dataset.firstContentMs,
          drawnBytes: Number(line.dataset.wireBytes),
          drawnFetches: Number(line.dataset.fetches),
          browserFirstContent: fcp ? Math.round(fcp.startTime) : null,
          browserBytes: loads.reduce((a, e) => a + e.transferSize, 0),
          browserFetches: loads.length
        };
      })()`);
      assert.equal(typeof v.browserFirstContent, 'number', 'Chrome reports first content');
      assert.equal(v.drawnFirstContent, String(v.browserFirstContent), `the drawn first content is the browser's: ${v.text}`);
      assert.match(v.text, new RegExp(`first content ${v.browserFirstContent} ms`));
      assert.doesNotMatch(v.text, /not reported/, 'a browser that reports a figure is never said not to');
      assert.equal(v.drawnBytes, v.browserBytes, 'the drawn wire is the tab\'s own navigation and resource transferSize');
      assert.equal(v.drawnFetches, v.browserFetches);
      assert.ok(v.browserBytes > 450_000, 'the fetch after the load is on the wire');
      assert.match(v.text, /on the wire over \d+ fetches \(not under the 400 KB target\)/, 'and a load over the target is marked over it');
      assert.match(v.text, /after the load; a later fetch is added when it ends\.$/);
      await tab.close();
      await b.cdp.send('Target.disposeBrowserContext', { browserContextId });
    }
  } finally {
    await b.close();
    site.server.close();
  }
});

test('a report costs the same however long the failure\'s text, and a cut text always ends with an ellipsis', () => {
  // Before: clean() walked the whole text, 250 ms for 4,000,000 characters (the R3-15 re-check, 21 Oct).
  const huge = `${'é'.repeat(2_000)}${'x'.repeat(4_000_000)}`;
  assert.equal(reportBody('/db', huge), reportBody('/db', 'é'.repeat(2_000)), 'what is past the cut never fits, so the body is the same');
  /** @type {number[]} */
  const ms = [];
  for (let i = 0; i < 5; i += 1) {
    const t = performance.now();
    reportBody('/db', huge);
    ms.push(performance.now() - t);
  }
  assert.ok(Math.min(...ms) < 25, `a 4,000,002-character text cleaned in ${ms.map((m) => m.toFixed(2)).join(', ')} ms`);
  // A text whose readable words start past the cut: the cut is said, never passed off as the whole report.
  assert.equal(decodeURIComponent(String(reportBody('/db', `x${' '.repeat(4_000_000)}y`)).split('what=')[1]), 'x…');
  assert.equal(decodeURIComponent(String(reportBody('/db', '\u0001'.repeat(5_000))).split('what=')[1]), '…', 'a cut text that cleans to nothing is still said as cut');
  assert.equal(reportBody(`/${'a'.repeat(4_000_000)}`, 'x'), null, 'a page path of any length is refused as past 256 bytes');
  assert.equal(reportBody('/db', 'é'.repeat(MAX_REPORT_BYTES + 1)), reportBody('/db', 'é'.repeat(2_000)), 'one code point past the cut is cut');
  // Longer than the cut in UTF-16 units but not in code points (an emoji is two units, one code point): not cut
  // early, only fitted, whole code points and `…`, the same body as for a text the cut did shorten.
  const wide = decodeURIComponent(String(reportBody('/db', '😀'.repeat(600))).split('what=')[1]);
  assert.ok(wide.endsWith('…') && [...wide.slice(0, -1)].every((c) => c === '😀'), wide);
  assert.equal(reportBody('/db', '😀'.repeat(600)), reportBody('/db', '😀'.repeat(MAX_REPORT_BYTES * 2)));
  // The reporter keys and sends the cut text: one long failure is one report.
  /** @type {string[]} */
  const sent = [];
  const r = createPageReporter({ send: async (body) => void sent.push(body), page: () => '/db', now: () => 0 });
  assert.equal(r.report('error', huge), true);
  assert.equal(r.report('error', `${'é'.repeat(2_000)}${'y'.repeat(10)}`), false, 'the same first 1,025 code points are the same failure');
});

test('the ration never holds reports back for a clock that went back, and its own clock only moves forward', async () => {
  let now = 3_600_000;
  /** @type {string[]} */
  const sent = [];
  /** @type {string[]} */
  const warned = [];
  const r = createPageReporter({ send: async (body) => void sent.push(body), page: () => '/db', now: () => now, warn: (l) => warned.push(l) });
  assert.equal(r.report('error', 'a'), true);
  now = 0;
  assert.equal(r.report('error', 'a'), true, 'a failure last sent "an hour from now" is not held back for that hour');
  for (let i = 0; i < REPORTS_PER_MINUTE - 1; i += 1) r.report('request', `GET /x${i}`);
  assert.equal(r.report('error', 'b'), false, 'the ration holds inside the new window');
  now = MINUTE_MS;
  assert.equal(r.report('error', 'b'), true, 'and a minute later the window is new');
  // A step back inside the fault and send-failure warnings: said again, not silent for an hour.
  let page = '/db';
  const faulty = createPageReporter({ send: async () => { throw new Error('HTTP 503'); }, page: () => { if (page === '') throw new Error('no page'); return page; }, now: () => now, warn: (l) => warned.push(l) });
  now = 3_600_000;
  faulty.report('error', 'c');
  await flush();
  page = '';
  faulty.report('error', 'd');
  now = 0;
  faulty.report('error', 'e');
  page = '/db';
  faulty.report('error', 'f');
  await flush();
  assert.equal(warned.filter((l) => l.includes('could not write a report (no page)')).length, 2, warned.join('\n'));
  assert.equal(warned.filter((l) => l.includes('could not take the report')).length, 2, warned.join('\n'));
  assert.match(source('src/lib/page-report.js'), /const now = options\.now \?\? monotonic;/);
  assert.match(source('src/lib/page-report.js'), /typeof performance !== 'undefined' && typeof performance\.now === 'function' \? \(\) => performance\.now\(\)/);
});

test('the harness removes only a profile it made in this process, refuses a run past its ceiling, and a page named twice', () => {
  // Before: the name alone was checked, so a directory that merely looked like a profile was removed.
  // Made in a directory of this test's own, so a run never adds a look-alike to the temporary
  // directory, where the harness's own profiles are counted (L-R315-14).
  const lookalike = mkdtempSync(join(mkdtempSync(join(tmpdir(), 'r315-lookalike-')), harness.PROFILE_PREFIX));
  writeFileSync(join(lookalike, 'kept'), 'kept');
  assert.throws(() => harness.removeProfile(lookalike), /refusing to remove .*: not a profile this harness made in this process/);
  assert.equal(readFileSync(join(lookalike, 'kept'), 'utf8'), 'kept', 'a look-alike is untouched');
  assert.equal(harness.parseArgs(['--base', 'http://h:1', '--n', String(harness.MAX_N)]).n, harness.MAX_N);
  assert.throws(() => harness.parseArgs(['--base', 'http://h:1', '--n', String(harness.MAX_N + 1)]), /--n must be at most 10000/);
  assert.throws(() => harness.parseArgs(['--base', 'http://h:1', '--n', '1e9']), /--n must be at most 10000/);
  assert.throws(() => harness.parseArgs(['--base', 'http://h:1', '--pages', '/db,/speed,/db']), /--pages names \/db twice: each page is measured once a run/);
  assert.deepEqual(harness.parseArgs(['--base', 'http://h:1', '--pages', '/db,/speed']).pages, ['/db', '/speed']);
  assert.equal(sheet.parseArgs(['--base', 'http://h:1', '--n', '10000']).n, 10_000);
  assert.throws(() => sheet.parseArgs(['--base', 'http://h:1', '--n', '10001']), /--n must be at most 10000/);
  // A file that cannot be opened is said as the sheet's own refusal, naming the path.
  const dir = mkdtempSync(join(tmpdir(), 'r315-open-'));
  assert.throws(() => sheet.writeWhole(join(dir, 'missing', 'sheet.md'), 'x'), /^Error: could not write .*\/missing\/sheet\.md whole: ENOENT/);
});

test('a browser that ignores SIGTERM is killed after the wait, and a profile that cannot be removed at exit is said, never thrown', { timeout: 120_000 }, async () => {
  const fake = mkdtempSync(join(tmpdir(), 'r315-stubborn-'));
  const stubborn = join(fake, 'stubborn');
  // SIGTERM ignored, and the ignore survives the exec: only SIGKILL ends it.
  writeFileSync(stubborn, "#!/bin/sh\ntrap '' TERM\necho \"DevTools listening on ws://127.0.0.1:1/devtools/browser/x\" >&2\nexec sleep 60\n", { mode: 0o755 });
  const root = mkdtempSync(join(tmpdir(), 'r315-stubborn-root-'));
  const t = Date.now();
  await assert.rejects(harness.launch(stubborn, false, 30_000, root), /could not connect to ws:\/\/127\.0\.0\.1:1/);
  const took = Date.now() - t;
  assert.ok(took >= harness.STOP_WAIT_MS - 50, `the browser was given ${harness.STOP_WAIT_MS} ms after SIGTERM before SIGKILL (took ${took} ms)`);
  assert.deepEqual(readdirSync(root), [], 'and its profile is gone once it has left');
  if (process.getuid?.() === 0) return; // root removes from a read-only directory: the next case cannot be built as root.
  // At exit, a profile whose directory is read-only: the guard says so on stderr and the process still ends as it would.
  const chrome = harness.parseArgs(['--base', 'http://h:1']).chrome;
  assert.ok(chrome, 'this test needs Chrome or Chromium: set BRUTEX_CHROME');
  const locked = mkdtempSync(join(tmpdir(), 'r315-locked-'));
  const script = join(fake, 'locks.mjs');
  writeFileSync(script, [
    `import * as h from ${JSON.stringify(new URL('ci/browser-harness.mjs', web).href)};`,
    'import { chmodSync } from "node:fs";',
    `const b = await h.launch(${JSON.stringify(chrome)}, false, 30000, process.argv[2]);`,
    'process.stdout.write(JSON.stringify({ pid: b.pid, profile: b.profile }) + "\\n");',
    'chmodSync(process.argv[2], 0o555);',
    'process.exit(0);'
  ].join('\n'));
  const { spawnSync } = await import('node:child_process');
  const ran = spawnSync(process.execPath, [script, locked], { encoding: 'utf8', timeout: 60_000 });
  const { chmodSync } = await import('node:fs');
  chmodSync(locked, 0o755);
  assert.equal(ran.status, 0, ran.stderr);
  const started = JSON.parse(ran.stdout.split('\n')[0]);
  assert.match(ran.stderr, new RegExp(`browser-harness: could not stop the browser ${started.pid} or remove ${started.profile.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}: `));
  /** @param {number} pid */
  const running = (pid) => {
    try {
      process.kill(pid, 0);
      return true;
    } catch {
      return false;
    }
  };
  const end = Date.now() + 10_000;
  while (running(started.pid) && Date.now() < end) await new Promise((r) => setTimeout(r, 50));
  assert.equal(running(started.pid), false, 'the browser was killed even though its profile could not be removed');
});

test('a browser\'s helpers end with it: nothing writes into its profile after the harness removed it, on a stop, an exit or a signal', { timeout: 120_000 }, async () => {
  // Before (the R3-15 repair 2 run, 22 Oct, at a load average near 140): a
  // harness sent SIGTERM killed the browser alone and removed its profile at
  // once, and Chrome's network service, still running, wrote
  // `Default/Reporting and NEL` into it again, so the profile was left. This
  // browser's helper does the same on purpose: it ignores SIGTERM, outlives
  // the browser, and writes into the profile every 20 ms until it is killed.
  const fake = mkdtempSync(join(tmpdir(), 'r315-helper-'));
  /** @param {string} ws the DevTools address it prints @param {string} pidFile where the helper writes its pid */
  const browserWithHelper = (ws, pidFile) => {
    const path = join(fake, `browser-${readdirSync(fake).length}`);
    writeFileSync(path, [
      '#!/bin/sh',
      'for a in "$@"; do case "$a" in --user-data-dir=*) P="${a#--user-data-dir=}";; esac; done',
      `sh -c 'trap "" TERM; echo $$ > "$1"; while :; do mkdir -p "$2/Default" && : > "$2/Default/Reporting and NEL"; sleep 0.02; done' helper ${JSON.stringify(pidFile)} "$P" &`,
      // Listening only once the helper ignores SIGTERM: its pid is written after its trap.
      `while [ ! -s ${JSON.stringify(pidFile)} ]; do sleep 0.01; done`,
      `echo "DevTools listening on ${ws}" >&2`,
      'exec sleep 60'
    ].join('\n') + '\n', { mode: 0o755 });
    return path;
  };
  /** @param {number} pid */
  const running = (pid) => {
    try {
      process.kill(pid, 0);
      return true;
    } catch {
      return false;
    }
  };
  /** @param {string} pidFile */
  const helperOf = async (pidFile) => {
    const end = Date.now() + 10_000;
    while (!existsSync(pidFile) || readFileSync(pidFile, 'utf8').trim() === '') {
      assert.ok(Date.now() < end, 'the helper started');
      await new Promise((r) => setTimeout(r, 20));
    }
    return Number(readFileSync(pidFile, 'utf8'));
  };
  /** @param {number} helper @param {string} root @param {string} how */
  const nothingLeft = async (helper, root, how) => {
    try {
      const end = Date.now() + 5_000;
      while (running(helper) && Date.now() < end) await new Promise((r) => setTimeout(r, 20));
      assert.equal(running(helper), false, `${how}: the helper that ignored SIGTERM was ended with the browser's group`);
      // Ten of the helper's 20 ms writes: a helper still running would have written the profile again by now.
      await new Promise((r) => setTimeout(r, 200));
      assert.deepEqual(readdirSync(root), [], `${how}: nothing wrote the profile again after it was removed`);
    } finally {
      if (running(helper)) process.kill(helper, 'SIGKILL');
    }
  };
  // A stop: the launch fails (nothing listens on port 1), so the harness stops what it started.
  const stopRoot = mkdtempSync(join(tmpdir(), 'r315-helper-root-'));
  const stopPid = join(fake, 'helper-stop.pid');
  const t = Date.now();
  await assert.rejects(harness.launch(browserWithHelper('ws://127.0.0.1:1/devtools/browser/x', stopPid), false, 30_000, stopRoot), /could not connect to ws:\/\/127\.0\.0\.1:1/);
  const took = Date.now() - t;
  // First, so a helper left running is killed (its finally) before anything else can fail.
  await nothingLeft(await helperOf(stopPid), stopRoot, 'stop');
  assert.ok(took >= harness.STOP_WAIT_MS - 50, `a helper that outlived the browser was given the same wait before it was killed (took ${took} ms)`);
  // An exit and a signal: a harness that launched, against a DevTools address that answers the handshake.
  const script = join(fake, 'ends.mjs');
  writeFileSync(script, [
    `import * as h from ${JSON.stringify(new URL('ci/browser-harness.mjs', web).href)};`,
    'import http from "node:http";',
    'import { createHash } from "node:crypto";',
    'const server = http.createServer();',
    'server.on("upgrade", (req, socket) => {',
    '  const accept = createHash("sha1").update(`${req.headers["sec-websocket-key"]}258EAFA5-E914-47DA-95CA-C5AB0DC85B11`).digest("base64");',
    '  socket.write(`HTTP/1.1 101 Switching Protocols\\r\\nUpgrade: websocket\\r\\nConnection: Upgrade\\r\\nSec-WebSocket-Accept: ${accept}\\r\\n\\r\\n`);',
    '});',
    'await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));',
    'process.env.R315_WS = `ws://127.0.0.1:${server.address().port}/devtools/browser/x`;',
    'const b = await h.launch(process.argv[2], false, 30000, process.argv[3]);',
    'process.stdout.write(JSON.stringify({ pid: b.pid, profile: b.profile }) + "\\n");',
    'if (process.argv[4] === "exit") setTimeout(() => process.exit(0), 100);'
  ].join('\n'));
  const { spawn: run } = await import('node:child_process');
  for (const how of ['exit', 'SIGTERM']) {
    const root = mkdtempSync(join(tmpdir(), `r315-helper-${how}-`));
    const pidFile = join(fake, `helper-${how}.pid`);
    const child = run(process.execPath, [script, browserWithHelper('$R315_WS', pidFile), root, how], { stdio: ['ignore', 'pipe', 'pipe'] });
    let err = '';
    child.stderr.on('data', (c) => (err += c));
    const line = await new Promise((resolve, reject) => {
      let text = '';
      child.stdout.on('data', (c) => {
        text += c;
        if (text.includes('\n')) resolve(text.split('\n')[0]);
      });
      child.on('exit', (code) => reject(new Error(`${how}: the harness left (${code}) before it launched: ${err}`)));
    });
    const started = JSON.parse(String(line));
    const helper = await helperOf(pidFile);
    const ended = new Promise((resolve) => child.on('exit', (code, signal) => resolve({ code, signal })));
    if (how === 'SIGTERM') child.kill('SIGTERM');
    const { code, signal } = /** @type {{code: number | null, signal: string | null}} */ (await ended);
    assert.deepEqual([code, signal], how === 'exit' ? [0, null] : [null, 'SIGTERM'], `${how} keeps its meaning: ${err}`);
    assert.equal(err, '', `${how}: the guard said nothing went wrong`);
    const end = Date.now() + 5_000;
    while (running(started.pid) && Date.now() < end) await new Promise((r) => setTimeout(r, 20));
    assert.equal(running(started.pid), false, `${how}: the browser was killed`);
    await nothingLeft(helper, root, how);
  }
});

test('a stop signals a group only while it is the browser\'s: a group that has gone reads as gone, and then only the browser\'s own handle is signalled', async () => {
  const { spawn } = await import('node:child_process');
  assert.equal(harness.groupRunning(undefined), false, 'a browser that never started has no group');
  const gone = spawn(process.execPath, ['-e', ''], { detached: true });
  await new Promise((resolve) => gone.on('exit', resolve));
  assert.equal(harness.groupRunning(gone.pid), false, 'a group whose every process has left is not running');
  /** @type {(string | number)[]} */
  const sent = [];
  /** @param {string | number} s */
  const handle = (s) => (sent.push(s), false);
  // Signal 0, which signals nothing: an id freed a moment ago is never sent a real signal by this test.
  harness.signalGroup({ pid: gone.pid, kill: handle }, 0);
  assert.deepEqual(sent, [0], 'a group that cannot be signalled falls to the browser\'s own handle, and nothing is thrown');
  const live = spawn(process.execPath, ['-e', 'setTimeout(() => {}, 30000)'], { detached: true });
  try {
    assert.equal(harness.groupRunning(live.pid), true, 'a detached child leads a group of its own');
    harness.signalGroup({ pid: live.pid, kill: handle }, 'SIGKILL');
    const end = Date.now() + 5_000;
    while (harness.groupRunning(live.pid) && Date.now() < end) await new Promise((r) => setTimeout(r, 20));
    assert.equal(harness.groupRunning(live.pid), false, 'SIGKILL to the group ended it');
    assert.deepEqual(sent, [0], 'through the group, never the handle');
  } finally {
    live.kill('SIGKILL');
  }
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * A stand-in for the app in a browser: pages that load the REAL
 * `$lib/page-report.js`, a `/page-report` that keeps what it is sent, and a
 * `/logs.json` that serves it back.
 *
 * AS THE APP'S BUILD DOES (repair 4): every page names its app entry
 * (`<link rel="modulepreload">` to `/_app/immutable/entry/app.T1.js`), whose
 * `dictionary` lists the stand-in's routes, so the harness can ask which
 * page was drawn; every answer is stamped with `x-brutex-build` (`build`;
 * `null` stamps none, a function is asked per answer). `fallback` answers
 * every other path 200 with a shell, as the api does; `noEntry` drops the
 * entry from every page; `/stall` answers its headers and part of its body
 * and never ends, `/hang` never answers. `stop()` ends every connection.
 *
 * `hangSources` never answers `/sources.json`; `onFirstPage` is called once,
 * when the first page is asked for.
 *
 * @param {{noReporter?: boolean, reportStatus?: number, forget?: boolean, bareDb?: boolean, fallback?: boolean,
 *   noEntry?: boolean, build?: string | null | ((path: string) => string), hangSources?: boolean, onFirstPage?: () => void}} o
 */
async function browserStandIn(o) {
  const reporter = source('src/lib/page-report.js');
  const speedLib = source('src/lib/speed.js');
  /** @type {object[]} */
  const log = [];
  /** @type {string[]} every request the stand-in was sent, `METHOD /path` */
  const requests = [];
  const script = o.noReporter ? '' : `<script type="module">import '/lib/page-report.js';</script>`;
  const entry = '/_app/immutable/entry/app.T1.js';
  const preload = o.noEntry ? '' : `<link rel="modulepreload" href="${entry}">`;
  /** @param {string} title @param {string} [extra] */
  const page = (title, extra = '') => `<!doctype html><meta charset="utf-8"><title>${title}</title>${preload}<body><h1>${title}</h1>${extra}${script}`;
  // A ROUND TRIP INSIDE ONE DOCUMENT, as the app's router makes one: a link
  // is taken in place (`history.pushState`, nothing loaded), the page in view
  // is unmounted and the next one mounted, and the browser's back button
  // mounts again by `popstate`. `/trip` draws the Speed page's own line with
  // the REAL `$lib/speed.js`, mounted as the page mounts it; `/trip-db`
  // fetches 450,000 bytes, as `/db` fetches its own.
  const trip = '<!doctype html><meta charset="utf-8"><title>trip</title><body><nav><a id="to-db" href="/trip-db">db</a> <a id="to-speed" href="/trip">speed</a></nav><main></main><script type="module">' +
    "import { watchSelf, selfText } from '/lib/speed.js'; const main = document.querySelector('main'); let stop = null;" +
    "const mount = () => { stop?.(); stop = null; main.textContent = ''; delete main.dataset.done;" +
    " if (location.pathname === '/trip') { const line = document.createElement('p'); line.setAttribute('data-speed-self', ''); main.append(line);" +
    "  stop = watchSelf(window, (r) => { line.textContent = selfText(r, '/trip'); line.dataset.wireBytes = String(r.wireBytes); line.dataset.fetches = String(r.fetches); }); }" +
    " else { main.textContent = 'db'; fetch('/big').then((a) => a.text()).then(() => (main.dataset.done = 'yes')); } };" +
    "document.addEventListener('click', (e) => { const a = e.target.closest('a'); if (!a) return; e.preventDefault(); history.pushState({}, '', a.getAttribute('href')); mount(); });" +
    "addEventListener('popstate', mount); mount();</script>";
  /** @type {Record<string, string>} */
  const pages = {
    '/trip': trip,
    '/trip-db': trip,
    '/db': o.bareDb ? page('db') : page('db', `<input aria-controls="db-search-list"><div id="db-search-list"></div>` +
      // The feed picker and the expiry picker, each the shared Picker as /db draws it: a menu of radios.
      `<div class="field"><span class="lab">Broker feed</span><div class="picker"><button class="pbtn" title="Feed A. THE PAGE'S WHOLE SCOPE: …" onclick="menu(this, ['Feed A', 'Feed B'], 0, () => fetch('/feeds.json'))">Feed A</button></div></div>` +
      `<div class="field"><span class="lab" title="Which contract — futures and options show nothing without one.">Expiry</span><div class="picker"><button class="pbtn" onclick="menu(this, ['None chosen', '24 Sep 2099'], -1, () => { document.querySelector('#next').disabled = false; })">None chosen</button></div></div>` +
      `<table><tr><th aria-sort="none"><button class="sortbtn" title="Change % — the browser orders this column itself, over at most 20,000 bars.">chg</button></th></tr></table><button id="next" aria-label="Next page" disabled onclick="fetch('/feeds.json')">next</button>` +
      `<script>function menu(at, names, held, then) { const m = document.createElement('div'); m.className = 'pmenu'; for (const [i, n] of names.entries()) { const l = document.createElement('label'); const r = document.createElement('input'); r.type = 'radio'; r.name = 'pk'; r.checked = i === held; r.onchange = () => { m.remove(); then(); }; l.append(r, n); m.append(l); } at.after(m); }` +
      `document.querySelector('input').addEventListener('input', () => { document.querySelector('#db-search-list').innerHTML = '<div role="option" onclick="fetch(\\'/feeds.json\\')">NIFTY</div>'; }); setInterval(() => fetch('/inspection.json', { method: 'HEAD' }), 300);</script>`),
    '/gaps': page('gaps', `<script>fetch('/feeds.json'); fetch('/broken').catch(() => null); setInterval(() => fetch('/feeds.json'), 300); setTimeout(() => { throw new Error('boom'); }, 0); setTimeout(() => { throw 'plain'; }, 0);</script>`),
    '/speed': page('speed', `<script>new Worker('/lib/worker.js');</script>`),
    '/blank': `<!doctype html><title>blank</title>${preload}<body>${script}`,
    // THE SPEED PAGE'S OWN LINE, drawn by the REAL `$lib/speed.js` as the
    // page draws it: client-rendered (nothing to paint until the module has
    // run), and a fetch of 450,000 bytes after the load, so a reading taken
    // at mount has neither the paint nor the whole wire. Opened at `/self/`
    // it does what the app's router does first: it drops the trailing slash
    // from the address (`history.replaceState`), so the page's path is
    // `/self` while the document is `/self/` (repair 4).
    '/self': `<!doctype html><meta charset="utf-8"><title>self</title>${preload}<body><p data-speed-self></p><script type="module">` +
      "import { watchSelf, selfText, selfWhose } from '/lib/speed.js'; const line = document.querySelector('[data-speed-self]');" +
      "if (location.pathname.length > 1 && location.pathname.endsWith('/')) history.replaceState(history.state, '', location.pathname.slice(0, -1) + location.search);" +
      'watchSelf(window, (r) => { line.textContent = selfText(r, location.pathname); line.dataset.figuresOf = selfWhose(r, location.pathname); line.dataset.firstContentMs = r.firstContentMs ?? ""; line.dataset.wireBytes = String(r.wireBytes); line.dataset.fetches = String(r.fetches); });' +
      "addEventListener('load', () => setTimeout(() => fetch('/big').then((a) => a.text()), 100));</script>",
    // A request failed AFTER the reporter is installed, as on every page of the app.
    '/reporting': page('reporting', `<script type="module">import '/lib/page-report.js'; fetch('/broken').catch(() => null);</script>`),
    // A page the first time it is asked; every later ask stalls (see the server below).
    '/later': page('later')
  };
  // The routes the stand-in's build would list: its pages, `/self/` being `/self`.
  const dictionary = Object.fromEntries(Object.keys(pages).map((p, i) => [p, [i + 2]]));
  /** Every answer the stand-in holds open, ended by `stop()`. @type {import('node:http').ServerResponse[]} */
  const held = [];
  /** @type {{seen: boolean, later: Map<string, number>}} */
  const firstPage = { seen: false, later: new Map() };
  /** @param {string} path */
  const stamp = (path) => (o.build === null ? {} : { 'x-brutex-build': typeof o.build === 'function' ? o.build(path) : (o.build ?? 'f'.repeat(40)) });
  const server = http.createServer((req, res) => {
    const u = new URL(String(req.url), 'http://x');
    requests.push(`${req.method} ${u.pathname}`);
    /** @param {number} status @param {string} type @param {string} body @param {Record<string, string>} [extra] */
    const send = (status, type, body, extra = {}) => {
      res.writeHead(status, { 'content-type': type, ...stamp(u.pathname), ...extra });
      res.end(body);
    };
    if (u.pathname === entry) return send(200, 'text/javascript', `export const dictionary = ${JSON.stringify(dictionary)};`);
    if (u.pathname === '/stall') {
      res.writeHead(200, { 'content-type': 'text/html; charset=utf-8', ...stamp(u.pathname) });
      res.write(`<!doctype html><title>stall</title>${preload}<p>part of a page; the rest never comes`);
      return void held.push(res);
    }
    if (u.pathname === '/hang') return void held.push(res);
    // `/later` is a page the first `after` times it is asked (1 unless `?after=` says), and stalls as `/stall` does every time after.
    const asks = (firstPage.later.get(u.pathname + u.search) ?? 0) + 1;
    firstPage.later.set(u.pathname + u.search, asks);
    if (u.pathname === '/later' && asks > Number(u.searchParams.get('after') ?? 1)) {
      res.writeHead(200, { 'content-type': 'text/html; charset=utf-8', ...stamp(u.pathname) });
      res.write(`<!doctype html><title>later</title>${preload}<p>part of a page; the rest never comes`);
      return void held.push(res);
    }
    // A page whose app entry, named in a script it does not run, is never answered.
    if (u.pathname === '/hang-entry') return send(200, 'text/html; charset=utf-8', '<!doctype html><title>hang-entry</title><body><h1>hang-entry</h1><script type="text/plain">import("/_app/immutable/entry/app.HANG.js")</script>');
    if (u.pathname === '/_app/immutable/entry/app.HANG.js') return void held.push(res);
    if (o.hangSources && u.pathname === '/sources.json') return void held.push(res);
    if (o.onFirstPage && u.pathname in pages && !firstPage.seen) {
      firstPage.seen = true;
      o.onFirstPage();
    }
    if (u.pathname === '/self/') return send(200, 'text/html; charset=utf-8', pages['/self']);
    if (u.pathname === '/lib/page-report.js') return send(200, 'text/javascript', reporter);
    if (u.pathname === '/lib/speed.js') return send(200, 'text/javascript', speedLib);
    if (u.pathname === '/big') return send(200, 'text/plain', 'x'.repeat(450_000), { 'cache-control': 'no-store' });
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
    if (u.pathname === '/sources.json' && !o.bareDb) return send(200, 'application/json', JSON.stringify(EDGE_SOURCES));
    if (u.pathname === '/bars/window.json' && !o.bareDb) return send(200, 'application/json', JSON.stringify({ total: edgeTotal(u.searchParams) }));
    if (u.pathname === '/broken') return void res.socket?.destroy();
    if (u.pathname in pages) return send(200, 'text/html; charset=utf-8', pages[u.pathname]);
    // As the api answers a path it has no file for: the app's shell, 200 (which reads what the shell reads).
    if (o.fallback && req.method === 'GET') return send(200, 'text/html; charset=utf-8', page('shell', "<script>fetch('/feeds.json');</script>"));
    send(404, 'text/plain', 'no');
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', () => resolve(undefined)));
  const stop = () => {
    for (const r of held) r.destroy();
    server.closeAllConnections();
    server.close();
  };
  return { server, log, requests, stop, base: `http://127.0.0.1:${/** @type {import('node:net').AddressInfo} */ (server.address()).port}` };
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

// ---------------------------------------------------------------------------
// Repair round 1 (20 Oct): the re-check's findings, each failing before.
// ---------------------------------------------------------------------------

test('a lone surrogate is written as U+FFFD, and a reporter that throws never changes the page\'s own answer', async () => {
  // Before: encodeURIComponent threw URIError on the lone half.
  const body = /** @type {string} */ (reportBody('/db\uD800', 'cut \uDC00 here 😀 whole'));
  assert.equal(new URLSearchParams(body).get('page'), `/db${REPLACEMENT}`);
  assert.equal(new URLSearchParams(body).get('what'), `cut ${REPLACEMENT} here 😀 whole`, 'a paired surrogate is kept, a lone one replaced');
  const long = /** @type {string} */ (reportBody('/db', `${'\uD800'.repeat(2_000)}`));
  assert.ok(Buffer.byteLength(long) <= MAX_REPORT_BYTES, 'a text of lone halves is still cut inside the ceiling');
  // Installed: a 500 whose URL holds a lone half is ANSWERED, and reported.
  /** @type {{url: string, init: any}[]} */
  const calls = [];
  const win = fakeWindow(async (/** @type {any} */ input, /** @type {any} */ init) => {
    calls.push({ url: String(input), init });
    if (String(input) === REPORT_ROUTE) return response(202);
    if (String(input).includes('net')) throw new TypeError('Failed to fetch');
    return response(500);
  });
  /** @type {string[]} */
  const warned = [];
  let clock = 0;
  const off = installPageReport(win, { now: () => clock, warn: (line) => warned.push(line) });
  const answered = await win.fetch('/x.json?q=\uD800');
  assert.equal(answered.status, 500, 'the page gets the answer the server gave, not a rejection');
  await assert.rejects(win.fetch('/net.json?q=\uDBFF'), /Failed to fetch/, 'and a request with no answer throws its own error, not the reporter\'s');
  await flush();
  const sent = calls.filter((c) => c.url === REPORT_ROUTE).map((c) => new URLSearchParams(c.init.body).get('what'));
  assert.deepEqual(sent, [`request: GET /x.json?q=${REPLACEMENT} answered HTTP 500`, `request: GET /net.json?q=${REPLACEMENT} got no answer: Failed to fetch`]);
  // A page path that throws when read: the report is a fault, counted and said once a minute.
  Object.defineProperty(win.location, 'pathname', { get() { throw new Error('no location'); } });
  clock = 1;
  assert.equal((await win.fetch('/y.json')).status, 500);
  assert.equal((await win.fetch('/z.json')).status, 500);
  win.emit('unhandledrejection', { reason: { get message() { throw new Error('unreadable'); } } });
  win.emit('error', { get message() { throw new Error('unreadable'); } });
  await flush();
  assert.equal(warned.length, 1, warned.join('\n'));
  assert.match(warned[0], /^the page reporter could not write a report \(no location\); 1 reporting faults since this tab opened$/);
  clock = 1 + MINUTE_MS;
  assert.equal((await win.fetch('/w.json')).status, 500);
  assert.equal(warned.length, 2);
  assert.match(warned[1], /; 5 reporting faults since this tab opened$/, 'every fault is counted, the console told once a minute');
  off();
  // The reporter alone: a page() that throws, a clock that throws, a console that throws.
  /** @type {string[]} */
  const said = [];
  const r = createPageReporter({ send: async () => {}, page: () => { throw new Error('gone'); }, now: () => 0, warn: (l) => said.push(l) });
  assert.equal(r.report('error', 'x'), false);
  assert.equal(r.counts().faults, 1);
  assert.equal(said.length, 1);
  const broken = createPageReporter({ send: async () => {}, page: () => '/db', now: () => { throw new Error('no clock'); }, warn: () => { throw new Error('no console'); } });
  assert.equal(broken.report('error', 'x'), false, 'never throws, whatever throws under it');
  assert.equal(broken.counts().faults, 1);
  const loud = createPageReporter({ send: async () => { throw new Error('down'); }, page: () => '/db', now: () => 0, warn: () => { throw new Error('no console'); } });
  assert.equal(loud.report('error', 'x'), true);
  await flush();
  await flush();
  assert.deepEqual(loud.counts(), { sent: 1, heldBack: 0, sendFailures: 1, faults: 1 }, 'a console that throws on the unsent report is a fault, never an unhandled rejection');
});

test('a time and a size are printed in the unit their rounding reaches', () => {
  // Before: 999,950-999,999 µs printed "1000.0 ms" and 9,996-9,999 µs "10.00 ms".
  assert.equal(microsText(999_949), '999.9 ms');
  assert.equal(microsText(999_950), '1.00 s');
  assert.equal(microsText(999_999), '1.00 s');
  assert.equal(microsText(1_000_000), '1.00 s');
  assert.equal(microsText(9_995), '9.99 ms');
  assert.equal(microsText(9_996), '10.0 ms');
  assert.equal(microsText(9_999), '10.0 ms');
  assert.equal(microsText(999), '999 µs');
  assert.equal(microsText(1_000), '1.00 ms');
  // Heap and wire: KB under a megabyte, MB from there, decimal as every target.
  assert.equal(bytesText(3_800_000), '3.8 MB', 'a 3.8 MB heap printed 3800.0 KB');
  assert.equal(bytesText(520_300_000), '520.3 MB');
  assert.equal(bytesText(409_900), '409.9 KB');
  assert.equal(bytesText(999_949), '999.9 KB');
  assert.equal(bytesText(999_950), '1.0 MB', 'never 1000.0 KB');
  assert.equal(bytesText(0), '0.0 KB');
  assert.equal(bytesText(null), 'not reported by this browser');
  assert.equal(bytesText(Number.NaN), 'not reported by this browser');
  // The page draws the library's own-load sentence (R315-19), and the
  // sentence prints the wire and the heap through bytesText.
  const page = source('src/routes/speed/+page.svelte');
  assert.match(page, />\{selfText\(self, here\)\}<\/p>/);
  const lib = source('src/lib/speed.js');
  const sentence = lib.slice(lib.indexOf('export function selfText'), lib.indexOf('export function watchSelf'));
  assert.match(sentence, /\$\{bytesText\(r\.wireBytes\)\} on the wire/);
  assert.match(sentence, /script heap \$\{bytesText\(r\.heapBytes\)\}/);
  assert.doesNotMatch(page + sentence, /const kb = /, 'one size rule, the library\'s');
});

test('a failed reading leaves the earlier report drawn under a line that says it is the earlier one', () => {
  assert.equal(earlierReading('failed', true, '20 Oct 10:02:03'), 'The figures below are the earlier reading, from 20 Oct 10:02:03: the latest reading failed, and nothing below is from it.');
  assert.equal(earlierReading('failed', false, 'x'), null, 'a first reading that failed has no earlier one');
  assert.equal(earlierReading('ready', true, 'x'), null);
  assert.equal(earlierReading('reading', true, 'x'), null, 'a reading under way is said by the button');
  const page = source('src/routes/speed/+page.svelte');
  assert.match(page, /const earlier = \$derived\(earlierReading\(read\.phase, read\.report !== null, stampLabel\(read\.at\)\)\);/);
  assert.match(page, /\{#if read\.phase === 'failed'\}\n\s+<p class="refused" role="alert" data-speed-failed>\{read\.why\}<\/p>\n\s+\{#if earlier\}<p class="refused earlier" data-speed-earlier>\{earlier\}<\/p>\{\/if\}/);
  // The failed branch keeps the earlier reading's time: `at` is set only when a reading is ready.
  assert.match(page, /read = \{ \.\.\.read, phase: 'failed', why: /);
  assert.match(page, /read = \{ phase: 'ready', report, build: buildOf\(r\.headers\), why: '', at: Date\.now\(\) \};/);
});

test('the page budget refuses a client build it cannot weigh, and a build that weighed nothing', async () => {
  const { pageBudget } = await import('../vite.config.js');
  /** @param {boolean} ssr */
  const plugin = (ssr) => {
    const p = /** @type {any} */ (pageBudget());
    p.configResolved({ build: { ssr } });
    return p;
  };
  // The server build alone, its client build never weighed: before, a pass in silence.
  const lonely = plugin(true);
  assert.equal(lonely.writeBundle({ dir: tmpdir() }), undefined, 'the server build ships nothing to a browser and is not weighed');
  assert.throws(() => lonely.closeBundle(), /the page budget weighed no page: the server build closed and no client build it started was weighed/);
  // A client build whose output has no entry: before, a return in silence.
  const server = plugin(true);
  const client = plugin(false);
  assert.throws(() => client.writeBundle({ dir: mkdtempSync(join(tmpdir(), 'r315-noentry-')) }), /the page budget found no client build to weigh: .*r315-noentry-.* holds no _app\/immutable\/entry/);
  assert.throws(() => client.writeBundle({}), /the build named no output directory holds no _app\/immutable\/entry/);
  assert.throws(() => server.closeBundle(), /^Error: the page budget found no client build to weigh: the build named no output directory/, 'the server\'s close repeats the client\'s own refusal, never a vaguer one');
  // A client build inside its budget is weighed, and the server build then closes.
  const server2 = plugin(true);
  const inside = plugin(false);
  inside.writeBundle({ dir: synthBuild({ pageBytes: 1_000 }) });
  assert.equal(server2.closeBundle(), undefined);
  assert.equal(inside.closeBundle(), undefined, 'the client build\'s own close asks nothing');
  // One past it fails the build, by name.
  const server3 = plugin(true);
  assert.throws(() => plugin(false).writeBundle({ dir: synthBuild({ pageBytes: budgetOf('/gaps') + 1 }) }), /1 page\(s\) past their budget in \$lib\/speed\.js BUNDLE_BUDGETS: \/gaps: .* — OVER ITS BUDGET/);
  // Found building the 20 Oct merged tree: /db past its budget surfaced only
  // as "weighed no page", the server's close over the client's refusal.
  assert.throws(() => server3.closeBundle(), /^Error: 1 page\(s\) past their budget in \$lib\/speed\.js BUNDLE_BUDGETS: \/gaps: .* — OVER ITS BUDGET/);
  // A server build that starts afresh forgets the last build's refusal.
  const server4 = plugin(true);
  plugin(false).writeBundle({ dir: synthBuild({ pageBytes: 1_000 }) });
  assert.equal(server4.closeBundle(), undefined);
});

test('the harness opens on the app\'s own default instrument, or the one --instrument names, and refuses one that is not a key', () => {
  assert.equal(harness.EDGE_INSTRUMENT, SOURCES_DEFAULTS.key, 'read from the app, never typed in the harness');
  assert.equal(linkOfKey(harness.EDGE_INSTRUMENT)?.sym, DEFAULT_INSTRUMENT, '/sources and /db open on one instrument, so the harness measures /db\'s');
  assert.equal(harness.parseArgs(['--base', 'http://h:1']).instrument, harness.EDGE_INSTRUMENT);
  const own = harness.INTERACTIONS;
  assert.deepEqual(own.map((i) => i.open), ['/db', '/db', '/db?seg=options', null], '/db\'s own opening is its bare address, as measured');
  assert.deepEqual(/** @type {any} */ (own[0].steps[0]).type[1], DEFAULT_INSTRUMENT);
  const other = harness.interactionsFor('NSE-EQ-RELIANCE');
  assert.deepEqual(other.map((i) => i.open), ['/db?sym=RELIANCE', '/db?sym=RELIANCE', '/db?seg=options&sym=RELIANCE', null]);
  assert.equal(/** @type {any} */ (other[0].steps[0]).type[1], 'RELIANCE');
  assert.ok(Object.isFrozen(other) && other.every((i) => Object.isFrozen(i)));
  assert.equal(harness.parseArgs(['--base', 'http://h:1', '--instrument', 'NSE-INDEX-BANKNIFTY']).instrument, 'NSE-INDEX-BANKNIFTY');
  for (const bad of ['garbage', 'nse-index-nifty', '']) {
    assert.throws(() => harness.parseArgs(['--base', 'http://h:1', '--instrument', bad]), /--instrument .* is not an instrument key/, bad);
  }
  assert.throws(() => harness.parseArgs(['--base', 'http://h:1', '--instrument']), /--instrument needs a value/);
  assert.throws(() => harness.interactionsFor('garbage'), /is not an instrument key/);
});

test('a sheet and a capture are written whole: a new file renamed over the old, nothing left beside it', () => {
  const dir = mkdtempSync(join(tmpdir(), 'r315-whole-'));
  const out = join(dir, 'sheet.md');
  sheet.writeWhole(out, 'first\n');
  assert.equal(readFileSync(out, 'utf8'), 'first\n');
  sheet.writeWhole(out, 'second, longer\n');
  assert.equal(readFileSync(out, 'utf8'), 'second, longer\n');
  assert.deepEqual(readdirSync(dir), ['sheet.md'], 'no temporary file is left beside it');
  // A path the rename cannot take (a directory there): refused by name, its own file taken back, the directory untouched.
  mkdirSync(join(dir, 'taken'));
  writeFileSync(join(dir, 'taken', 'inside'), 'kept');
  assert.throws(() => sheet.writeWhole(join(dir, 'taken'), 'x'), /could not write .*taken whole: /);
  assert.deepEqual(readdirSync(dir).sort(), ['sheet.md', 'taken']);
  assert.equal(readFileSync(join(dir, 'taken', 'inside'), 'utf8'), 'kept');
  // A directory that does not exist: refused before anything is made.
  assert.throws(() => sheet.writeWhole(join(dir, 'missing', 'sheet.md'), 'x'), /ENOENT/);
  // Both commands write through it, and a write that fails is named, never a stack (repair 4).
  assert.match(source('ci/speed-sheet.mjs'), /if \(args\.out\) \{\s*try \{\s*writeWhole\(args\.out, text\);\s*\} catch \(why\) \{/);
  assert.match(source('ci/browser-harness.mjs'), /if \(args\.capture\) \{\s*try \{\s*writeWhole\(args\.capture, /);
  assert.match(source('ci/browser-harness.mjs'), /if \(args\.out\) \{\s*try \{\s*writeWhole\(args\.out, text\);\s*\} catch \(why\) \{/);
  assert.doesNotMatch(source('ci/browser-harness.mjs') + source('ci/speed-sheet.mjs'), /writeFileSync\((args|path)/);
});

test('a browser the harness started leaves nothing behind: not its profile, not its process, after a close, a failed launch, an exit or a signal', { timeout: 180_000 }, async () => {
  const chrome = harness.parseArgs(['--base', 'http://h:1']).chrome;
  assert.ok(chrome, 'this test needs Chrome or Chromium: set BRUTEX_CHROME');
  const noSandbox = process.getuid?.() === 0;
  /** @param {number | undefined} pid */
  const running = (pid) => {
    try {
      process.kill(/** @type {number} */ (pid), 0);
      return true;
    } catch {
      return false;
    }
  };
  /** @param {() => boolean} done @param {number} ms */
  const until = async (done, ms) => {
    const end = Date.now() + ms;
    while (!done() && Date.now() < end) await new Promise((r) => setTimeout(r, 50));
    return done();
  };
  // A close: before, the profile stayed and close() did not wait for the browser to leave.
  const root = mkdtempSync(join(tmpdir(), 'r315-profiles-'));
  const b = await harness.launch(chrome, noSandbox, 30_000, root);
  assert.match(b.profile, new RegExp(`^${root}/${harness.PROFILE_PREFIX}[A-Za-z0-9]{6}$`));
  assert.ok(existsSync(b.profile) && running(b.pid) && b.alive());
  await b.close();
  assert.equal(b.alive(), false);
  assert.equal(running(b.pid), false, 'the browser has left when close() returns');
  assert.deepEqual(readdirSync(root), [], 'and its profile is gone');
  // A launch that fails takes back what it made.
  const failed = mkdtempSync(join(tmpdir(), 'r315-failed-'));
  const fake = mkdtempSync(join(tmpdir(), 'r315-fake-chrome-'));
  const mute = join(fake, 'mute');
  writeFileSync(mute, '#!/bin/sh\nexec sleep 30\n', { mode: 0o755 });
  const liar = join(fake, 'liar');
  writeFileSync(liar, '#!/bin/sh\necho "DevTools listening on ws://127.0.0.1:1/devtools/browser/x" >&2\nexec sleep 30\n', { mode: 0o755 });
  const unrunnable = join(fake, 'unrunnable');
  writeFileSync(unrunnable, 'not a program', { mode: 0o644 });
  await assert.rejects(harness.launch(mute, false, 200, failed), /printed no DevTools address in 200 ms/);
  await assert.rejects(harness.launch(liar, false, 30_000, failed), /could not connect to ws:\/\/127\.0\.0\.1:1/);
  await assert.rejects(harness.launch(unrunnable, false, 30_000, failed), /the browser could not be started: .*EACCES/);
  assert.deepEqual(readdirSync(failed), [], 'no profile is left by a launch that failed');
  // Only a profile this harness made is ever removed.
  assert.throws(() => harness.removeProfile(fake), /refusing to remove .*: not a profile this harness made/);
  assert.throws(() => harness.removeProfile(join(fake, `${harness.PROFILE_PREFIX}abcdefg`)), /refusing to remove/);
  assert.ok(existsSync(mute), 'a refused path is untouched');
  // An exit and a signal: a harness process that leaves without close().
  const script = join(fake, 'leaves.mjs');
  writeFileSync(script, [
    `import * as h from ${JSON.stringify(new URL('ci/browser-harness.mjs', web).href)};`,
    `const b = await h.launch(${JSON.stringify(chrome)}, ${noSandbox}, 30000, process.argv[3]);`,
    'process.stdout.write(JSON.stringify({ pid: b.pid, profile: b.profile }) + "\\n");',
    'if (process.argv[2] === "exit") process.exit(0);',
    'if (process.argv[2] === "throw") setTimeout(() => { throw new Error("the harness died"); }, 10);',
    'setInterval(() => {}, 1000);'
  ].join('\n'));
  const { spawn: run } = await import('node:child_process');
  for (const how of ['exit', 'throw', 'SIGTERM', 'SIGINT', 'SIGHUP']) {
    const where = mkdtempSync(join(tmpdir(), `r315-leaves-${how}-`));
    const child = run(process.execPath, [script, how, where], { stdio: ['ignore', 'pipe', 'pipe'] });
    const line = await new Promise((resolve, reject) => {
      let text = '';
      child.stdout.on('data', (c) => {
        text += c;
        if (text.includes('\n')) resolve(text.split('\n')[0]);
      });
      child.on('exit', (code) => reject(new Error(`${how}: the harness left (${code}) before it started a browser`)));
    });
    const started = JSON.parse(String(line));
    assert.ok(running(started.pid), `${how}: the browser is running`);
    const ended = new Promise((resolve) => child.on('exit', (code, signal) => resolve({ code, signal })));
    if (how.startsWith('SIG')) child.kill(/** @type {NodeJS.Signals} */ (how));
    const { code, signal } = /** @type {{code: number | null, signal: string | null}} */ (await ended);
    if (how === 'exit') assert.equal(code, 0);
    else if (how === 'throw') assert.equal(code, 1, 'an uncaught error still ends the process as it would have');
    else assert.equal(signal, how, `${how} keeps its meaning: the process ends by it`);
    assert.ok(await until(() => !running(started.pid), 10_000), `${how}: the browser ${started.pid} was left running`);
    assert.deepEqual(readdirSync(where), [], `${how}: the profile was left`);
  }
});

test('the browser is started before any page is timed, on a page that asks the server nothing, in a context thrown away', () => {
  // Before: the first cold sample of the first page carried the browser's
  // own start (1,108 ms on a five-line page at n = 1, against 20-28 ms for
  // every later cold load), and the command test above failed by it.
  assert.match(harness.WARM_UP_PAGE, /^data:text\/html,.*<p>warm-up<\/p>$/, 'text, so the paint path runs, and a data: address, so the server is asked nothing');
  const src = source('ci/browser-harness.mjs');
  const run = src.slice(src.indexOf('export async function runHarness'));
  assert.ok(run.indexOf('await warmUp(cdp, open, args.settleMs);') > 0, 'runHarness warms the browser');
  assert.ok(run.indexOf('await warmUp(cdp, open, args.settleMs);') < run.indexOf('for (const page of args.pages)'), 'before the first page is timed');
  const warm = src.slice(src.indexOf('async function warmUp'), src.indexOf('export async function runHarness'));
  assert.match(warm, /Target\.createBrowserContext[\s\S]*loadOnce\(tab, cdp, WARM_UP_PAGE, settleMs\)[\s\S]*Target\.disposeBrowserContext/, 'in a context of its own, thrown away after');
});

// ---------------------------------------------------------------------------
// Repair round 3 (23 Oct): verify r3's two blocking findings, each failing
// on a6915b19e by name.
// ---------------------------------------------------------------------------

test('a page mounted again in the same tab says its figures are the tab\'s: the second watch of one document is a revisit, and a new document is its own load again', () => {
  // Before (verify r3, 22 Oct): /speed -> /db -> /speed drew the tab's 1.2 MB
  // over 63 fetches as "This page's own load" with no word that they were
  // the tab's, against the page's own 408.6 KB over 35: the caveat was said
  // only for a document opened on another page.
  const { selfWhose } = speedModule;
  const all = ['paint', 'navigation', 'resource'];
  const tabs = "so these are the tab's figures since it opened, not this page's alone.";
  const doc = (/** @type {string} */ path) => ({ entryType: 'navigation', name: `http://h:1${path}?feed=a`, transferSize: 9_000 });
  const own = selfAdd(selfStart(all), [doc('/speed')]);
  assert.equal(own.revisit, false, 'a first watch is no revisit');
  assert.equal(selfWhose(own, '/speed'), 'page');
  assert.doesNotMatch(selfText(own, '/speed'), /tab's figures/, 'the document\'s own load, first watched, is this page\'s');
  const back = selfAdd(selfAdd(selfStart(all, true), [doc('/speed')]), [{ entryType: 'resource', transferSize: 1 }], { loaded: true });
  assert.equal(back.revisit, true, 'a revisit is carried through every later reading');
  assert.equal(selfWhose(back, '/speed'), 'tab');
  assert.ok(selfText(back, '/speed').endsWith(` This tab opened on /speed, left it and came back by a link inside the app, ${tabs}`), selfText(back, '/speed'));
  // The document's path not delivered (a browser with no wire, or not yet): the tab left this page and came back.
  const blind = selfStart(['paint'], true);
  assert.equal(selfWhose(blind, '/speed'), 'tab');
  assert.ok(selfText(blind, '/speed').endsWith(` This tab left this page and came back by a link inside the app, ${tabs}`));
  // Opened on another page: said as before, whether or not this page was shown already.
  for (const again of [false, true]) {
    const linked = selfAdd(selfStart(all, again), [doc('/db')]);
    assert.equal(selfWhose(linked, '/speed'), 'tab');
    assert.ok(selfText(linked, '/speed').endsWith(` This tab opened on /db and came here by a link inside the app, ${tabs}`));
  }
  assert.equal(selfWhose(selfStart(['paint']), '/speed'), 'page', 'a first watch whose document is not known yet claims nothing');
  assert.equal(selfStart(all, /** @type {any} */ ('yes')).revisit, false, 'only true is a revisit');
  // watchSelf: the second watch of one window is a revisit; a new window (a new document) is not.
  class Observer {
    static supportedEntryTypes = all;
    observe() {}
    disconnect() {}
  }
  const win = { PerformanceObserver: Observer, performance: { now: () => 1 }, document: { readyState: 'complete' } };
  /** @type {any[][]} */
  const seen = [[], [], [], [], [], [], []];
  watchSelf(win, (r) => seen[0].push(r))();
  watchSelf(win, (r) => seen[1].push(r))();
  watchSelf(win, (r) => seen[2].push(r))();
  watchSelf({ ...win }, (r) => seen[3].push(r))();
  assert.deepEqual(seen.slice(0, 4).map((s) => s[0].revisit), [false, true, true, false]);
  // A document with no observer is still one document; no window at all is never remembered.
  const bare = {};
  watchSelf(bare, (r) => seen[4].push(r))();
  watchSelf(bare, (r) => seen[5].push(r))();
  watchSelf(null, () => {})();
  watchSelf(null, (r) => seen[6].push(r))();
  assert.deepEqual([seen[4][0].revisit, seen[5][0].revisit, seen[6][0].revisit], [false, true, false]);
  // The page draws whose figures they are, from the library's one rule.
  const page = source('src/routes/speed/+page.svelte');
  assert.match(page, /data-figures-of=\{selfWhose\(self, here\)\}/);
  assert.match(page, /const stopSelf = watchSelf\(typeof window === 'undefined' \? null : window, \(reading\) => \(self = reading\)\);/, 'one watch a mount, so a remount is the document\'s second');
});

test('the Speed page\'s own line after a round trip inside the app, in a real browser: a page mounted again says the tab\'s figures, and a new document is its own load again', { timeout: 120_000 }, async () => {
  const chrome = harness.parseArgs(['--base', 'http://h:1']).chrome;
  assert.ok(chrome, 'this test needs Chrome or Chromium: set BRUTEX_CHROME');
  const site = await browserStandIn({});
  const b = await harness.launch(chrome, process.getuid?.() === 0);
  try {
    const { browserContextId } = await b.cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
    const tab = await harness.openTab(b.cdp, browserContextId, { readOnly: true, base: site.base });
    const total = "[...performance.getEntriesByType('navigation'), ...performance.getEntriesByType('resource')].reduce((a, e) => a + e.transferSize, 0)";
    /** Until the line has drawn the tab's whole timeline, at most 10 s. @param {string} path */
    const drawn = (path) => tab.evaluate(`(async () => { const end = Date.now() + 10000; while (Date.now() < end) { const l = document.querySelector('[data-speed-self]'); if (location.pathname === ${JSON.stringify(path)} && l && Number(l.dataset.wireBytes) === ${total} && ${total} > 0) return true; await new Promise((r) => setTimeout(r, 20)); } return false; })()`);
    /** Until the other page's fetch has ended, at most 10 s. */
    const fetched = () => tab.evaluate(`(async () => { const end = Date.now() + 10000; while (Date.now() < end) { if (location.pathname === '/trip-db' && document.querySelector('main').dataset.done === 'yes') return true; await new Promise((r) => setTimeout(r, 20)); } return false; })()`);
    const read = `(() => { const l = document.querySelector('[data-speed-self]'); return { text: l.textContent, bytes: Number(l.dataset.wireBytes), fetches: Number(l.dataset.fetches), browserBytes: ${total} }; })()`;
    await harness.loadOnce(tab, b.cdp, `${site.base}/trip`, 300);
    assert.ok(await drawn('/trip'), 'the first mount drew the document\'s load');
    const first = await tab.evaluate(read);
    assert.doesNotMatch(first.text, /tab's figures|came back|came here/, `the document's own load, first watched, is this page's: ${first.text}`);
    // Leave by a link inside the app; the other page fetches 450,000 bytes.
    await tab.evaluate("document.querySelector('#to-db').click()");
    assert.ok(await fetched(), 'the other page fetched');
    // Back by a link inside the app: the same document, mounted again.
    await tab.evaluate("document.querySelector('#to-speed').click()");
    assert.ok(await drawn('/trip'), 'the second mount drew the tab\'s whole timeline');
    const back = await tab.evaluate(read);
    assert.ok(back.bytes >= first.bytes + 450_000, `the drawn wire carries the other page's fetch: ${back.bytes} against ${first.bytes}`);
    assert.match(back.text, / This tab opened on \/trip, left it and came back by a link inside the app, so these are the tab's figures since it opened, not this page's alone\.$/, `after a round trip the figures are said to be the tab's: ${back.text}`);
    // Back and back again by the browser's own button (popstate): the third mount is the tab's too.
    await tab.evaluate('history.back()');
    assert.ok(await fetched(), 'the other page, by popstate, fetched again');
    await tab.evaluate('history.back()');
    assert.ok(await drawn('/trip'), 'the third mount drew the tab\'s whole timeline');
    const third = await tab.evaluate(read);
    assert.ok(third.bytes >= back.bytes + 450_000);
    assert.match(third.text, / left it and came back by a link inside the app, so these are the tab's figures since it opened, not this page's alone\.$/);
    // A new document in the same tab is its own load again.
    await harness.loadOnce(tab, b.cdp, `${site.base}/trip`, 300);
    assert.ok(await drawn('/trip'));
    const fresh = await tab.evaluate(read);
    assert.doesNotMatch(fresh.text, /tab's figures|came back|came here/, `a new document is its own load: ${fresh.text}`);
    assert.ok(fresh.bytes < back.bytes, 'and its wire starts again');
    await tab.close();
    await b.cdp.send('Target.disposeBrowserContext', { browserContextId });
  } finally {
    await b.close();
    site.server.close();
  }
});

test('the harness refuses an idle wait its timer cannot hold, and the idle row prints the time the clock read, never the time asked', { timeout: 60_000 }, async () => {
  // Before (verify r3, 22 Oct): --idle-ms had no ceiling; Node fired a
  // setTimeout of 2,147,483,648 ms after 1 ms, and the sheet printed
  // "| 1 | 1 | 2147484 s | 0 | none |", a pass row for an idle never measured.
  const { MAX_WAIT_MS, IDLE_EARLY_MS, idleFor, idleRow } = harness;
  assert.equal(MAX_WAIT_MS, 2 ** 31 - 1, 'the largest delay a Node timer holds');
  assert.equal(IDLE_EARLY_MS, 1);
  assert.equal(harness.parseArgs(['--base', 'http://h:1', '--idle-ms', String(MAX_WAIT_MS)]).idleMs, MAX_WAIT_MS);
  assert.throws(() => harness.parseArgs(['--base', 'http://h:1', '--idle-ms', '2147483648']), /^Error: --idle-ms must be at most 2147483647$/);
  // As a command: refused before any browser is started, exit 2, by name.
  const { execFile } = await import('node:child_process');
  const script = new URL('ci/browser-harness.mjs', web).pathname;
  const ran = /** @type {{code: number, stderr: string}} */ (await new Promise((resolve) => execFile(process.execPath, [script, '--base', 'http://127.0.0.1:9', '--n', '1', '--pages', '/speed', '--no-interactions', '--read-only', '--idle-tabs', '1', '--idle-ms', '2147483648'], (error, _stdout, stderr) => resolve({ code: error ? Number(error.code) : 0, stderr }))));
  assert.deepEqual([ran.code, ran.stderr], [2, 'browser-harness: --idle-ms must be at most 2147483647\n']);
  // The wait is measured by the clock.
  let clock = 1_000;
  assert.equal(await idleFor(5_000, { wait: async (ms) => void (clock += ms + 3), now: () => clock }), 5_003);
  clock = 0;
  assert.equal(await idleFor(MAX_WAIT_MS, { wait: async () => void (clock += 1), now: () => clock }), 1, 'a wait that ended early is measured as early');
  for (const bad of [MAX_WAIT_MS + 1, -1, 1.5, Number.NaN]) {
    await assert.rejects(idleFor(bad), new RegExp(`^Error: an idle wait of ${bad} ms is not one a timer holds \\(a whole number from 0 to 2147483647\\)$`));
  }
  const real = await idleFor(30);
  assert.ok(real + IDLE_EARLY_MS >= 30 && real < 30_000, `a real 30 ms wait read ${real} ms`);
  // The row prints the measured time; a wait short of the asked one is NOT MEASURED and fails by name.
  assert.deepEqual(idleRow({ count: 1, visible: 1, askedMs: 3_600_000, idledMs: 3_600_012.4, requests: 0, other: [] }), { row: '| 1 | 1 | 3600 s | 0 | none |', failures: [], notes: [] });
  assert.deepEqual(idleRow({ count: 1, visible: 1, askedMs: MAX_WAIT_MS, idledMs: 2.6, requests: 0, other: [] }), {
    row: '| 1 | 1 | NOT MEASURED: 2.6 ms of 2147483647 ms asked | 0 | none |',
    failures: ['1 idle tabs: NOT MEASURED — the wait ended after 2.6 ms of the 2147483647 ms asked'],
    notes: []
  });
  assert.deepEqual(idleRow({ count: 2, visible: 2, askedMs: 1_000, idledMs: 999, requests: 3, other: ['GET /feeds.json'] }), {
    row: '| 2 | 2 | 1 s | 3 | GET /feeds.json |',
    failures: ['2 idle tabs asked for more than the header-only up check: GET /feeds.json'],
    notes: []
  }, 'a wait that ended at most IDLE_EARLY_MS early is the timer\'s own rounding');
  // A tab the browser hid is said, short or not (the idle hour of 23 Oct: 2 of 20 visible).
  assert.deepEqual(idleRow({ count: 20, visible: 2, askedMs: 3_600_000, idledMs: 3_600_000, requests: 9, other: [] }).notes, ['20 idle tabs: 18 of them were hidden, and a hidden page may poll less than a visible one']);
  assert.equal(idleRow({ count: 20, visible: 0, askedMs: 1_000, idledMs: 5, requests: 0, other: [] }).notes.length, 1);
  assert.equal(idleRow({ count: 2, visible: 2, askedMs: 1_000, idledMs: 998.9, requests: 0, other: [] }).failures.length, 1, 'past it, not');
  // The run waits through idleFor and prints through idleRow.
  const src = source('ci/browser-harness.mjs');
  const run = src.slice(src.indexOf('export async function runHarness'), src.indexOf('export function readOnlyVerdict'));
  assert.match(run, /const idledMs = await idleFor\(args\.idleMs\);/);
  assert.match(run, /const idle = idleRow\(\{ count, visible, askedMs: args\.idleMs, idledMs, requests: requests\.length, other: v\.other \}\);\s*lines\.push\(idle\.row\);\s*failures\.push\(\.\.\.idle\.failures\);\s*notes\.push\(\.\.\.idle\.notes\);/);
  assert.doesNotMatch(run, /args\.idleMs \/ 1_000|setTimeout\(r, args\.idleMs\)/, 'the asked time is never printed as the measured one');
});

test('the sheet and the harness read a number in decimal digits only, bound every wait by what a timer holds and the restarts by MAX_N, and a page under two spellings is one page', () => {
  // Before (verify r3 follow-ups, 22 Oct): `--n 0x10` was read as 16;
  // `--timeout-ms` and `--up-wait-ms` had no ceiling, so a timeout past
  // 2^31 - 1 ran as another than the one asked (Node 24.1 cuts a socket's
  // timeout to 2^31 - 1, measured 23 Oct); `--restarts` had no ceiling; and
  // /speed, /speed/ and /SPEED were three pages.
  assert.equal(harness.MAX_WAIT_MS, sheet.MAX_WAIT_MS, 'one ceiling for both commands');
  assert.equal(sheet.MAX_N, harness.MAX_N);
  const base = ['--base', 'http://h:1'];
  for (const [name, odd] of [['n', '0x10'], ['n', '0b11'], ['n', '0o7'], ['n', ' 12'], ['n', '12 '], ['n', '1.5'], ['n', ''], ['n', '+3'], ['n', 'Infinity']]) {
    const why = new RegExp(`^Error: --${name} must be a whole number of at least 1, in decimal digits$`);
    assert.throws(() => harness.parseArgs([...base, `--${name}`, odd]), why, `the harness read ${JSON.stringify(odd)}`);
    assert.throws(() => sheet.parseArgs([...base, `--${name}`, odd]), why, `the sheet read ${JSON.stringify(odd)}`);
  }
  assert.equal(harness.parseArgs([...base, '--n', '1e3']).n, 1_000, 'an exponent is still decimal');
  // A list of numbers is read by the same rule, each member.
  assert.throws(() => harness.parseArgs([...base, '--idle-tabs', '1,0x10']), /^Error: --idle-tabs is a comma list of whole numbers from 1 to 64$/);
  assert.throws(() => harness.parseArgs([...base, '--idle-tabs', '1, 2']), /^Error: --idle-tabs is a comma list of whole numbers from 1 to 64$/);
  assert.deepEqual(harness.parseArgs([...base, '--idle-tabs', '1,1e1,064']).idleTabs, [1, 10, 64]);
  assert.throws(() => sheet.parseArgs([...base, '--clients', '0x10']), /^Error: --clients is a comma list of whole numbers from 1 to 512$/);
  assert.throws(() => sheet.parseArgs([...base, '--clients', '1,2 ']), /^Error: --clients is a comma list of whole numbers from 1 to 512$/);
  assert.deepEqual(sheet.parseArgs([...base, '--clients', '1,1e1,512']).clients, [1, 10, 512]);
  assert.equal(sheet.parseArgs([...base, '--n', '0016']).n, 16, 'leading zeros are decimal');
  for (const name of ['settle-ms', 'step-wait-ms', 'idle-ms']) {
    assert.throws(() => harness.parseArgs([...base, `--${name}`, '2147483648']), new RegExp(`^Error: --${name} must be at most 2147483647$`));
    assert.equal(harness.parseArgs([...base, `--${name}`, '2147483647'])[name === 'settle-ms' ? 'settleMs' : name === 'step-wait-ms' ? 'stepWaitMs' : 'idleMs'], 2_147_483_647);
  }
  for (const name of ['timeout-ms', 'up-wait-ms']) {
    assert.throws(() => sheet.parseArgs([...base, `--${name}`, '2147483648']), new RegExp(`^Error: --${name} must be at most 2147483647$`));
    assert.equal(sheet.parseArgs([...base, `--${name}`, '2147483647'])[name === 'timeout-ms' ? 'timeoutMs' : 'upWaitMs'], 2_147_483_647);
  }
  assert.throws(() => sheet.parseArgs([...base, '--restarts', '10001']), /^Error: --restarts must be at most 10000$/);
  assert.equal(sheet.parseArgs([...base, '--restarts', '10000']).restarts, 10_000);
  // One page, one spelling — as the app's router reads it (repair 4, verify
  // r4: the case and every trailing slash were folded, which the router
  // does not do, and dot segments, escapes, a fragment and an empty query
  // were not, which the browser and the router do).
  /** @type {[string, string][]} */
  const twice = [
    ['/speed,/speed/', '--pages names /speed twice (as /speed and /speed/): each page is measured once a run'],
    ['/db?a=1,/db/?a=1', '--pages names /db?a=1 twice (as /db?a=1 and /db/?a=1): each page is measured once a run'],
    ['/db,/speed,/db', '--pages names /db twice: each page is measured once a run'],
    ['/speed,/speed/../speed', '--pages names /speed twice (as /speed and /speed/../speed): each page is measured once a run'],
    ['/speed,/%73peed', '--pages names /speed twice (as /speed and /%73peed): each page is measured once a run'],
    ['/speed,/speed#x', '--pages names /speed twice (as /speed and /speed#x): each page is measured once a run'],
    ['/speed,/speed?', '--pages names /speed twice (as /speed and /speed?): each page is measured once a run'],
    ['/speed,/speed/.', '--pages names /speed twice (as /speed and /speed/.): each page is measured once a run']
  ];
  for (const [pages, why] of twice) assert.throws(() => harness.parseArgs([...base, '--pages', pages]), (e) => /** @type {Error} */ (e).message === why, pages);
  assert.deepEqual(harness.parseArgs([...base, '--pages', '/,/db,/db?a=1,/db?a=A,/speed']).pages, ['/', '/db', '/db?a=1', '/db?a=A', '/speed'], 'a different query or page is another page');
  assert.deepEqual(harness.parseArgs([...base, '--pages', '/speed,/SPEED,/Db/,/db,/,//']).pages, ['/speed', '/SPEED', '/Db/', '/db', '/', '//'], 'another case, or a second trailing slash, is another address: the run then finds it is no page of the app');
  assert.deepEqual(['/speed/', '/SPEED//', '/', '//', '/DB/?k=NSE-A', '/db?k=nse-a', '/speed/../speed', '/%73peed', '/speed#x', '/speed?', '/speed/.', '/speed%2F'].map(harness.pageKey),
    ['/speed', '/SPEED//', '/', '//', '/DB?k=NSE-A', '/db?k=nse-a', '/speed', '/speed', '/speed', '/speed', '/speed', '/speed%2F'], 'the router\'s reading: case kept, one trailing slash, escapes decoded but a reserved one, the query never');
  assert.equal(harness.pageKey('/%E0%A4%A'), null, 'an escape that is not UTF-8 is no path a router reads');
  assert.throws(() => harness.parseArgs([...base, '--pages', '/%E0%A4%A']), /^Error: --pages names \/%E0%A4%A, whose path no router can read: a percent escape that is not UTF-8$/);
  const long = `/${'/'.repeat(200_000)}x`;
  const t0 = performance.now();
  assert.equal(harness.pageKey(long), long);
  assert.ok(performance.now() - t0 < 1_000, 'a path of slashes is read once, never backtracked');
});

test('a wire with entries the browser gave no size for, or dropped, is said as a floor, and a fetch count is drawn only with a wire', () => {
  // Before (verify r3 follow-ups, 22 Oct): a sum missing entries was printed
  // as the whole wire, and data-fetches read 0 where the browser gives no wire.
  const all = ['paint', 'navigation', 'resource'];
  const sized = selfAdd(selfStart(all), [{ entryType: 'resource', transferSize: 1_000 }]);
  assert.match(selfText(sized, '/speed'), / · 1\.0 KB on the wire over 1 fetches · /, 'every entry sized: the whole');
  const unsized = selfAdd(sized, [{ entryType: 'resource' }]);
  assert.match(selfText(unsized, '/speed'), / · at least 1\.0 KB on the wire over 2 fetches, 1 of them with no size from this browser · /);
  const dropped = selfAdd(sized, [], { dropped: 4 });
  assert.match(selfText(dropped, '/speed'), / · at least 1\.0 KB on the wire over 1 fetches, and 4 more the browser dropped before this page could read them · /);
  assert.doesNotMatch(selfText(selfStart(['paint']), '/speed'), /at least/, 'no wire, no floor');
  const page = source('src/routes/speed/+page.svelte');
  assert.match(page, /data-fetches=\{self\.wire === 'observed' \? self\.fetches : undefined\}/);
});

// ---------------------------------------------------------------------------
// Repair round 4 (23 Oct): verify r4's three blocking findings and its
// follow-ups in R3-15's files, each failing on the tree before it.
// ---------------------------------------------------------------------------

test('every DevTools command is bounded: one the browser does not answer is refused by name, a late answer is dropped, and a closed connection refuses what waits and what comes', async () => {
  // Before (verify r4, 23 Oct): Cdp.send set a Promise per id with no
  // bound, so a browser that never answered held the run for ever.
  const { Cdp, CdpTimeout } = harness;
  /** @type {any[]} */
  const sent = [];
  /** @type {any} */
  const ws = new EventTarget();
  ws.send = (/** @type {string} */ text) => sent.push(JSON.parse(text));
  const cdp = new Cdp(ws, 50);
  const answer = (/** @type {object} */ m) => ws.dispatchEvent(new MessageEvent('message', { data: JSON.stringify(m) }));
  const t0 = performance.now();
  await assert.rejects(cdp.send('Page.navigate', { url: 'x' }), (e) => e instanceof CdpTimeout && /** @type {Error} */ (e).message === 'the browser did not answer Page.navigate within 50 ms');
  assert.ok(performance.now() - t0 < 1_000, 'refused at its bound');
  // A late answer is dropped: nothing throws, nothing is answered twice.
  assert.doesNotThrow(() => answer({ id: sent[0].id, result: {} }));
  // An answer in time resolves; a command's own bound is its own.
  const inTime = cdp.send('Runtime.evaluate', { expression: '1' }, 's1', 5_000);
  answer({ id: sent[1].id, result: { ok: 1 } });
  assert.deepEqual(await inTime, { ok: 1 });
  assert.equal(sent[1].sessionId, 's1');
  const refused = cdp.send('Bogus.method');
  answer({ id: sent[2].id, error: { message: "'Bogus.method' wasn't found", code: -32601 } });
  await assert.rejects(refused, /^Error: 'Bogus\.method' wasn't found \(-32601\)$/);
  // Events still reach their listeners.
  /** @type {any[]} */
  const events = [];
  const off = cdp.on((m) => events.push(m.method));
  answer({ method: 'Page.loadEventFired', params: {} });
  off();
  assert.deepEqual(events, ['Page.loadEventFired']);
  // A closed connection refuses what waits, and every later command, at once.
  const waiting = cdp.send('Target.createTarget', {}, undefined, 60_000);
  ws.dispatchEvent(new Event('close'));
  await assert.rejects(waiting, /^Error: Target\.createTarget: the browser's DevTools connection closed$/);
  await assert.rejects(cdp.send('Target.closeTarget'), /^Error: Target\.closeTarget: the browser's DevTools connection closed$/);
  assert.equal(sent.length, 4, 'nothing is sent on a closed connection');
  // The bounds.
  assert.equal(harness.CDP_WAIT_MS, 30_000);
  assert.equal(harness.LOAD_WAIT_MS, 30_000);
  assert.equal(harness.FRAME_WAIT_MS, 2_000);
  assert.equal(harness.answerWithin(2_000), 32_000, 'a page-side wait, and the time its answer travels');
  assert.equal(harness.answerWithin(harness.MAX_WAIT_MS), harness.MAX_WAIT_MS, 'never more than a timer holds');
  assert.equal(harness.parseArgs(['--base', 'http://h:1']).loadWaitMs, 30_000);
  assert.equal(harness.parseArgs(['--base', 'http://h:1', '--load-wait-ms', '100']).loadWaitMs, 100);
  assert.throws(() => harness.parseArgs(['--base', 'http://h:1', '--load-wait-ms', '99']), /^Error: --load-wait-ms must be a whole number of at least 100, in decimal digits$/);
  assert.throws(() => harness.parseArgs(['--base', 'http://h:1', '--load-wait-ms', '2147483648']), /^Error: --load-wait-ms must be at most 2147483647$/);
});

test('a load that does not end within its bound fails its page by name, and the run goes on to its sheet and a non-zero exit', { timeout: 240_000 }, async () => {
  // Before (verify r4, 23 Oct): loadOnce waited for Page.loadEventFired with
  // no bound, so /stall (its headers and part of its body, never the end)
  // printed nothing and wrote no sheet in 9 min 49 s, and a server paused
  // with SIGSTOP held a cold load for the whole pause.
  const chrome = harness.parseArgs(['--base', 'http://h:1']).chrome;
  assert.ok(chrome, 'this test needs Chrome or Chromium: set BRUTEX_CHROME');
  const root = process.getuid?.() === 0 ? ['--no-sandbox'] : [];
  const site = await browserStandIn({});
  const dir = mkdtempSync(join(tmpdir(), 'r315-stall-'));
  try {
    const t0 = performance.now();
    const r = await harness.runHarness(harness.parseArgs(['--base', site.base, '--n', '2', '--pages', '/stall,/hang,/speed', '--no-interactions', '--no-idle', '--no-report-check', '--settle-ms', '100', '--load-wait-ms', '1500', ...root]));
    const took = performance.now() - t0;
    const text = r.lines.join('\n');
    const stalled = 'the load did not end within 1500 ms (its document was answered HTTP 200 and did not finish)';
    const unanswered = 'the load did not end within 1500 ms (no answer to its document)';
    assert.match(text, new RegExp(`^\\| /stall \\| cold \\| 0 \\| NOT MEASURED: ${stalled.replace(/[()]/g, '\\$&')} \\| — \\| — \\| — \\| FAILS \\|$`, 'm'), text);
    assert.match(text, new RegExp(`^\\| /hang \\| cold \\| 0 \\| NOT MEASURED: ${unanswered.replace(/[()]/g, '\\$&')} \\| — \\| — \\| — \\| FAILS \\|$`, 'm'), text);
    assert.match(text, /^\| \/speed \| cold \| 2 \| [\d.]+ \/ [\d.]+ \/ [\d.]+ \|/m, 'the run goes on to the next page');
    assert.match(text, /^\| \/speed \| warm \| 2 \|/m);
    assert.deepEqual(r.failures.filter((f) => /^\/(stall|hang) /.test(f)), [`/stall cold: ${stalled}`, `/hang cold: ${unanswered}`]);
    assert.equal(r.ok, false);
    assert.ok(took < 60_000, `bounded: the run took ${took.toFixed(0)} ms`);
    assert.equal(r.lines.filter((l) => l.startsWith('| /stall ')).length, 1, 'a page whose load did not end is not loaded again');
    // A page whose cold loads end and whose warm load does not: the cold row stands, the warm row says why.
    const later = await harness.runHarness(harness.parseArgs(['--base', site.base, '--n', '1', '--pages', '/later', '--no-interactions', '--no-idle', '--no-report-check', '--settle-ms', '100', '--load-wait-ms', '1500', ...root]));
    const laterText = later.lines.join('\n');
    assert.match(laterText, /^\| \/later \| cold \| 1 \| [\d.]+ \/ [\d.]+ \/ [\d.]+ \|.*\| holds \|$/m, laterText);
    assert.ok(laterText.includes(`| /later | warm | 0 | NOT MEASURED: ${stalled} | — | — | — | FAILS |`), laterText);
    assert.deepEqual(later.failures, [`/later warm: ${stalled}`]);
    // A timed warm load that does not end, after the first warm load did: said the same way.
    const second = await harness.runHarness(harness.parseArgs(['--base', site.base, '--n', '1', '--pages', '/later?after=2', '--no-interactions', '--no-idle', '--no-report-check', '--settle-ms', '100', '--load-wait-ms', '1500', ...root]));
    assert.deepEqual(second.failures, [`/later?after=2 warm: ${stalled}`]);
    assert.ok(second.lines.includes(`| /later?after=2 | warm | 0 | NOT MEASURED: ${stalled} | — | — | — | FAILS |`), second.lines.join('\n'));
    // As a command: the sheet is written and the exit is 1.
    const { execFile } = await import('node:child_process');
    const script = new URL('ci/browser-harness.mjs', web).pathname;
    const out = join(dir, 'sheet.md');
    const ran = /** @type {{code: number, stdout: string}} */ (await new Promise((resolve) => execFile(process.execPath, [script, '--base', site.base, '--n', '1', '--pages', '/stall', '--no-interactions', '--no-idle', '--no-report-check', '--load-wait-ms', '1000', '--out', out, ...root], (error, stdout) => resolve({ code: error ? Number(error.code) : 0, stdout }))));
    assert.equal(ran.code, 1, ran.stdout);
    assert.match(readFileSync(out, 'utf8'), /^# Browser harness — FAIL[\s\S]*- FAILED: \/stall cold: the load did not end within 1000 ms \(its document was answered HTTP 200 and did not finish\)/);
  } finally {
    site.stop();
  }
});

test('the two frames a settle waits for are bounded: a hidden tab draws none, is said so, and is never waited on', { timeout: 120_000 }, async () => {
  // Before (verify r4, 23 Oct): settle's last step waited for two animation
  // frames with no bound, and a hidden tab draws no frame (3 of 3 hidden
  // tabs gave none in 10 s), so a load in a hidden tab never settled.
  const chrome = harness.parseArgs(['--base', 'http://h:1']).chrome;
  assert.ok(chrome, 'this test needs Chrome or Chromium: set BRUTEX_CHROME');
  const b = await harness.launch(chrome, process.getuid?.() === 0);
  try {
    const { browserContextId } = await b.cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
    const page = 'data:text/html,<!doctype html><title>t</title><p>t</p>';
    const tabs = [];
    for (let i = 0; i < 3; i += 1) {
      const t = await harness.openTab(b.cdp, browserContextId);
      await harness.loadOnce(t, b.cdp, page, 50);
      tabs.push(t);
    }
    /** @type {unknown[]} */
    const states = [];
    for (const t of tabs) states.push(await t.evaluate('document.visibilityState'));
    assert.ok(states.includes('hidden') && states.at(-1) === 'visible', `a window draws its newest tab: ${states.join(', ')}`);
    for (const [i, t] of tabs.entries()) {
      const t0 = performance.now();
      assert.equal(await t.settle(50, 1_000, 500), true);
      const took = performance.now() - t0;
      assert.equal(t.frames.drew, states[i] === 'visible', `tab ${i} (${states[i]}): drew ${t.frames.drew}`);
      assert.ok(took < 1_000 + 500 + harness.FRAME_SLACK_MS + 1_000, `tab ${i} (${states[i]}) settled in ${took.toFixed(0)} ms`);
    }
    // A page-side wait is bounded here too: one the page never answers is late after its time and the slack.
    const hiddenTab = tabs[states.indexOf('hidden')];
    const t2 = performance.now();
    assert.deepEqual(await hiddenTab.within('new Promise(() => {})', 100), { value: undefined, late: true });
    assert.ok(performance.now() - t2 < 100 + harness.FRAME_SLACK_MS + 1_000, 'never answered: late after the bound');
    assert.deepEqual(await tabs[2].within('Promise.resolve(1 + 1)', 100), { value: 2, late: false });
    // A load in a hidden tab says it drew no frame, and is judged so by name; its paint and frame waits are each bounded.
    const t1 = performance.now();
    const hidden = await harness.loadOnce(hiddenTab, b.cdp, page, 50);
    const bound = 2 * (harness.FRAME_WAIT_MS + harness.FRAME_SLACK_MS) + 14_000;
    assert.ok(performance.now() - t1 < bound, `a load in a hidden tab ends within ${bound} ms: ${(performance.now() - t1).toFixed(0)} ms`);
    assert.equal(hidden.ended, true);
    assert.equal(hidden.drew, false);
    assert.ok(harness.judge('/x', 'cold', [hidden]).failures.includes('/x cold: 1 of 1 loads drew no frame within 2000 ms of settling (a hidden tab draws none), so they are not settled loads'));
    const visible = await harness.loadOnce(tabs[2], b.cdp, page, 50);
    assert.equal(visible.drew, true);
    assert.deepEqual(harness.judge('/x', 'cold', [visible]).failures.filter((f) => f.includes('frame')), []);
    for (const t of tabs) await t.close();
    await b.cdp.send('Target.disposeBrowserContext', { browserContextId });
  } finally {
    await b.close();
  }
  // The page-side wait: true once two frames are drawn, false when its time passed first.
  assert.match(harness.twoFrames(500), /setTimeout\(\(\) => r\(false\), 500\)/);
  assert.match(harness.twoFrames(500), /requestAnimationFrame\(\(\) => requestAnimationFrame\(\(\) => \{ clearTimeout\(t\); r\(true\); \}\)\)/);
});

test('a page the app does not route fails by name and is never passed: the build that answered names its routes, case kept, one trailing slash the same page', { timeout: 240_000 }, async () => {
  // Before (verify r4, 23 Oct): the server answers every path with the
  // app's shell and 200, and judge failed a load only on its status, so
  // `--pages /nope,/SPEED` wrote "# Browser harness — PASS", every row
  // holding, for two pages the app does not have.
  const { routeOf, notTheApps, READ_ROUTES } = harness;
  const routes = ['/', '/audit', '/db', '/speed', '/(group)/inside'];
  assert.deepEqual(routeOf('/speed', routes), { route: '/speed', unmatched: [] });
  assert.deepEqual(routeOf('/speed/', routes), { route: '/speed', unmatched: [] }, 'one trailing slash is the same page');
  assert.deepEqual(routeOf('/%73peed', routes), { route: '/speed', unmatched: [] }, 'an escape is decoded, as the router decodes it');
  assert.deepEqual(routeOf('/db?seg=options', routes), { route: '/db', unmatched: [] }, 'a query is not the path');
  assert.deepEqual(routeOf('/inside', routes), { route: '/(group)/inside', unmatched: [] }, 'a group is no part of the path');
  assert.deepEqual(routeOf('/', routes), { route: '/', unmatched: [] });
  for (const p of ['/SPEED', '/nope', '//', '/speed//', '/speed%2F']) assert.equal(routeOf(p, routes).route, null, p);
  assert.equal(notTheApps('/speed', { routes }), '');
  assert.equal(notTheApps('/SPEED', { routes }), 'the app has no such page: no route of the build that answered matches /SPEED (its routes: /, /audit, /db, /speed, /(group)/inside), so it drew its not-found page');
  assert.equal(notTheApps('/a/1', { routes: ['/', '/a/[id]'] }), 'the harness cannot tell whether the app has this page: no plain route of the build that answered is /a/1, and it does not match the routes with a parameter (/a/[id])');
  assert.equal(notTheApps('/a', { routes: null, why: 'the document names no app entry' }), "the document is not the app's: the document names no app entry");
  assert.equal(notTheApps('/a', undefined), "the harness could not read the app's routes in the page");
  assert.match(READ_ROUTES, /Promise\.race\(\[import\(at\)\.then\(\(m\) => \(\{ m \}\)\), new Promise\(\(r\) => setTimeout\(\(\) => r\(null\), 5000\)\)\]\)/, 'the import is bounded in the page');
  assert.match(READ_ROUTES, /const d = imported\.m\.dictionary;/);
  assert.equal(harness.ROUTES_WAIT_MS, 5_000);
  assert.match(source('ci/browser-harness.mjs'), /const read = await tab\.within\(READ_ROUTES, ROUTES_WAIT_MS\);/, 'and here');
  // In a real browser, against a stand-in that answers every path with the shell and 200, as the api does.
  const chrome = harness.parseArgs(['--base', 'http://h:1']).chrome;
  assert.ok(chrome, 'this test needs Chrome or Chromium: set BRUTEX_CHROME');
  const root = process.getuid?.() === 0 ? ['--no-sandbox'] : [];
  const dir = mkdtempSync(join(tmpdir(), 'r315-routes-'));
  const capture = join(dir, 'capture.json');
  const site = await browserStandIn({ fallback: true });
  try {
    const r = await harness.runHarness(harness.parseArgs(['--base', site.base, '--n', '2', '--pages', '/nope,/SPEED,/speed/,/hang-entry', '--no-interactions', '--no-idle', '--no-report-check', '--settle-ms', '100', '--capture', capture, ...root]));
    // An app entry that never loads is said once its bound has passed, never waited on.
    assert.ok(r.failures.includes(`/hang-entry cold: the document is not the app's: its app entry /_app/immutable/entry/app.HANG.js did not load within ${harness.ROUTES_WAIT_MS} ms`), r.failures.join('\n'));
    const listed = '/trip, /trip-db, /db, /gaps, /speed, /blank, /self, /reporting, /later';
    const no = (/** @type {string} */ p) => `the app has no such page: no route of the build that answered matches ${p} (its routes: ${listed}), so it drew its not-found page`;
    const text = r.lines.join('\n');
    for (const p of ['/nope', '/SPEED']) {
      assert.ok(text.includes(`| ${p} | cold | 1 | NOT MEASURED: ${no(p)} | — | — | — | FAILS |`), text);
      assert.ok(r.failures.includes(`${p} cold: ${no(p)}`), r.failures.join('\n'));
    }
    assert.match(text, /^\| \/speed\/ \| cold \| 2 \| [\d.]+ \/ [\d.]+ \/ [\d.]+ \|/m, 'a page under a spelling the router takes is measured');
    assert.match(text, /^\| \/speed\/ \| warm \| 2 \|.*\| holds \|$/m);
    assert.equal(r.ok, false);
    assert.match(harness.renderHarness(r, { n: 2, base: site.base }), /^# Browser harness — FAIL/, 'never a PASS sheet for a page the app does not have');
    const saved = JSON.parse(readFileSync(capture, 'utf8'));
    assert.ok(saved.requests.some((/** @type {any} */ q) => q.page === '/speed/' && q.url === '/feeds.json'), 'the page measured is captured');
    assert.deepEqual(saved.requests.filter((/** @type {any} */ q) => q.page !== '/speed/'), [], 'a page not measured is not captured');
  } finally {
    site.stop();
  }
  // A document that is not the app's: refused by name.
  const bare = await browserStandIn({ noEntry: true });
  try {
    const r = await harness.runHarness(harness.parseArgs(['--base', bare.base, '--n', '1', '--pages', '/speed', '--no-interactions', '--no-idle', '--no-report-check', '--settle-ms', '100', ...root]));
    assert.ok(r.failures.includes("/speed cold: the document is not the app's: the document names no app entry (/_app/immutable/entry/app.*.js)"), r.failures.join('\n'));
  } finally {
    bare.stop();
  }
});

test('a tab opened straight at a page under a spelling the router takes is that page\'s own load: /speed/ is /speed', { timeout: 120_000 }, async () => {
  // Before (verify r4, 23 Oct): opened at /speed/, the router made the
  // address /speed while the document stayed /speed/, and the exact
  // comparison drew "This tab opened on /speed/ and came here by a link
  // inside the app" over the document's own 414,363 B load.
  const { routePath, samePage, selfWhose } = speedModule;
  assert.deepEqual(['/speed/', '/speed', '/%73peed', '/SPEED', '/speed//', '//', '/', '/a%2Fb', '/%25', '/%E0%A4%A'].map(routePath), ['/speed', '/speed', '/speed', '/SPEED', '/speed//', '//', '/', '/a%2Fb', '/%25', null]);
  assert.equal(samePage('/speed/', '/speed'), true);
  assert.equal(samePage('/%E0%A4%A', '/%E0%A4%A'), true, 'a path no router reads is compared as written');
  assert.equal(samePage('/SPEED', '/speed'), false, 'the router matches case-sensitively');
  const all = ['paint', 'navigation', 'resource'];
  const doc = (/** @type {string} */ path) => selfAdd(selfStart(all), [{ entryType: 'navigation', name: `http://h:1${path}`, transferSize: 6_000 }]);
  for (const path of ['/speed/', '/%73peed', '/speed?feed=a']) {
    assert.equal(selfWhose(doc(path), '/speed'), 'page', path);
    assert.doesNotMatch(selfText(doc(path), '/speed'), /tab's figures|came here|came back/, path);
  }
  assert.equal(selfWhose(doc('/db'), '/speed'), 'tab');
  assert.ok(selfText(doc('/db'), '/speed').endsWith(" This tab opened on /db and came here by a link inside the app, so these are the tab's figures since it opened, not this page's alone."));
  assert.equal(selfWhose(selfAdd(selfStart(all, true), [{ entryType: 'navigation', name: 'http://h:1/speed/', transferSize: 1 }]), '/speed'), 'tab', 'a second watch of one document is the tab\'s, under any spelling');
  // In a real browser: a fresh tab opened straight at /self/, whose page drops the slash as the router does.
  const chrome = harness.parseArgs(['--base', 'http://h:1']).chrome;
  assert.ok(chrome, 'this test needs Chrome or Chromium: set BRUTEX_CHROME');
  const site = await browserStandIn({});
  const b = await harness.launch(chrome, process.getuid?.() === 0);
  try {
    const { browserContextId } = await b.cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
    const tab = await harness.openTab(b.cdp, browserContextId, { readOnly: true, base: site.base });
    const opened = await harness.loadOnce(tab, b.cdp, `${site.base}/self/`, 300);
    assert.equal(opened.ended, true);
    const v = await tab.evaluate(`(async () => {
      const total = () => [...performance.getEntriesByType('navigation'), ...performance.getEntriesByType('resource')].reduce((a, e) => a + e.transferSize, 0);
      const end = Date.now() + 10000;
      let l = document.querySelector('[data-speed-self]');
      while (Date.now() < end && !(l && Number(l.dataset.wireBytes) === total() && total() > 450000)) { await new Promise((r) => setTimeout(r, 20)); l = document.querySelector('[data-speed-self]'); }
      return { path: location.pathname, doc: new URL(performance.getEntriesByType('navigation')[0].name).pathname, figuresOf: l.dataset.figuresOf, text: l.textContent, bytes: Number(l.dataset.wireBytes), browserBytes: total() };
    })()`, 20_000);
    assert.deepEqual([v.path, v.doc], ['/self', '/self/'], 'the address lost its slash; the document kept it');
    assert.equal(v.figuresOf, 'page', v.text);
    assert.doesNotMatch(v.text, /tab's figures|came here|came back/, v.text);
    assert.equal(v.bytes, v.browserBytes, 'the drawn wire is the document\'s own whole load');
    await tab.close();
    await b.cdp.send('Target.disposeBrowserContext', { browserContextId });
  } finally {
    await b.close();
    site.stop();
  }
});

test('the harness refuses an output it cannot write before it starts a browser, keeps its sheet when the capture fails at the end, says a run it could not finish, and names the build that answered', { timeout: 240_000 }, async () => {
  // Before (verify r4 follow-ups, 23 Oct): a --capture that could not be
  // written threw away the whole run at its end (exit 2, no sheet), and the
  // sheet never named the build that answered.
  const { unwritable } = harness;
  const dir = mkdtempSync(join(tmpdir(), 'r315-out-'));
  assert.equal(unwritable('--out', join(dir, 'sheet.md')), '');
  assert.match(unwritable('--capture', join(dir, 'missing', 'c.json')), /^--capture .*\/missing\/c\.json: .*\/missing cannot take a new file: ENOENT/);
  assert.equal(unwritable('--out', dir), `--out ${dir} is a directory`);
  writeFileSync(join(dir, 'file'), 'x');
  assert.equal(unwritable('--out', join(dir, 'file', 'x.md')), `--out ${join(dir, 'file', 'x.md')}: ${join(dir, 'file')} is not a directory`);
  // As a command: refused before any browser is started (the browser named does not exist, and is never reached).
  const { spawnSync } = await import('node:child_process');
  const script = new URL('ci/browser-harness.mjs', web).pathname;
  const early = spawnSync(process.execPath, [script, '--base', 'http://127.0.0.1:1', '--chrome', '/nonexistent/chrome', '--capture', join(dir, 'missing', 'c.json')], { encoding: 'utf8' });
  assert.equal(early.status, 2);
  assert.match(early.stderr, /^browser-harness: --capture .*c\.json: .*cannot take a new file: ENOENT/);
  // A capture that cannot be written at the end: the sheet still comes back, with the failure.
  const chrome = harness.parseArgs(['--base', 'http://h:1']).chrome;
  assert.ok(chrome, 'this test needs Chrome or Chromium: set BRUTEX_CHROME');
  const root = process.getuid?.() === 0 ? ['--no-sandbox'] : [];
  const hook = join(dir, 'gives-up.mjs');
  writeFileSync(hook, "export default async () => { throw new Error('the hook gave up'); };\n");
  const site = await browserStandIn({});
  try {
    const args = { ...harness.parseArgs(['--base', site.base, '--n', '1', '--pages', '/speed', '--no-interactions', '--no-idle', '--no-report-check', '--settle-ms', '100', '--hook', hook, ...root]), capture: join(dir, 'missing', 'c.json') };
    const r = await harness.runHarness(args);
    assert.match(r.lines.join('\n'), /^\| \/speed \| cold \| 1 \|/m, 'what was measured stands');
    assert.ok(r.failures.some((f) => /^the capture could not be written: could not write .*c\.json whole: ENOENT/.test(f)), r.failures.join('\n'));
    // A run that could not finish says so, and keeps its sheet.
    assert.ok(r.failures.includes('the run stopped before it finished: the hook gave up'), r.failures.join('\n'));
    assert.ok(r.lines.includes('The run stopped here: the hook gave up.'));
    // The build that answered.
    assert.deepEqual(r.builds, ['f'.repeat(40)]);
    assert.match(harness.renderHarness(r, { n: 1, base: site.base }), new RegExp(`^n = 1 cold and 1 warm loads a page against ${site.base.replace(/[.]/g, '\\.')}, answered by build f{40}\\.$`, 'm'));
  } finally {
    site.stop();
  }
  // Two builds in one run fail it by name; an unstamped server is said.
  let flip = 0;
  const mixed = await browserStandIn({ build: (/** @type {string} */ path) => (path === '/speed' && (flip += 1) % 2 === 0 ? 'd'.repeat(40) : 'c'.repeat(40)) });
  try {
    const r = await harness.runHarness(harness.parseArgs(['--base', mixed.base, '--n', '2', '--pages', '/speed', '--no-interactions', '--no-idle', '--no-report-check', '--settle-ms', '100', ...root]));
    assert.ok(r.failures.some((f) => /^the pages were answered by 2 builds \((c{40}, d{40}|d{40}, c{40})\): a deploy during the run, so its figures are not one build's$/.test(f)), r.failures.join('\n'));
    assert.match(harness.renderHarness(r, { n: 2, base: mixed.base }), /, answered by 2 builds: /);
  } finally {
    mixed.stop();
  }
  const plain = await browserStandIn({ build: null });
  try {
    const r = await harness.runHarness(harness.parseArgs(['--base', plain.base, '--n', '1', '--pages', '/speed', '--no-interactions', '--no-idle', '--no-report-check', '--settle-ms', '100', ...root]));
    assert.ok(r.notes.includes('page loads answered with no x-brutex-build header, so no build is named for them: 3'), r.notes.join('\n'));
    assert.match(harness.renderHarness(r, { n: 1, base: plain.base }), /, answered by no named build\.$/m);
  } finally {
    plain.stop();
  }
  assert.equal(harness.headerOf({ 'X-Brutex-Build': 'abc' }, 'x-brutex-build'), 'abc', 'a header in any case');
  assert.equal(harness.headerOf(undefined, 'x-brutex-build'), null);
});

test('the sheet asks only its own server, asks a HEAD as a HEAD, says each page a capture lacks, reads the capture before the server, and measures one client count once', async () => {
  // Before (verify r4 follow-ups, 23 Oct): a capture URL with a backslash
  // was sent to another host; a HEAD request was field-checked with a whole
  // GET (591,200,898 B of /store.json per ask); an empty capture passed with
  // no word; a missing --from-pages file was an uncaught ENOENT after the
  // server was asked; and --clients 10,10 timed one count twice.
  for (const url of ['/\\evil.invalid/x.json', '/\t/evil.invalid/x.json', '/\\\\evil.invalid/x.json', '/a\\b']) {
    assert.throws(() => sheet.readCapture(JSON.stringify({ requests: [{ page: '/x', method: 'GET', url }] })), /is not a path of this server/, JSON.stringify(url));
  }
  assert.equal(sheet.onOrigin('/a?b=1', 'http://h:1'), true);
  assert.equal(sheet.onOrigin('/\\evil.invalid/x', 'http://h:1'), false);
  assert.equal(sheet.onOrigin('/\n/evil.invalid/x', 'http://h:1'), false);
  let connections = 0;
  const server = http.createServer((_req, res) => res.end('{}'));
  server.on('connection', () => (connections += 1));
  await new Promise((resolve) => server.listen(0, '127.0.0.1', () => resolve(undefined)));
  const base = `http://127.0.0.1:${/** @type {import('node:net').AddressInfo} */ (server.address()).port}`;
  try {
    const off = await sheet.timed(base, '/\\evil.invalid/x.json');
    assert.equal(off.status, 0);
    assert.equal(/** @type {any} */ (off).error, `/\\evil.invalid/x.json is not a path of ${base}: never sent`);
    assert.equal(connections, 0, 'nothing was sent anywhere');
    assert.equal((await sheet.timed(base, '/x')).status, 200, 'its own paths still are');
  } finally {
    server.close();
  }
  // A HEAD is asked as a HEAD; what its status cannot tell is said, never passed and never failed.
  /** @type {string[]} */
  const asked = [];
  /** @param {string} path @param {{method?: string}} [o] */
  const garbledFeed = new URLSearchParams({ feed: GARBLED }).toString();
  const send = async (path, o = {}) => {
    asked.push(`${o.method} ${path}`);
    if (path.includes(garbledFeed)) return answer(400, '{"error":"no such feed"}');
    return answer(200, o.method === 'HEAD' ? '' : '{"same":true}');
  };
  const head = { page: '/db', route: '/store.json', label: '', method: 'HEAD', url: '/store.json?feed=gdfl&watch=1', builder: '(the page, captured)' };
  const get = { page: '/db', route: '/x.json', label: '', method: 'GET', url: '/x.json?feed=gdfl&limit=2', builder: 'b' };
  const checked = await sheet.fieldCheck(send, [head, get]);
  assert.deepEqual(asked.filter((a) => a.includes('/store.json')), ['HEAD /store.json?feed=gdfl&watch=1', `HEAD ${garbledUrl(head.url, 'feed')}`, `HEAD ${garbledUrl(head.url, 'watch')}`], 'never a GET for a HEAD request');
  assert.deepEqual(checked.unchecked, ['the fields watch of HEAD /store.json?feed=gdfl&watch=1 (/db): each garbled value was answered 200 like the original, and a HEAD answer has no body to tell a field read from one ignored']);
  assert.deepEqual(checked.map((i) => [i.request.url, i.field, i.how]), [['/x.json?feed=gdfl&limit=2', 'limit', 'garbled']], 'a GET is still judged by its bytes');
  // Each measured page a capture holds no request of is said.
  const lacking = await sheet.runSheet(argsFor(), { send: standIn({ ignore: new Set() }).send, log: () => {}, capture: JSON.stringify({ requests: [{ page: '/db/', method: 'GET', url: '/aliases.json' }, { page: '/gaps?x=1', method: 'GET', url: '/gaps.json?feed=zerodha' }] }) });
  for (const page of MEASURED_PAGES) {
    const line = `${page}: the capture holds no request of this page, so what it builds inline is not timed`;
    assert.equal(lacking.notMeasured.includes(line), !['/db', '/gaps'].includes(page), page);
  }
  const empty = await sheet.runSheet(argsFor(), { send: standIn({ ignore: new Set() }).send, log: () => {}, capture: JSON.stringify({ requests: [] }) });
  assert.equal(empty.notMeasured.filter((l) => l.includes('the capture holds no request of this page')).length, MEASURED_PAGES.length);
  assert.match(sheet.renderSheet(empty, argsFor()), /- NOT MEASURED: \/gaps: the capture holds no request of this page, so what it builds inline is not timed/);
  // A capture file that cannot be read is refused before the server is asked anything.
  const dir = mkdtempSync(join(tmpdir(), 'r315-from-'));
  let calls = 0;
  const missing = await sheet.runSheet({ ...argsFor(), fromPages: join(dir, 'absent.json') }, { send: async () => ((calls += 1), answer(200)), log: () => {} });
  assert.equal(calls, 0, 'the server was asked nothing');
  assert.equal(missing.ok, false);
  assert.match(missing.failures[0], /^the capture of the pages' own requests could not be read: --from-pages .*absent\.json: ENOENT/);
  const { spawnSync } = await import('node:child_process');
  const cli = spawnSync(process.execPath, [new URL('ci/speed-sheet.mjs', web).pathname, '--base', 'http://127.0.0.1:1', '--from-pages', join(dir, 'absent.json')], { encoding: 'utf8' });
  assert.equal(cli.status, 1);
  assert.match(cli.stdout, /- FAILED: the capture of the pages' own requests could not be read: --from-pages .*absent\.json: ENOENT/);
  assert.doesNotMatch(cli.stderr, /at .*speed-sheet\.mjs/, 'no stack');
  // One client count, one pass; the answer bound has a ceiling.
  assert.throws(() => sheet.parseArgs(['--base', 'http://h:1', '--clients', '10,10']), /^Error: --clients names 10 twice: each client count is measured once a run$/);
  assert.throws(() => sheet.parseArgs(['--base', 'http://h:1', '--clients', '1,1e1,10']), /^Error: --clients names 10 twice/);
  assert.equal(sheet.parseArgs(['--base', 'http://h:1', '--max-answer-bytes', String(sheet.MAX_TEXT_BYTES)]).maxAnswerBytes, 64_000_000);
  assert.throws(() => sheet.parseArgs(['--base', 'http://h:1', '--max-answer-bytes', '9007199254740991']), /^Error: --max-answer-bytes must be at most 64000000$/);
});

test('a load whose navigation the browser does not answer, or whose page stops answering once loaded, is not a load of the page: each said by name, and any other refusal thrown', async () => {
  // The two ways loadOnce's bound can end a load besides its own timer,
  // driven through a stand-in protocol and tab (repair 4).
  const { CdpTimeout } = harness;
  /** @param {Record<string, (params: any) => Promise<any>>} answers @param {{settle?: () => Promise<boolean>}} [o] */
  const rig = (answers, o = {}) => {
    /** @type {Set<(m: any) => void>} */
    const listeners = new Set();
    /** @type {string[]} */
    const sent = [];
    const cdp = /** @type {any} */ ({
      on: (/** @type {(m: any) => void} */ f) => (listeners.add(f), () => listeners.delete(f)),
      send: async (/** @type {string} */ method, /** @type {any} */ params) => {
        sent.push(method);
        const a = answers[method];
        if (a === undefined) return {};
        return a(params);
      },
      fire: (/** @type {any} */ m) => { for (const f of listeners) f(m); },
      listeners,
      sent
    });
    const tab = /** @type {any} */ ({
      sessionId: 's',
      doc: { status: 0, build: null },
      frames: { drew: null },
      reset() { this.doc.status = 0; },
      seen: () => [{ method: 'GET', url: 'http://h:1/x', bytes: 7, status: 200, failed: null }],
      settle: o.settle ?? (async () => true),
      within: async () => ({ value: { fcp: 1 }, late: false }),
      evaluate: async () => ({ fcp: 1, longest: 0 })
    });
    return { cdp, tab };
  };
  // The navigation itself is not answered within its bound.
  const silent = rig({ 'Page.navigate': async () => { throw new CdpTimeout('the browser did not answer Page.navigate within 100 ms'); } });
  const a = await harness.loadOnce(silent.tab, silent.cdp, 'http://h:1/x', 10, 100);
  assert.deepEqual([a.ended, a.why, a.wire, a.fcp, a.drew], [false, 'the load did not end within 100 ms (no answer to its document)', 7, null, false]);
  assert.ok(silent.cdp.sent.includes('Page.stopLoading'), 'what the tab is still loading is stopped');
  assert.equal(silent.cdp.listeners.size, 0, 'the load-event listener is removed');
  // Any other refusal of the navigation is the protocol's own, and is thrown.
  const refused = rig({ 'Page.navigate': async () => { throw new Error('Cannot navigate to invalid URL (-32000)'); } });
  await assert.rejects(harness.loadOnce(refused.tab, refused.cdp, 'nonsense', 10, 100), /^Error: Cannot navigate to invalid URL \(-32000\)$/);
  // Loaded, then the page stops answering: its settle is not answered.
  /** @param {() => Promise<boolean>} settle */
  const loadedThen = (settle) => {
    const r = rig({ 'Page.navigate': async () => { setTimeout(() => r.cdp.fire({ sessionId: 's', method: 'Page.loadEventFired' }), 5); return {}; } }, { settle });
    return r;
  };
  const stuck = loadedThen(async () => { throw new CdpTimeout('the browser did not answer Runtime.evaluate within 3000 ms'); });
  stuck.tab.doc.status = 200;
  const b = await harness.loadOnce(stuck.tab, stuck.cdp, 'http://h:1/x', 10, 5_000);
  assert.equal(b.ended, false);
  assert.match(b.why, /^the page stopped answering \d+ ms after its load began: the browser did not answer Runtime\.evaluate within 3000 ms$/);
  const broken = loadedThen(async () => { throw new TypeError('not a timeout'); });
  await assert.rejects(harness.loadOnce(broken.tab, broken.cdp, 'http://h:1/x', 10, 5_000), /^TypeError: not a timeout$/);
  // A load that ends is a load: its figures, ended and drawn as the tab said.
  const fine = loadedThen(async () => true);
  fine.tab.frames.drew = true;
  const c = await harness.loadOnce(fine.tab, fine.cdp, 'http://h:1/x', 10, 5_000);
  assert.deepEqual([c.ended, c.why, c.fcp, c.drew, c.wire], [true, '', 1, true, 7]);
});

test('a sort edge the server does not answer is NOT MEASURED by name, within the load bound', { timeout: 120_000 }, async () => {
  const chrome = harness.parseArgs(['--base', 'http://h:1']).chrome;
  assert.ok(chrome, 'this test needs Chrome or Chromium: set BRUTEX_CHROME');
  const root = process.getuid?.() === 0 ? ['--no-sandbox'] : [];
  const site = await browserStandIn({ hangSources: true });
  try {
    const t0 = performance.now();
    const r = await harness.runHarness(harness.parseArgs(['--base', site.base, '--n', '1', '--pages', '/speed', '--no-idle', '--no-report-check', '--settle-ms', '100', '--step-wait-ms', '200', '--load-wait-ms', '800', ...root]));
    assert.ok(r.notes.some((n) => /^sort at the 20,000-row browser sort edge: NOT MEASURED — finding the sort edge: \/sources\.json\?exchange=NSE&segment=INDEX&symbol=NIFTY had no whole answer within 800 ms: /.test(n)), r.notes.join('\n'));
    assert.ok(performance.now() - t0 < 90_000);
  } finally {
    site.stop();
  }
});

test('a sheet that cannot be written at the end of a run is named on stderr, after the sheet is on stdout, with exit 2: the harness and the speed sheet', { timeout: 120_000 }, async () => {
  if (process.getuid?.() === 0) return void assert.ok(true, 'permissions do not bind root: not run as root');
  const { execFile } = await import('node:child_process');
  const { chmodSync } = await import('node:fs');
  /** @param {string} script @param {string[]} argv */
  const run = (script, argv) => /** @type {Promise<{code: number, stdout: string, stderr: string}>} */ (new Promise((resolve) => execFile(process.execPath, [new URL(script, web).pathname, ...argv], (error, stdout, stderr) => resolve({ code: error ? Number(error.code) : 0, stdout, stderr }))));
  // The harness: its output directory is made read-only once the browser asks for the page, after the start's check.
  const dir = mkdtempSync(join(tmpdir(), 'r315-late-'));
  const site = await browserStandIn({ onFirstPage: () => chmodSync(dir, 0o500) });
  try {
    const ran = await run('ci/browser-harness.mjs', ['--base', site.base, '--n', '1', '--pages', '/speed', '--no-interactions', '--no-idle', '--no-report-check', '--settle-ms', '100', '--out', join(dir, 'sheet.md')]);
    assert.equal(ran.code, 2, ran.stderr);
    assert.match(ran.stdout, /^# Browser harness — PASS/, 'the sheet is on stdout');
    assert.match(ran.stderr, /^browser-harness: could not write .*sheet\.md whole: EACCES/);
  } finally {
    chmodSync(dir, 0o700);
    site.stop();
  }
  // The speed sheet: the stand-in server makes the directory read-only when it is first asked.
  const sheetDir = mkdtempSync(join(tmpdir(), 'r315-late-sheet-'));
  let first = true;
  const server = http.createServer((req, res) => {
    if (first) {
      first = false;
      chmodSync(sheetDir, 0o500);
    }
    res.writeHead(503, { 'content-type': 'text/plain' });
    res.end('down');
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', () => resolve(undefined)));
  try {
    const base = `http://127.0.0.1:${/** @type {import('node:net').AddressInfo} */ (server.address()).port}`;
    const ran = await run('ci/speed-sheet.mjs', ['--base', base, '--out', join(sheetDir, 'sheet.md')]);
    assert.equal(ran.code, 2, ran.stderr);
    assert.match(ran.stdout, /^# Speed sheet — FAIL[\s\S]*is not up/, 'the sheet is on stdout');
    assert.match(ran.stderr, /^speed-sheet: could not write .*sheet\.md whole: EACCES/);
    assert.doesNotMatch(ran.stderr, /at .*speed-sheet\.mjs/, 'no stack');
  } finally {
    chmodSync(sheetDir, 0o700);
    server.close();
  }
  assert.equal(sheet.onOrigin('/x', 'not a base'), false, 'a base no URL reader reads holds no path');
});
