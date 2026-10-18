// R3-06, D-4980: /gaps says, per month, whether its answer was read off the
// month summary or from the bars, and why — never a silent slow path.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { readFrom } from '../src/lib/gap-verdict.js';

test('a kept summary reads as the summary and no bar', () => {
  const read = readFrom({ summary: 'kept', summary_fallback: null, bar_records_read: 0 });
  assert.equal(read.kind, 'kept');
  assert.equal(read.word, 'summary');
  assert.match(read.why, /no bar record read/);
});

test('a built month names the records it read and the reason', () => {
  const read = readFrom({
    summary: 'built',
    summary_fallback: 'absent: no month summary is kept beside the month',
    bar_records_read: 404352
  });
  assert.equal(read.kind, 'built');
  assert.equal(read.word, 'bars · 4,04,352 records');
  assert.equal(read.why, 'absent: no month summary is kept beside the month');
});

test('a built month with no reason or count is still named, never blank', () => {
  const read = readFrom({ summary: 'built' });
  assert.equal(read.word, 'bars');
  assert.match(read.why, /gave no reason/);
});

test('an absent file and an older server are told apart from both', () => {
  assert.equal(readFrom({ summary: 'absent_file' }).kind, 'none');
  for (const month of [{}, { summary: 'sideways' }]) {
    const read = readFrom(month);
    assert.equal(read.kind, 'unknown');
    assert.equal(read.word, 'not stated');
  }
});

test('the /gaps page shows how each month and the answer were read', () => {
  const page = readFileSync(new URL('../src/routes/gaps/+page.svelte', import.meta.url), 'utf8');
  assert.match(page, /readFrom\(m\)/, 'each month row names how it was read');
  assert.match(page, /<th>Read from<\/th>/);
  assert.match(page, /b\.summaries/, 'the census names months read off a summary');
});

test('a kept summary says a record damaged after it was written is not checked', () => {
  const read = readFrom({ summary: 'kept', summary_fallback: null, bar_records_read: 0 });
  assert.match(read.why, /not checked here/);
});
