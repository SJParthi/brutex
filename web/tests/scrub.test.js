import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { SCRUB_MAX_KEYS, findingText, scrubNote, scrubUrl } from '../src/lib/db/scrub.js';

test('/db asks /scrub.json for the series it shows, or asks nothing (DB-B74)', () => {
  assert.equal(scrubUrl(null, { keys: ['NSE-INDEX-NIFTY'], timeframe: '1s' }), null);
  assert.equal(scrubUrl('gdfl', null), null);
  assert.equal(scrubUrl('gdfl', { keys: [], timeframe: '1s' }), null);
  // More than one rung is no single series: nothing is asked.
  assert.equal(scrubUrl('gdfl', { keys: ['NSE-INDEX-NIFTY'], timeframe: '' }), null);
  assert.equal(scrubUrl('gdfl', { keys: ['NSE-INDEX-NIFTY'], timeframe: '1s' }),
    '/scrub.json?feed=gdfl&timeframe=1s&key=NSE-INDEX-NIFTY');
  // A key is escaped, never spliced.
  assert.equal(scrubUrl('gdfl', { keys: ['NSE-CASH-M&M'], timeframe: '1s' }),
    '/scrub.json?feed=gdfl&timeframe=1s&key=NSE-CASH-M%26M');
  // The server reads at most SCRUB_MAX_KEYS keys; the URL never names more.
  const many = Array.from({ length: SCRUB_MAX_KEYS + 9 }, (_, i) => `K${i}`);
  const asked = new URL(String(scrubUrl('gdfl', { keys: many, timeframe: '1s' })), 'http://x').searchParams.getAll('key');
  assert.equal(asked.length, SCRUB_MAX_KEYS);
  assert.equal(asked[0], 'K0');
});

test('every kind of finding is a sentence naming its series, month and pass (DB-B74, DB-B65)', () => {
  assert.equal(findingText('NSE-INDEX-NIFTY', { kind: 'bad', month: '2020-07', block: 21, first: 1533, last: 1605, pass: 2 }),
    'NSE-INDEX-NIFTY 2020-07: block 21 (records 1533–1605) does not match its stored checksum — found by scrub pass 2');
  assert.match(findingText('K', { kind: 'unverifiable', month: '2019-11', why: 'no .crc', pass: 1 }),
    /^K 2019-11: cannot be verified \(no \.crc\) — scrub pass 1$/);
  assert.match(findingText('K', { kind: 'misfiled', month: '2019-11', pass: 3 }),
    /^K 2019-11: filed under another name \(no reason recorded\) — scrub pass 3$/);
  assert.match(findingText('K', { kind: 'missing', month: '2019-11', pass: 0 }),
    /^K 2019-11: the census holds this month and its file is gone — scrub an earlier pass$/);
  // A kind this page does not know is printed by its own word, never dropped.
  assert.equal(findingText('K', { kind: 'novel', month: '2019-11', pass: 1 }), 'K 2019-11: novel — scrub pass 1');
  assert.equal(findingText('K', {}), 'K ?: an unnamed finding — scrub an earlier pass');
});

test('"never looked" and "nothing found" are different notes, and a finding warns (DB-B74)', () => {
  const never = scrubNote({ feed: 'gdfl', recorded: false, why: 'run `cli scrub-blocks gdfl`' }, 1);
  assert.equal(never.tone, 'muted');
  assert.match(never.head, /^Blocks never scrubbed for gdfl: .* run `cli scrub-blocks gdfl`$/);
  assert.deepEqual(never.lines, []);
  assert.match(scrubNote(null, 1).head, /^Blocks never scrubbed for this feed/);

  const clean = scrubNote({ feed: 'gdfl', recorded: true, pass: 2, state: 'complete',
    series: [{ key: 'NSE-INDEX-NIFTY', timeframe: '1s', more: 0, findings: [] }] }, 1);
  assert.equal(clean.tone, 'muted');
  assert.equal(clean.head, 'Scrub pass 2 (complete) names no damaged block in this selection.');

  // A pass still running vouches for nothing it has not reached.
  const running = scrubNote({ feed: 'gdfl', recorded: true, pass: 3, state: 'in progress', series: [] }, 0);
  assert.equal(running.head,
    'Scrub pass 3 (in progress) names no damaged block in this selection. Months it has not reached yet are not vouched for.');

  const found = scrubNote({ feed: 'gdfl', recorded: true, pass: 2, state: 'complete', truncated: true,
    series: [
      { key: 'NSE-INDEX-NIFTY', timeframe: '1s', more: 4,
        findings: [{ kind: 'bad', month: '2020-07', block: 21, first: 1533, last: 1605, pass: 2 }] },
      { key: 'NSE-INDEX-BANKNIFTY', timeframe: '1s', more: 0,
        findings: [{ kind: 'missing', month: '2019-11', pass: 2 }] }
    ] }, 5);
  assert.equal(found.tone, 'warn');
  assert.equal(found.lines.length, 2);
  assert.equal(found.head,
    'Scrub pass 2 (complete): 6 finding(s) stand against this selection (2 of 5 series asked; narrow the selection to ask the rest). 4 more are not listed. The findings ledger is still being indexed after a restart; findings may be missing until it has caught up, so ask again.');

  // Findings the server counted but did not list still warn.
  const counted = scrubNote({ feed: 'gdfl', recorded: true, pass: 1, state: 'complete',
    series: [{ key: 'K', more: 7, findings: [] }] }, 1);
  assert.equal(counted.tone, 'warn');
  assert.match(counted.head, /7 finding\(s\) stand .* 7 more are not listed\./);
});

test('/db mounts the scrub note under the grid and the dev proxy carries /scrub.json', () => {
  const grid = readFileSync(new URL('../src/routes/db/DbBarsGrid.svelte', import.meta.url), 'utf8');
  assert.match(grid, /import DbScrubNote from '\.\/DbScrubNote\.svelte'/);
  assert.match(grid, /<DbScrubNote \{db\} \/>/);
  const note = readFileSync(new URL('../src/routes/db/DbScrubNote.svelte', import.meta.url), 'utf8');
  assert.match(note, /Scrub findings not read: \{failed\}/, 'a failed read is said, never blanked');
  const vite = readFileSync(new URL('../vite.config.js', import.meta.url), 'utf8');
  assert.match(vite, /'\/scrub\.json'/);
});
