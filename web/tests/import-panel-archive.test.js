// AH-04 (g-ui-dom-vs-json-9, part of g-ui-dom-vs-json-15; D-5881): one
// archive end for /ingest and /audit, the import journal's last day named
// apart, and a not-done list that renders a day named by more than one row.
// Every fixture is invented; no journal line or vendor row is copied.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { compile } from 'svelte/compiler';
import { render } from 'svelte/server';
import { archiveEnd, importPanel, notDoneRows, reach } from '../src/lib/import-panel.js';
import { feedSpan } from '../src/lib/ingest-months.js';

const audit = readFileSync(new URL('../src/routes/audit/+page.svelte', import.meta.url), 'utf8');
const ingest = readFileSync(new URL('../src/routes/ingest/+page.svelte', import.meta.url), 'utf8');
const panelSrc = readFileSync(new URL('../src/lib/ImportPanel.svelte', import.meta.url), 'utf8');

/** A `/feeds.json` GDFL descriptor, invented. @param {Record<string, unknown>} archive */
const gdfl = (archive) => ({ wire: 'gdfl', archive: { from: '2018-09-03', to: '2026-09-24', source: 'an invented source', ...archive } });

/** The rule /ingest applies inline, evaluated on the same descriptor. @param {any} feed */
const ingestRule = (feed) => {
  const activeSpan = feedSpan(feed);
  return activeSpan.kind === 'archive' && activeSpan.bindsTo ? activeSpan.to : null;
};

const summary = (/** @type {Record<string, unknown>} */ over = {}) => ({
  route: '/imports.json?feed=gdfl',
  journal: { present: true, path: '/store/imports/gdfl.journal', lines: 120 },
  span: { from: '2018-09-03', to: '2026-09-30' },
  counts: { done: 100, incomplete: 2, open: 1, refused: 0 },
  kinds: [
    { kind: 'indices', last_day: '2026-09-30', last_verb: 'done', last_line: 90 },
    { kind: 'options', last_day: '2020-09-15', last_verb: 'done', last_line: 119 }
  ],
  ...over
});

/** Compile Svelte source to a server module whose imports resolve from here. @param {string} src @param {string} name */
async function serverComponent(src, name) {
  const lib = new URL('../src/lib/', import.meta.url).href;
  const code = compile(src, { generate: 'server', filename: name }).js.code
    .replace(/from '(svelte[^']*)'/g, (_, spec) => `from '${import.meta.resolve(spec)}'`)
    .replace(/from '\$lib\/([^']+)'/g, (_, file) => `from '${lib}${file}'`);
  return (await import(`data:text/javascript,${encodeURIComponent(code)}`)).default;
}

test('import_panel_names_the_archive_end_and_the_journal_day_apart', async () => {
  // ONE RULE, BOTH PAGES: every descriptor shape gives /audit's archive end
  // the value /ingest's inline rule gives.
  const shapes = [
    gdfl({ to_binds: false }), // R3-05: a dated measurement, an open end
    gdfl({ to_binds: true }),
    gdfl({}), // an older server: both ends bind
    { wire: 'gdfl', archive: null }, // no archive span stated
    { wire: 'zerodha', window: { from: '2018-01-01', to: '2026-09-30', source: 'settings' } },
    null
  ];
  for (const feed of shapes) assert.equal(archiveEnd(feed), ingestRule(feed), JSON.stringify(feed));
  assert.equal(archiveEnd(gdfl({ to_binds: false })), null);
  assert.equal(archiveEnd(gdfl({ to_binds: true })), '2026-09-24');
  assert.equal(archiveEnd({ wire: 'gdfl', archive: null }), null, 'an archive span absent from feeds.json states no end');

  // The pages hand the panel exactly that: /ingest its inline rule, /audit
  // the shared helper over the same /feeds.json descriptor, and NOT the
  // import journal's last day it passed before.
  assert.match(ingest, /archiveTo=\{activeSpan\.kind === 'archive' && activeSpan\.bindsTo \? activeSpan\.to : null\}/);
  assert.match(audit, /archiveTo=\{archiveEnd\(feeds\.all\.find\(\(f\) => f\.wire === feeds\.active\) \?\? null\)\}/);
  assert.doesNotMatch(audit, /archiveTo=\{payload\.imports\?\.span\?\.to/);

  // The journal's last day is its own fact, compared, never passed off as the end.
  const p = importPanel(summary(), null, { now: 0, archiveTo: '2026-09-24' });
  assert.deepEqual([p.archiveTo, p.journalTo, p.reach], ['2026-09-24', '2026-09-30', 'after']);
  assert.equal(reach('2026-09-01', '2026-09-24'), 'before', 'a journal last day before the archive end');
  assert.equal(reach('2026-09-24', '2026-09-24'), 'at');
  assert.equal(reach(null, '2026-09-24'), null, 'an empty journal states no day');
  assert.equal(reach('2026-09-30', null), null);
  assert.equal(importPanel(summary({ span: null }), null, { now: 0 }).journalTo, null);
  assert.equal(importPanel(summary({ span: { to: 'yesterday' } }), null, { now: 0 }).journalTo, null);

  // RENDERED: given one archive end, both pages draw the same sentence, and
  // the journal's day is named apart.
  const Panel = await serverComponent(panelSrc, 'ImportPanel.svelte');
  const bound = render(Panel, { props: { summary: summary(), archiveTo: '2026-09-24', now: 0 } }).body;
  assert.match(bound, /of an archive ending <span class="mono[^"]*">2026-09-24<\/span>/);
  assert.match(bound, /The import journal reaches <span class="mono[^"]*">2026-09-30<\/span>, past the archive end/);
  const open = render(Panel, { props: { summary: summary(), archiveTo: null, now: 0 } }).body;
  assert.doesNotMatch(open, /an archive ending/);
  assert.match(open, /\/feeds\.json states no binding archive end/);
  assert.match(open, /The import journal reaches <span class="mono[^"]*">2026-09-30<\/span> \(its last imported day/);
});

test('the_not_done_list_renders_a_day_named_by_two_filters', async () => {
  // Two `incomplete` rows under different filters and one `open` row, all
  // `options|2020-09-16`: the shape /imports.json answered on /ingest.
  const rows = [
    { kind: 'options', day: '2020-09-16', outcome: 'incomplete', filter: 'NIFTY*', line: 4101, reason: 'stopped' },
    { kind: 'options', day: '2020-09-16', outcome: 'incomplete', filter: '*', line: 4100 },
    { kind: 'options', day: '2020-09-16', outcome: 'open', filter: '*', line: 4102 }
  ];
  const out = notDoneRows(rows);
  assert.equal(out.length, 3);
  assert.equal(new Set(out.map((r) => r.id)).size, 3, 'no two rows share a key');
  assert.deepEqual(out.map((r) => r.key), Array(3).fill('options|2020-09-16'), 'the day key is still the day');
  // A row equal to another in every field still renders, numbered.
  const twins = notDoneRows([rows[0], rows[0], rows[0]]);
  assert.deepEqual(twins.map((r) => r.id.endsWith('#2')), [false, true, false]);
  assert.equal(new Set(twins.map((r) => r.id)).size, 3);
  // Extremes: none, garbage, and 500.
  assert.deepEqual(notDoneRows([]), []);
  assert.deepEqual(notDoneRows(/** @type {any} */ (null)), []);
  const many = notDoneRows(Array.from({ length: 500 }, (_, i) => ({ kind: 'options', day: '2020-09-16', outcome: 'incomplete', filter: '*', line: i % 2 ? null : 7 })));
  assert.equal(new Set(many.map((r) => r.id)).size, 500);

  // The list keys by that id, and every row renders with its filter.
  assert.match(panelSrc, /const days = \$derived\(notDoneRows\(notDone\)\);/);
  assert.match(panelSrc, /\{#each days as d \(d\.id\)\}/);
  const Panel = await serverComponent(panelSrc, 'ImportPanel.svelte');
  const html = render(Panel, { props: { summary: summary(), notDone: rows, now: 0 } }).body;
  assert.equal((html.match(/<span class="mono[^"]*">2020-09-16<\/span> options/g) ?? []).length, 3, html);
  assert.match(html, /3 day\(s\) not done/);
  assert.match(html, / · filter NIFTY\* · line 4101/);
});
