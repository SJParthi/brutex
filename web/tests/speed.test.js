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
    // One byte over: said to the byte, since both figures round to 400.0 KB (repair 8; verify r8 nit).
    assert.throws(() => plugin.writeBundle.call(ctx, { dir: big }), /1 page\(s\) past their budget.*\/gaps: 400001 B on first load.*budget 400000 B — OVER ITS BUDGET/);
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
  assert.equal(budgetOf('/ingest'), 550_000, 'raised for DATA-1w\'s and R4-18\'s panels and R4-28\'s F&O modes on the 25 Oct merged tree (D-5915 item 36)');
  assert.equal(budgetOf('/backtest'), 1_125_000);
  const v = budgetVerdict([{ route: '/db', node: 6, js: 600_000, css: 100_000, bytes: 700_000, files: [] }, { route: '/gaps', node: 7, js: 1, css: 1, bytes: 400_001, files: [] }, { route: '/audit', node: 3, js: 1, css: 1, bytes: 400_000, files: [] }]);
  assert.deepEqual(v.map((x) => [x.over, x.overTarget]), [[false, true], [true, true], [false, false]], 'exactly the budget, and exactly the target, are inside');
  // Figures that round to one are said to the byte (repair 8; verify r8 nit: "545.6 KB ..., budget 545.6 KB — OVER").
  assert.equal(v[1].line, '/gaps: 400001 B on first load (0.0 KB script, 0.0 KB style), budget 400000 B — OVER ITS BUDGET — over the 400000 B wire target');
  assert.equal(v[0].line, '/db: 700.0 KB on first load (600.0 KB script, 100.0 KB style), budget 825.0 KB — over the 400.0 KB wire target');
  assert.equal(v[2].line, '/audit: 400.0 KB on first load (0.0 KB script, 0.0 KB style), budget 400.0 KB', 'equal figures are one figure');
  const nearTarget = budgetVerdict([{ route: '/db', node: 6, js: 400_040, css: 0, bytes: 400_040, files: [] }])[0].line;
  assert.equal(nearTarget, '/db: 400040 B on first load (400.0 KB script, 0.0 KB style), budget 825000 B — over the 400000 B wire target', 'near the wire target only');
  const nearBudget = budgetVerdict([{ route: '/db', node: 6, js: 825_001, css: 0, bytes: 825_001, files: [] }])[0].line;
  assert.equal(nearBudget, '/db: 825001 B on first load (825.0 KB script, 0.0 KB style), budget 825000 B — OVER ITS BUDGET — over the 400000 B wire target', 'near a budget that is not the target');
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
  // Repair 9: where the commands print, beside --out unless --command-log names it; one file is never two of them.
  assert.equal(a.commandLog, null, 'no --out and no --command-log: a file in the system\'s temporary directory, made at the first command');
  assert.equal(b.commandLog, 'o.md.commands.log');
  assert.equal(sheet.parseArgs(['--base', 'http://h:1', '--out', 'o.md', '--command-log', 'c.log']).commandLog, 'c.log');
  assert.throws(() => sheet.parseArgs(['--base', 'http://h:1', '--out', 'o.md', '--command-log', 'o.md']), /^Error: --command-log and --out name one file \(o\.md\): what the commands print would be written into the sheet$/);
  assert.throws(() => sheet.parseArgs(['--base', 'http://h:1', '--from-pages', 'c.json', '--command-log', 'c.json']), /^Error: --command-log and --from-pages name one file \(c\.json\): what the commands print would be written into the capture$/);
  assert.throws(() => sheet.parseArgs(['--base', 'http://h:1', '--command-log']), /^Error: --command-log needs a value$/);
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
    // The whole answer is bounded (repair 5): no headers came within it, and that is said.
    assert.equal(/** @type {any} */ (late).error, 'no whole answer within 20 ms (not even its headers came)');
    assert.equal(/** @type {any} */ (late).timedOut, true);
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
  assert.deepEqual(ctx, { feed: 'zerodha', instrument: 'NSE-INDEX-NIFTY', timeframe: '1min', month: '2026-08', day: '2026-08-31', dayFrom: 'newest', from: '2026-08-01', to: '2026-08-31' });
  const gdfl = await sheet.discover(get({ [url]: rungs }), { ...args, feed: 'gdfl' });
  assert.deepEqual([gdfl.timeframe, gdfl.day, gdfl.dayFrom], ['1s', '2026-07-01', 'first'], 'a rung with no last day starts at its month, a day taken (repair 8)');
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
    // Repair 6: a timed row holds no answer, so the reason is what many at once would ask of the server.
    assert.ok(run.notMeasured.includes(`warm at 2 clients: ${r.url.startsWith('/inspection') ? 'HEAD' : 'GET'} ${r.url} answered ${r.bytes} bytes, past --max-answer-bytes 40; 2 at once would ask ${2 * r.bytes} bytes of the server at once, so it is timed at one client only`), r.url);
  }
  assert.match(sheet.renderSheet(run, argsFor()), /NOT MEASURED: warm at 2 clients: GET \/census\/facets\.json\?feed=zerodha answered 59 bytes, past --max-answer-bytes 40; 2 at once would ask 118 bytes of the server at once/);
  // Inside the bound (the default), nothing is skipped.
  const all = await sheet.runSheet(argsFor(), { send: server.send, log: () => {}, capture: JSON.stringify({ requests: [] }) });
  assert.equal(all.rows.filter((r) => r.phase === 'warm' && r.clients === 2).length, cold.length);
  assert.ok(!all.notMeasured.some((l) => l.includes('--max-answer-bytes')));
});

test('every phase of the sheet is run or said NOT MEASURED, and every row is stamped with one build', async () => {
  const server = standIn({ ignore: new Set() });
  // A writer that exits at once, against a server slow enough for the exit to land mid-pass; a restart
  // command that restarts the stand-in (repair 9: one that restarts nothing times nothing), each printing
  // into the commands' log.
  const dir = mkdtempSync(join(tmpdir(), 'r315-phases-'));
  const start = join(dir, 'start');
  writeFileSync(start, '1');
  const slow = standIn({ ignore: new Set(), delayMs: 3, started: () => Number(readFileSync(start, 'utf8')) });
  const done = await sheet.runSheet({ ...argsFor(), writer: 'true', restart: `echo $$ > '${start}'`, restarts: 2, commandLog: join(dir, 'commands.log') }, { send: slow.send, log: () => {} });
  assert.ok(done.failures.some((f) => f.includes('the --writer command ended before the warm+writer pass did: it exited 0, so part of that pass ran with nothing writing')), done.failures.join('\n'));
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
  const bad = await sheet.runSheet({ ...argsFor(), restart: 'false', restarts: 2, commandLog: join(dir, 'commands.log') }, { send: server.send, log: () => {} });
  assert.ok(bad.failures.includes('restart 1: the --restart command exited 1'));
  // After the restart: down for two checks, then up, and the first /aliases.json fails.
  const state = { restarted: false, down: 0 };
  const flaky = standIn({ ignore: new Set(), up: () => !(state.down > 0 && state.down-- > 0), failFirstOf: '/aliases.json', failWhen: () => state.restarted, started: () => (state.restarted ? 2 : 1) });
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

test('a real browser\'s row is checked for what does not depend on the machine: measured, its verdict its failures\', and failed only on a timed judgement', () => {
  // Repair 7 (verify r7, 25 Oct): the tests that run a real Chrome pinned a
  // row to `| holds |`, so one slow paint failed a correct build. Their
  // check, `measuredRow`, is held here to the harness's own rows (`judge`):
  // it must let every timed judgement through and nothing else.
  const load = (/** @type {object} */ o) => ({ fcp: 100, longest: 10, heap: 10e6, wire: 300e3, settled: true, status: 200, drew: true, ...o });
  /** @param {string} page @param {object[]} loads @param {string[]} [more] */
  const run = (page, loads, more = []) => {
    const j = harness.judge(page, 'cold', /** @type {any} */ (loads));
    return { lines: ['| page | ...', j.row], failures: [...j.failures, ...more] };
  };
  assert.deepEqual(measuredRow(run('/a', [load({})]), '/a', 'cold', 1), [], 'a row that holds');
  // Every timed judgement, alone and together, the targets cell FAILS.
  const slow = run('/a?x=1', [load({ fcp: 1628 }), load({ fcp: null, longest: 51 }), load({ drew: false })]);
  assert.deepEqual(measuredRow(slow, '/a?x=1', 'cold', 3), [
    '/a?x=1 cold: 1 of 3 loads painted no first content',
    '/a?x=1 cold: first content p99 1628.0 ms, not under 300 ms',
    '/a?x=1 cold: a pause of 51.0 ms, over 50 ms',
    `/a?x=1 cold: 1 of 3 loads drew no frame within ${harness.FRAME_WAIT_MS} ms of settling (a hidden tab draws none), so they are not settled loads`
  ]);
  assert.deepEqual(measuredRow(run('/a', [load({ fcp: null })]), '/a', 'cold', 1), ['/a cold: 1 of 1 loads painted no first content'], 'a paint later than the wait is a timed judgement');
  // Never a judgement the machine's speed cannot cause, never another page's, never a row that disagrees with its failures.
  for (const [r, why] of /** @type {[any, RegExp][]} */ ([
    [run('/a', [load({ status: 404 })]), /fails only on how fast the machine drew it, never on: \/a cold: 1 of 1 loads were not the page \(HTTP 404\)/],
    [run('/a', [load({ wire: 400e3 })]), /never on: \/a cold: 400\.0 KB on the wire, not under 400 KB/],
    [run('/a', [load({ heap: 100e6 })]), /never on: \/a cold: script heap 100\.0 MB, not under 100 MB/],
    [run('/a', [load({ settled: false })]), /never on: \/a cold: 1 loads still had a request in flight after 30 s/],
    [run('/a', [load({})], ['/a cold: an uncaught error: Error: boom']), /never on: \/a cold: an uncaught error: Error: boom/],
    [run('/a', [load({})], ['/a cold: first content p99 400.0 ms, not under 300 ms']), /the targets cell agrees with the row's failures/],
    [{ lines: ['| /a | cold | 0 | NOT MEASURED: the load did not end within 10 ms (no answer to its document) | — | — | — | FAILS |'], failures: [] }, /\/a cold is measured over 1 loads/],
    [{ lines: [], failures: [] }, /one \/a cold row/],
    [{ lines: [run('/a', [load({})]).lines[1], run('/a', [load({})]).lines[1]], failures: [] }, /one \/a cold row/]
  ])) assert.throws(() => measuredRow(r, '/a', 'cold', 1), why);
  assert.throws(() => measuredRow(run('/a', [load({}), load({})]), '/a', 'cold', 1), /\/a cold is measured over 1 loads/, 'n is the row\'s');
  assert.equal(timedJudgement('/b cold: first content p99 400.0 ms, not under 300 ms', '/a'), false, 'another page\'s judgement is not this page\'s');
  assert.equal(timedJudgement('/a idle: first content p99 400.0 ms, not under 300 ms', '/a'), false, 'cold or warm only');
  assert.equal(timedJudgement('/a warm: a pause of 60.0 ms, over 50 ms', '/a'), true);
  // Repair 8 (verify r8 follow-up): on a page that polls, a capped settle is the clock's too; on another page, refused (above).
  assert.deepEqual(measuredRow(run('/a', [load({ settled: false })]), '/a', 'cold', 1, { polls: true }), ['/a cold: 1 loads still had a request in flight after 30 s']);
  assert.throws(() => measuredRow(run('/a', [load({ status: 404 })]), '/a', 'cold', 1, { polls: true }), /never on: \/a cold: 1 of 1 loads were not the page/, 'polling excuses the settle cap only');
  // An interaction: each click timed or named by the clock's two outcomes, a pause only as its judgement, never NOT MEASURED.
  const rowOf = (/** @type {string} */ line, /** @type {string[]} */ failures = []) => ({ lines: ['| interaction | n | ...', line], failures });
  const frames = `picker click: 1 click(s) drew no frame within ${harness.FRAME_WAIT_MS} ms, not timed`;
  assert.equal(measuredInteraction(rowOf('| picker click | 2 | 10.0 / 12.0 / 12.0 | 3.0 |'), 'picker click', 2), 2);
  assert.equal(measuredInteraction(rowOf('| picker click | 1 | 10.0 / 10.0 / 10.0 | 3.0 |', [frames, 'picker click: a pause of 60.0 ms, over 50 ms']), 'picker click', 2), 1);
  assert.equal(measuredInteraction(rowOf('| picker click | 0 | NOT TIMED: no click settled within 30 s | 3.0 |', ['picker click: 2 click(s) still had a request in flight 30 s later, not timed']), 'picker click', 2), 0);
  for (const [r, why] of /** @type {[any, RegExp][]} */ ([
    [rowOf('| picker click | 0 | NOT MEASURED: #db-search is not on /db | — |'), /picker click is measured on a page that has its controls/],
    [rowOf('| picker click | 1 | 10.0 / 10.0 / 10.0 | 3.0 |'), /each of its 2 clicks timed or named \(1 timed, 0 named\)/],
    [rowOf('| picker click | 2 | 10.0 / 12.0 / 12.0 | 3.0 |', [frames]), /each of its 2 clicks timed or named \(2 timed, 1 named\)/],
    [rowOf('| picker click | 2 | 10.0 / 12.0 / 12.0 | 3.0 |', ['picker click: NOT MEASURED — /db: the page stopped answering']), /never on: picker click: NOT MEASURED/],
    [rowOf('| picker click | 0 | 10.0 / 12.0 / 12.0 | 3.0 |', ['picker click: 2 click(s) still had a request in flight 30 s later, not timed']), /figures exactly when a click was timed/],
    [{ lines: [], failures: [] }, /one picker click row/]
  ])) assert.throws(() => measuredInteraction(r, 'picker click', 2), why);
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
  let starts = 0;
  const blank = await sheet.runSheet({ ...argsFor(), restart: 'x', restarts: 1, allowUnstamped: true }, { send: standIn({ ignore: new Set(), build: null, drop: '/aliases.json', started: () => starts }).send, log: () => {}, restart: async () => ((starts += 1), 0) });
  assert.ok(blank.failures.includes('restart 1: the first /aliases.json answered dropped'), blank.failures.join('\n'));
  assert.deepEqual(blank.rows.filter((r) => r.phase === 'first after restart')[0].builds, [], 'an unstamped server stamps no build on a restart row');
  void gone;
  const slow = sheet.renderSheet({ ...blank, rows: [{ ...blank.rows[0], p50: 12.345, p99: 9.5, max: 10 }] }, argsFor());
  assert.match(slow, /\| 12\.3 \| 9\.50 \| 10\.0 \|/, 'from ten ms, one decimal');
  // Repair 9: the list now opens with the note of the restart that was proved.
  assert.match(slow, /\|\n\n- (?:first after restart: |SKIPPED: |NOT MEASURED: |FAILED: )/, 'a blank line between the table and the list');
  // A writer killed at the end of its pass, by a signal, is said as ended by its signal.
  const killed = await sheet.runSheet({ ...argsFor(), writer: 'sleep 5', commandLog: join(dir, 'commands.log') }, { send: standIn({ ignore: new Set() }).send, log: () => {} });
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
    // By default the run's lines go to stderr through `saidOn` (repair 8): each is taken here, and its write called back.
    const write = process.stderr.write;
    /** @type {string[]} */
    const progress = [];
    process.stderr.write = /** @type {any} */ ((/** @type {any} */ s, /** @type {any} */ cb) => (progress.push(String(s)), typeof cb === 'function' && cb(), true));
    let real;
    try {
      real = await sheet.runSheet(args);
    } finally {
      process.stderr.write = write;
    }
    assert.ok(progress.length >= 4 && progress.every((l) => /^speed sheet: .*\n$/.test(l)), progress.join(''));
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

// Its bound has room for a slow machine (repair 8): it ran 55 s at this Mac's load, and 92 s and 296 s with Chrome at
// background QoS (taskpolicy -b, 26 Oct), the second 4 s inside the 300 s it had.
test('the harness in a real browser: the four figures, the interactions, idle tabs, the reporter end to end, hooks and the capture', { timeout: 600_000 }, async () => {
  const chrome = harness.parseArgs(['--base', 'http://h:1']).chrome;
  assert.ok(chrome, 'the browser harness test needs Chrome or Chromium: set BRUTEX_CHROME');
  const root = process.getuid?.() === 0 ? ['--no-sandbox'] : [];
  // `/speed` throws once a load: only the shared reporter check loads it here (repair 6).
  const site = await browserStandIn({ speedThrows: true });
  const dir = mkdtempSync(join(tmpdir(), 'r315-harness-'));
  const hook = join(dir, 'hook.mjs');
  // The hook loads /gaps, which throws, in a tab of its own (repair 6: a hook's tabs were not heard).
  writeFileSync(hook, 'export default async (h) => { const tab = await h.openTab(h.cdp, undefined); try { await h.loadOnce(tab, h.cdp, `${h.base}/gaps`, h.settleMs); } finally { await tab.close(); } return { rows: [`hook saw ${h.base}`], failures: [`hook failure`] }; };\n');
  const silent = join(dir, 'silent.mjs');
  writeFileSync(silent, 'export default async () => undefined;\n');
  const capture = join(dir, 'capture.json');
  try {
    const args = harness.parseArgs(['--base', site.base, '--n', '2', '--pages', '/db,/gaps,/blank,/missing', '--idle-tabs', '1,2', '--idle-ms', '1000', '--settle-ms', '100', '--hook', hook, '--hook', silent, '--capture', capture, ...root]);
    const r = await harness.runHarness(args);
    const text = r.lines.join('\n');
    // Measured, and judged by how fast this machine drew it, which is not what this test is about (repair 7; verify r7).
    // The /db stand-in polls its up check every 300 ms, and each click is timed only as fast as the machine draws and
    // settles it, so a capped settle and a click not timed are named outcomes of its clock too (repair 8; verify r8).
    measuredRow(r, '/db', 'cold', 2, { polls: true });
    measuredRow(r, '/db', 'warm', 2, { polls: true });
    for (const name of ['picker click', 'feed switch', 'page turn of option cells', 'sort at the 20,000-row browser sort edge']) measuredInteraction(r, name, 2);
    assert.match(text, /^\| 1 \| 1 \| 1 s \| \d+ \| none \|$/m);
    assert.match(text, /^\| 2 \| 2 \| 1 s \| \d+ \| GET \/feeds\.json \|$/m, 'two idle tabs, each in its own window, both visible');
    assert.match(text, /Shared reporter: a failed \/feeds\.json\?harness=\d+ was reported \(202\) and found in \/logs\.json\./);
    assert.match(text, /hook saw http:\/\/127\.0\.0\.1:\d+/);
    assert.ok(r.failures.includes('2 idle tabs asked for more than the header-only up check: GET /feeds.json'), r.failures.join('\n'));
    // Every tab's uncaught errors, by where, counted (repair 5: only the warm tab's were read).
    assert.ok(r.failures.some((f) => f.startsWith('/gaps warm: an uncaught error, thrown 3 times: Error: boom')), r.failures.join('\n'));
    assert.ok(r.failures.some((f) => f.startsWith('/gaps warm: an uncaught error, thrown 3 times: Uncaught')), 'a thrown string is named by its text');
    assert.ok(r.failures.some((f) => f.startsWith('/gaps cold: an uncaught error, thrown 2 times: Error: boom')), 'a cold load\'s error is heard too, once a load');
    assert.ok(r.failures.some((f) => f.startsWith('2 idle tabs: an uncaught error: Error: boom')), 'and an idle tab\'s');
    // Repair 6 (verify r6 follow-up): the shared reporter check's tab and a hook's tabs are the run's tabs too.
    assert.ok(r.failures.some((f) => f.startsWith('the shared reporter check: an uncaught error: Error: speed boom')), r.failures.join('\n'));
    assert.ok(r.failures.some((f) => f.startsWith(`hook ${hook}: an uncaught error: Error: boom`)), 'a hook\'s tab is heard');
    assert.ok(!r.failures.some((f) => f.startsWith('/db') && f.includes('an uncaught error')), 'a page that throws nothing is said to throw nothing');
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
    /** @param {string} pages @returns {Promise<{code: number, stdout: string}>} */
    const ran = (pages) => new Promise((resolve) => execFile(process.execPath, [script, '--base', site.base, '--n', '1', '--pages', pages, '--no-interactions', '--no-idle', '--no-report-check', '--settle-ms', '100', '--out', out, ...root], (error, stdout) => resolve({ code: error ? Number(error.code) : 0, stdout })));
    // `/speed`: the exit is the verdict its sheet states, whichever verdict this machine's figures give (repair 7;
    // verify r7: it was pinned to PASS, which one slow paint of a busy machine turns into a FAIL of a correct build).
    const speed = await ran('/speed');
    const sheetText = readFileSync(out, 'utf8');
    const verdict = /^# Browser harness — (PASS|FAIL)\n/.exec(sheetText);
    assert.ok(verdict, sheetText);
    assert.equal(speed.code, verdict[1] === 'PASS' ? 0 : 1, speed.stdout);
    const failed = sheetText.split('\n').filter((l) => l.startsWith('- FAILED: ')).map((l) => l.slice('- FAILED: '.length));
    assert.equal(failed.length === 0, verdict[1] === 'PASS', 'a sheet fails exactly when it names a failure');
    for (const f of failed) assert.ok(timedJudgement(f, '/speed'), `a FAIL only on how fast this machine drew /speed: ${f}`);
    // `/blank` paints nothing on any machine: FAIL, exit 1.
    const blank = await ran('/blank');
    assert.equal(blank.code, 1, blank.stdout);
    assert.match(readFileSync(out, 'utf8'), /^# Browser harness — FAIL[\s\S]*- FAILED: \/blank cold: 1 of 1 loads painted no first content/);
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
 * when the first page is asked for; `speedThrows` makes `/speed` throw an
 * uncaught error once a load (repair 6: the shared reporter check loads it).
 *
 * `/later?paint=N` (repair 7) is `/later` whose head names a stylesheet the
 * stand-in answers N ms after it is asked: a stylesheet in the head blocks
 * the first paint and the load event alike, so the load ends and its first
 * content is drawn no sooner than N ms after the document was asked for,
 * whatever the machine. It makes a cold row fail its first-content target
 * on purpose, so a test can show its own claim does not depend on that row.
 *
 * @param {{noReporter?: boolean, reportStatus?: number, forget?: boolean, bareDb?: boolean, fallback?: boolean,
 *   noEntry?: boolean, build?: string | null | ((path: string) => string), hangSources?: boolean, onFirstPage?: () => void,
 *   speedThrows?: boolean}} o
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
  /** @param {string} title @param {string} [extra] @param {string} [head] */
  const page = (title, extra = '', head = '') => `<!doctype html><meta charset="utf-8"><title>${title}</title>${preload}${head}<body><h1>${title}</h1>${extra}${script}`;
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
    '/speed': page('speed', `<script>new Worker('/lib/worker.js');${o.speedThrows ? " setTimeout(() => { throw new Error('speed boom'); }, 0);" : ''}</script>`),
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
    // `/later?paint=N`: drawn no sooner than N ms after it is asked (a stylesheet in its head, answered N ms late).
    if (u.pathname === '/later' && u.searchParams.has('paint')) return send(200, 'text/html; charset=utf-8', page('later', '', `<link rel="stylesheet" href="/slow.css?ms=${Number(u.searchParams.get('paint'))}">`));
    if (u.pathname === '/slow.css') return void setTimeout(() => send(200, 'text/css', 'h1 { color: #123456; }'), Number(u.searchParams.get('ms')));
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

/**
 * THE JUDGEMENTS OF A REAL BROWSER'S ROW THAT HOW BUSY THE MACHINE IS CAN
 * CHANGE (repair 7; verify r7, 25 Oct). First content and the longest pause
 * are wall-clock readings of a real Chrome: one cold first paint of a
 * stand-in page took 1,628 ms in a full `npm test` on this Mac (load
 * average 10 to 20), its row failed the 300 ms target, and a test about a
 * load that does not end failed with it (one counted run of three). A paint
 * later than the harness's paint wait is said as a load that painted
 * nothing, and a frame not drawn within its wait as a load not settled:
 * both are the same clock. The heap and the wire of a stand-in page are not
 * on this list: they are its bytes, not its timing.
 */
const TIMED_JUDGEMENTS = [
  new RegExp(`^first content p99 \\d+\\.\\d ms, not under ${PAGE_TARGETS.firstContentP99Ms} ms$`),
  new RegExp(`^a pause of \\d+\\.\\d ms, over ${PAGE_TARGETS.longestPauseMs} ms$`),
  /^\d+ of \d+ loads painted no first content$/,
  new RegExp(`^\\d+ of \\d+ loads drew no frame within ${harness.FRAME_WAIT_MS} ms of settling \\(a hidden tab draws none\\), so they are not settled loads$`)
];

/**
 * The load bound of a run that must tell a load that never ends from one
 * that ends (repair 7). At 1,500 ms the stall test's cold loads, which must
 * end, had too little room: a load that took longer under a busy machine
 * was said not to end, the harness doing its job on the wrong page. Ten
 * seconds is several times the slowest stand-in load this Mac has shown
 * (L-R315-28); a load that never ends is said the same way at any bound,
 * so the only cost is the wait for each one that never ends.
 */
const STALL_BOUND_MS = 10_000;

/** Whether `failure` is one of `page`'s timed judgements, cold or warm. @param {string} failure @param {string} page */
function timedJudgement(failure, page) {
  return ['cold', 'warm'].some((phase) => failure.startsWith(`${page} ${phase}: `) && TIMED_JUDGEMENTS.some((re) => re.test(failure.slice(`${page} ${phase}: `.length))));
}

/**
 * A real browser's row, checked for all that does not depend on how busy
 * the machine is (repair 7): there is one such row, measured over `n`
 * loads, each figure a reading (first content may be `not painted`); every
 * failure the run gave that row is a timed judgement (TIMED_JUDGEMENTS),
 * never a load that was not the page, did not end, did not settle, threw,
 * or was too heavy; and its targets cell says `holds` exactly when it has
 * none. The targets themselves are tested on fixed figures (`judge`), where
 * the answer is the same on every machine. Answers the row's failures.
 *
 * A PAGE THAT POLLS (`polls`, repair 8; verify r8 follow-up, 26 Oct): a
 * stand-in that asks every 300 ms settles in the gaps between its asks, so a
 * load that ran out the 30 s settle cap there is the machine's clock too (a
 * throttled Chrome bunched its timers: 2 of 5 runs); on any other page it
 * is the page's own fault, and refused.
 *
 * @param {{lines: string[], failures: string[]}} r
 * @param {string} page
 * @param {'cold' | 'warm'} phase
 * @param {number} n
 * @param {{polls?: boolean}} [o]
 * @returns {string[]}
 */
function measuredRow(r, page, phase, n, o = {}) {
  const rows = r.lines.filter((l) => l.startsWith(`| ${page} | ${phase} | `));
  assert.equal(rows.length, 1, `one ${page} ${phase} row:\n${r.lines.join('\n')}`);
  const figure = '\\d+\\.\\d / \\d+\\.\\d / \\d+\\.\\d';
  const m = new RegExp(`^\\| [^|]+ \\| ${phase} \\| ${n} \\| (?:${figure}|not painted) \\| ${figure} \\| ${figure} \\| ${figure} \\| (holds|FAILS) \\|$`).exec(rows[0]);
  assert.ok(m, `${page} ${phase} is measured over ${n} loads, each figure a reading: ${rows[0]}`);
  const own = r.failures.filter((f) => f.startsWith(`${page} ${phase}: `));
  const capped = (/** @type {string} */ f) => o.polls === true && /^\d+ loads still had a request in flight after 30 s$/.test(f.slice(`${page} ${phase}: `.length));
  for (const f of own) assert.ok(timedJudgement(f, page) || capped(f), `${page} ${phase} fails only on how fast the machine drew it, never on: ${f}`);
  assert.equal(m[1], own.length === 0 ? 'holds' : 'FAILS', `the targets cell agrees with the row's failures (${own.join('; ') || 'none'})`);
  return own;
}

/**
 * An interaction's row, checked as {@link measuredRow} checks a page's
 * (repair 8; verify r8 follow-up, 26 Oct: the row was pinned to n = 2, and a
 * click is timed only if its tab draws two frames within `FRAME_WAIT_MS`
 * and settles within 30 s, both the machine's clock): measured on a page
 * that has its controls, never NOT MEASURED; each of its `n` clicks timed or
 * named as one of those two outcomes; and a pause past its target only as
 * the timed judgement it is. Answers how many clicks were timed.
 *
 * @param {{lines: string[], failures: string[]}} r
 * @param {string} name
 * @param {number} n
 * @returns {number}
 */
function measuredInteraction(r, name, n) {
  const rows = r.lines.filter((l) => l.startsWith(`| ${name} | `));
  assert.equal(rows.length, 1, `one ${name} row:\n${r.lines.join('\n')}`);
  const m = new RegExp(`^\\| ${name} \\| (\\d+) \\| (?:([\\d.]+ / [\\d.]+ / [\\d.]+)|NOT TIMED: no click settled within 30 s) \\| [\\d.]+ \\|$`).exec(rows[0]);
  assert.ok(m, `${name} is measured on a page that has its controls: ${rows[0]}`);
  const timed = Number(m[1]);
  assert.equal(m[2] === undefined, timed === 0, `${name}: figures exactly when a click was timed: ${rows[0]}`);
  let named = 0;
  for (const f of r.failures.filter((x) => x.startsWith(`${name}: `))) {
    const k = new RegExp(`^${name}: (\\d+) click\\(s\\) (?:drew no frame within ${harness.FRAME_WAIT_MS} ms|still had a request in flight 30 s later), not timed$`).exec(f);
    if (k) named += Number(k[1]);
    else assert.match(f, new RegExp(`^${name}: a pause of \\d+\\.\\d ms, over ${PAGE_TARGETS.longestPauseMs} ms$`), `${name} fails only on how fast the machine drew it, never on: ${f}`);
  }
  assert.equal(timed + named, n, `${name}: each of its ${n} clicks timed or named (${timed} timed, ${named} named)`);
  return timed;
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
 * `/speed.json` answers a report whose `started_ms` is `started()`, the
 * server's own start: a stand-in "restarts" when it changes (repair 9; the
 * sheet proves each restart by it), and one never given `started` never does.
 *
 * @param {{ignore: Set<string>, build?: string | null | (() => string), status?: Record<string, number>,
 *   drop?: string, up?: () => boolean, failFirstOf?: string, failWhen?: () => boolean, delayMs?: number,
 *   started?: () => unknown}} o
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
        : u.pathname === SPEED_ROUTE
          ? JSON.stringify({ ...report([]), started_ms: o.started ? o.started() : report([]).started_ms, read })
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

test('a load that does not end within its bound fails its page by name, and the run goes on to its sheet and a non-zero exit', { timeout: 300_000 }, async () => {
  // Before (verify r4, 23 Oct): loadOnce waited for Page.loadEventFired with
  // no bound, so /stall (its headers and part of its body, never the end)
  // printed nothing and wrote no sheet in 9 min 49 s, and a server paused
  // with SIGSTOP held a cold load for the whole pause.
  //
  // WHAT THIS TEST CLAIMS DOES NOT DEPEND ON HOW BUSY THE MACHINE IS (repair
  // 7; verify r7, 25 Oct). It pinned each page that ends to its targets cell
  // (`| holds |`) and to a list of failures with nothing else in it, so one
  // cold first paint of 1,628 ms failed it in a full `npm test` (one counted
  // run of three), and at a 1,500 ms bound a cold load that took longer to
  // END was itself said not to end. Now a page that ends is checked for being
  // measured (`measuredRow`), whatever its figures say; the loads that must
  // end have a bound they end well inside; and `/later?paint=400` fails its
  // first-content target on purpose, every run, beside its warm stall.
  const chrome = harness.parseArgs(['--base', 'http://h:1']).chrome;
  assert.ok(chrome, 'this test needs Chrome or Chromium: set BRUTEX_CHROME');
  const root = process.getuid?.() === 0 ? ['--no-sandbox'] : [];
  const site = await browserStandIn({});
  const dir = mkdtempSync(join(tmpdir(), 'r315-stall-'));
  // A bound a load that ends ends well inside: see L-R315-28 for the cold loads measured on this Mac. A load that never ends is said the same way at any bound; only the wait for it is longer.
  const bound = STALL_BOUND_MS;
  const stalled = `the load did not end within ${bound} ms (its document was answered HTTP 200 and did not finish)`;
  const unanswered = `the load did not end within ${bound} ms (no answer to its document)`;
  /** @param {string} pages @param {string} n */
  const run = (pages, n) => harness.runHarness(harness.parseArgs(['--base', site.base, '--n', n, '--pages', pages, '--no-interactions', '--no-idle', '--no-report-check', '--settle-ms', '100', '--load-wait-ms', String(bound), ...root]));
  try {
    const t0 = performance.now();
    const r = await run('/stall,/hang,/speed', '2');
    const took = performance.now() - t0;
    const text = r.lines.join('\n');
    assert.match(text, new RegExp(`^\\| /stall \\| cold \\| 0 \\| NOT MEASURED: ${stalled.replace(/[()]/g, '\\$&')} \\| — \\| — \\| — \\| FAILS \\|$`, 'm'), text);
    assert.match(text, new RegExp(`^\\| /hang \\| cold \\| 0 \\| NOT MEASURED: ${unanswered.replace(/[()]/g, '\\$&')} \\| — \\| — \\| — \\| FAILS \\|$`, 'm'), text);
    // The run goes on to the next page, and measures it: how its figures meet the targets is this machine's, not this test's.
    measuredRow(r, '/speed', 'cold', 2);
    measuredRow(r, '/speed', 'warm', 2);
    assert.deepEqual(r.failures.filter((f) => !f.startsWith('/speed ')), [`/stall cold: ${stalled}`, `/hang cold: ${unanswered}`]);
    assert.equal(r.ok, false);
    assert.ok(took < 2 * bound + 90_000, `bounded: two loads that never end, at ${bound} ms each, and five that end; the run took ${took.toFixed(0)} ms`);
    assert.equal(r.lines.filter((l) => l.startsWith('| /stall ')).length, 1, 'a page whose load did not end is not loaded again');
    // A page whose cold load ends and whose warm load does not: the cold row stands, measured and judged, and the warm
    // row says why. Drawn no sooner than 400 ms (`paint`), its cold row FAILS its first-content target every run, so
    // the stall is shown said beside a failing cold row, as verify r7's 1,628 ms paint made it in a full run.
    const later = await run('/later?paint=400', '1');
    const laterText = later.lines.join('\n');
    assert.ok(measuredRow(later, '/later?paint=400', 'cold', 1).length > 0, 'a first content drawn no sooner than 400 ms fails its 300 ms target');
    assert.ok(laterText.includes(`| /later?paint=400 | warm | 0 | NOT MEASURED: ${stalled} | — | — | — | FAILS |`), laterText);
    assert.deepEqual(later.failures.filter((f) => !f.startsWith('/later?paint=400 cold: ')), [`/later?paint=400 warm: ${stalled}`]);
    // A timed warm load that does not end, after the first warm load did: said the same way, whatever the cold row says.
    const second = await run('/later?after=2', '1');
    measuredRow(second, '/later?after=2', 'cold', 1);
    assert.deepEqual(second.failures.filter((f) => !f.startsWith('/later?after=2 cold: ')), [`/later?after=2 warm: ${stalled}`]);
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
    // A page under a spelling the router takes is measured, cold and warm; its figures against the targets are this machine's (repair 7).
    measuredRow(r, '/speed/', 'cold', 2);
    measuredRow(r, '/speed/', 'warm', 2);
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
    // The sheet is on stdout, whichever verdict this machine's figures give it (repair 7: it was pinned to PASS).
    assert.match(ran.stdout, /^# Browser harness — (PASS|FAIL)\n/, 'the sheet is on stdout');
    for (const f of ran.stdout.split('\n').filter((l) => l.startsWith('- FAILED: '))) assert.ok(timedJudgement(f.slice('- FAILED: '.length), '/speed'), `a FAIL only on how fast this machine drew /speed: ${f}`);
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

// ---------------------------------------------------------------------------
// Repair round 5 (24 Oct): verify r5's blocking finding — the speed sheet
// waited with no bound and printed nothing — and its cheap follow-ups, each
// failing on 23ec9bf0f by name.
// ---------------------------------------------------------------------------

/** @param {number} ms */
const nap = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/** Whether a process is still running, asked with signal 0. @param {number} pid */
const running = (pid) => {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
};

/** Waits, at most `ms`, for a process to end; whether it did. @param {number} pid @param {number} [ms] */
const endsWithin = async (pid, ms = 10_000) => {
  const end = Date.now() + ms;
  while (running(pid) && Date.now() < end) await nap(20);
  return !running(pid);
};

/** The pid a command wrote into `file`, once it is there. @param {string} file */
const pidIn = async (file) => {
  const end = Date.now() + 10_000;
  while (Date.now() < end) {
    try {
      const text = readFileSync(file, 'utf8').trim();
      if (/^\d+$/.test(text)) return Number(text);
    } catch {
      // not written yet
    }
    await nap(20);
  }
  throw new Error(`no pid was written into ${file}`);
};

/**
 * A real server for the sheet: every route answers as the strict stand-in
 * does, except `trickle`, whose answer starts and never ends — one byte every
 * 100 ms, so its socket is never idle long.
 *
 * @param {string} trickle
 */
async function tricklingServer(trickle) {
  const strict = standIn({ ignore: new Set() });
  const asks = { trickle: 0 };
  /** @type {import('node:http').ServerResponse[]} */
  const open = [];
  const server = http.createServer(async (req, res) => {
    if (new URL(String(req.url), 'http://x').pathname === trickle) {
      asks.trickle += 1;
      res.writeHead(200, { 'content-type': 'application/json', 'x-brutex-build': 'c'.repeat(40) });
      res.write('{');
      const t = setInterval(() => res.write(' '), 100);
      res.on('close', () => clearInterval(t));
      open.push(res);
      return;
    }
    const a = await strict.send('', String(req.url), { method: req.method });
    if (a.status === 0) return void res.socket?.destroy();
    res.writeHead(a.status, { 'content-type': 'application/json', ...(a.build ? { 'x-brutex-build': a.build } : {}) });
    res.end(a.body);
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', () => resolve(undefined)));
  const base = `http://127.0.0.1:${/** @type {import('node:net').AddressInfo} */ (server.address()).port}`;
  return {
    base,
    asks,
    stop: () => {
      for (const res of open) res.destroy();
      server.close();
    }
  };
}

/** @param {string[]} argv @param {number} [killAfterMs] */
const sheetCommand = (argv, killAfterMs = 60_000) => /** @type {Promise<{code: number | null, signal: string | null, stdout: string, stderr: string, ms: number}>} */ (new Promise((resolve) => {
  const t0 = Date.now();
  import('node:child_process').then(({ execFile }) => execFile(process.execPath, [new URL('ci/speed-sheet.mjs', web).pathname, ...argv], { timeout: killAfterMs }, (error, stdout, stderr) => resolve({ code: error ? (typeof error.code === 'number' ? error.code : null) : 0, signal: error?.signal ?? null, stdout, stderr, ms: Date.now() - t0 })));
}));

test('an answer that never ends is ended at --timeout-ms from its send and named; its row ends there and the request is asked no more, so the sheet ends in seconds with exit 1', { timeout: 120_000 }, async () => {
  // Before (verify r5, 23 Oct): timed() bounded a request only by its
  // socket's idle time, so an answer sending a byte every 500 ms was never
  // ended, and the sheet with --timeout-ms 2000 was still waiting after 90 s
  // with nothing printed but its first line.
  const site = await tricklingServer('/speed.json');
  try {
    // One request: the idle gaps (100 ms) are inside the bound; the whole answer is not.
    const t0 = Date.now();
    const a = /** @type {any} */ (await sheet.timed(site.base, '/speed.json', { timeoutMs: 400 }));
    const took = Date.now() - t0;
    assert.equal(a.status, 0);
    assert.equal(a.timedOut, true);
    assert.match(a.error, /^no whole answer within 400 ms \(its headers came, then \d+ bytes of its body\)$/);
    assert.equal(a.build, 'c'.repeat(40), 'the headers came and named their build: it is kept');
    assert.ok(took >= 390 && took < 5_000, `ended at its whole bound, not by idle time (took ${took} ms)`);
    // A whole run: n = 5, two client counts.
    site.asks.trickle = 0;
    const args = { ...sheet.parseArgs(['--base', site.base, '--n', '5', '--clients', '1,2', '--timeout-ms', '400']), feed: CTX.feed, timeframe: CTX.timeframe, month: CTX.month, day: CTX.day };
    /** @type {string[]} */
    const logged = [];
    const r0 = Date.now();
    const run = await sheet.runSheet(args, { log: (l) => logged.push(l) });
    const ran = Date.now() - r0;
    assert.ok(ran < 30_000, `the run ended (took ${ran} ms)`);
    assert.equal(site.asks.trickle, 1, 'the request that ran its whole bound was sent once, and no more');
    const cut = /** @type {any} */ (run.rows.find((x) => x.url === '/speed.json' && x.phase === 'cold'));
    assert.deepEqual([cut.n, cut.classes, cut.builds, cut.notAsked], [1, { 'no answer': 1 }, ['c'.repeat(40)], 4]);
    assert.ok(run.failures.some((f) => /^\/speed \/speed\.json \(cold, 1 clients\): 1 no answer — no whole answer within 400 ms \(its headers came, then \d+ bytes of its body\)$/.test(f)), run.failures.join('\n'));
    assert.ok(!run.failures.some((f) => f.includes('/speed.json') && f.includes('no named build')), 'a cut answer that named its build raises no second failure');
    assert.ok(run.notMeasured.some((l) => /^cold at 1 clients: GET \/speed\.json: 4 of its 5 requests were not sent, after one had no whole answer within 400 ms/.test(l)), run.notMeasured.join('\n'));
    for (const clients of [1, 2]) {
      assert.ok(run.notMeasured.includes(`warm at ${clients} clients: GET /speed.json was not asked: it had no whole answer within --timeout-ms 400 in the cold pass at 1 clients, and each ask would wait as long`), run.notMeasured.join('\n'));
    }
    assert.ok(run.rows.filter((x) => x.url !== '/speed.json').every((x) => x.classes.answered === x.n && x.n === 5), 'every other request was timed in full');
    assert.ok(logged.some((l) => /^speed sheet: GET \/speed\.json had no whole answer within 400 ms .* in the cold pass at 1 clients; it is asked no more this run$/.test(l)), logged.join('\n'));
    assert.ok(logged.includes('speed sheet: warm at 2 clients'), 'each pass is said as it starts');
    // As a command, the verifier's own case: the sheet on stdout, the request named, exit 1.
    const cmd = await sheetCommand(['--base', site.base, '--n', '1', '--clients', '1', '--feed', CTX.feed, '--timeframe', CTX.timeframe, '--month', CTX.month, '--day', CTX.day, '--timeout-ms', '2000']);
    assert.equal(cmd.code, 1, cmd.stderr);
    assert.ok(cmd.ms < 30_000, `the command ended (took ${cmd.ms} ms)`);
    assert.match(cmd.stdout, /^# Speed sheet — FAIL/);
    assert.match(cmd.stdout, /- FAILED: \/speed \/speed\.json \(cold, 1 clients\): 1 no answer — no whole answer within 2000 ms \(its headers came, then \d+ bytes of its body\)/);
    assert.match(cmd.stdout, /- NOT MEASURED: warm at 1 clients: GET \/speed\.json was not asked/);
  } finally {
    site.stop();
  }
});

test('a garbled ask the server fails on, gives no answer to or answers busy is said by name; one that runs its whole bound ends the check of its request; a request whose own ask runs out is asked no more this run', async () => {
  // Before (verify r5 follow-up, 23 Oct): a garbled field answered 5xx, GET
  // or HEAD, was reported neither as ignored nor as a failure.
  /** @type {string[]} */
  const asked = [];
  /** @param {string} error */
  const late = (error) => ({ status: 0, error, ms: 9, build: null, body: '', bytes: 0, headers: {}, timedOut: true });
  /** @param {string} path @param {{method?: string}} [o] */
  const send = async (path, o = {}) => {
    asked.push(`${o.method} ${path}`);
    const u = new URL(path, 'http://x');
    const g = (/** @type {string} */ k) => u.searchParams.get(k) === GARBLED;
    if (u.pathname === '/slow.json') return late('no whole answer within 9 ms (not even its headers came)');
    if (u.pathname === '/down.json') return answer(503);
    if (g('a')) return answer(500, 'boom');
    if (g('b')) return answer(429);
    if (g('c')) return late('no whole answer within 9 ms (its headers came, then 1 bytes of its body)');
    if (g('d')) return answer(400, '{"error":"d must be a day"}');
    return answer(200, o.method === 'HEAD' ? '' : '{"same":true}');
  };
  /** @param {string} url @param {string} [method] */
  const req = (url, method = 'GET') => ({ page: '/p', route: new URL(url, 'http://x').pathname, label: '', method, url, builder: 'b' });
  const x = '/x.json?a=1&b=1&c=1&d=1&e=1';
  const checked = /** @type {any} */ (await sheet.fieldCheck(send, [req(x), req('/h.json?a=1', 'HEAD'), req('/slow.json?a=1'), req('/down.json?a=1&b=1')]));
  assert.deepEqual(checked.failed, [
    `/p GET ${garbledUrl(x, 'a')}: the field a, garbled, was answered HTTP 500, so whether the server reads it is not known`,
    `/p GET ${garbledUrl(x, 'c')}: the field c, garbled, was answered no whole answer within 9 ms (its headers came, then 1 bytes of its body), so whether the server reads it is not known`,
    `/p HEAD ${garbledUrl('/h.json?a=1', 'a')}: the field a, garbled, was answered HTTP 500, so whether the server reads it is not known`
  ]);
  assert.deepEqual(checked.unchecked, [
    `the field b of GET ${x} (/p): its garbled value was answered busy (HTTP 429), which tells neither a read nor an ignore`,
    `the fields d, e of GET ${x} (/p): not asked garbled, after an earlier garbled ask of it had no whole answer within 9 ms (its headers came, then 1 bytes of its body)`
  ]);
  assert.deepEqual(checked.map((/** @type {any} */ i) => i.field), [], 'nothing it could not compare is called ignored');
  assert.deepEqual(checked.stalled.map((/** @type {any} */ s) => [s.request.url, s.said]), [['/slow.json?a=1', 'no whole answer within 9 ms (not even its headers came)']]);
  assert.deepEqual(asked.filter((a) => a.includes('/slow.json') || a.includes('/down.json')), ['GET /slow.json?a=1', 'GET /down.json?a=1&b=1'], 'no garbled ask of a request whose own ask ran out, or was not answered 2xx');
  assert.ok(!asked.some((a) => a.includes(garbledUrl(x, 'd')) || a.includes(garbledUrl(x, 'e'))), 'nor of a field after a garbled ask ran out');
  // In a run: a request whose field-check ask ran out fails by name and is timed in no pass.
  const strict = standIn({ ignore: new Set() });
  let dayflags = 0;
  /** @param {string} b @param {string} path @param {{method?: string}} [o] */
  const runSend = async (b, path, o = {}) => {
    if (new URL(path, 'http://x').pathname === '/dayflags.json') {
      dayflags += 1;
      return late('no whole answer within 60000 ms (not even its headers came)');
    }
    return strict.send(b, path, o);
  };
  const run = await sheet.runSheet(argsFor(), { send: runSend, log: () => {} });
  const url = String(dayflagsUrl(CTX.feed, CTX.instrument, CTX.from, CTX.to));
  assert.equal(dayflags, 1, 'asked once, in the field check, and no more');
  assert.ok(run.failures.includes(`/db GET ${url} (the field check): no whole answer within 60000 ms (not even its headers came)`), run.failures.join('\n'));
  for (const pass of ['cold at 1 clients', 'warm at 1 clients', 'warm at 2 clients']) {
    assert.ok(run.notMeasured.includes(`${pass}: GET ${url} was not asked: it had no whole answer within --timeout-ms 60000 in the field check, and each ask would wait as long`), pass);
  }
  assert.ok(!run.rows.some((r) => r.url === url));
  // A garbled ask the server fails on fails the run.
  const failing = await sheet.runSheet(argsFor(), { send: async (b, path, o = {}) => (path.includes(new URLSearchParams({ feed: GARBLED }).toString()) && path.startsWith('/census') ? answer(502) : strict.send(b, path, o)), log: () => {} });
  assert.equal(failing.ok, false);
  assert.ok(failing.failures.includes(`/db GET ${garbledUrl(String(sheet.requestsFor(CTX).requests.find((q) => q.route === '/census/facets.json')?.url), 'feed')}: the field feed, garbled, was answered HTTP 502, so whether the server reads it is not known`), failing.failures.join('\n'));
});

test('a --restart command that does not exit within --restart-wait-ms is ended with its whole process group, and the run fails by name and goes on to its sheet; a writer ends with its group; a sheet sent a signal leaves no command running', { timeout: 180_000 }, async () => {
  // Before (verify r5, 23 Oct): the restart pass waited for the command's
  // exit with no bound (--restart 'sleep 1000' printed nothing in 60 s), and
  // the writer was ended by a SIGTERM to its shell alone.
  const dir = mkdtempSync(join(tmpdir(), 'r315-commands-'));
  // What every command prints goes to the commands' log (repair 9).
  const logFd = sheet.openCommandLog(join(dir, 'commands.log'));
  // One run, bounded: a command that never exits, with a child of its own in the background.
  const bg = join(dir, 'bg.pid');
  const t0 = Date.now();
  const said = await sheet.restartWithin(`sleep 1000 & echo $! > '${bg}'; sleep 1000`, 300, logFd);
  const took = Date.now() - t0;
  assert.equal(said, 'did not exit within 300 ms (--restart-wait-ms), so its process group was sent SIGTERM and left');
  assert.ok(took >= 290 && took < 300 + sheet.COMMAND_STOP_MS, `ended at its bound (took ${took} ms)`);
  assert.ok(await endsWithin(await pidIn(bg)), 'its background child was ended with it: the whole group');
  // In time, a command is its exit code, or what ended it.
  assert.equal(await sheet.restartWithin('exit 3', 30_000, logFd), 3);
  assert.equal(await sheet.restartWithin('kill -TERM $$', 30_000, logFd), 'was ended by SIGTERM');
  // What a restart that exited in time started in the background (a server) is left running.
  const served = join(dir, 'served.pid');
  assert.equal(await sheet.restartWithin(`sleep 1000 & echo $! > '${served}'`, 30_000, logFd), 0);
  const server = await pidIn(served);
  try {
    assert.ok(running(server), 'a command that exited in time is not followed into its group');
  } finally {
    process.kill(server, 'SIGKILL');
  }
  // A command that ignores SIGTERM is killed past COMMAND_STOP_MS, with its group.
  assert.equal(sheet.COMMAND_STOP_MS, 5_000);
  const stubborn = sheet.run("trap '' TERM; sleep 1000", logFd);
  await nap(300);
  const t1 = Date.now();
  assert.equal(await stubborn.stop(), `was sent SIGTERM and, still running ${sheet.COMMAND_STOP_MS} ms later, SIGKILL`);
  assert.ok(Date.now() - t1 >= sheet.COMMAND_STOP_MS - 50);
  assert.equal(sheet.groupRunning(stubborn.child.pid), false, 'no process of its group is left');
  assert.equal(await stubborn.stop(), 'had already left', 'a second stop signals nothing');
  // A process of the group that outlives the command and ignores SIGTERM is waited for, bounded, then killed with the group.
  const orphan = join(dir, 'orphan.pid');
  const outlived = sheet.run(`(trap '' TERM; exec sleep 1000) & echo $! > '${orphan}'; sleep 1000`, logFd);
  const left = await pidIn(orphan);
  const t2 = Date.now();
  assert.equal(await outlived.stop(), `was sent SIGTERM and, still running ${sheet.COMMAND_STOP_MS} ms later, SIGKILL`);
  assert.ok(Date.now() - t2 >= sheet.COMMAND_STOP_MS - 50, 'its group was given the wait after the command itself left');
  assert.ok(await endsWithin(left), 'and the process that outlived it was killed with the group');
  // A command that exited with a process of its group still running: the stop ends that process too.
  const behind = join(dir, 'behind.pid');
  const quick = sheet.run(`sleep 1000 & echo $! > '${behind}'`, logFd);
  assert.equal(await quick.wait, 0);
  const straggler = await pidIn(behind);
  assert.ok(running(straggler));
  assert.equal(await quick.stop(), 'was sent SIGTERM and left');
  assert.ok(await endsWithin(straggler), 'what the command left behind ended with its group');
  // In a run: the restart pass ends by name, and the sheet is still made.
  const r = await sheet.runSheet({ ...argsFor(), restart: 'sleep 1000', restarts: 2, restartWaitMs: 300, commandLog: join(dir, 'commands.log') }, { send: standIn({ ignore: new Set() }).send, log: () => {} });
  assert.ok(r.failures.includes('restart 1: the --restart command did not exit within 300 ms (--restart-wait-ms), so its process group was sent SIGTERM and left'), r.failures.join('\n'));
  assert.ok(r.notMeasured.includes('first after restart: timed after 0 of the 2 restarts asked: the --restart command failed at restart 1'), r.notMeasured.join('\n'));
  assert.ok(!r.rows.some((x) => x.phase === 'first after restart'), 'no restart row for a pass that timed nothing');
  // A writer: its whole group ends with its pass, and one --clients count is one pass.
  const wbg = join(dir, 'writer-bg.pid');
  /** @type {string[]} */
  const logged = [];
  // Its pass is slowed (25 ms an answer) so the writer's shell has started its child before the pass ends.
  const w = await sheet.runSheet({ ...argsFor(), clients: [1], writer: `sleep 1000 & echo $! > '${wbg}'; sleep 1000`, commandLog: join(dir, 'commands.log') }, { send: standIn({ ignore: new Set(), delayMs: 25 }).send, log: (l) => logged.push(l) });
  assert.equal(w.rows.filter((x) => x.phase === 'warm+writer').length, w.rows.filter((x) => x.phase === 'cold').length, 'one --clients count, one warm+writer pass');
  assert.ok(await endsWithin(await pidIn(wbg)), 'the writer\'s background child ended with its group');
  assert.ok(logged.includes('speed sheet: the --writer command\'s process group was sent SIGTERM and left'), logged.join('\n'));
  // As a command, the verifier's own case: --restart 'sleep 1000', bounded by the up wait when no --restart-wait-ms is given.
  const site = await tricklingServer('/never-asked');
  try {
    const ctx = ['--feed', CTX.feed, '--timeframe', CTX.timeframe, '--month', CTX.month, '--day', CTX.day];
    const pid = join(dir, 'restart.pid');
    const cliLog = join(dir, 'cli-commands.log');
    const cmd = await sheetCommand(['--base', site.base, '--n', '1', '--clients', '1', ...ctx, '--timeout-ms', '2000', '--up-wait-ms', '2000', '--restart', `echo restart-said; echo $$ > '${pid}'; exec sleep 1000`, '--restarts', '1', '--command-log', cliLog]);
    assert.equal(cmd.code, 1, cmd.stderr);
    assert.ok(cmd.ms < 30_000, `the command ended (took ${cmd.ms} ms)`);
    assert.match(cmd.stdout, /- FAILED: restart 1: the --restart command did not exit within 2000 ms \(--restart-wait-ms\), so its process group was sent SIGTERM and left/);
    assert.ok(await endsWithin(await pidIn(pid)), 'nothing of the restart command is left');
    // Repair 9: what a command prints goes to the commands' log, which the sheet names; never to the sheet's
    // stdout, which is the sheet alone, nor to its stderr, which a command must not share.
    assert.ok(!cmd.stdout.includes('restart-said') && !cmd.stderr.includes('restart-said'), `${cmd.stdout}\n${cmd.stderr}`);
    assert.match(readFileSync(cliLog, 'utf8'), /^=== \d{4}-\d{2}-\d{2}T[^ ]+Z restart 1 of 1, the --restart command: echo restart-said; [^\n]*\nrestart-said\n$/);
    assert.ok(cmd.stdout.includes(`- what the --restart command printed is kept in ${cliLog}, each run under a line that names it`), cmd.stdout);
    assert.ok(cmd.stderr.includes(`speed sheet: what the commands print goes to ${cliLog}`), cmd.stderr);
    // A sheet sent SIGTERM while its --restart command runs ends that command's group too.
    const pid2 = join(dir, 'restart-2.pid');
    const { spawn } = await import('node:child_process');
    const child = spawn(process.execPath, [new URL('ci/speed-sheet.mjs', web).pathname, '--base', site.base, '--n', '1', '--clients', '1', ...ctx, '--restart', `echo $$ > '${pid2}'; exec sleep 1000`, '--restarts', '1', '--restart-wait-ms', '600000', '--command-log', cliLog], { stdio: 'ignore' });
    const sleeper = await pidIn(pid2);
    const exited = new Promise((resolve) => child.once('exit', (code, signal) => resolve({ code, signal })));
    child.kill('SIGTERM');
    assert.deepEqual(await exited, { code: null, signal: 'SIGTERM' }, 'the sheet still ends by the signal it was sent');
    assert.ok(await endsWithin(sleeper), 'and the command it was waiting on was ended with it');
  } finally {
    site.stop();
  }
});

test('the up wait after a restart is --up-wait-ms in all: each check within it waits at most the time left, and a server not up ends the restart pass by name; a first ask that runs out after a restart is asked no more', { timeout: 60_000 }, async () => {
  // Before: each up check was bounded by --timeout-ms (60 s by default), so
  // an up check that was never answered held the wait 60 s past its 400 ms,
  // and a server not up was asked every request all the same.
  // Each restart the stand-in's own start changes (repair 9: the sheet proves each restart by it).
  let starts = 0;
  const strict = standIn({ ignore: new Set(), started: () => starts });
  let restarted = false;
  /** @type {number[]} */
  const limits = [];
  /** @param {string} b @param {string} path @param {{method?: string, timeoutMs?: number}} [o] */
  const send = async (b, path, o = {}) => {
    if (restarted && new URL(path, 'http://x').pathname === UP_ROUTE) {
      const ms = Number(o.timeoutMs);
      limits.push(ms);
      await nap(ms);
      return { status: 0, error: `no whole answer within ${ms} ms (not even its headers came)`, ms, build: null, body: '', bytes: 0, headers: {}, timedOut: true };
    }
    return strict.send(b, path, o);
  };
  const t0 = Date.now();
  const r = await sheet.runSheet({ ...argsFor(), timeoutMs: 60_000, upWaitMs: 400, restart: 'x', restarts: 3 }, { send, log: () => {}, restart: async () => ((restarted = true), 0) });
  const took = Date.now() - t0;
  assert.ok(limits.length >= 1 && limits.every((l) => l >= 1 && l <= 400), `each up check waited at most the time left: ${limits.join(', ')}`);
  assert.ok(took < 15_000, `the run ended (took ${took} ms)`);
  assert.ok(r.failures.includes('restart 1: the server was not up 400 ms after the restart'), r.failures.join('\n'));
  assert.ok(r.notMeasured.includes('first after restart: timed after 0 of the 3 restarts asked: the server was not up after restart 1'), r.notMeasured.join('\n'));
  assert.ok(!r.failures.some((f) => f.startsWith('restart 2')), 'restarts 2 and 3 were not run');
  // A first ask that runs its whole bound after a restart is asked no more after the next ones.
  let after = false;
  let aliases = 0;
  /** @param {string} b @param {string} path @param {{method?: string}} [o] */
  const send2 = async (b, path, o = {}) => {
    if (after && new URL(path, 'http://x').pathname === '/aliases.json') {
      aliases += 1;
      return { status: 0, error: 'no whole answer within 60000 ms (its headers came, then 0 bytes of its body)', ms: 60_000, build: 'c'.repeat(40), body: '', bytes: 0, headers: {}, timedOut: true };
    }
    return strict.send(b, path, o);
  };
  const r2 = await sheet.runSheet({ ...argsFor(), restart: 'x', restarts: 3 }, { send: send2, log: () => {}, restart: async () => ((after = true), (starts += 1), 0) });
  assert.equal(aliases, 1, 'asked after the first restart, and no more');
  assert.ok(r2.failures.includes('restart 1: the first /aliases.json answered no whole answer within 60000 ms (its headers came, then 0 bytes of its body)'), r2.failures.join('\n'));
  assert.ok(r2.notMeasured.includes('first after restart: GET /aliases.json was asked after 1 of the 3 restarts run: it had no whole answer within --timeout-ms 60000 in the first ask after restart 1, and is asked no more'), r2.notMeasured.join('\n'));
  assert.equal(r2.rows.find((x) => x.url === '/aliases.json' && x.phase === 'first after restart')?.n, 1);
  assert.equal(r2.rows.find((x) => x.url === '/feeds.json' && x.phase === 'first after restart')?.n, 3, 'every other request is asked after every restart');
});

test('the sheet\'s and the harness\'s command lines: --restart-wait-ms is the up wait unless given and has a ceiling, and an output that names the input it replaces is refused', () => {
  const base = ['--base', 'http://h:1'];
  assert.equal(sheet.parseArgs(base).restartWaitMs, 120_000, 'the up wait\'s default');
  assert.equal(sheet.parseArgs([...base, '--up-wait-ms', '2000']).restartWaitMs, 2_000, 'the up wait given');
  assert.equal(sheet.parseArgs([...base, '--up-wait-ms', '2000', '--restart-wait-ms', '9000']).restartWaitMs, 9_000);
  assert.throws(() => sheet.parseArgs([...base, '--restart-wait-ms', '0']), /^Error: --restart-wait-ms must be a whole number of at least 1, in decimal digits$/);
  assert.throws(() => sheet.parseArgs([...base, '--restart-wait-ms', String(sheet.MAX_WAIT_MS + 1)]), /^Error: --restart-wait-ms must be at most 2147483647$/);
  assert.throws(() => sheet.parseArgs([...base, '--restart-wait-ms', '0x10']), /in decimal digits/);
  // Before (verify r5 follow-up): --out over the capture it read, and the harness's --out over its own --capture, were taken.
  assert.throws(() => sheet.parseArgs([...base, '--from-pages', 'cap.json', '--out', './cap.json']), /^Error: --out and --from-pages name one file \(\.\/cap\.json\): the sheet would be written over the capture it reads$/);
  assert.equal(sheet.parseArgs([...base, '--from-pages', 'cap.json', '--out', 'sheet.md']).out, 'sheet.md');
  assert.throws(() => harness.parseArgs([...base, '--capture', 'c.json', '--out', `${process.cwd()}/c.json`]), /^Error: --out and --capture name one file \(.*c\.json\): the sheet would be written over the capture$/);
  assert.equal(harness.parseArgs([...base, '--capture', 'c.json', '--out', 'o.md']).out, 'o.md');
});

test('a browser that prints its DevTools address and never finishes the WebSocket handshake is refused within the launch\'s wait, and stopped', { timeout: 60_000 }, async () => {
  // Before (verify r5 follow-up): the handshake had no timer of its own, so
  // such a browser held the run for Node's own 300 s.
  const net = await import('node:net');
  /** @type {import('node:net').Socket[]} */
  const sockets = [];
  const silent = net.createServer((s) => sockets.push(s));
  await new Promise((resolve) => silent.listen(0, '127.0.0.1', () => resolve(undefined)));
  const port = /** @type {import('node:net').AddressInfo} */ (silent.address()).port;
  const fake = mkdtempSync(join(tmpdir(), 'r315-mute-'));
  const mute = join(fake, 'mute');
  writeFileSync(mute, `#!/bin/sh\necho "DevTools listening on ws://127.0.0.1:${port}/devtools/browser/x" >&2\nexec sleep 60\n`, { mode: 0o755 });
  const root = mkdtempSync(join(tmpdir(), 'r315-mute-root-'));
  try {
    const t = Date.now();
    await assert.rejects(harness.launch(mute, false, 1_000, root), new RegExp(`^Error: could not connect to ws://127\\.0\\.0\\.1:${port}/devtools/browser/x: no handshake within 1000 ms$`));
    const took = Date.now() - t;
    assert.ok(took >= 990 && took < 1_000 + 2 * harness.STOP_WAIT_MS, `refused at its wait (took ${took} ms)`);
    assert.ok(sockets.length >= 1, 'it did connect, and was never answered');
    assert.deepEqual(readdirSync(root), [], 'the browser was stopped and its profile removed');
  } finally {
    for (const s of sockets) s.destroy();
    silent.close();
  }
});

test('a capture is read only as a regular file a capture could be: a named pipe nobody writes, a directory and a file past MAX_TEXT_BYTES are refused by name before the server is asked', { timeout: 60_000 }, async () => {
  // Before: --from-pages was read with readFileSync, so a named pipe with no
  // writer blocked the whole process for good, a wait with no bound.
  const { spawnSync } = await import('node:child_process');
  const { truncateSync } = await import('node:fs');
  const dir = mkdtempSync(join(tmpdir(), 'r315-pipe-'));
  const pipe = join(dir, 'capture.json');
  assert.equal(spawnSync('mkfifo', [pipe]).status, 0, 'mkfifo made the pipe');
  const cmd = await sheetCommand(['--base', 'http://127.0.0.1:1', '--from-pages', pipe], 30_000);
  assert.equal(cmd.signal, null, 'the command ended by itself, not by its timeout');
  assert.equal(cmd.code, 1, cmd.stderr);
  assert.ok(cmd.ms < 20_000, `it ended at once (took ${cmd.ms} ms)`);
  assert.match(cmd.stdout, /- FAILED: the capture of the pages' own requests could not be read: --from-pages .*capture\.json: not a regular file, and only a regular file is read/);
  let calls = 0;
  const send = async () => ((calls += 1), answer(200));
  const asDir = await sheet.runSheet({ ...argsFor(), fromPages: dir }, { send, log: () => {} });
  assert.match(asDir.failures[0], /: not a regular file, and only a regular file is read$/);
  const big = join(dir, 'big.json');
  writeFileSync(big, '');
  truncateSync(big, sheet.MAX_TEXT_BYTES + 1);
  const tooBig = await sheet.runSheet({ ...argsFor(), fromPages: big }, { send, log: () => {} });
  assert.match(tooBig.failures[0], new RegExp(`: ${sheet.MAX_TEXT_BYTES + 1} bytes, past the ${sheet.MAX_TEXT_BYTES} a capture may be$`));
  assert.equal(calls, 0, 'the server was asked nothing');
  const ok = join(dir, 'ok.json');
  writeFileSync(ok, JSON.stringify({ requests: [] }));
  assert.equal(sheet.readCapturedFile(ok), '{"requests":[]}', 'a regular file is read whole');
});

// ---------------------------------------------------------------------------
// Repair round 6 (24 Oct): verify r6's two blocking findings — the restart
// rows named the run's build, never their own, and an answer was held whole
// in memory with no bound in bytes — and its cheap follow-ups, each failing
// on 2db88da6d by name.
// ---------------------------------------------------------------------------

/**
 * The strict stand-in with each answer's build chosen by its route and by
 * how many restarts have run; its `restart` counts one and exits 0, and its
 * own start (`started_ms`) is that count, so each restart is one.
 *
 * @param {(path: string, restarts: number) => string | null} buildOf
 */
function switching(buildOf) {
  const state = { restarts: 0 };
  const strict = standIn({ ignore: new Set(), started: () => state.restarts });
  return {
    state,
    /** @param {string} b @param {string} path @param {{method?: string}} [o] */
    send: async (b, path, o = {}) => ({ ...(await strict.send(b, path, o)), build: buildOf(new URL(path, 'http://x').pathname, state.restarts) }),
    restart: async () => ((state.restarts += 1), 0)
  };
}

test('a restart that brings the server back as another build fails by name: the up check names the build it found, each restart row is stamped with the builds its own answers named and their largest bytes, and one row answered by two builds fails', async () => {
  // Before (verify r6, 24 Oct): every 'first after restart' row was stamped
  // with the run's commit and 0 bytes, and the up check after a restart read
  // only the status, so a restart that brought back another build passed.
  const A = 'a'.repeat(40);
  const B = 'b'.repeat(40);
  // (1) The verifier's case: after the restart everything answers B.
  const back = switching((_p, n) => (n === 0 ? A : B));
  /** @type {string[]} */
  const logged = [];
  const r1 = await sheet.runSheet({ ...argsFor(), restart: 'x', restarts: 2 }, { send: back.send, log: (l) => logged.push(l), restart: back.restart });
  assert.equal(r1.ok, false);
  assert.ok(r1.failures.includes(`restart 1: the server came back as ${B}, not ${A} (its HEAD /inspection.json said so)`), r1.failures.join('\n'));
  assert.ok(r1.notMeasured.includes("first after restart: timed after 0 of the 2 restarts asked: the server came back as another build than the run's at restart 1"), r1.notMeasured.join('\n'));
  assert.ok(!r1.rows.some((x) => x.phase === 'first after restart'), 'no row is stamped for a build the run did not measure');
  assert.equal(back.state.restarts, 1, 'the pass ended at the restart that came back as another build');
  assert.ok(logged.some((l) => new RegExp(`^speed sheet: restart 1: up \\d+ ms after the command, answered by ${B}$`).test(l)), logged.join('\n'));
  // (2) The up check answers the run's build and the requests another: each answer named, each row its own answers'.
  const split = switching((p, n) => (n === 0 || p === UP_ROUTE ? A : B));
  const r2 = await sheet.runSheet({ ...argsFor(), restart: 'x', restarts: 3 }, { send: split.send, log: () => {}, restart: split.restart });
  const after = r2.rows.filter((x) => x.phase === 'first after restart');
  assert.equal(after.length, sheet.requestsFor(r2.ctx ?? CTX).requests.length);
  const feeds = after.find((x) => x.url === '/feeds.json');
  assert.deepEqual(feeds?.builds, [B], 'the row is stamped with the build its answers named');
  assert.equal(feeds?.bytes, (await standIn({ ignore: new Set() }).send('', '/feeds.json')).bytes, "and its bytes are its largest answer's");
  assert.deepEqual(after.find((x) => x.url === UP_ROUTE)?.builds, [A]);
  assert.ok(r2.failures.includes(`restart 1: GET /feeds.json was answered by ${B}, not ${A}`), r2.failures.join('\n'));
  assert.ok(r2.notMeasured.includes("first after restart: timed after 1 of the 3 restarts asked: an answer after restart 1 came from another build than the run's"), r2.notMeasured.join('\n'));
  assert.equal(split.state.restarts, 1);
  assert.match(sheet.renderSheet(r2, argsFor()), new RegExp(`^\\| ${'b'.repeat(12)} \\| \\(every page\\) \\| \`/feeds\\.json\` \\| \\(no query\\) \\| first after restart \\| 1 \\| 1 \\|`, 'm'));
  // (3) One row answered by two builds: restart 1 all A, restart 2's /aliases.json B.
  const late = switching((p, n) => (n === 2 && p === '/aliases.json' ? B : A));
  const r3 = await sheet.runSheet({ ...argsFor(), restart: 'x', restarts: 2 }, { send: late.send, log: () => {}, restart: late.restart });
  const aliases = r3.rows.find((x) => x.url === '/aliases.json' && x.phase === 'first after restart');
  assert.deepEqual(aliases?.builds, [A, B]);
  assert.ok(r3.failures.includes(`/aliases.json (first after restart, 1 clients) was answered by 2 builds: ${A}, ${B}`), r3.failures.join('\n'));
  assert.ok(r3.failures.includes(`restart 2: GET /aliases.json was answered by ${B}, not ${A}`), r3.failures.join('\n'));
  assert.match(sheet.renderSheet(r3, argsFor()), /^\| MIXED \| \/db \| `\/aliases\.json` \| \(no query\) \| first after restart \|/m);
  // (4) An answer that names no build after a restart, the run stamped: named too.
  const bare = switching((p, n) => (n > 0 && p === '/aliases.json' ? null : A));
  const r4 = await sheet.runSheet({ ...argsFor(), restart: 'x', restarts: 1 }, { send: bare.send, log: () => {}, restart: bare.restart });
  assert.ok(r4.failures.includes(`restart 1: GET /aliases.json was answered by no named build, not ${A}`), r4.failures.join('\n'));
  assert.deepEqual(r4.rows.find((x) => x.url === '/aliases.json' && x.phase === 'first after restart')?.unnamed, 1);
  // (5) A clean pass: every restart row its own answers', all A, and nothing failed.
  const same = switching(() => A);
  const r5 = await sheet.runSheet({ ...argsFor(), restart: 'x', restarts: 2 }, { send: same.send, log: () => {}, restart: same.restart });
  assert.equal(r5.ok, true, r5.failures.join('\n'));
  const clean = r5.rows.filter((x) => x.phase === 'first after restart');
  assert.ok(clean.length > 0 && clean.every((x) => x.builds.length === 1 && x.builds[0] === A && x.unnamed === 0 && x.n === 2 && (x.url === UP_ROUTE || x.bytes > 0)), JSON.stringify(clean.map((x) => [x.url, x.builds, x.bytes])));
});

test('a row whose answers named no build beside answers that named one is answered by two builds, and a run whose server named none fails a row another build answered', async () => {
  // Before (verify r6): an answer with no build header was dropped from a row's builds, so a server swapped for one that stamps nothing passed.
  let k = 0;
  const flip = await sheet.runSheet(argsFor(), { send: standIn({ ignore: new Set(), build: () => ((k += 1) % 3 === 0 ? null : 'c'.repeat(40)) }).send, log: () => {} });
  assert.ok(flip.failures.some((f) => /^\/\S+ \(cold, 1 clients\) was answered by 2 builds: c{40}, no named build \(\d+ of its answers\)$/.test(f)), flip.failures.join('\n'));
  assert.match(sheet.renderSheet(flip, argsFor()), /^\| MIXED \|/m);
  // The up check names no build (allowed), and the rows' answers name one: another build than the run's.
  let first = true;
  const later = await sheet.runSheet({ ...argsFor(), allowUnstamped: true }, { send: standIn({ ignore: new Set(), build: () => (first ? ((first = false), null) : 'c'.repeat(40)) }).send, log: () => {} });
  assert.ok(later.failures.some((f) => /^\/\S+ \(cold\) was answered by c{40}, not the run's, whose server named no build$/.test(f)), later.failures.join('\n'));
  assert.equal(sheet.otherBuild('x', 'x'), null);
  assert.equal(sheet.otherBuild(null, 'x'), 'no named build, not x');
});

test('as a command over the network: a --restart that brings the server back as another build ends the restart pass by name and exits 1, never PASS', { timeout: 60_000 }, async () => {
  // The verifier's stand-in run, in one process: the --restart command writes another build for the server to stamp.
  const dir = mkdtempSync(join(tmpdir(), 'r315-switch-'));
  const buildFile = join(dir, 'build');
  writeFileSync(buildFile, 'c'.repeat(40));
  const strict = standIn({ ignore: new Set() });
  const server = http.createServer(async (req, res) => {
    const a = await strict.send('', String(req.url), { method: req.method });
    if (a.status === 0) return void res.socket?.destroy();
    res.writeHead(a.status, { 'content-type': 'application/json', 'x-brutex-build': readFileSync(buildFile, 'utf8').trim() });
    res.end(req.method === 'HEAD' ? undefined : a.body);
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', () => resolve(undefined)));
  const base = `http://127.0.0.1:${/** @type {import('node:net').AddressInfo} */ (server.address()).port}`;
  try {
    const ctx = ['--feed', CTX.feed, '--timeframe', CTX.timeframe, '--month', CTX.month, '--day', CTX.day];
    const cmd = await sheetCommand(['--base', base, '--n', '1', '--clients', '1', ...ctx, '--timeout-ms', '5000', '--up-wait-ms', '5000', '--restart', `printf %s ${'d'.repeat(40)} > '${buildFile}'`, '--restarts', '2', '--command-log', join(dir, 'commands.log')]);
    assert.equal(cmd.code, 1, cmd.stderr);
    assert.match(cmd.stdout, /^# Speed sheet — FAIL/);
    assert.ok(cmd.stdout.includes(`- FAILED: restart 1: the server came back as ${'d'.repeat(40)}, not ${'c'.repeat(40)} (its HEAD /inspection.json said so)`), cmd.stdout);
    assert.doesNotMatch(cmd.stdout, /\| first after restart \|/, 'no restart row is printed for a build the run did not measure');
    assert.match(cmd.stderr, new RegExp(`speed sheet: restart 1: up \\d+ ms after the command, answered by ${'d'.repeat(40)}`));
  } finally {
    server.close();
  }
});

/**
 * A server whose answers are `size` bytes of `a`, sent in 1 MiB writes as
 * fast as the socket drains; `sized(path)` names the size of a path's answer,
 * or `null` to answer as the strict stand-in does.
 *
 * @param {(path: string, query: URLSearchParams) => number | null} sized
 */
async function sizedServer(sized) {
  const strict = standIn({ ignore: new Set() });
  const chunk = Buffer.alloc(1 << 20, 0x61);
  const server = http.createServer(async (req, res) => {
    const u = new URL(String(req.url), 'http://x');
    const size = [...u.searchParams.values()].includes(GARBLED) ? null : sized(u.pathname, u.searchParams);
    if (size === null) {
      const a = await strict.send('', String(req.url), { method: req.method });
      res.writeHead(a.status, { 'content-type': 'application/json', 'x-brutex-build': 'c'.repeat(40) });
      return void res.end(req.method === 'HEAD' ? undefined : a.body);
    }
    res.writeHead(200, { 'content-type': 'application/json', 'x-brutex-build': 'c'.repeat(40) });
    let left = size;
    let open = true;
    res.on('close', () => (open = false));
    const pump = () => {
      while (open && left > 0) {
        const n = Math.min(left, chunk.length);
        left -= n;
        if (!res.write(n === chunk.length ? chunk : chunk.subarray(0, n))) return void res.once('drain', pump);
      }
      if (open) res.end();
    };
    pump();
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', () => resolve(undefined)));
  return { base: `http://127.0.0.1:${/** @type {import('node:net').AddressInfo} */ (server.address()).port}`, close: () => server.close() };
}

test('an answer is held to a bound in bytes as well as in time: a timed row keeps its first HEAD_BYTES, discovery and the field check at most MAX_TEXT_BYTES, past which the bytes are hashed as they come; a run against answers far past every bound stays under its stated memory bound', { timeout: 300_000 }, async () => {
  // Before (verify r6, 24 Oct): timed() kept every chunk until the answer
  // ended, so an answer that streamed fast grew the sheet to 4.78 GB within
  // a 5 s --timeout-ms, and Buffer.concat then made a second copy.
  assert.equal(sheet.HEAD_BYTES, 4_096);
  const site = await sizedServer((_p, q) => (q.has('size') ? Number(q.get('size')) : null));
  try {
    const exact = /** @type {any} */ (await sheet.timed(site.base, `/x?size=${sheet.MAX_TEXT_BYTES}`, { timeoutMs: 120_000 }));
    assert.deepEqual([exact.status, exact.bytes, exact.body.length, exact.kept], [200, sheet.MAX_TEXT_BYTES, sheet.MAX_TEXT_BYTES, 'text'], 'exactly the bound is still text');
    const past = /** @type {any} */ (await sheet.timed(site.base, `/x?size=${sheet.MAX_TEXT_BYTES + 1}`, { timeoutMs: 120_000 }));
    assert.deepEqual([past.status, past.bytes, past.kept], [200, sheet.MAX_TEXT_BYTES + 1, 'digest']);
    assert.equal(past.body, sheet.answerText(Buffer.alloc(sheet.MAX_TEXT_BYTES + 1, 0x61)), 'one byte past it: the digest of the very bytes, hashed as they came, as answerText hashes them in hand');
    const head = /** @type {any} */ (await sheet.timed(site.base, '/x?size=1048576', { keep: 'head', timeoutMs: 60_000 }));
    assert.deepEqual([head.status, head.bytes, head.body, head.kept], [200, 1_048_576, 'a'.repeat(sheet.HEAD_BYTES), 'head'], 'a timed row keeps the start and counts every byte');
    const small = /** @type {any} */ (await sheet.timed(site.base, '/x?size=10', { keep: 'head' }));
    assert.deepEqual([small.body, small.kept], ['a'.repeat(10), 'text']);
  } finally {
    site.close();
  }
  // Who keeps what: every timed ask (a row's, each after a restart) keeps the head; discovery, the field check and the up checks keep the whole.
  /** @type {string[]} */
  const kept = [];
  let starts = 0;
  const strict = standIn({ ignore: new Set(), started: () => starts });
  /** @param {string} b @param {string} path @param {{method?: string, keep?: string}} [o] */
  const spy = async (b, path, o = {}) => {
    const u = new URL(path, 'http://x');
    kept.push(`${o.keep ?? 'whole'} ${o.method ?? 'GET'} ${u.pathname}${[...u.searchParams.values()].includes(GARBLED) ? ' garbled' : ''}`);
    return strict.send(b, path, o);
  };
  const spied = await sheet.runSheet({ ...argsFor(), n: 2, clients: [1], restart: 'x', restarts: 1 }, { send: spy, log: () => {}, restart: async () => ((starts += 1), 0) });
  assert.equal(spied.ok, true, spied.failures.join('\n'));
  const { requests } = sheet.requestsFor(spied.ctx ?? CTX);
  const withQuery = requests.filter((r) => r.url.includes('?'));
  const fields = withQuery.reduce((n, r) => n + new Set(new URL(r.url, 'http://x').searchParams.keys()).size, 0);
  // Repair 9: after the restart the up check's answer and the read that proved the restart are their routes' first asks.
  assert.equal(kept.filter((k) => k.startsWith('head ')).length, requests.length * 5 - 2, 'n = 2 cold, n = 2 warm and one after the restart, each keeping the head, but for the two asks the restart needed');
  assert.equal(kept.filter((k) => k === `whole ${UP_METHOD} ${UP_ROUTE}`).length, 2, 'the up check at the start and after the restart');
  assert.equal(kept.filter((k) => k === `whole GET ${SPEED_ROUTE}`).length, 2, 'the server\'s start, read before the restart and after it, whole');
  assert.equal(kept.filter((k) => k.startsWith('whole GET ') && !k.endsWith(' garbled') && k !== `whole GET ${SPEED_ROUTE}`).length, withQuery.length, 'the field check keeps the whole of each original');
  assert.equal(kept.filter((k) => k.startsWith('whole ') && k.endsWith(' garbled')).length, fields, 'and of each garbled answer');
  assert.equal(kept.length, requests.length * 5 + 2 + withQuery.length + fields, 'and nothing else is asked');
  // A discovery answer past the text bound, or not JSON, is refused by name.
  const args = sheet.parseArgs(['--base', 'http://h:1']);
  await assert.rejects(sheet.discover(async () => ({ ...answer(200, '(an answer of 9 bytes, sha256 x)'), bytes: 9, kept: 'digest' }), args), /^Error: discovering the selection: \/feeds\.json answered 9 bytes, past the 64000000 the sheet keeps as text$/);
  await assert.rejects(sheet.discover(async () => answer(200, 'not json'), args), /^Error: discovering the selection: \/feeds\.json answered HTTP 200 with a body that is not JSON \(/);
  // A whole run, in a process of its own, against answers of 1.2 GB on a timed route and on a field-checked one.
  const FLOOD = 1_200_000_000;
  const flood = await sizedServer((p) => (p === '/speed.json' || p === '/dayflags.json' ? FLOOD : null));
  try {
    const code = [
      `const sheet = await import(${JSON.stringify(new URL('ci/speed-sheet.mjs', web).href)});`,
      `const args = { ...sheet.parseArgs(['--base', ${JSON.stringify(flood.base)}, '--n', '1', '--clients', '1', '--timeout-ms', '120000']), feed: 'zerodha', timeframe: '1min', month: '2026-09', day: '2026-09-30' };`,
      'const r = await sheet.runSheet(args, { log: () => {} });',
      "process.stdout.write(JSON.stringify({ peak: process.resourceUsage().maxRSS * 1024, ok: r.ok, failures: r.failures, rows: r.rows.map((x) => ({ url: x.url, phase: x.phase, bytes: x.bytes })) }));"
    ].join('\n');
    const { execFile } = await import('node:child_process');
    const out = await new Promise((resolve, reject) => execFile(process.execPath, ['--input-type=module', '-e', code], { maxBuffer: 1 << 24, timeout: 240_000 }, (error, stdout, stderr) => (error ? reject(new Error(`${error.message}\n${stderr}`)) : resolve(JSON.parse(stdout)))));
    const ran = /** @type {{peak: number, ok: boolean, failures: string[], rows: {url: string, phase: string, bytes: number}[]}} */ (out);
    assert.equal(ran.ok, true, ran.failures.join('\n'));
    const speed = ran.rows.find((x) => x.url === '/speed.json' && x.phase === 'cold');
    const dayflags = ran.rows.find((x) => x.url.startsWith('/dayflags.json') && x.phase === 'cold');
    assert.deepEqual([speed?.bytes, dayflags?.bytes], [FLOOD, FLOOD], 'every byte of each answer was counted');
    // The stated bound (docs/06-limits.md L-R315-26): under 1,000,000,000 B at its peak, every answer far past it.
    assert.ok(ran.peak < 1_000_000_000, `the sheet's peak memory was ${ran.peak} B while it read answers of ${FLOOD} B`);
  } finally {
    flood.close();
  }
});

test('the sheet\'s follow-ups: one file by its device and inode, a day and a month the calendar has, --out checked before the run, a sheet past 64 KB reaches a pipe whole, and a writer killed is said in the sheet', { timeout: 120_000 }, async () => {
  const { symlinkSync, realpathSync } = await import('node:fs');
  const base = ['--base', 'http://h:1'];
  // Two spellings of one file (verify r6 follow-up): a link to it, and the real path of a linked directory.
  const dir = mkdtempSync(join(tmpdir(), 'r315-same-'));
  const cap = join(dir, 'cap.json');
  writeFileSync(cap, '{"requests":[]}');
  symlinkSync(cap, join(dir, 'link.json'));
  assert.ok(sheet.sameFile(cap, join(dir, 'link.json')));
  assert.throws(() => sheet.parseArgs([...base, '--from-pages', cap, '--out', join(dir, 'link.json')]), /^Error: --out and --from-pages name one file/);
  assert.throws(() => harness.parseArgs([...base, '--capture', cap, '--out', join(dir, 'link.json')]), /^Error: --out and --capture name one file/);
  const real = realpathSync(cap);
  if (real !== cap) assert.throws(() => sheet.parseArgs([...base, '--from-pages', cap, '--out', real]), /name one file/, 'a linked directory and its real path are one');
  const upper = join(dir, 'CAP.json');
  if (existsSync(upper)) assert.throws(() => sheet.parseArgs([...base, '--from-pages', cap, '--out', upper]), /name one file/, 'on a volume that folds case, another case is the same file');
  assert.equal(sheet.sameFile(cap, join(dir, 'other.json')), false, 'a path that names nothing yet is not the file');
  assert.equal(sheet.parseArgs([...base, '--from-pages', cap, '--out', join(dir, 'sheet.md')]).out, join(dir, 'sheet.md'));
  // A day and a month the calendar has (verify r6 follow-up: 2024-13 and 2024-02-30 were taken).
  assert.equal(sheet.parseArgs([...base, '--day', '2024-02-29', '--month', '2024-02']).day, '2024-02-29');
  // Repair 7 (verify r7 follow-up): this line took `--month 2024-12` with that day; a day must be one of its month's.
  assert.throws(() => sheet.parseArgs([...base, '--day', '2024-02-29', '--month', '2024-12']), /^Error: --day 2024-02-29 is not a day of --month 2024-12$/);
  // Repair 9 (verify r9 follow-up): year 0000 is refused here, as the server refuses it, never asked.
  for (const [flag, v] of [['--day', '2024-02-30'], ['--day', '2023-02-29'], ['--day', '2024-13-01'], ['--day', '2024-00-10'], ['--day', '2024-04-31'], ['--month', '2024-13'], ['--month', '2024-00'], ['--month', '0000-01'], ['--day', '0000-12-31']]) {
    assert.throws(() => sheet.parseArgs([...base, flag, v]), flag === '--day' ? /^Error: --day is YYYY-MM-DD, a day the calendar has$/ : /^Error: --month is YYYY-MM, a month the calendar has$/, `${flag} ${v}`);
  }
  assert.equal(sheet.calendarHas('0020-01-31'), true, 'a year is read as written, never as 1920');
  assert.equal(sheet.calendarHas('0001-01-01'), true, 'the first day of the common era');
  assert.equal(sheet.parseArgs([...base, '--month', '0001-01']).month, '0001-01');
  // --out is checked before the server is asked anything (verify r6 follow-up: only at the end).
  let asked = 0;
  const quiet = http.createServer((_req, res) => {
    asked += 1;
    res.end();
  });
  await new Promise((resolve) => quiet.listen(0, '127.0.0.1', () => resolve(undefined)));
  try {
    const qbase = `http://127.0.0.1:${/** @type {import('node:net').AddressInfo} */ (quiet.address()).port}`;
    const refused = await sheetCommand(['--base', qbase, '--out', join(dir, 'missing', 'sheet.md')]);
    assert.equal(refused.code, 2, refused.stderr);
    assert.match(refused.stderr, /^speed-sheet: --out .*\/missing\/sheet\.md: .*\/missing cannot take a new file: ENOENT/);
    assert.equal(refused.stdout, '');
    assert.equal(asked, 0, 'the server was asked nothing');
  } finally {
    quiet.close();
  }
  // A sheet past 64 KB reaches a pipe whole (verify r6 follow-up: 73,728 of 101,986 B, the FAILED lines lost).
  const many = { requests: Array.from({ length: 300 }, (_, i) => ({ page: '/gaps', method: 'GET', url: `/missing-${i}.json` })) };
  const capture = join(dir, 'many.json');
  writeFileSync(capture, JSON.stringify(many));
  const strict = standIn({ ignore: new Set(), status: Object.fromEntries(many.requests.map((r) => [r.url, 404])) });
  const site = http.createServer(async (req, res) => {
    const a = await strict.send('', String(req.url), { method: req.method });
    res.writeHead(a.status, { 'content-type': 'application/json', 'x-brutex-build': 'c'.repeat(40) });
    res.end(req.method === 'HEAD' ? undefined : a.body);
  });
  await new Promise((resolve) => site.listen(0, '127.0.0.1', () => resolve(undefined)));
  try {
    const sbase = `http://127.0.0.1:${/** @type {import('node:net').AddressInfo} */ (site.address()).port}`;
    const out = join(dir, 'many.md');
    const big = await sheetCommand(['--base', sbase, '--n', '1', '--clients', '1', '--feed', CTX.feed, '--timeframe', CTX.timeframe, '--month', CTX.month, '--day', CTX.day, '--from-pages', capture, '--out', out]);
    assert.equal(big.code, 1, big.stderr);
    const file = readFileSync(out, 'utf8');
    assert.ok(file.length > 65_536, `the sheet is past 64 KB (${file.length} B)`);
    assert.equal(big.stdout.length, file.length, 'stdout took the whole sheet');
    assert.equal(big.stdout, file);
    assert.match(big.stdout, /- FAILED: \/gaps \/missing-299\.json \(warm, 1 clients\): 1 refused — HTTP 404/);
  } finally {
    site.close();
  }
  assert.equal(sheet.STDOUT_WAIT_MS, 60_000);
  // A writer the sheet had to kill is a failure in the sheet (verify r6 follow-up: said on stderr only).
  assert.equal(sheet.KILLED, `was sent SIGTERM and, still running ${sheet.COMMAND_STOP_MS} ms later, SIGKILL`);
  const killed = await sheet.runSheet({ ...argsFor(), clients: [1], writer: "trap '' TERM; while :; do sleep 1; done", commandLog: join(dir, 'commands.log') }, { send: standIn({ ignore: new Set(), delayMs: 5 }).send, log: () => {} });
  assert.ok(killed.failures.includes(`the --writer command's process group ${sheet.KILLED}: what it was writing was ended by force, not asked to stop`), killed.failures.join('\n'));
});

test('a browser\'s stderr is kept as its last lines only and read no more once its address is found, and an address cut between two writes is taken whole', { timeout: 60_000 }, async () => {
  // Before (verify r6 follow-up): the whole of the browser's stderr was kept for the run and searched again on every chunk.
  assert.equal(harness.STDERR_TAIL, 8_192);
  const net = await import('node:net');
  /** @type {import('node:net').Socket[]} */
  const sockets = [];
  const silent = net.createServer((s) => sockets.push(s));
  await new Promise((resolve) => silent.listen(0, '127.0.0.1', () => resolve(undefined)));
  const port = /** @type {import('node:net').AddressInfo} */ (silent.address()).port;
  const fake = mkdtempSync(join(tmpdir(), 'r315-tail-'));
  const chatty = join(fake, 'chatty');
  // 200 KB of other lines first, the address in two writes, then more lines while the launch waits.
  writeFileSync(chatty, `#!/bin/sh\ni=0; while [ $i -lt 2000 ]; do echo "noise line $i ................................................................................................." >&2; i=$((i+1)); done\nprintf 'DevTools listening on ws://127.0.0.1:${port}/devtools/' >&2\nsleep 0.2\nprintf 'browser/whole\\n' >&2\nwhile :; do echo "after the address" >&2; sleep 0.01; done\n`, { mode: 0o755 });
  const root = mkdtempSync(join(tmpdir(), 'r315-tail-root-'));
  try {
    await assert.rejects(harness.launch(chatty, false, 1_500, root), new RegExp(`^Error: could not connect to ws://127\\.0\\.0\\.1:${port}/devtools/browser/whole: no handshake within 1500 ms$`), 'the address is the whole line, never the half the first write held');
    assert.deepEqual(readdirSync(root), [], 'the browser was stopped and its profile removed');
  } finally {
    for (const s of sockets) s.destroy();
    silent.close();
  }
  // The source keeps a tail, never the whole, and searches each chunk before it cuts (repair 7).
  assert.match(source('ci/browser-harness.mjs'), /const seen = text \+ c;\s*const m = \/DevTools listening on \(ws:\\\/\\\/\\S\+\)\\r\?\\n\/\.exec\(seen\);[\s\S]{0,400}?text = seen\.slice\(-STDERR_TAIL\);/);
});

test('the end of a command waits for stdout to take its whole sheet, bounded: a stream that takes nothing is said on stderr and let go', async () => {
  // Before (verify r6 follow-up): process.exit right after the write cut a piped sheet past 64 KB.
  const { Writable } = await import('node:stream');
  /** @type {string[]} */
  const taken = [];
  const sink = new Writable({ write(chunk, _encoding, done) { taken.push(String(chunk)); done(); } });
  assert.equal(await sheet.written(sink, 'the sheet\n', 'speed-sheet', 1_000), true);
  assert.deepEqual(taken, ['the sheet\n']);
  /** @type {string[]} */
  const said = [];
  const write = process.stderr.write;
  process.stderr.write = /** @type {any} */ ((/** @type {any} */ s) => (said.push(String(s)), true));
  let ended;
  const t0 = Date.now();
  try {
    ended = await sheet.written(/** @type {any} */ ({ write: () => true }), 'x'.repeat(10), 'speed-sheet', 50);
  } finally {
    process.stderr.write = write;
  }
  assert.equal(ended, false, 'a stream that never takes it is let go at the bound');
  const took = Date.now() - t0;
  assert.ok(took >= 45 && took < 5_000, `at the bound, not before and not long after (took ${took} ms)`);
  assert.deepEqual(said, ['speed-sheet: stdout did not take the whole sheet (10 B) within 50 ms; what it did not take is lost\n']);
  // Both commands exit through it.
  for (const file of ['ci/speed-sheet.mjs', 'ci/browser-harness.mjs']) assert.match(source(file), /const flushed = written\(process\.stdout, text, '[a-z-]+'\);[\s\S]*await flushed;\s*process\.exit\(code\);/, file);
});

// ---------------------------------------------------------------------------
// Repair round 7 (25 Oct): verify r7's follow-ups in R3-15's own files (its
// two blocking findings are the real-browser checks above, `measuredRow`,
// and `/ingest`'s budget, D-5915 item 36).
// ---------------------------------------------------------------------------

test('a reader that closes the pipe early is said on stderr, and the command keeps its run\'s exit code', { timeout: 60_000 }, async () => {
  // Before (verify r7 follow-up, 25 Oct): the stream's `error` had no
  // listener, so EPIPE from a reader that had gone (`| head -1`) was thrown
  // as uncaught, and a run that PASSED exited 1 with a stack. The child ends
  // as both commands' `main` ends: `written`, then `process.exit(code)`.
  const { spawn } = await import('node:child_process');
  const module = new URL('ci/speed-sheet.mjs', web).href;
  const ends = `import { written } from ${JSON.stringify(module)}; process.stderr.write('ready\\n'); process.stdin.resume();` +
    " process.stdin.on('end', async () => { const flushed = written(process.stdout, 'x'.repeat(1_000_000), 'speed-sheet'); await flushed; process.exit(0); });";
  const child = spawn(process.execPath, ['--input-type=module', '-e', ends], { stdio: ['pipe', 'pipe', 'pipe'] });
  let said = '';
  const exited = /** @type {Promise<{code: number | null, signal: string | null}>} */ (new Promise((resolve) => child.on('exit', (code, signal) => resolve({ code, signal }))));
  child.stderr.on('data', (c) => {
    said += c;
    // The reader goes away first, then the child is told to write.
    if (said.includes('ready\n') && !child.stdout.destroyed) {
      child.stdout.destroy();
      child.stdin.end();
    }
  });
  const { code, signal } = await exited;
  assert.deepEqual([code, signal], [0, null], said);
  assert.match(said, /^ready\nspeed-sheet: stdout was closed before it took the whole sheet \(1000000 B\): EPIPE; what it did not take is lost\n$/);
  // A stream that fails as it is written to is said the same way, and its later failures are heard too.
  const { Writable } = await import('node:stream');
  const broken = new Writable({ write(_chunk, _encoding, done) { done(Object.assign(new Error('write EPIPE'), { code: 'EPIPE' })); } });
  /** @type {string[]} */
  const lines = [];
  const write = process.stderr.write;
  process.stderr.write = /** @type {any} */ ((/** @type {any} */ s) => (lines.push(String(s)), true));
  let took;
  try {
    took = await sheet.written(broken, 'abc', 'browser-harness', 5_000);
    assert.doesNotThrow(() => broken.emit('error', Object.assign(new Error('again'), { code: 'EPIPE' })), 'a later failure of the same stream is heard, never thrown');
  } finally {
    process.stderr.write = write;
  }
  assert.equal(took, false);
  assert.deepEqual(lines, ['browser-harness: stdout was closed before it took the whole sheet (3 B): EPIPE; what it did not take is lost\n'], 'said once');
  // One without a code is said by its message.
  const plain = new Writable({ write(_chunk, _encoding, done) { done(new Error('the disk went away')); } });
  /** @type {string[]} */
  const plainLines = [];
  process.stderr.write = /** @type {any} */ ((/** @type {any} */ s) => (plainLines.push(String(s)), true));
  try {
    assert.equal(await sheet.written(plain, 'abcd', 'speed-sheet', 5_000), false);
  } finally {
    process.stderr.write = write;
  }
  assert.deepEqual(plainLines, ['speed-sheet: stdout was closed before it took the whole sheet (4 B): the disk went away; what it did not take is lost\n']);
  // One listener a stream, however many sheets it is handed: none is left behind a call (a first cut added one a
  // call, and the twelfth drew Node's MaxListenersExceededWarning).
  const sink = new Writable({ write(_chunk, _encoding, done) { done(); } });
  const listening = sink.listenerCount('error');
  for (let i = 0; i < 12; i += 1) assert.equal(await sheet.written(sink, 'x', 'speed-sheet', 5_000), true);
  assert.equal(sink.listenerCount('error'), listening + 1);
  // The newest sheet hears the stream's failure, said once.
  const later = new Writable({ write(_chunk, _encoding, done) { done(Object.assign(new Error('write EPIPE'), { code: 'EPIPE' })); } });
  /** @type {string[]} */
  const twice = [];
  process.stderr.write = /** @type {any} */ ((/** @type {any} */ s) => (twice.push(String(s)), true));
  try {
    assert.equal(await sheet.written(later, 'one', 'speed-sheet', 5_000), false);
    assert.equal(await sheet.written(later, 'two!', 'speed-sheet', 5_000), false);
    await new Promise((resolve) => setImmediate(resolve));
  } finally {
    process.stderr.write = write;
  }
  assert.equal(later.listenerCount('error'), 1);
  assert.deepEqual(twice, ['speed-sheet: stdout was closed before it took the whole sheet (3 B): EPIPE; what it did not take is lost\n', 'speed-sheet: stdout was closed before it took the whole sheet (4 B): ERR_STREAM_DESTROYED; what it did not take is lost\n']);
});

test('a browser\'s DevTools address is searched for in its chunk before the tail is cut: an address followed in one write by more than STDERR_TAIL characters is taken', { timeout: 60_000 }, async () => {
  // Before (verify r7 follow-up, 25 Oct): the tail was cut to its last
  // STDERR_TAIL characters first and searched after, so an address followed
  // in its own chunk by more was cut away unread, and the launch waited out
  // its bound: "the browser printed no DevTools address".
  const net = await import('node:net');
  /** @type {import('node:net').Socket[]} */
  const sockets = [];
  const silent = net.createServer((s) => sockets.push(s));
  await new Promise((resolve) => silent.listen(0, '127.0.0.1', () => resolve(undefined)));
  const port = /** @type {import('node:net').AddressInfo} */ (silent.address()).port;
  const fake = mkdtempSync(join(tmpdir(), 'r315-early-'));
  const loud = join(fake, 'loud');
  // One write: the address, then four tails' worth of one more line, then nothing.
  writeFileSync(loud, `#!${process.execPath}\nprocess.stderr.write('DevTools listening on ws://127.0.0.1:${port}/devtools/browser/early\\n' + 'y'.repeat(${harness.STDERR_TAIL * 4}) + '\\n');\nsetInterval(() => {}, 1000);\n`, { mode: 0o755 });
  const root = mkdtempSync(join(tmpdir(), 'r315-early-root-'));
  try {
    await assert.rejects(harness.launch(loud, false, 1_500, root), new RegExp(`^Error: could not connect to ws://127\\.0\\.0\\.1:${port}/devtools/browser/early: no handshake within 1500 ms$`), 'the address was found in its chunk');
    assert.deepEqual(readdirSync(root), [], 'the browser was stopped and its profile removed');
  } finally {
    for (const s of sockets) s.destroy();
    silent.close();
  }
});

test('one idle-tab count is one idle row, and a day is one of its month\'s, given or found', async () => {
  // Before (verify r7 follow-ups, 25 Oct): `--idle-tabs 1,1` was taken, and
  // `--day` outside `--month` too; a day given alone was read in the rung's
  // newest month, and a month given alone took the rung's newest day though
  // that day was in another month.
  assert.throws(() => harness.parseArgs(['--base', 'http://h:1', '--idle-tabs', '1,1']), /^Error: --idle-tabs names 1 twice: each idle-tab count is measured once a run$/);
  assert.throws(() => harness.parseArgs(['--base', 'http://h:1', '--idle-tabs', '1,10,20,10']), /^Error: --idle-tabs names 10 twice: /);
  assert.deepEqual(harness.parseArgs(['--base', 'http://h:1', '--idle-tabs', '20,1,10']).idleTabs, [20, 1, 10], 'any order, each once');
  const base = ['--base', 'http://h:1'];
  assert.throws(() => sheet.parseArgs([...base, '--day', '2026-09-30', '--month', '2026-08']), /^Error: --day 2026-09-30 is not a day of --month 2026-08$/);
  assert.throws(() => sheet.parseArgs([...base, '--day', '2025-08-01', '--month', '2026-08']), /^Error: --day 2025-08-01 is not a day of --month 2026-08$/, 'the year is part of the month');
  assert.deepEqual([sheet.parseArgs([...base, '--day', '2026-08-31', '--month', '2026-08']).day, sheet.parseArgs([...base, '--day', '2026-08-31']).month], ['2026-08-31', null]);
  const args = sheet.parseArgs(base);
  const rungs = { rungs: [{ feed: 'zerodha', rung: '1min', stored: true, last_month: '2026-08', last_day: '2026-08-31' }] };
  const url = String(rungsUrl('NSE-INDEX-NIFTY'));
  const get = async (/** @type {string} */ p) => (p === url ? answer(200, JSON.stringify(rungs)) : answer(404));
  const dayAlone = await sheet.discover(get, { ...args, feed: 'zerodha', day: '2025-03-05' });
  assert.deepEqual([dayAlone.month, dayAlone.day, dayAlone.from, dayAlone.to, dayAlone.dayFrom], ['2025-03', '2025-03-05', '2025-03-01', '2025-03-31', 'given'], 'a day given alone is read in its own month');
  const monthAlone = await sheet.discover(get, { ...args, feed: 'zerodha', month: '2025-03' });
  assert.deepEqual([monthAlone.month, monthAlone.day, monthAlone.dayFrom], ['2025-03', '2025-03-01', 'first'], 'the rung\'s newest day is not in the month given: the month starts at its first day, taken');
  const newestIn = await sheet.discover(get, { ...args, feed: 'zerodha', month: '2026-08' });
  assert.deepEqual([newestIn.day, newestIn.dayFrom], ['2026-08-31', 'newest'], 'the rung\'s newest day, when it is in the month given');
  const all = await sheet.discover(async () => answer(404), { ...args, feed: 'zerodha', timeframe: '1min', month: '2025-03', day: '2025-03-03' });
  assert.equal(all.dayFrom, 'given', 'all three given: nothing asked');
});

test('a day the sheet took rather than found is said in the sheet: the first of a month given alone is not known to be a session', async () => {
  // Before (verify r8 follow-up, 26 Oct): `--month 2025-03` alone timed
  // 2025-03-01, a Saturday, and its empty answer as the day's, saying nothing.
  assert.equal(sheet.dayTakenNote({ ...CTX, dayFrom: 'given' }), null);
  assert.equal(sheet.dayTakenNote({ ...CTX, dayFrom: 'newest' }), null);
  assert.equal(sheet.dayTakenNote(CTX), null, 'a context that does not say how its day was chosen was handed in whole');
  assert.equal(sheet.dayTakenNote({ ...CTX, month: '2025-03', day: '2025-03-01', dayFrom: 'first' }), 'the day 2025-03-01 was taken, not found: it is the first of 2025-03, and the rung\'s newest day is not in that month, so whether it holds a session is not known here and the day\'s rows may time a day with no bars; pass --day to name a session day');
  const args = sheet.parseArgs(['--base', 'http://stand.in', '--n', '1', '--clients', '1', '--month', '2026-08']);
  const taken = await sheet.runSheet({ ...args, feed: 'zerodha' }, { send: standIn({ ignore: new Set() }).send, log: () => {} });
  assert.deepEqual([taken.ctx?.month, taken.ctx?.day, taken.ok], ['2026-08', '2026-08-01', true], taken.failures.join('\n'));
  assert.deepEqual(taken.notes, [sheet.dayTakenNote(/** @type {any} */ (taken.ctx))]);
  assert.match(sheet.renderSheet(taken, args), /^# Speed sheet — PASS\n[\s\S]*\n- the day 2026-08-01 was taken, not found: /, 'said in the sheet, its verdict its own');
  const found = await sheet.runSheet({ ...args, month: '2026-09', feed: 'zerodha' }, { send: standIn({ ignore: new Set() }).send, log: () => {} });
  assert.deepEqual([found.ctx?.day, found.notes], ['2026-09-30', []], 'the rung\'s newest day: nothing to say');
});

// ---------------------------------------------------------------------------
// Repair round 8 (26 Oct): verify r8's two blocking findings, a closed
// stderr (it ended a run, or a command's exit) and the newest-sheet-only
// listener (a surviving mutant), each failing on ade26e944.
// ---------------------------------------------------------------------------

/**
 * Runs a command with stdout and stderr piped here and, unless `close` is
 * `keep`, closes the reader of stderr (and of stdout too, for `both`) once
 * stderr has given its first whole line, as `2>&1 | head -1` does; `gone`
 * (a file) is made right after, for a child that waits on it.
 *
 * @param {string[]} argv
 * @param {'keep' | 'stderr' | 'both'} close
 * @param {string} [gone]
 * @returns {Promise<{code: number | null, signal: string | null, stdout: string, stderr: string, ms: number}>}
 */
const closingReader = (argv, close, gone) => new Promise((resolve) => {
  const t0 = Date.now();
  void import('node:child_process').then(({ spawn }) => {
    const child = spawn(process.execPath, argv, { stdio: ['ignore', 'pipe', 'pipe'] });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (c) => {
      stdout += c;
    });
    child.stderr.on('data', (c) => {
      stderr += c;
      if (close === 'keep' || !stderr.includes('\n') || child.stderr.destroyed) return;
      child.stderr.destroy();
      if (close === 'both') child.stdout.destroy();
      if (gone !== undefined) writeFileSync(gone, '');
    });
    child.on('close', (code, signal) => resolve({ code, signal, stdout, stderr, ms: Date.now() - t0 }));
  });
});

test('a command\'s lines on stderr are said through one listener a stream: a reader that has gone is counted, never thrown, and the first line not taken is named', async () => {
  // Before (verify r8, 26 Oct): neither command listened on process.stderr,
  // and Node's console keeps no listener there, so the EPIPE of a line
  // written after the reader had gone was thrown, a tick after its write.
  const { Writable } = await import('node:stream');
  /** @type {string[]} */
  const took = [];
  const fine = new Writable({ write(chunk, _e, done) { took.push(String(chunk)); done(); } });
  const a = sheet.saidOn(fine);
  assert.equal(sheet.saidOn(fine), a, 'one object a stream');
  assert.equal(fine.listenerCount('error'), 1, 'one listener a stream, however often it is asked for');
  a.say('one');
  a.say('two');
  assert.equal(a.pending(), 2, 'two lines in flight');
  assert.equal(await a.settled(5_000), true);
  assert.equal(a.pending(), 0);
  assert.deepEqual(took, ['one\n', 'two\n']);
  assert.equal(a.closed(), null);
  assert.equal(await sheet.stderrNote(a, 5_000), null, 'a stderr that took every line: no note');
  // The reader goes after the first line: the second line's write fails (its callback, then the stream's error, heard).
  /** @type {string[]} */
  const got = [];
  const closing = new Writable({ write(chunk, _e, done) { if (got.length === 0) { got.push(String(chunk)); done(); } else done(Object.assign(new Error('write EPIPE'), { code: 'EPIPE' })); } });
  const b = sheet.saidOn(closing);
  b.say('speed sheet: first');
  b.say('speed sheet: second');
  b.say('speed sheet: third');
  assert.equal(await b.settled(5_000), true, 'every line called back, the lost ones too');
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(b.closed(), { why: 'EPIPE', step: 'speed sheet: second', lost: 2 });
  b.say('speed sheet: fourth');
  assert.deepEqual(b.closed(), { why: 'EPIPE', step: 'speed sheet: second', lost: 3 }, 'a line after the failure is counted at once, never written');
  assert.equal(b.pending(), 0);
  assert.deepEqual(got, ['speed sheet: first\n']);
  assert.equal(await sheet.stderrNote(b, 5_000), 'stderr was closed (EPIPE) from the line "speed sheet: second": 3 of this run\'s own lines on it were not taken');
  // A stream that failed under another writer (a hook's line): closed, none of its own lost until the next; the first failure is the one said.
  const other = new Writable({ write(_c, _e, done) { done(); } });
  const c = sheet.saidOn(other);
  other.emit('error', Object.assign(new Error('write EPIPE'), { code: 'EPIPE' }));
  other.emit('error', Object.assign(new Error('read ECONNRESET'), { code: 'ECONNRESET' }));
  assert.deepEqual(c.closed(), { why: 'EPIPE', step: null, lost: 0 });
  assert.equal(await sheet.stderrNote(c, 5_000), 'stderr was closed (EPIPE) during the run; none of this run\'s own lines on it was lost');
  c.say('after');
  assert.deepEqual(c.closed(), { why: 'EPIPE', step: 'after', lost: 1 });
  // A failure its write's callback reports and no error event follows (not a Node stream) is heard the same; one with no code by its message.
  const quiet = { on() {}, write(/** @type {string} */ _s, /** @type {(e: Error) => void} */ cb) { cb(new Error('the disk went away')); return false; } };
  const d = sheet.saidOn(/** @type {any} */ (quiet));
  d.say('x');
  d.say('y');
  assert.deepEqual(d.closed(), { why: 'the disk went away', step: 'x', lost: 2 });
  // A stderr that has not taken a line: the wait ends at its bound, and is said; each wait after one that settled is its own.
  let firstOf = true;
  const half = new Writable({ write(_c, _e, done) { if (firstOf) { firstOf = false; setTimeout(done, 5); } } });
  const e = sheet.saidOn(half);
  e.say('taken');
  assert.equal(await e.settled(5_000), true);
  e.say('held');
  const t0 = Date.now();
  assert.equal(await e.settled(50), false, 'a line still in flight at the bound');
  assert.ok(Date.now() - t0 >= 45, 'at the bound, not before');
  assert.equal(e.pending(), 1);
  assert.equal(await sheet.stderrNote(e, 50), 'stderr had not taken 1 of this run\'s own lines 50 ms after the run');
  assert.equal(e.closed(), null);
  // Two in flight, the first taken and the second not: not settled.
  let firstOf2 = true;
  const half2 = new Writable({ write(_c, _e, done) { if (firstOf2) { firstOf2 = false; setTimeout(done, 5); } } });
  const f = sheet.saidOn(half2);
  f.say('taken');
  f.say('held');
  assert.equal(await f.settled(100), false, 'one line taken of two is not every line');
  assert.equal(f.pending(), 1);
});

test('every sheet still waiting on a stream hears its failure, each said once by its own size, and none stays behind its end', async () => {
  // Before (verify r8, 26 Oct; mutant R8-M12): only the newest sheet heard
  // the stream's failure, and the line that handed it over could be deleted
  // with every test green; a sheet whose write was in flight behind another
  // waited out its whole bound, then said the wrong reason.
  const { Writable } = await import('node:stream');
  /** @type {string[]} */
  const lines = [];
  /** @type {(() => void)[]} */
  const later = [];
  const write = process.stderr.write;
  // Each line is taken, and its write called back a moment later: the count of the command's own stderr is kept.
  process.stderr.write = /** @type {any} */ ((/** @type {any} */ s, /** @type {any} */ cb) => (lines.push(String(s)), typeof cb === 'function' && later.push(cb), true));
  const err = sheet.saidOn(process.stderr);
  try {
    // Verify r8's case exactly: the first sheet ends at its bound, the second's write is still in flight when the stream fails.
    const stuck = new Writable({ write() {} });
    assert.equal(await sheet.written(stuck, 'one', 'speed-sheet', 200), false);
    assert.equal(sheet.sheetsWaitingOn(stuck), 0, 'a sheet that ended at its bound waits no more');
    const before = err.pending();
    const second = sheet.written(stuck, 'two', 'speed-sheet', 5_000);
    assert.equal(sheet.sheetsWaitingOn(stuck), 1);
    const t1 = Date.now();
    setTimeout(() => stuck.destroy(Object.assign(new Error('write EPIPE'), { code: 'EPIPE' })), 50);
    assert.equal(await second, false);
    const took = Date.now() - t1;
    assert.ok(took < 2_500, `the second sheet ended at the failure, not at its 5,000 ms bound (took ${took} ms)`);
    assert.deepEqual(lines, ['speed-sheet: stdout did not take the whole sheet (3 B) within 200 ms; what it did not take is lost\n', 'speed-sheet: stdout was closed before it took the whole sheet (3 B): EPIPE; what it did not take is lost\n']);
    assert.equal(err.pending(), before + 1, 'the line went out through the command\'s counted stderr (saidOn)');
    assert.equal(sheet.sheetsWaitingOn(stuck), 0);
    // Two sheets waiting at once: both hear the failure, in the order they were handed, each by its own size.
    lines.length = 0;
    const held = new Writable({ write() {} });
    const t2 = Date.now();
    const first = sheet.written(held, 'one', 'browser-harness', 5_000);
    const next = sheet.written(held, 'four', 'browser-harness', 5_000);
    assert.equal(sheet.sheetsWaitingOn(held), 2);
    held.destroy(Object.assign(new Error('write EPIPE'), { code: 'EPIPE' }));
    assert.deepEqual(await Promise.all([first, next]), [false, false]);
    assert.ok(Date.now() - t2 < 2_500, 'both at the failure');
    assert.deepEqual(lines, ['browser-harness: stdout was closed before it took the whole sheet (3 B): EPIPE; what it did not take is lost\n', 'browser-harness: stdout was closed before it took the whole sheet (4 B): EPIPE; what it did not take is lost\n']);
    assert.equal(sheet.sheetsWaitingOn(held), 0);
    assert.equal(held.listenerCount('error'), 1, 'still one listener');
    // A stream that answers the sheet after its bound, and fails then: let go at the bound, said once; its late failure says nothing more.
    lines.length = 0;
    const slowly = new Writable({ write(_c, _e, done) { setTimeout(() => done(Object.assign(new Error('write EPIPE'), { code: 'EPIPE' })), 150); } });
    assert.equal(await sheet.written(slowly, 'late', 'speed-sheet', 30), false);
    assert.equal(sheet.sheetsWaitingOn(slowly), 0);
    await new Promise((resolve) => setTimeout(resolve, 300));
    assert.deepEqual(lines, ['speed-sheet: stdout did not take the whole sheet (4 B) within 30 ms; what it did not take is lost\n'], 'said once, at the bound');
    // A stream that takes each sheet: none waits behind its end; a stream never handed one has none.
    const sink = new Writable({ write(_c, _e, done) { done(); } });
    for (let i = 0; i < 12; i += 1) assert.equal(await sheet.written(sink, 'x', 'speed-sheet', 5_000), true);
    assert.equal(sheet.sheetsWaitingOn(sink), 0);
    assert.equal(sheet.sheetsWaitingOn({}), 0);
  } finally {
    process.stderr.write = write;
    for (const cb of later) cb();
  }
  assert.equal(err.closed(), null, 'this process\'s stderr was never failed by the test');
});

test('the waits at a command\'s end leave no timer behind: a process whose streams take their lines ends when its work does', { timeout: 60_000 }, async () => {
  // A wait that clears no timer holds the process for its whole bound after its stream has answered.
  const { execFile } = await import('node:child_process');
  const module = new URL('ci/speed-sheet.mjs', web).href;
  const code = `import { Writable } from 'node:stream'; import { saidOn, written } from ${JSON.stringify(module)};` +
    ' const slow = () => new Writable({ write(_c, _e, done) { setTimeout(done, 20); } });' +
    " const s = saidOn(slow()); s.say('one'); const settled = await s.settled(30_000);" +
    " const took = await written(slow(), 'the sheet', 'speed-sheet', 30_000);" +
    ' process.stdout.write(JSON.stringify({ settled, took }));';
  const t0 = Date.now();
  const ran = /** @type {{error: Error | null, stdout: string}} */ (await new Promise((resolve) => execFile(process.execPath, ['--input-type=module', '-e', code], { timeout: 25_000 }, (error, stdout) => resolve({ error, stdout }))));
  const ms = Date.now() - t0;
  assert.equal(ran.error, null, String(ran.error));
  assert.deepEqual(JSON.parse(ran.stdout), { settled: true, took: true });
  assert.ok(ms < 10_000, `the process ended with its work, not at a 30,000 ms bound (took ${ms} ms)`);
});

test('a sheet whose stderr reader goes after its first line runs to its end: its exit is its verdict, --out is whole, and the sheet names the first line stderr did not take', { timeout: 120_000 }, async () => {
  // Before (verify r8, 26 Oct): the next line's EPIPE was thrown a tick
  // later, so the sheet exited 1 within milliseconds, its --out never
  // written and nothing said; with stdout closed too (2>&1 | head -1), the same.
  const strict = standIn({ ignore: new Set() });
  const server = http.createServer(async (req, res) => {
    const a = await strict.send('', String(req.url), { method: req.method });
    if (a.status === 0) return void res.socket?.destroy();
    res.writeHead(a.status, { 'content-type': 'application/json', ...(a.build ? { 'x-brutex-build': a.build } : {}) });
    res.end(a.body);
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', () => resolve(undefined)));
  const base = `http://127.0.0.1:${/** @type {import('node:net').AddressInfo} */ (server.address()).port}`;
  const dir = mkdtempSync(join(tmpdir(), 'r315-stderr-'));
  try {
    const argv = [new URL('ci/speed-sheet.mjs', web).pathname, '--base', base, '--n', '1', '--clients', '1', '--feed', CTX.feed, '--timeframe', CTX.timeframe, '--month', CTX.month, '--day', CTX.day];
    // The same run with its readers kept: its lines, PASS, exit 0, and a sheet that says nothing of stderr.
    const kept = await closingReader([...argv, '--out', join(dir, 'kept.md')], 'keep');
    assert.deepEqual([kept.code, kept.signal], [0, null], kept.stderr);
    const lines = kept.stderr.split('\n').filter((l) => l !== '');
    assert.ok(lines.length >= 4 && lines.every((l) => l.startsWith('speed sheet: ')), kept.stderr);
    assert.match(kept.stdout, /^# Speed sheet — PASS\n/);
    assert.equal(readFileSync(join(dir, 'kept.md'), 'utf8'), kept.stdout);
    assert.doesNotMatch(kept.stdout, /stderr/);
    for (const close of /** @type {const} */ (['stderr', 'both'])) {
      const out = join(dir, `${close}.md`);
      const ran = await closingReader([...argv, '--out', out], close);
      assert.deepEqual([ran.code, ran.signal], [0, null], `${close}: the exit is the run's verdict, PASS (${ran.ms} ms): ${ran.stderr}`);
      assert.ok(existsSync(out), `${close}: --out was written`);
      const text = readFileSync(out, 'utf8');
      assert.match(text, /^# Speed sheet — PASS\n/, close);
      const m = /^- stderr was closed \(EPIPE\) from the line "(.+)": (\d+) of this run's own lines on it were not taken$/m.exec(text);
      assert.ok(m, `${close}: the sheet names the first line stderr did not take:\n${text}`);
      const at = lines.indexOf(m[1]);
      assert.ok(at >= 1, `${close}: a line of the run, after the one that was read: ${m[1]}`);
      assert.equal(Number(m[2]), lines.length - at, `${close}: that line and every one after it`);
      assert.ok(ran.stderr.startsWith(`${lines[0]}\n`), `${close}: the reader had the first line: ${ran.stderr}`);
      assert.equal(ran.stdout, close === 'stderr' ? text : '', `${close}: stdout is the sheet, the same as --out, unless it was closed first`);
      assert.deepEqual(readdirSync(dir).filter((n) => n.endsWith('.tmp')), [], 'no file left half written');
    }
  } finally {
    server.close();
  }
});

test('a sheet whose stderr is closed from its start and whose run ends before it times anything still writes its --out whole, naming the line stderr lost', { timeout: 60_000 }, async () => {
  // Every return of the run carries its notes, so a run that ends early can still say what stderr lost (repair 8):
  // its one line, the warning of an unstamped server, goes to a stderr already closed, and discovery then fails.
  const server = http.createServer((req, res) => {
    res.writeHead(req.method === 'HEAD' && req.url === UP_ROUTE ? 200 : 404, { 'content-type': 'application/json' });
    res.end(req.method === 'HEAD' ? undefined : '{"error":"not here"}');
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', () => resolve(undefined)));
  const base = `http://127.0.0.1:${/** @type {import('node:net').AddressInfo} */ (server.address()).port}`;
  const dir = mkdtempSync(join(tmpdir(), 'r315-stderr-early-'));
  const out = join(dir, 'sheet.md');
  try {
    const { spawn } = await import('node:child_process');
    const child = spawn(process.execPath, [new URL('ci/speed-sheet.mjs', web).pathname, '--base', base, '--allow-unstamped', '--out', out], { stdio: ['ignore', 'pipe', 'pipe'] });
    child.stderr.destroy();
    let stdout = '';
    child.stdout.on('data', (c) => {
      stdout += c;
    });
    const [code, signal] = await new Promise((resolve) => child.on('close', (c, s) => resolve([c, s])));
    assert.deepEqual([code, signal], [1, null], 'the run failed by name: its exit is that verdict');
    assert.ok(existsSync(out), '--out was written');
    const text = readFileSync(out, 'utf8');
    assert.equal(stdout, text);
    assert.match(text, /^# Speed sheet — FAIL\n/);
    assert.match(text, /\n- stderr was closed \(EPIPE\) from the line "WARNING: the server names no build \(x-brutex-build\); a sheet that cannot name its commit measures no build — allowed by --allow-unstamped": 1 of this run's own lines on it were not taken\n- FAILED: discovering the selection: \/feeds\.json answered HTTP 404\n$/);
  } finally {
    server.close();
  }
});

test('a harness whose stderr reader goes mid-run runs to its end: its exit is its verdict, --out is whole, and the sheet says stderr was closed', { timeout: 240_000 }, async () => {
  // Before (verify r8, 26 Oct): a line written to the closed stderr — a
  // hook's here, the harness's own at a stop it could not finish, or
  // `written`'s at the end with stdout gone too — was thrown a tick later:
  // exit 1 and no --out.
  const site = await browserStandIn({});
  const dir = mkdtempSync(join(tmpdir(), 'r315-harness-stderr-'));
  try {
    const root = process.getuid?.() === 0 ? ['--no-sandbox'] : [];
    const script = new URL('ci/browser-harness.mjs', web).pathname;
    for (const close of /** @type {const} */ (['stderr', 'both'])) {
      const gone = join(dir, `${close}.gone`);
      const hook = join(dir, `${close}-hook.mjs`);
      // The hook says a line, waits (bounded) until the reader has gone, says another into the closed pipe, and goes on.
      writeFileSync(hook, [
        "import { existsSync } from 'node:fs';",
        'export default async () => {',
        "  process.stderr.write('hook: first\\n');",
        `  for (let i = 0; i < 800 && !existsSync(${JSON.stringify(gone)}); i += 1) await new Promise((r) => setTimeout(r, 25));`,
        "  process.stderr.write('hook: second\\n');",
        '  await new Promise((r) => setTimeout(r, 200));',
        "  return { rows: ['the hook said two lines'] };",
        '};',
        ''
      ].join('\n'));
      const out = join(dir, `${close}.md`);
      const ran = await closingReader([script, '--base', site.base, '--n', '1', '--pages', '/speed', '--no-interactions', '--no-idle', '--no-report-check', '--settle-ms', '100', '--hook', hook, '--out', out, ...root], close, gone);
      assert.ok(existsSync(gone), `${close}: the reader went after the hook's first line`);
      assert.ok(existsSync(out), `${close}: --out was written (exit ${ran.code}, signal ${ran.signal}): ${ran.stderr}`);
      const text = readFileSync(out, 'utf8');
      const verdict = /^# Browser harness — (PASS|FAIL)\n/.exec(text);
      assert.ok(verdict, text);
      assert.deepEqual([ran.code, ran.signal], [verdict[1] === 'PASS' ? 0 : 1, null], `${close}: the exit is the verdict its sheet states:\n${text}`);
      for (const f of text.split('\n').filter((l) => l.startsWith('- FAILED: ')).map((l) => l.slice('- FAILED: '.length))) assert.ok(timedJudgement(f, '/speed'), `${close}: a FAIL only on how fast this machine drew /speed: ${f}`);
      assert.match(text, /\nHook [^\n]*:\nthe hook said two lines\n/, `${close}: the run went on past the line stderr lost`);
      assert.match(text, /^- stderr was closed \(EPIPE\) during the run; none of this run's own lines on it was lost$/m, `${close}:\n${text}`);
      assert.ok(ran.stderr.startsWith('hook: first\n'), ran.stderr);
      assert.equal(ran.stdout, close === 'stderr' ? text : '', `${close}: stdout is the sheet, the same as --out, unless it was closed first`);
    }
  } finally {
    site.server.close();
  }
});

test('both commands say their lines through the counted stderr from their first line, and a sheet\'s note comes before what it did not measure', () => {
  for (const [file, name] of /** @type {const} */ ([['ci/speed-sheet.mjs', 'speed-sheet'], ['ci/browser-harness.mjs', 'browser-harness']])) {
    const src = source(file);
    const main = src.slice(src.indexOf('async function main() {'));
    assert.match(main, /^async function main\(\) \{\n(?:\s*\/\/[^\n]*\n)*\s*const said = saidOn\(process\.stderr\);\n/, `${file}: the listener is the first thing main does`);
    assert.doesNotMatch(main, /console\.|process\.stderr\.write/, `${file}: every line of main through said.say`);
    assert.match(main, /const note = await stderrNote\(said\);/, file);
    assert.equal((main.match(new RegExp(`said\\.say\\(\`${name}: `, 'g')) ?? []).length, name === 'speed-sheet' ? 2 : 3, file);
  }
  assert.doesNotMatch(source('ci/speed-sheet.mjs') + source('ci/browser-harness.mjs'), /console\.error|process\.stderr\.write\(/, 'no line reaches stderr around the count');
  assert.match(source('ci/speed-sheet.mjs'), /const log = deps\.log \?\? saidOn\(process\.stderr\)\.say;/);
  const text = sheet.renderSheet({ ok: true, rows: [], ignored: [], failures: [], notMeasured: ['nm'], skipped: ['sk'], ctx: null, commit: null, notes: ['stderr was closed (EPIPE) from the line "x": 1 of this run\'s own lines on it were not taken'] }, { n: 1, clients: [1] });
  assert.match(text, /^# Speed sheet — PASS\n/, 'a note never changes the verdict');
  assert.match(text, /\n\n- stderr was closed \(EPIPE\) from the line "x": 1 of this run's own lines on it were not taken\n- SKIPPED: sk\n- NOT MEASURED: nm\n$/);
  assert.doesNotMatch(sheet.renderSheet({ ok: true, rows: [], ignored: [], failures: [], notMeasured: [], skipped: [], ctx: null, commit: null }, { n: 1, clients: [1] }), /\n- /, 'no notes, no line');
});

// ---------------------------------------------------------------------------
// Repair round 9 (26 Oct): verify r9's two blocking findings, a restart never
// proved and a command given the sheet's own stderr, and its cheap
// follow-ups, each failing on 45e58698d.
// ---------------------------------------------------------------------------

/** The selection, as the command line names it. */
const CTX_ARGS = ['--feed', CTX.feed, '--timeframe', CTX.timeframe, '--month', CTX.month, '--day', CTX.day];

/** The speed sheet's own file, to run as a command. */
const SHEET = new URL('ci/speed-sheet.mjs', web).pathname;

/**
 * A real server for the sheet over the strict stand-in, whose own start
 * (`started_ms`) is the number in `startFile`: a `--restart` command that
 * writes a new one there is a restart, one that does not is none. Each
 * answer comes `delayMs` late, so a pass takes time.
 *
 * @param {string} startFile
 * @param {number} delayMs
 */
async function startedServer(startFile, delayMs) {
  const strict = standIn({ ignore: new Set(), delayMs, started: () => Number(readFileSync(startFile, 'utf8')) });
  const server = http.createServer(async (req, res) => {
    const a = await strict.send('', String(req.url), { method: req.method });
    if (a.status === 0) return void res.socket?.destroy();
    res.writeHead(a.status, { 'content-type': 'application/json', ...(a.build ? { 'x-brutex-build': a.build } : {}) });
    res.end(req.method === 'HEAD' ? undefined : a.body);
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', () => resolve(undefined)));
  return { base: `http://127.0.0.1:${/** @type {import('node:net').AddressInfo} */ (server.address()).port}`, close: () => server.close() };
}

test('a restart is proved by the server\'s own start before anything is timed: a --restart that restarted nothing fails by name and times nothing, and the two asks a restart needs are their routes\' first after it', async () => {
  // Before (verify r9, 26 Oct): the pass read only the command's exit code,
  // the up check and the build it named, so a command that exited 0 and
  // restarted nothing published warm answers as 'first after restart' rows
  // under a PASS.
  // (1) The verifier's case: the command exits 0 and the server's start never changes.
  let ran = 0;
  /** @type {string[]} */
  const logged = [];
  const none = await sheet.runSheet({ ...argsFor(), restart: 'true', restarts: 3 }, { send: standIn({ ignore: new Set() }).send, log: (l) => logged.push(l), restart: async () => ((ran += 1), 0) });
  assert.equal(none.ok, false);
  assert.deepEqual(none.failures, ['restart 1: the server at http://stand.in did not restart: its /speed.json said started_ms 1791600000000 before the --restart command and after it']);
  assert.ok(none.notMeasured.includes('first after restart: timed after 0 of the 3 restarts asked: the server did not restart at restart 1'), none.notMeasured.join('\n'));
  assert.ok(!none.rows.some((x) => x.phase === 'first after restart'), 'no warm answer is published as a first');
  assert.equal(ran, 1, 'the pass ended at the restart that restarted nothing');
  assert.ok(!logged.some((l) => l.includes('the server restarted')), logged.join('\n'));
  assert.ok(!none.notes.some((n) => n.startsWith('first after restart')), 'no restart is said proved');
  assert.match(sheet.renderSheet(none, argsFor()), /^# Speed sheet — FAIL\n/);
  // A later restart that restarts nothing ends the pass there; what the earlier one timed stands, with its proof.
  let n2 = 0;
  const later = await sheet.runSheet({ ...argsFor(), restart: 'x', restarts: 3 }, { send: standIn({ ignore: new Set(), started: () => Math.min(n2, 1) }).send, log: () => {}, restart: async () => ((n2 += 1), 0) });
  assert.ok(later.failures.includes('restart 2: the server at http://stand.in did not restart: its /speed.json said started_ms 1 before the --restart command and after it'), later.failures.join('\n'));
  assert.ok(later.notMeasured.includes('first after restart: timed after 1 of the 3 restarts asked: the server did not restart at restart 2'), later.notMeasured.join('\n'));
  assert.ok(later.rows.filter((x) => x.phase === 'first after restart').every((x) => x.n === 1));
  assert.ok(later.notes.includes('first after restart: 1 restart timed, each proved by the server\'s own start (/speed.json started_ms) changing across the --restart command: 0 to 1'), later.notes.join('\n'));
  // (2) A server that restarts each time: each restart proved and said, and after each the up check and the
  // read that proved it are their routes' samples, never asked twice.
  let starts = 100;
  let restarted = 0;
  /** @type {string[]} */
  const asks = [];
  const strict = standIn({ ignore: new Set(), started: () => starts });
  /** @param {string} b @param {string} path @param {{method?: string}} [o] */
  const spy = async (b, path, o = {}) => {
    if (restarted > 0) asks.push(`${o.method ?? 'GET'} ${path}`);
    return strict.send(b, path, o);
  };
  /** @type {string[]} */
  const lines = [];
  const proved = await sheet.runSheet({ ...argsFor(), restart: 'x', restarts: 3 }, { send: spy, log: (l) => lines.push(l), restart: async () => ((restarted += 1), (starts += 5), 0) });
  assert.equal(proved.ok, true, proved.failures.join('\n'));
  const { requests } = sheet.requestsFor(proved.ctx ?? CTX);
  const firsts = proved.rows.filter((x) => x.phase === 'first after restart');
  assert.equal(firsts.length, requests.length);
  assert.ok(firsts.every((x) => x.n === 3 && x.classes.answered === 3), JSON.stringify(firsts.map((x) => [x.url, x.n, x.classes])));
  assert.ok(proved.notes.includes('first after restart: 3 restarts timed, each proved by the server\'s own start (/speed.json started_ms) changing across the --restart command: 100 to 105, 105 to 110, 110 to 115'), proved.notes.join('\n'));
  assert.ok(lines.includes('speed sheet: restart 2: the server restarted: started_ms 105 before the command, 110 after it'), lines.join('\n'));
  const count = (/** @type {string} */ k) => asks.filter((a) => a === k).length;
  assert.equal(count(`${UP_METHOD} ${UP_ROUTE}`), 3, 'the up route is asked once a restart: its up check, which is its sample');
  assert.equal(count(`GET ${SPEED_ROUTE}`), 5, '/speed.json: the proof after each of the 3 restarts, and the start before restarts 2 and 3; never a third time');
  for (const r of requests.filter((x) => x.url !== UP_ROUTE && x.url !== SPEED_ROUTE)) assert.equal(count(`${r.method} ${r.url}`), 3, `${r.url} is asked once a restart`);
  assert.equal(asks.length, 3 + 5 + 3 * (requests.length - 2), 'and nothing else is asked after a restart');
  // (3) A start that cannot be read is said by name, and ends the pass before anything is timed.
  /** @param {(path: string, restarts: number) => object | null} answerOf what /speed.json answers instead, by the restarts run @param {object} [o] */
  const unreadable = async (answerOf, o = {}) => {
    const state = { restarts: 0 };
    const base = standIn({ ignore: new Set(), started: () => state.restarts, ...o });
    /** @param {string} b @param {string} path @param {{method?: string}} [so] */
    const send = async (b, path, so = {}) => {
      const a = await base.send(b, path, so);
      const other = new URL(path, 'http://x').pathname === SPEED_ROUTE ? answerOf(path, state.restarts) : null;
      return other === null ? a : { ...a, ...other };
    };
    const r = await sheet.runSheet({ ...argsFor(), restart: 'x', restarts: 2 }, { send, log: () => {}, restart: async () => ((state.restarts += 1), 0) });
    assert.ok(!r.rows.some((x) => x.phase === 'first after restart'), 'nothing is timed after a restart not proved');
    return { r, restarts: state.restarts };
  };
  const before404 = await unreadable(() => ({ status: 404 }));
  assert.ok(before404.r.failures.includes('restart 1: the server\'s start could not be read before the --restart command, so a restart could not be told from none: GET /speed.json answered HTTP 404'), before404.r.failures.join('\n'));
  assert.ok(before404.r.notMeasured.includes('first after restart: timed after 0 of the 2 restarts asked: the server\'s start could not be read at restart 1'), before404.r.notMeasured.join('\n'));
  assert.equal(before404.restarts, 0, 'the command is not run when the start before it cannot be read');
  const after503 = await unreadable((_p, n) => (n > 0 ? { status: 503, headers: {} } : null));
  assert.ok(after503.r.failures.includes('restart 1: the server\'s start could not be read after the --restart command, so a restart could not be told from none: GET /speed.json answered HTTP 503'), after503.r.failures.join('\n'));
  assert.equal(after503.restarts, 1);
  const noStamp = await unreadable(() => null, { started: () => 'soon' });
  assert.ok(noStamp.r.failures.includes('restart 1: the server\'s start could not be read before the --restart command, so a restart could not be told from none: GET /speed.json answered without a whole started_ms'), noStamp.r.failures.join('\n'));
  // A /speed.json asked no more this run (it ran out its bound in the cold pass) is not asked for the start either.
  let speedAsks = 0;
  const stalledSpeed = standIn({ ignore: new Set() });
  const r4 = await sheet.runSheet({ ...argsFor(), restart: 'x', restarts: 2 }, {
    send: async (b, path, o = {}) => {
      if (new URL(path, 'http://x').pathname !== SPEED_ROUTE) return stalledSpeed.send(b, path, o);
      speedAsks += 1;
      return { status: 0, error: 'no whole answer within 60000 ms (not even its headers came)', ms: 60_000, build: null, body: '', bytes: 0, headers: {}, timedOut: true };
    },
    log: () => {},
    restart: async () => 0
  });
  assert.ok(r4.failures.includes('restart 1: the server\'s start could not be read before the --restart command, so a restart could not be told from none: GET /speed.json is asked no more this run: it had no whole answer within --timeout-ms 60000 in the cold pass at 1 clients'), r4.failures.join('\n'));
  assert.equal(speedAsks, 1, 'asked once, in the cold pass, and never again');
});

test('the server\'s start is read from its /speed.json answer, or why not is said: a refusal, no answer, an answer kept as its digest, a body that is not JSON, and a started_ms that is not a whole number', () => {
  const ok = { status: 200, headers: {}, body: JSON.stringify(report([])), ms: 1, bytes: 10, build: null, kept: /** @type {const} */ ('text') };
  assert.deepEqual(sheet.startedOf(ok), { started: 1_791_600_000_000 });
  assert.deepEqual(sheet.startedOf({ ...ok, body: '{"started_ms":0}' }), { started: 0 }, 'a start at 0 is a start');
  assert.deepEqual(sheet.startedOf({ ...ok, status: 404 }), { why: 'GET /speed.json answered HTTP 404' });
  assert.deepEqual(sheet.startedOf({ status: 0, error: 'ECONNREFUSED', ms: 1, build: null, body: '', bytes: 0, headers: {} }), { why: 'GET /speed.json answered ECONNREFUSED' });
  assert.deepEqual(sheet.startedOf({ ...ok, kept: 'digest', bytes: 70_000_000 }), { why: 'GET /speed.json answered 70000000 bytes, past the 64000000 the sheet keeps as text' });
  assert.match(sheet.startedOf({ ...ok, body: 'not json' }).why ?? '', /^GET \/speed\.json answered HTTP 200 with a body that is not JSON \(/);
  for (const body of ['null', '[]', '{}', '{"started_ms":"1"}', '{"started_ms":-1}', '{"started_ms":1.5}', '{"started_ms":9007199254740993}']) {
    assert.deepEqual(sheet.startedOf({ ...ok, body }), { why: 'GET /speed.json answered without a whole started_ms' }, body);
  }
  // An answer from a stand-in that does not say what it kept is read as text.
  const { kept: _k, ...bare } = ok;
  assert.deepEqual(sheet.startedOf(bare), { started: 1_791_600_000_000 });
});

test('as a command over the network: a --restart that restarts nothing exits 1 by name with no first-after-restart row, and one that restarts the server passes with each restart\'s proof in the sheet', { timeout: 60_000 }, async () => {
  // The verifier's run in one process: --restart 'true' printed '# Speed sheet — PASS', exit 0, with 14 rows.
  const dir = mkdtempSync(join(tmpdir(), 'r315-proof-'));
  const start = join(dir, 'start');
  writeFileSync(start, '1791600000000');
  const site = await startedServer(start, 0);
  try {
    const argv = ['--base', site.base, '--n', '1', '--clients', '1', ...CTX_ARGS, '--timeout-ms', '5000', '--up-wait-ms', '5000'];
    const none = await sheetCommand([...argv, '--restart', 'true', '--restarts', '3', '--out', join(dir, 'none.md')]);
    assert.equal(none.code, 1, none.stdout);
    assert.match(none.stdout, /^# Speed sheet — FAIL\n/);
    assert.ok(none.stdout.includes(`\n- FAILED: restart 1: the server at ${site.base} did not restart: its /speed.json said started_ms 1791600000000 before the --restart command and after it\n`), none.stdout);
    assert.doesNotMatch(none.stdout, /\| first after restart \|/, 'no warm answer is printed as a first');
    const real = await sheetCommand([...argv, '--restart', `echo $$ > '${start}'`, '--restarts', '2', '--out', join(dir, 'real.md')]);
    assert.equal(real.code, 0, real.stdout);
    assert.match(real.stdout, /^# Speed sheet — PASS\n/);
    assert.equal((real.stdout.match(/\| first after restart \|/g) ?? []).length, sheet.requestsFor(CTX).requests.length);
    assert.match(real.stdout, /\n- first after restart: 2 restarts timed, each proved by the server's own start \(\/speed\.json started_ms\) changing across the --restart command: 1791600000000 to (\d+), \1 to \d+\n/);
    assert.match(readFileSync(join(dir, 'real.md.commands.log'), 'utf8'), /^=== \S+ restart 1 of 2, the --restart command: echo \$\$ > [^\n]+\n=== \S+ restart 2 of 2, the --restart command: echo \$\$ > [^\n]+\n$/);
  } finally {
    site.close();
  }
});

test('a sheet whose stderr reader has gone keeps its --writer and --restart commands running: what they print goes to the commands\' log the sheet names, the writer lasts its whole pass, and the exit is the verdict', { timeout: 120_000 }, async () => {
  // Before (verify r9, 26 Oct): both commands were given the sheet's own
  // stderr, so with its reader gone (2>&1 | head -1) a writer that printed
  // died of SIGPIPE ("ended before the warm+writer pass did", a FAIL) and a
  // restart that printed failed "was ended by SIGPIPE".
  const dir = mkdtempSync(join(tmpdir(), 'r315-gone-'));
  const start = join(dir, 'start');
  writeFileSync(start, '1');
  const site = await startedServer(start, 5);
  try {
    const out = join(dir, 'sheet.md');
    const argv = [SHEET, '--base', site.base, '--n', '3', '--clients', '1', ...CTX_ARGS, '--writer', 'while :; do echo writer-tick; sleep 0.05; done', '--restart', `echo restarting; echo $$ > '${start}'`, '--restarts', '2', '--out', out];
    const ran = await closingReader(argv, 'both');
    assert.ok(existsSync(out), `--out was written (exit ${ran.code}): ${ran.stderr}`);
    const text = readFileSync(out, 'utf8');
    assert.deepEqual([ran.code, ran.signal], [0, null], text);
    assert.match(text, /^# Speed sheet — PASS\n/);
    assert.doesNotMatch(text, /--writer command ended before/);
    assert.match(text, /\n- the --writer command's process group was sent SIGTERM and left at the end of the warm\+writer pass\n/);
    assert.ok(text.includes(`\n- what the --writer and --restart commands printed is kept in ${out}.commands.log, each run under a line that names it\n`), text);
    assert.match(text, /\n- stderr was closed \(EPIPE\) from the line "[^"\n]+": \d+ of this run's own lines on it were not taken\n/);
    assert.match(readFileSync(`${out}.commands.log`, 'utf8'), /^=== \S+ the --writer command: while :; do echo writer-tick; sleep 0\.05; done\n(?:writer-tick\n)+=== \S+ restart 1 of 2, the --restart command: echo restarting; [^\n]+\nrestarting\n=== \S+ restart 2 of 2, the --restart command: echo restarting; [^\n]+\nrestarting\n$/);
    assert.equal(ran.stdout, '', 'stdout was closed too, so the sheet is the file\'s');
  } finally {
    site.close();
  }
});

test('a stderr reader that is alive and not reading neither stalls a --writer nor freezes the sheet: the run ends in its own time plus at most STDERR_SETTLE_MS, --out is whole, and the sheet notes the lines stderr had not taken', { timeout: 180_000 }, async () => {
  // Before (verify r9, 26 Oct): a child's start cleared O_NONBLOCK on the
  // stderr it shared with the sheet, so once the writer had filled the pipe
  // the sheet's next line blocked the whole process until the reader left
  // (11.8 s traced, 30 s on the real command), past every timer, and the
  // writer itself waited on the full pipe.
  const { constants: c, openSync, closeSync, writeSync } = await import('node:fs');
  const { spawn, spawnSync } = await import('node:child_process');
  const dir = mkdtempSync(join(tmpdir(), 'r315-stalled-'));
  const start = join(dir, 'start');
  writeFileSync(start, '1');
  // Each answer 50 ms late: the writer's pass lasts long enough for it to print past what a pipe holds.
  const site = await startedServer(start, 50);
  /**
   * One run of the sheet whose stderr is a named pipe held open here and never read; `full` fills the pipe
   * before the sheet starts, so none of its lines can be taken. The reader leaves at 40 s whatever happens.
   *
   * @param {string} name @param {boolean} full
   */
  const stalledRun = async (name, full) => {
    const fifo = join(dir, `${name}.fifo`);
    assert.equal(spawnSync('mkfifo', [fifo]).status, 0);
    const reader = openSync(fifo, c.O_RDONLY | c.O_NONBLOCK);
    const end = openSync(fifo, c.O_WRONLY | c.O_NONBLOCK);
    let filled = 0;
    if (full) {
      const chunk = Buffer.alloc(1_024, 0x2e);
      try {
        for (;;) filled += writeSync(end, chunk);
      } catch (why) {
        if (/** @type {NodeJS.ErrnoException} */ (why).code !== 'EAGAIN') throw why;
      }
    }
    const out = join(dir, `${name}.md`);
    const t0 = Date.now();
    const child = spawn(process.execPath, [SHEET, '--base', site.base, '--n', '1', '--clients', '1', ...CTX_ARGS, '--writer', "head -c 400000 /dev/zero | tr '\\0' w; sleep 100", '--out', out], { stdio: ['ignore', 'pipe', end] });
    closeSync(end);
    let stdout = '';
    child.stdout.on('data', (d) => (stdout += d));
    let open = true;
    const leave = setTimeout(() => {
      open = false;
      closeSync(reader);
    }, 40_000);
    const code = await new Promise((resolve) => child.on('close', (cc) => resolve(cc)));
    const ms = Date.now() - t0;
    clearTimeout(leave);
    if (open) closeSync(reader);
    return { code, ms, stdout, out, filled };
  };
  try {
    // (1) The verifier's case: the pipe empty at the start, the writer printing 400 KB.
    const first = await stalledRun('empty', false);
    assert.ok(first.ms < 20_000, `the sheet was not frozen until the reader left at 40 s (took ${first.ms} ms)`);
    assert.equal(first.code, 0, first.stdout);
    const text1 = readFileSync(first.out, 'utf8');
    assert.equal(first.stdout, text1, 'stdout is the sheet, the same as --out');
    assert.match(text1, /^# Speed sheet — PASS\n/);
    // The writer was never stalled: it printed into its log past the 64 KB a pipe holds, where on the stalled
    // pipe it would have stopped at the first full buffer.
    const log1 = readFileSync(`${first.out}.commands.log`, 'utf8');
    assert.match(log1.slice(0, log1.indexOf('\n') + 1), /^=== \S+ the --writer command: head -c 400000 [^\n]+\n$/);
    const printed1 = log1.slice(log1.indexOf('\n') + 1);
    assert.ok(printed1.length >= 65_536 && /^w+$/.test(printed1), `the writer printed ${printed1.length} B into its log`);
    // (2) The pipe full from the start: none of the sheet's lines is taken, and the run waits STDERR_SETTLE_MS for them, no more.
    const second = await stalledRun('full', true);
    assert.ok(second.filled > 0, 'the pipe was filled');
    assert.equal(second.code, 0, second.stdout);
    const text2 = readFileSync(second.out, 'utf8');
    assert.match(text2, /^# Speed sheet — PASS\n/);
    assert.match(text2, /\n- stderr had not taken \d+ of this run's own lines 1000 ms after the run\n/);
    assert.ok(second.ms >= sheet.STDERR_SETTLE_MS, `the settle wait was spent (took ${second.ms} ms)`);
    assert.ok(second.ms - first.ms < sheet.STDERR_SETTLE_MS + 5_000, `and no more than it, give or take this machine (${second.ms} ms against ${first.ms} ms)`);
    const log2 = readFileSync(`${second.out}.commands.log`, 'utf8');
    const printed2 = log2.slice(log2.indexOf('\n') + 1);
    assert.ok(printed2.length >= 65_536 && /^w+$/.test(printed2), `the writer was never stalled: it printed ${printed2.length} B into its log`);
  } finally {
    site.close();
  }
});

test('the end of a run waits STDERR_SETTLE_MS for stderr to take the run\'s own lines: a line taken 200 ms late is waited for, and a stderr that takes none is let go at the bound', async () => {
  // Verify r9 follow-up (mutant R9-M09, 1 for 1_000): no test pinned the default bound at a command's end.
  const { Writable } = await import('node:stream');
  assert.equal(sheet.STDERR_SETTLE_MS, 1_000);
  const late = sheet.saidOn(new Writable({ write(_c, _e, done) { setTimeout(done, 200); } }));
  late.say('speed sheet: the last line');
  const t0 = Date.now();
  assert.equal(await sheet.stderrNote(late), null, 'a stderr that took the last line 200 ms late took every line');
  assert.ok(Date.now() - t0 >= 150, 'and it was waited for');
  const never = sheet.saidOn(new Writable({ write() {} }));
  never.say('speed sheet: a line never taken');
  const t1 = Date.now();
  assert.equal(await sheet.stderrNote(never), 'stderr had not taken 1 of this run\'s own lines 1000 ms after the run');
  const waited = Date.now() - t1;
  assert.ok(waited >= 950 && waited < 5_000, `let go at the bound (waited ${waited} ms)`);
});

test('the commands\' log: a regular file only, appended to, each run of a command under a line that names it; a log that cannot be kept refuses its command by name, never runs it with its output lost; and how a writer ended is named', { timeout: 120_000 }, async () => {
  const { constants: c, openSync, closeSync } = await import('node:fs');
  const { spawnSync, execFile } = await import('node:child_process');
  const dir = mkdtempSync(join(tmpdir(), 'r315-cmdlog-'));
  // A regular file, appended to: what was there stays, and a command's stdout and stderr both land in it.
  const path = join(dir, 'commands.log');
  writeFileSync(path, 'kept\n');
  const fd = sheet.openCommandLog(path);
  assert.equal(sheet.heading(fd, '=== a heading'), null);
  const echoed = sheet.run('echo out; echo err >&2', fd);
  assert.equal(await echoed.wait, 0);
  closeSync(fd);
  assert.equal(readFileSync(path, 'utf8'), 'kept\n=== a heading\nout\nerr\n');
  // A heading that cannot be written is its reason, never a throw.
  assert.match(String(sheet.heading(2 ** 30, 'x')), /^EBADF/, 'a descriptor no file has');
  // Refused: a directory, a named pipe nobody reads (at the open, never waited on) and one somebody does.
  assert.throws(() => sheet.openCommandLog(dir), /EISDIR/);
  const fifo = join(dir, 'log.fifo');
  assert.equal(spawnSync('mkfifo', [fifo]).status, 0);
  assert.throws(() => sheet.openCommandLog(fifo), /ENXIO/);
  const reader = openSync(fifo, c.O_RDONLY | c.O_NONBLOCK);
  try {
    assert.throws(() => sheet.openCommandLog(fifo), /^Error: not a regular file, and what the commands print is kept only in one$/);
  } finally {
    closeSync(reader);
  }
  // In a run: a log that cannot be opened refuses the writer and the restart by name.
  const noWriter = await sheet.runSheet({ ...argsFor(), writer: 'echo never', commandLog: dir }, { send: standIn({ ignore: new Set() }).send, log: () => {} });
  assert.ok(noWriter.failures.some((f) => f.startsWith(`the --writer command was not run: what it prints could not be kept in ${dir}: EISDIR`)), noWriter.failures.join('\n'));
  assert.ok(noWriter.notMeasured.includes('warm+writer: the --writer command was not run (its FAILED line says why)'), noWriter.notMeasured.join('\n'));
  assert.ok(!noWriter.rows.some((x) => x.phase === 'warm+writer'));
  assert.ok(!noWriter.notes.some((n) => n.includes('printed is kept')), noWriter.notes.join('\n'));
  const noRestart = await sheet.runSheet({ ...argsFor(), restart: 'echo never', restarts: 2, commandLog: dir }, { send: standIn({ ignore: new Set() }).send, log: () => {} });
  assert.ok(noRestart.failures.some((f) => f.startsWith(`restart 1: the --restart command was not run: what it prints could not be kept in ${dir}: EISDIR`)), noRestart.failures.join('\n'));
  assert.ok(noRestart.notMeasured.includes('first after restart: timed after 0 of the 2 restarts asked: the --restart command failed at restart 1'), noRestart.notMeasured.join('\n'));
  // How a writer that ended early ended is named: its exit code, or the signal that ended it.
  const exits = await sheet.runSheet({ ...argsFor(), clients: [1], writer: 'exit 3', commandLog: path }, { send: standIn({ ignore: new Set(), delayMs: 25 }).send, log: () => {} });
  assert.ok(exits.failures.includes('the --writer command ended before the warm+writer pass did: it exited 3, so part of that pass ran with nothing writing'), exits.failures.join('\n'));
  assert.ok(exits.notes.includes('the --writer command\'s process group had already left at the end of the warm+writer pass'), exits.notes.join('\n'));
  const piped = await sheet.runSheet({ ...argsFor(), clients: [1], writer: 'kill -PIPE $$', commandLog: path }, { send: standIn({ ignore: new Set(), delayMs: 25 }).send, log: () => {} });
  assert.ok(piped.failures.includes('the --writer command ended before the warm+writer pass did: it was ended by SIGPIPE, so part of that pass ran with nothing writing'), piped.failures.join('\n'));
  assert.match(readFileSync(path, 'utf8'), /\n=== \S+ the --writer command: exit 3\n=== \S+ the --writer command: kill -PIPE \$\$\n$/);
  // A heading that cannot be written (a log opened read-only here, as a failing disk would leave the writes)
  // refuses each command by name, adds nothing, and the file is not named as their log.
  const ro = join(dir, 'read-only.log');
  writeFileSync(ro, 'before\n');
  const roRun = await sheet.runSheet({ ...argsFor(), writer: 'echo never', restart: 'echo never', restarts: 1, commandLog: ro }, { send: standIn({ ignore: new Set() }).send, log: () => {}, openLog: (p) => openSync(p, 'r') });
  assert.ok(roRun.failures.some((f) => f.startsWith(`the --writer command was not run: what it prints could not be kept in ${ro}: EBADF`)), roRun.failures.join('\n'));
  assert.ok(roRun.failures.some((f) => f.startsWith(`restart 1: the --restart command was not run: what it prints could not be kept in ${ro}: EBADF`)), roRun.failures.join('\n'));
  assert.ok(!roRun.notes.some((n) => n.includes('printed is kept')), roRun.notes.join('\n'));
  assert.equal(readFileSync(ro, 'utf8'), 'before\n');
  // As a command: an unwritable --command-log is refused before the run, and only when a command is given;
  // with no --out and no --command-log, the log is made in the temporary directory.
  const start = join(dir, 'start');
  writeFileSync(start, '1');
  const site = await startedServer(start, 0);
  try {
    const argv = [SHEET, '--base', site.base, '--n', '1', '--clients', '1', ...CTX_ARGS];
    /** @param {string} file @param {string[]} args @param {NodeJS.ProcessEnv} [env] @returns {Promise<{code: number, stdout: string, stderr: string}>} */
    const exec = (file, args, env) => new Promise((resolve) => execFile(file, args, { env: env ?? process.env, timeout: 60_000 }, (error, stdout, stderr) => resolve({ code: error ? Number(error.code) : 0, stdout, stderr })));
    const missing = join(dir, 'missing', 'c.log');
    const refused = await exec(process.execPath, [...argv, '--writer', 'true', '--command-log', missing]);
    assert.equal(refused.code, 2, refused.stderr);
    assert.match(refused.stderr, /^speed-sheet: --command-log .*\/missing\/c\.log: .*\/missing cannot take a new file: ENOENT/);
    assert.equal(refused.stdout, '');
    const unused = await exec(process.execPath, [...argv, '--command-log', missing]);
    assert.equal(unused.code, 0, `${unused.stdout}${unused.stderr}`);
    const temp = await exec(process.execPath, [...argv, '--writer', 'echo in-the-temporary-directory; sleep 100'], { ...process.env, TMPDIR: dir });
    assert.equal(temp.code, 0, temp.stdout);
    const m = /\n- what the --writer command printed is kept in (\S+), each run under a line that names it\n/.exec(temp.stdout);
    assert.ok(m && m[1].startsWith(join(dir, 'speed-sheet-')) && m[1].endsWith('.commands.log'), temp.stdout);
    assert.match(readFileSync(m[1], 'utf8'), /^=== \S+ the --writer command: echo in-the-temporary-directory; sleep 100\nin-the-temporary-directory\n$/);
  } finally {
    site.close();
  }
});

test('no command the sheet or the harness starts is given either one\'s own stdout or stderr', () => {
  // Verify r9 (26 Oct): run() spawned with stdio ['ignore', 2, 2].
  for (const file of ['ci/speed-sheet.mjs', 'ci/browser-harness.mjs']) {
    const src = source(file);
    const spawns = src.match(/spawn\([^;]*\);/g) ?? [];
    assert.ok(spawns.length >= 1, file);
    for (const s of spawns) assert.doesNotMatch(s, /stdio:\s*(?:'inherit'|\[[^\]]*(?:\b[12]\b|'inherit')[^\]]*\])/, `${file}: ${s}`);
  }
  assert.match(source('ci/speed-sheet.mjs'), /spawn\(command, \{ shell: true, stdio: \['ignore', log, log\], detached: true \}\)/);
});
