// A VALUE SORT THAT READ ROWS SAYS SO UNDER THE GRID (D-3349, WP-39; DB-N14).
//
// `/bars/window.json` serves a price, volume or open-interest order from each
// month's `.srt` sort index and names, in `sort_index`, every row it had to
// read instead — the months, why, and the command that indexes them. These
// tests hold the sentence the grid shows for each shape of that report. No
// row here is market data.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { sortIndexNote } from '../src/lib/database-pages.js';

const report = (/** @type {Record<string, unknown>} */ over = {}) => ({
  sort_index: {
    column: 'c', by_time: false, indexed_rows: 4000, indexed_runs: 2, cut_runs: 0,
    scanned_rows: 0, scanned_months: 0, scanned_why: [], build: [], ...over
  }
});

test('every row served by an index says nothing; a time order says nothing', () => {
  assert.equal(sortIndexNote(report()), null);
  assert.equal(sortIndexNote({ sort_index: null }), null);
  assert.equal(sortIndexNote({}), null);
  assert.equal(sortIndexNote(null), null);
});

test('rows read instead are counted, their months named with why, and the build command given', () => {
  const note = sortIndexNote(report({
    indexed_rows: 43000, scanned_rows: 1200, scanned_months: 1,
    scanned_why: ['NSE/INDEX/WP39FIX 2031-02: 1200 rows read: the month has no .srt sort index'],
    build: ['cli srt-build gdfl NSE/INDEX/WP39FIX 1s 2031-02 2031-02']
  }));
  assert.equal(
    note,
    'Ordered by c: 43,000 row(s) came from sort indexes and 1,200 row(s) in 1 month(s) were read instead — ' +
      'NSE/INDEX/WP39FIX 2031-02: 1200 rows read: the month has no .srt sort index. ' +
      'Index them with: cli srt-build gdfl NSE/INDEX/WP39FIX 1s 2031-02 2031-02'
  );
});

test('months past the named ones are counted, never dropped', () => {
  const note = sortIndexNote(report({
    scanned_rows: 30, scanned_months: 14,
    scanned_why: ['m1: 10 rows read: a time-of-day bound keeps windows no index run is cut to']
  }));
  assert.match(note ?? '', /30 row\(s\) in 14 month\(s\) were read instead/);
  assert.match(note ?? '', /; and 13 more month\(s\)\.$/);
  assert.doesNotMatch(note ?? '', /Index them with/, 'a time-of-day bound has no command that indexes it');
});

test('open interest over series that carry none is said to be time order', () => {
  assert.match(sortIndexNote(report({ column: 'oi', by_time: true })) ?? '', /time order/);
});

test('the grid shows the note with the other notes under it', () => {
  const grid = readFileSync(new URL('../src/routes/db/DbBarsGrid.svelte', import.meta.url), 'utf8');
  assert.match(grid, /sortIndexNote\(barState\.answer\)/);
  assert.match(grid, /\{#if db\.barSortNote\}\s*<p class="bnote">\{db\.barSortNote\}<\/p>/);
});
