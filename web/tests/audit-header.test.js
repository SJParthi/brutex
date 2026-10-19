// AH-04 (g-ui-dom-vs-json-4, D-5880): the /audit header counts this feed's
// runs, and the "Last commit" cell says what its stamp stands on. Invented.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { commitLine, journalHeader, sinceCommit } from '../src/lib/audit-header.js';

const page = readFileSync(new URL('../src/routes/audit/+page.svelte', import.meta.url), 'utf8');
const ist = (/** @type {number} */ s) => `T${s}`;

test('the_journal_header_names_feed_scoped_records_and_runs', () => {
  const journal = { records: 4_217_222, feed_scope: { feed: 'gdfl', records: 0, runs: 0, unattributed_records: 12, error: null } };
  const h = journalHeader(journal);
  assert.equal(h.text, 'gdfl: 0 runs · 0 records', 'this feed\'s runs and records, never the whole journal called runs');
  assert.match(h.title, /42,17,222 records over every feed, 12 of them unattributed/);
  assert.equal(journalHeader({ records: 410, feed_scope: { feed: 'zerodha', records: 410, runs: 1 } }).text, 'zerodha: 1 runs · 410 records');
  // A scope that failed, or is absent, is said; never a zero.
  assert.equal(journalHeader({ records: 5, feed_scope: { feed: 'gdfl', records: 0, runs: 0, error: 'the index is torn' } }).text, 'gdfl: runs not counted');
  assert.match(journalHeader({ records: 5, feed_scope: { feed: 'gdfl', error: 'the index is torn' } }).title, /the index is torn/);
  assert.equal(journalHeader({ records: 5 }).text, 'this feed: runs not counted');
  assert.equal(journalHeader(null).text, 'this feed: runs not counted');
  // An empty journal is a measured zero.
  assert.equal(journalHeader({ records: 0, feed_scope: { feed: 'gdfl', records: 0, runs: 0 } }).text, 'gdfl: 0 runs · 0 records');
  // The page draws it, and no longer prints the all-feed record count as runs.
  assert.match(page, /\{@const header = journalHeader\(payload\.journal\)\}/);
  assert.match(page, /<span class="txt">\{header\.text\}<\/span>/);
  assert.doesNotMatch(page, /\{n0\(payload\.journal\.records\)\} runs/);
});

test('the_last_commit_cell_names_the_newest_census_file_a_lower_bound_and_a_clock_skew', () => {
  const files = [
    { segment: 'INDEX', state: 'read', at: 100 },
    { segment: 'CASH', state: 'read', at: 300 },
    { segment: 'FNO', state: 'read', at: 200 }
  ];
  const store = { committed_at: 300, committed_from: { basis: 'newest census file', complete: true, files, note: null } };
  assert.equal(commitLine(store, 400, ist), 'T300 IST · newest of 3 current census file(s)');
  assert.doesNotMatch(commitLine(store, 400, ist), /manifest mtime/);
  const partial = { committed_at: 200, committed_from: { basis: 'newest census file', complete: false, files: [files[0], { segment: 'CASH', state: 'unread' }, files[2]], note: '1 of 3 segment epoch(s) could not be stat\'ed (CASH)' } };
  assert.match(commitLine(partial, 400, ist), /newest of 2 current census file\(s\) · AT LEAST this recent: 1 of 3/);
  assert.match(commitLine(store, 250, ist), /50 s AHEAD of the server's clock/);
  assert.equal(commitLine({ committed_at: 7, committed_from: { basis: 'manifest file', complete: true, files: [] } }, 9, ist), 'T7 IST · the census file');
  assert.match(commitLine({ committed_at: null, committed_from: { basis: 'none', note: 'a directory\'s mtime is not a commit' } }, 9, ist), /^unknown: a directory/);
  assert.match(commitLine({}, 9, ist), /^unknown/);
  // A commit later than the server's clock has no age, so it is never "just wrote".
  assert.equal(sinceCommit(store, 250), null);
  assert.equal(sinceCommit(store, 345), 45);
  assert.equal(sinceCommit({ committed_at: null }, 345), null);
  assert.match(page, /const sinceCommit = commitAge\(store, now\);/);
  assert.match(page, /\{commitLine\(payload\.store, payload\.at \?\? null, istStamp\)\}/);
});
