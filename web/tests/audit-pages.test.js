import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import { mergePages, nextOrdinal, pageFor } from '../src/lib/audit-pages.js';

const PER = 200;

/** What the server returns for `page` when the journal holds `total`. */
function serve(page, total) {
  const end = Math.max(0, total - page * PER);
  const start = Math.max(0, end - PER);
  const rows = [];
  for (let o = end - 1; o >= start; o -= 1) rows.push({ ordinal: o });
  return rows;
}

test('growth between reads leaves a named gap instead of a silent hole (P1-06-03)', () => {
  // 450 records: page 0 [250,450), then "load older" reads page 1 [50,250).
  const older = serve(1, 450);
  // A pull appends 30; page 0 refreshes to [280,480). [250,280) is on neither.
  const head = serve(0, 480);
  const merged = mergePages(head, older);
  assert.deepEqual(merged.gaps, [{ from: 250, to: 279 }]);
  // The next read targets the newest missing ordinal, on the page that holds
  // it for the total the server last reported.
  const want = nextOrdinal(merged);
  assert.equal(want, 279);
  const page = pageFor(/** @type {number} */ (want), 480, PER);
  assert.equal(page, 1);
  const filled = mergePages(head, [...older, ...serve(/** @type {number} */ (page), 480)]);
  assert.deepEqual(filled.gaps, []);
  assert.equal(filled.runs.length, 480 - 50);
});

test('an overlapping older page never repeats an ordinal (P1-06-03)', () => {
  // Page 0 read at 450 records; page 1 read after growth to 480 overlaps it.
  const head = serve(0, 450);
  const older = serve(1, 480);
  const merged = mergePages(head, older);
  const ordinals = merged.runs.map((r) => r.ordinal);
  assert.equal(new Set(ordinals).size, ordinals.length, 'no ordinal twice');
  assert.deepEqual(merged.gaps, []);
  assert.ok(ordinals.every((o, i) => i === 0 || ordinals[i - 1] > o), 'newest first');
});

test('the next read below a contiguous set is the oldest held minus one, and none below zero', () => {
  assert.equal(nextOrdinal(mergePages(serve(0, 450), [])), 249);
  assert.equal(nextOrdinal(mergePages(serve(0, 150), [])), null);
  assert.equal(pageFor(0, 0, PER), null);
  assert.equal(pageFor(449, 450, PER), 0);
  assert.equal(pageFor(250, 450, PER), 0);
  assert.equal(pageFor(249, 450, PER), 1);
  assert.equal(pageFor(450, 450, PER), null);
});

test('the audit page merges by ordinal and names a gap', () => {
  const page = readFileSync(new URL('../src/routes/audit/+page.svelte', import.meta.url), 'utf8');
  assert.match(page, /mergePages\(/);
  assert.match(page, /pageFor\(/);
  assert.doesNotMatch(page, /\[\.\.\.\(payload\?\.runs \?\? \[\]\), \.\.\.older\]/, 'no positional concatenation');
});
