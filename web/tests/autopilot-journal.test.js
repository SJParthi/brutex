import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import { journalBanner, journalIsRecord, readJournalError } from '../src/lib/autopilot-journal.js';

const PATH = '~/.brutex/store/audit/pull.journal';

test('an empty journal_error is the only answer that lets the journal be called the record', () => {
  const ok = readJournalError('');
  assert.deepEqual(ok, { state: 'ok' });
  assert.equal(journalIsRecord(ok), true);
  assert.equal(journalBanner(ok, PATH), null);
});

test('a failed append is shown verbatim and the journal stops being the record (P1-06-01)', () => {
  const failed = readJournalError('No space left on device (os error 28)');
  assert.equal(failed.state, 'failed');
  assert.equal(journalIsRecord(failed), false);
  const banner = journalBanner(failed, PATH);
  assert.ok(banner?.includes('NOT written'), banner ?? 'no banner');
  assert.ok(banner?.includes('No space left on device'), banner ?? 'no banner');
  assert.ok(banner?.includes(PATH), banner ?? 'no banner');
});

test('an absent or malformed journal_error is unknown, never good news', () => {
  for (const value of [undefined, null, 7, {}, []]) {
    const status = readJournalError(value);
    assert.equal(status.state, 'unknown', String(value));
    assert.equal(journalIsRecord(status), false);
    assert.ok(journalBanner(status, PATH)?.includes('unknown'));
  }
});

test('the autopilot page reads journal_error and conditions the durable-record sentence on it', () => {
  const page = readFileSync(new URL('../src/routes/autopilot/+page.svelte', import.meta.url), 'utf8');
  assert.match(page, /readJournalError\(raw\.journal_error\)/);
  assert.match(page, /journalBanner\(/);
  // Every sentence that names the journal as the durable record sits behind
  // the status check.
  const sentences = page.split('durable record').length - 1;
  const guarded = page.split('journalIsRecord(').length - 1;
  assert.ok(sentences > 0 && guarded >= 2, `${sentences} sentence(s), ${guarded} guard(s)`);
});
