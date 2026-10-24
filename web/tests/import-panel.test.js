// WP-61: the import journal as /ingest and /audit draw it (DB-G07, DB-G22,
// DB-A51). Every fixture is invented; no journal line or vendor row is copied.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { compile } from 'svelte/compiler';
import { render } from 'svelte/server';
import { readReasons, REASONS_SCHEMA } from '../src/lib/reasons.js';
import {
  ACTIVE_WITHIN_MS, censusRefusedSentence, dayRow, extrapolated, importPanel, importRoute,
  journalStep, kindLine, newestImportAt, nextDay, notDoneRows, OUTCOMES, outcomeWord, readImportExtras, reasonOf, span
} from '../src/lib/import-panel.js';

const NOW = 1_800_000_000_000;
const summary = (/** @type {Record<string, unknown>} */ over = {}) => ({
  route: '/imports.json?feed=gdfl',
  journal: { present: true, path: '/store/imports/gdfl.journal', lines: 120 },
  span: { from: '2018-09-03', to: '2026-09-24' },
  counts: { done: 100, done_with_refusals: 0, incomplete: 3, open: 1, refused: 2 },
  census_refused: { count: 0, days: [] },
  census_refused_then_done: { count: 7, days: ['options 2019-11-14'] },
  kinds: [
    { kind: 'indices', last_day: '2026-09-24', last_verb: 'done', last_line: 90, pace: { p50_ms_per_day: 80, p90_ms_per_day: 120, days_per_hour_at_p50: 45_000, newest_at_ms: NOW - 3_600_000 } },
    { kind: 'options', last_day: '2020-09-15', last_verb: 'recounted', last_line: 119, pace: { p50_ms_per_day: 44_000, p90_ms_per_day: 62_000, days_per_hour_at_p50: 82, newest_at_ms: NOW - 60_000 } }
  ],
  newest_not_done: { kind: 'indices', day: '2025-10-21', outcome: 'incomplete' },
  runs: [{ id: 7, line: 118, fields: { source: 'zip', started: '1791602895' } }],
  runs_total: 2,
  notes: ['an invented note'],
  ...over
});
const store = { capacity: { limit: 1_048_576, committed: 20_000, free: 1_028_576, full: false, segment: 'CASH' }, headroom: 1_028_576 };

test('DB-G22: kinds, options progress with a labelled ETA, the not-done count and the census capacity', () => {
  const panel = importPanel(summary(), store, { now: NOW, archiveTo: '2026-09-24', optionsLeft: 1_500 });
  assert.equal(panel.state, 'active', 'an options day was filed a minute ago');
  assert.match(panel.why, /filed a options day 1m 0s ago/);
  assert.deepEqual(panel.kinds.map((k) => [k.kind, k.lastDay, k.lastVerb, k.active]), [
    ['indices', '2026-09-24', 'done', false],
    ['options', '2020-09-15', 'recounted', true]
  ]);
  assert.equal(panel.options?.lastDay, '2020-09-15');
  assert.equal(panel.notDone, 6, 'incomplete + refused + open');
  assert.equal(panel.eta?.ms, 1_500 * 44_000);
  assert.match(panel.eta?.text ?? '', /EXTRAPOLATED: 1,500 day\(s\) times the measured p50 of 44s per day/);
  assert.deepEqual(panel.capacity, { limit: 1_048_576, committed: 20_000, free: 1_028_576, full: false, segment: 'CASH', headroom: 1_028_576 });
  assert.deepEqual(panel.censusRefused, { count: 0, days: [], thenDone: 7 });
  assert.equal(panel.runsTotal, 2);
  assert.equal(panel.newestNotDone?.day, '2025-10-21');
});

test('DB-B78 in part: a stopped import is "quiet" with the age of its newest event, never "importing"', () => {
  const stale = summary({
    kinds: [{ kind: 'options', last_day: '2020-09-15', last_verb: 'open', pace: { p50_ms_per_day: 44_000, newest_at_ms: NOW - 3 * 86_400_000 - 36_000_000 } }]
  });
  const panel = importPanel(stale, null, { now: NOW });
  assert.equal(panel.state, 'quiet');
  assert.match(panel.why, /no 'day imported' event for 3d 10h/);
  assert.equal(panel.capacity, null, 'no store block, no capacity claimed');
  assert.equal(panel.eta, null, 'days left unread: no ETA, never a zero');
  // The edge of "active".
  const edge = (/** @type {number} */ age) => kindLine({ kind: 'stocks', pace: { newest_at_ms: NOW - age } }, NOW).active;
  assert.equal(edge(ACTIVE_WITHIN_MS), true);
  assert.equal(edge(ACTIVE_WITHIN_MS + 1), false);
  assert.equal(kindLine({ kind: 'stocks' }, NOW).active, false, 'no event is not an active import');
  assert.equal(kindLine({ kind: 'stocks' }, NOW).age, null);
});

test('unread, erroring, absent and empty summaries are said, never drawn as an empty import', () => {
  assert.equal(importPanel(null, null, { now: NOW }).state, 'unread');
  assert.equal(importPanel('x', null, { now: NOW }).state, 'unread');
  const failed = importPanel({ error: 'the journal is torn at line 9' }, null, { now: NOW });
  assert.deepEqual([failed.state, failed.why], ['error', 'the journal is torn at line 9']);
  const absent = importPanel({ journal: { present: false, path: '/store/imports/x.journal' } }, null, { now: NOW });
  assert.equal(absent.state, 'absent');
  assert.match(absent.why, /no import journal at \/store\/imports\/x\.journal/);
  assert.doesNotMatch(absent.why, /GDFL/, 'the sentence names the journal, not a vendor');
  const empty = importPanel({ journal: { present: true }, kinds: [] }, null, { now: NOW });
  assert.equal(empty.state, 'quiet');
  assert.match(empty.why, /no import event is in the log/);
});

test('an ETA exists only for a whole day count and a measured pace', () => {
  assert.equal(extrapolated(null, 100), null);
  assert.equal(extrapolated(5, null), null);
  assert.equal(extrapolated(5, 0), null);
  assert.equal(extrapolated(-1, 100), null);
  assert.equal(extrapolated(1.5, 100), null);
  assert.equal(extrapolated(0, 100)?.ms, 0);
  assert.match(extrapolated(2, 30_000)?.text ?? '', /about 1m 0s left — EXTRAPOLATED/);
  assert.equal(span(-1), '—');
  assert.equal(span(90_061_000), '1d 1h');
  assert.equal(span(80), '80ms', 'a pace under a second is never "0s"');
  assert.equal(span(0), '0s');
  assert.equal(span(999.6), '1000ms');
  assert.equal(nextDay('2020-12-31'), '2021-01-01');
  assert.equal(nextDay('2020-02-28'), '2020-02-29');
  assert.equal(nextDay('31/12/2020'), null);
  assert.equal(nextDay(null), null);
});

test('a not-done day carries its stage, reason and the importer\'s own words', () => {
  const row = dayRow({
    kind: 'indices', day: '2025-10-21', outcome: 'incomplete', stage: 'calendar', reason: 'special_session', files: 8, failures: 2,
    logged: [{ about: 'calendar', why: 'an invented reason' }, { about: null, why: 'a second one' }, { message: 'no why' }]
  });
  assert.deepEqual([row.key, row.stage, row.reason, row.files, row.failures, row.unsettled], ['indices|2025-10-21', 'calendar', 'special_session', 8, 2, false]);
  assert.deepEqual(row.words, ['calendar: an invented reason', 'a second one']);
  assert.equal(dayRow({ kind: 'options', day: '2019-01-01', outcome: 'open' }).unsettled, true, 'a line with no stage says so');
  assert.equal(censusRefusedSentence({ count: 0, days: [], thenDone: 0 }), 'The census refused no import day.');
  assert.match(censusRefusedSentence({ count: 2, days: ['options 2019-11-14', 'options 2019-11-15'], thenDone: 210 }), /2 day\(s\) are refused by the census now \(options 2019-11-14 … options 2019-11-15\); 210 day\(s\)/);
  assert.match(censusRefusedSentence({ count: 0, days: [], thenDone: 210 }), /No day is refused by the census now; 210 day\(s\)/);
});

test('DB-G07: the pulse moves when a journal moves, even while the census stands still', () => {
  const at = (/** @type {number | null} */ journal, /** @type {number | null} */ importLines, /** @type {number | null} */ importAt) => ({ journal, importLines, importAt });
  assert.deepEqual(journalStep(at(10, 5, 100), at(10, 5, 100)), { moved: false, journal: 0, importLines: 0, importAt: null, shrank: '' });
  assert.deepEqual(journalStep(at(10, 5, 100), at(13, 5, 100)), { moved: true, journal: 3, importLines: 0, importAt: null, shrank: '' });
  assert.deepEqual(journalStep(at(10, 5, 100), at(10, 9, 100)), { moved: true, journal: 0, importLines: 4, importAt: null, shrank: '' });
  assert.deepEqual(journalStep(at(10, 5, 100), at(10, 5, 160)), { moved: true, journal: 0, importLines: 0, importAt: 160, shrank: '' });
  assert.equal(journalStep(at(10, 5, null), at(10, 5, 160)).moved, true, 'a first event is movement');
  assert.equal(journalStep(undefined, at(1, 1, 1)).moved, false, 'one answer measures nothing');
  assert.equal(journalStep(at(null, null, null), at(4, 4, null)).moved, false, 'an unknown count is not a zero to grow from');
  assert.equal(journalStep(at(10, 5, 100), at(8, 5, 100)).journal, 0, 'a shrink is not negative growth');
  // AND A SHRINK IS SAID (WP-61 repair): an append-only journal that got
  // shorter was clamped to "not moved" with no word.
  assert.deepEqual(journalStep(at(10, 5, 100), at(8, 5, 100)), {
    moved: false, journal: 0, importLines: 0, importAt: null, shrank: 'the pull journal’s record count fell from 10 to 8'
  });
  assert.equal(journalStep(at(1_200_000, 9, 100), at(1_200_004, 2, 100)).shrank, 'the import journal’s line count fell from 9 to 2');
  assert.equal(journalStep(at(10, 5, 100), at(0, 0, 100)).shrank, 'the pull journal’s record count fell from 10 to 0; the import journal’s line count fell from 5 to 0');
  assert.equal(journalStep(at(null, 5, 100), at(3, 5, 100)).shrank, '', 'an unknown count did not fall');
  assert.equal(journalStep(undefined, at(1, 1, 1)).shrank, '');
  {
    const page = readFileSync(new URL('../src/routes/audit/+page.svelte', import.meta.url), 'utf8');
    assert.match(page, /shrank: jStep\.shrank \|\| jSpan\.shrank/);
    assert.match(page, /\{#if pulse\.journals\.shrank\}[\s\S]{0,160}role="alert"/);
  }
  assert.equal(newestImportAt({ imports: { kinds: [{ pace: { newest_at_ms: 5 } }, { pace: { newest_at_ms: 9 } }, { pace: {} }] } }), 9);
  assert.equal(newestImportAt({}), null);
  const audit = readFileSync(new URL('../src/routes/audit/+page.svelte', import.meta.url), 'utf8');
  assert.match(audit, /\|\| journalsMoved\) state = 'moving'/);
  assert.match(audit, /<ImportPanel\s+summary=\{payload\.imports\}/);
  assert.match(audit, /data-audit-last-run=\{lastRunSays\.state\}/);
  assert.match(audit, /data-audit-feed-scope=/);
});

test('the not-done days are read from the route the summary names, bounded, and a failure is named', async () => {
  assert.equal(importRoute(summary()), '/imports.json?feed=gdfl');
  assert.equal(importRoute({ route: '/imports.json?feed=other_feed-2' }), '/imports.json?feed=other_feed-2');
  for (const bad of [null, {}, { route: 7 }, { route: '/store.json?feed=gdfl' }, { route: '/imports.json?feed=gdfl&outcome=open' }, { route: 'https://elsewhere.example/imports.json?feed=gdfl' }]) {
    assert.equal(importRoute(bad), null, JSON.stringify(bad));
  }
  /** @type {string[]} */
  const asked = [];
  const pages = /** @type {Record<string, any>} */ ({
    '/imports.json?feed=gdfl&outcome=incomplete': { rows: [{ kind: 'indices', day: '2025-10-21' }, { kind: 'stocks', day: '2025-10-21' }], rows_in_range: 5 },
    '/imports.json?feed=gdfl&outcome=refused': { rows: [{ kind: 'options', day: '2026-01-02' }], rows_in_range: 1 },
    '/imports.json?feed=gdfl&outcome=open': { rows: [], rows_in_range: 0 },
    '/imports.json?feed=gdfl&kind=indices&from=2020-09-16&to=2026-09-24': { rows: [], rows_in_range: 1_497 }
  });
  const request = async (/** @type {string} */ url) => {
    asked.push(url);
    const body = pages[url];
    return /** @type {any} */ ({ ok: body !== undefined, status: body === undefined ? 404 : 200, json: async () => body ?? { error: 'no such page' } });
  };
  const got = await readImportExtras(request, summary());
  assert.deepEqual(asked, Object.keys(pages), 'four small reads, each the summary\'s own route');
  assert.deepEqual(got.notDone?.map((r) => `${r.day} ${r.kind}`), ['2026-01-02 options', '2025-10-21 indices', '2025-10-21 stocks'], 'newest day first, then by kind');
  assert.equal(got.more, 3, 'rows past the page are counted, never dropped silently');
  assert.equal(got.optionsLeft, 1_497);
  assert.equal(got.notDoneError, null);
  // The options have reached the archive's end: nothing is left, and no read is made for it.
  asked.length = 0;
  const done = await readImportExtras(request, summary({ kinds: [{ kind: 'options', last_day: '2026-09-24' }] }));
  assert.equal(done.optionsLeft, 0);
  assert.equal(asked.length, 3);
  // A page that fails is named and the days are unknown, not an empty list.
  const broken = await readImportExtras(async () => /** @type {any} */ ({ ok: false, status: 503, json: async () => ({ error: 'the journal is being read' }) }), summary());
  assert.equal(broken.notDone, null);
  assert.match(broken.notDoneError ?? '', /\/imports\.json\?feed=gdfl&outcome=incomplete: the journal is being read/);
  // A summary that names no route is said, and nothing is asked.
  let calls = 0;
  const none = await readImportExtras(async () => { calls += 1; return /** @type {any} */ ({}); }, { kinds: [] });
  assert.equal(calls, 0);
  assert.equal(none.notDone, null);
  assert.match(none.notDoneError ?? '', /names no \/imports\.json route/);
});

// ── R4-05: the reasons table titles the panel (READ-02), and an unknown state
//    is never OK (FM-280). The table body is invented in /reasons.json's shape.

const panelSrc = readFileSync(new URL('../src/lib/ImportPanel.svelte', import.meta.url), 'utf8');

/** The panel compiled for the server, its `$lib` imports pointed at the files. @param {string} src */
async function serverPanel(src) {
  const lib = new URL('../src/lib/', import.meta.url).href;
  const code = compile(src, { generate: 'server', filename: 'ImportPanel.svelte' }).js.code
    .replace(/from '(svelte[^']*)'/g, (_, spec) => `from '${import.meta.resolve(spec)}'`)
    .replace(/from '\$lib\/([^']+)'/g, (_, file) => `from '${lib}${file}'`);
  return (await import(`data:text/javascript,${encodeURIComponent(code)}`)).default;
}

const reasonsTable = (/** @type {Record<string, unknown>} */ over = {}) => readReasons({
  schema: REASONS_SCHEMA,
  build: 'unstamped',
  count: 2,
  title_cut: 200,
  rows: [
    {
      code: 'needs_month_generation', sentence: 'the month already passed this day', control: 'press Repair',
      control_kind: 'repair', step: null, action: 'run the import with a repair reason for this month',
      title: 'needs_month_generation: the month already passed this day; press Repair'
    },
    {
      code: 'stopped', sentence: 'the run was asked to stop, and stopped between two files', control: 'operator: run again to resume',
      control_kind: 'operator', step: null, action: 'run the import again; it resumes from this day',
      title: 'stopped: the run was asked to stop, and stopped between two files; operator: run again to resume'
    }
  ],
  ...over
});

test('the_import_panel_titles_a_reason_from_the_table', () => {
  const table = reasonsTable();
  const row = dayRow({ kind: 'options', day: '2020-09-16', outcome: 'incomplete', stage: 'file', reason: 'needs_month_generation' }, table);
  // The acceptance: the sentence on hover beside the code, the action as the control's label.
  assert.deepEqual(row.reasonInfo, {
    code: 'needs_month_generation',
    known: true,
    title: 'needs_month_generation: the month already passed this day; press Repair',
    action: 'press Repair',
    words: 'run the import with a repair reason for this month'
  });
  assert.equal(row.reason, 'needs_month_generation', 'the code itself is kept as the journal wrote it');
  // Before the table lands: the code alone, nothing claimed, no "loading".
  assert.deepEqual(reasonOf('needs_month_generation', null), {
    code: 'needs_month_generation', known: false, title: 'needs_month_generation', action: null, words: null
  });
  assert.equal(dayRow({ reason: 'stopped' }).reasonInfo?.title, 'stopped');
  // A code the loaded table has no row for says so (the gate in pull fails on
  // the server side for such a code).
  assert.equal(
    reasonOf('zz_unnamed', table).title,
    'zz_unnamed: no sentence for this code is served by this build (/reasons.json), so the code is shown as the server sent it'
  );
  // A table that could not be read names why, beside the code.
  const broken = readReasons({ schema: 'reasons-v9' });
  assert.equal(
    reasonOf('stopped', broken).title,
    'stopped: the reasons table could not be read (unknown reasons schema "reasons-v9", page older than server), so the code is shown as the server sent it'
  );
  // No reason, no info; the newest not-done day is titled the same way.
  assert.equal(dayRow({ kind: 'indices', day: '2025-10-21', outcome: 'refused' }).reasonInfo, null);
  const panel = importPanel(summary({ newest_not_done: { kind: 'options', day: '2020-09-16', outcome: 'incomplete', reason: 'stopped' } }), store, { now: NOW, reasons: table });
  assert.equal(panel.newestNotDoneReason?.action, 'operator: run again to resume');
  assert.equal(importPanel(summary(), store, { now: NOW }).newestNotDoneReason, null, 'a newest day with no reason');
  // notDoneRows passes the table to every row.
  assert.equal(notDoneRows([{ reason: 'stopped' }, { reason: 'zz_unnamed' }], table).map((r) => r.reasonInfo?.known).join(), 'true,false');
  // The panel draws the code with its title and the control beside it, and
  // adds no paragraph: the reason sits inside its day's list item.
  assert.match(panelSrc, /<span\s+class="code"\s+title=\{why\.title\}>\{why\.code\}<\/span/);
  assert.match(panelSrc, /<span class="act" title=\{why\.words \?\? ''\}>\{why\.action\}<\/span>/);
  assert.match(panelSrc, /loadReasons\(fetch\)/);
  assert.doesNotMatch(panelSrc, /loading/i);
});

test('an_unknown_stage_state_is_shown_as_unknown_never_as_ok', async () => {
  assert.deepEqual(OUTCOMES, ['done', 'done_with_refusals', 'incomplete', 'refused', 'open']);
  for (const known of OUTCOMES) assert.equal(outcomeWord(known).known, true, known);
  assert.deepEqual(outcomeWord('done'), { word: 'done', known: true, ok: true, text: 'done' });
  assert.deepEqual(outcomeWord('done_with_refusals'), { word: 'done_with_refusals', known: true, ok: false, text: 'done with refusals' });
  // A state a later server names: said as unknown, never OK, whatever its count.
  for (const later of ['month_superseded', 'awaiting_evidence', 'waiting_on_sign_in', 'partial_press', 'DONE', 'done ']) {
    const said = outcomeWord(later);
    assert.deepEqual(said, { word: later, known: false, ok: false, text: `unknown state ${later}, page older than server` });
  }
  // Garbage: an absent or a non-string outcome.
  assert.equal(outcomeWord(undefined).text, 'unknown state (none), page older than server');
  assert.equal(outcomeWord(null).ok, false);
  assert.equal(outcomeWord(7).text, 'unknown state 7, page older than server');
  // COMBINATION: a later server's counts beside the known ones, one of them
  // at zero: the unknown cell is bad even at 0; `done` is the only OK one.
  const panel = importPanel(summary({ counts: { done: 5, month_superseded: 0, awaiting_evidence: 3, open: 0 } }), store, { now: NOW });
  assert.deepEqual(panel.cells.map((c) => [c.word, c.n, c.bad, c.text]), [
    ['done', 5, false, 'done'],
    ['month_superseded', 0, true, 'unknown state month_superseded, page older than server'],
    ['awaiting_evidence', 3, true, 'unknown state awaiting_evidence, page older than server'],
    ['open', 0, false, 'open']
  ]);
  assert.deepEqual(importPanel(summary({ counts: null }), store, { now: NOW }).cells, []);
  // A row and the newest not-done day in such a state.
  const row = dayRow({ kind: 'options', day: '2020-09-16', outcome: 'month_superseded', reason: 'needs_month_generation' });
  assert.equal(row.outcomeInfo.known, false);
  assert.equal(row.outcomeInfo.text, 'unknown state month_superseded, page older than server');
  const newest = importPanel(summary({ newest_not_done: { kind: 'options', day: '2020-09-16', outcome: 'awaiting_evidence' } }), store, { now: NOW });
  assert.equal(newest.newestNotDoneOutcome?.text, 'unknown state awaiting_evidence, page older than server');
  // The page as the server renders it: the words are on the page, in red.
  const Panel = await serverPanel(panelSrc);
  const html = render(Panel, {
    props: {
      summary: summary({ counts: { done: 5, month_superseded: 2 }, newest_not_done: { kind: 'options', day: '2020-09-16', outcome: 'awaiting_evidence' } }),
      notDone: [{ kind: 'options', day: '2020-09-16', outcome: 'partial_press', reason: 'stopped' }],
      now: 0
    }
  }).body;
  assert.match(html, /<span class="cell[^"]*\bbad\b[^"]*" data-outcome-known="false"><b>2<\/b> unknown state month_superseded, page older than server<\/span>/);
  assert.match(html, /<span class="cell[^"]*" data-outcome-known="true"><b>5<\/b> done<\/span>/);
  assert.doesNotMatch(html, /<span class="cell[^"]*\bbad\b[^"]*" data-outcome-known="true">/);
  assert.match(html, /unknown state awaiting_evidence, page older than server/);
  assert.match(html, /<b class="[^"]*\bbad\b[^"]*">unknown state partial_press, page older than server<\/b>/);
  // Server-side there is no table yet: the reason is its code, titled by itself.
  assert.match(html, /<span class="code[^"]*" title="stopped">stopped<\/span>/);
});
