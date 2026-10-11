import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  SCRUB_BUSY_NAMED, SCRUB_MAX_KEYS, SCRUB_REASK_MOST, SCRUB_REASK_MS, busyText, findingText, scrubAskAgainMs,
  scrubNote, scrubUrl
} from '../src/lib/db/scrub.js';

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

test('a month a pass met busy keeps its findings on the page and is named as not read (DB-B74, WP-81 repair 3)', () => {
  // THE ANSWER THE SERVER GIVES NOW for the second agent's case: pass 7 met
  // NIFTY 2020-07 busy and completed; block 16, found by pass 6, is kept.
  const kept = { kind: 'bad', month: '2020-07', block: 16, first: 1168, last: 1240, pass: 6, run: 600,
    kept_pass: 7, kept_run: 700 };
  assert.equal(findingText('NSE-INDEX-NIFTY', kept),
    'NSE-INDEX-NIFTY 2020-07: block 16 (records 1168–1240) does not match its stored checksum — found by scrub pass 6; kept, not read again: pass 7 met this month busy');
  // Every kind says it, and a run that cannot be dated is not called pass 0.
  for (const kind of ['unverifiable', 'misfiled', 'missing', 'novel']) {
    assert.match(findingText('K', { kind, month: '2020-07', pass: 6, kept_pass: 0 }),
      /scrub pass 6; kept, not read again: a later pass met this month busy$/, kind);
  }
  // A finding the newest pass read says nothing of the kind.
  assert.doesNotMatch(findingText('K', { kind: 'bad', month: '2020-07', pass: 7, kept_pass: null }), /kept/);

  const body = { feed: 'gdfl', recorded: true, pass: 7, state: 'complete', standing_busy: 1,
    series: [{ key: 'NSE-INDEX-NIFTY', timeframe: '1s', more: 0, findings: [kept],
      busy: [{ month: '2020-07', pass: 7, run: 700 }], busy_more: 0 }] };
  const note = scrubNote(body, 1);
  assert.equal(note.tone, 'warn');
  assert.equal(note.lines.length, 1);
  assert.equal(note.head,
    'Scrub pass 7 (complete): 1 finding(s) stand against this selection. 1 month(s) not read: busy (a writer held them when the pass reached them), earlier findings kept — NSE-INDEX-NIFTY 2020-07 (pass 7).');

  // A BUSY MONTH WITH NOTHING FOUND IN IT is still not a clean answer: the
  // note warns and never says the pass names no damaged block in the
  // selection without saying which months it did not read.
  const unread = scrubNote({ ...body, series: [{ key: 'NSE-INDEX-NIFTY', more: 0, findings: [],
    busy: [{ month: '2020-07', pass: 7, run: 700 }, { month: '2020-08', pass: 0, run: 9 }], busy_more: 3 }] }, 1);
  assert.equal(unread.tone, 'warn');
  assert.deepEqual(unread.lines, []);
  assert.equal(unread.head,
    'Scrub pass 7 (complete) names no damaged block in the months it read of this selection. 5 month(s) not read: busy (a writer held them when the pass reached them), earlier findings kept — NSE-INDEX-NIFTY 2020-07 (pass 7), NSE-INDEX-NIFTY 2020-08 (a pass that cannot be dated) and 3 more.');

  // The head names a bounded number of months and counts the rest.
  const many = Array.from({ length: SCRUB_BUSY_NAMED + 4 }, (_, i) => ({ month: `2020-${String(i + 1).padStart(2, '0')}`, pass: 7 }));
  const capped = busyText([{ key: 'K', busy: many, busy_more: 2 }]);
  assert.equal(capped.count, SCRUB_BUSY_NAMED + 6);
  assert.equal(capped.text.split(' (pass 7)').length - 1, SCRUB_BUSY_NAMED);
  assert.match(capped.text, / and 6 more\.$/);
  // No busy month, an old server's answer, or garbage: no sentence at all.
  assert.deepEqual(busyText([{ key: 'K', findings: [] }]), { count: 0, text: '' });
  assert.deepEqual(busyText(undefined), { count: 0, text: '' });
  assert.deepEqual(busyText([{ key: 'K', busy: 'x', busy_more: 'y' }]), { count: 0, text: '' });
});

test('/db asks again by itself while the server is still indexing its ledger, a bounded number of times (L-WP81-1)', () => {
  const partial = { recorded: true, truncated: true };
  assert.equal(scrubAskAgainMs(partial, 1), SCRUB_REASK_MS);
  assert.equal(scrubAskAgainMs(partial, SCRUB_REASK_MOST - 1), SCRUB_REASK_MS);
  assert.equal(scrubAskAgainMs(partial, SCRUB_REASK_MOST), null, 'bounded');
  assert.equal(scrubAskAgainMs(partial, NaN), null);
  assert.equal(scrubAskAgainMs({ recorded: true, truncated: false }, 1), null, 'a whole answer is not asked again');
  assert.equal(scrubAskAgainMs({ recorded: false, truncated: true }, 1), null);
  assert.equal(scrubAskAgainMs(null, 1), null);
  const note = readFileSync(new URL('../src/routes/db/DbScrubNote.svelte', import.meta.url), 'utf8');
  assert.match(note, /const wait = scrubAskAgainMs\(body, tries\);\s+if \(wait === null\) return;/);
  assert.match(note, /if \(controller\.signal\.aborted\) return;\s+\}\s+\} catch/, 'a changed selection stops the asking');
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
