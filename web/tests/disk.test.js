import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { STALE_COPY_DAYS, backupAlert, bytesText, diskSummary, scrubAlarm, scrubText } from '../src/lib/disk.js';

test('the disk line names free space, the store, the census kept and the missing copy (DB-B73)', () => {
  assert.equal(bytesText(0), '0 B');
  assert.equal(bytesText(1536), '1.5 KiB');
  assert.equal(bytesText(4 * 1024 ** 4), '4.0 TiB');
  assert.equal(bytesText(null), '—');
  assert.equal(bytesText(-1), '—');
  const none = diskSummary({
    filesystem: { total_bytes: 8 * 1024 ** 4, used_bytes: 4 * 1024 ** 4, free_bytes: 4 * 1024 ** 4 },
    feeds: [{ bar_bytes_lower_bound: 2 * 1024 ** 3 }, { bar_bytes_lower_bound: null }],
    census_kept_bytes: 3 * 1024 ** 2,
    backup: { recorded: false, why: 'no copy of this store is recorded' },
    retention: 'kept forever', projection_why: 'UNVERIFIED'
  });
  assert.equal(none.free, '4.0 TiB free of 8.0 TiB');
  assert.equal(none.store, 'bars ≥ 2.0 GiB');
  assert.equal(none.kept, 'census kept 3.0 MiB');
  assert.equal(none.backup, 'NO COPY of this store is recorded');
  assert.equal(none.alarm, true);
  assert.match(none.detail, /no copy of this store is recorded · kept forever · UNVERIFIED/);
  const copied = diskSummary({ filesystem: { free_bytes: 1, total_bytes: 2 }, feeds: [],
    census_kept_bytes: 0, backup: { recorded: true, last: 'copied=2026-10-11T01:00:00Z' } });
  assert.equal(copied.alarm, false);
  assert.equal(copied.backup, 'last copy: copied=2026-10-11T01:00:00Z');
  const unmeasured = diskSummary({ filesystem: { error: 'df failed' }, backup: { recorded: true, last: 'x' } });
  assert.equal(unmeasured.alarm, true);
  assert.match(unmeasured.free, /free space unknown: df failed/);
});

test('/ingest mounts the disk line', () => {
  const page = readFileSync(new URL('../src/routes/ingest/+page.svelte', import.meta.url), 'utf8');
  assert.match(page, /<DiskTile \/>/);
  assert.match(page, /import DiskTile from '\$lib\/DiskTile\.svelte'/);
});

test('the scrub line says, per feed, the pass, what it found and what was never looked at (DB-B74)', () => {
  assert.equal(scrubText(undefined), 'blocks never scrubbed');
  assert.equal(scrubText([]), 'blocks never scrubbed');
  // "never looked" is said by the feed's name, never merged into "nothing found".
  assert.equal(scrubText([{ feed: 'gdfl', scrub: { recorded: false, why: 'no scrub ran' } }]),
    'gdfl: blocks never scrubbed');
  const running = { feed: 'gdfl', scrub: { recorded: true, pass: 2, state: 'in progress', standing_findings: 3,
    totals: { months: 900, blocks: 9, bad: 3, unverifiable: 0, misfiled: 0, unbound: 0, busy: 0, missing: 0 } } };
  assert.equal(scrubText([running]), 'gdfl: scrub pass 2 in progress, 900 months, 3 bad blocks');
  const done = { feed: 'zerodha', scrub: { recorded: true, pass: 1, state: 'complete', standing_findings: 0,
    totals: { months: 5, blocks: 9, bad: 0, unverifiable: 2, misfiled: 1, unbound: 4, busy: 0, missing: 1 } } };
  assert.equal(scrubText([done]),
    'zerodha: scrub pass 1 complete, 5 months, 0 bad blocks, 2 unverifiable, 1 misfiled, 1 missing, 4 months bound by symbol hash only');
  assert.equal(scrubText([running, { feed: 'zerodha', scrub: { recorded: false } }]),
    'gdfl: scrub pass 2 in progress, 900 months, 3 bad blocks · zerodha: blocks never scrubbed');
  // A total the server did not send is a question mark, never a zero.
  assert.equal(scrubText([{ feed: 'gdfl', scrub: { recorded: true, pass: 1, state: 'complete', totals: {} } }]),
    'gdfl: scrub pass 1 complete, ? months, ? bad blocks');
});

test('a standing scrub finding raises the disk line\'s alarm, and a clean or unrun scrub does not (DB-B74)', () => {
  const base = { filesystem: { free_bytes: 1, total_bytes: 2 }, census_kept_bytes: 0,
    backup: { recorded: true, last: 'copied=2026-10-11T01:00:00Z' } };
  const feed = (/** @type {any} */ scrub) => [{ feed: 'gdfl', bar_bytes_lower_bound: 1, scrub }];
  assert.equal(scrubAlarm(undefined), false);
  assert.equal(scrubAlarm(feed({ recorded: false })), false);
  assert.equal(scrubAlarm(feed({ recorded: true, standing_findings: 0 })), false);
  assert.equal(scrubAlarm(feed({ recorded: true, standing_findings: 2 })), true);
  assert.equal(scrubAlarm(feed({ recorded: false, standing_findings: 2 })), false);
  assert.equal(diskSummary({ ...base, feeds: feed({ recorded: true, standing_findings: 0, pass: 1, totals: {} }) }).alarm, false);
  const alarmed = diskSummary({ ...base, feeds: feed({ recorded: true, standing_findings: 1, pass: 1,
    state: 'complete', totals: { months: 2, bad: 1 } }) });
  assert.equal(alarmed.alarm, true);
  assert.equal(alarmed.scrub, 'gdfl: scrub pass 1 complete, 2 months, 1 bad blocks');
});

test('a pass that met a month busy never reads as a clean one on the tile, and a half-built index says so (DB-B74, WP-81 repair 3)', () => {
  // THE SECOND AGENT'S CASE: pass 7 complete, bad=0 in the months it read,
  // one month met busy, and the block pass 6 found in that month kept. The
  // tile used to read "0 bad blocks" and raise no alarm.
  const kept = { feed: 'gdfl', bar_bytes_lower_bound: 1, scrub: { recorded: true, pass: 7, state: 'complete',
    standing_findings: 1, standing_busy: 1,
    totals: { months: 385442, blocks: 647416, bad: 0, unverifiable: 0, misfiled: 0, unbound: 0, busy: 1, missing: 0 } } };
  assert.equal(scrubText([kept]),
    'gdfl: scrub pass 7 complete, 385442 months, 0 bad blocks, 1 months NOT READ (busy), 1 findings standing with those kept from earlier passes');
  assert.equal(scrubAlarm([kept]), true, 'the kept finding still raises the alarm');
  // A busy month with nothing kept is said too, and is no alarm by itself.
  const unread = { feed: 'gdfl', scrub: { ...kept.scrub, standing_findings: 0 } };
  assert.match(scrubText([unread]), /0 bad blocks, 1 months NOT READ \(busy\), 0 findings standing/);
  assert.equal(scrubAlarm([unread]), false);
  // An answer of a server that sends no `standing_busy` falls back to the
  // pass's own `busy` total; none of either adds nothing.
  const older = { feed: 'gdfl', scrub: { recorded: true, pass: 2, state: 'complete',
    totals: { months: 9, bad: 0, busy: 2 } } };
  assert.equal(scrubText([older]),
    'gdfl: scrub pass 2 complete, 9 months, 0 bad blocks, 2 months NOT READ (busy), ? findings standing with those kept from earlier passes');
  assert.equal(scrubText([{ feed: 'gdfl', scrub: { recorded: true, pass: 2, state: 'complete', standing_busy: 0,
    totals: { months: 9, bad: 0, busy: 0 } } }]), 'gdfl: scrub pass 2 complete, 9 months, 0 bad blocks');
  // THE INDEX IS STILL BEING BUILT after a restart: its counts are of what it
  // has reached, and the tile says so instead of serving them as the whole.
  assert.equal(scrubText([{ feed: 'gdfl', scrub: { recorded: true, pass: 2, state: 'complete', truncated: true,
    standing_findings: 0, totals: { months: 9, bad: 0 } } }]),
    'gdfl: scrub pass 2 complete, 9 months, 0 bad blocks (findings still being indexed after a restart: counts may be low)');
});

test('every page says when no copy of the store is recorded, when it is old, and when its date cannot be read (RA-163)', () => {
  const now = Date.parse('2026-10-11T12:00:00Z');
  const receipt = (/** @type {string} */ copied) =>
    ({ backup: { recorded: true, last: `copied=${copied} dest=/Volumes/second raw_bytes=1 store_bytes=2 by=op` } });
  assert.equal(STALE_COPY_DAYS, 7);
  // No copy: the server's own reason is the detail.
  assert.deepEqual(backupAlert({ backup: { recorded: false, why: 'no copy of this store is recorded' } }, now),
    { text: 'NO COPY of this store is recorded.', detail: 'no copy of this store is recorded' });
  assert.deepEqual(backupAlert({ backup: { recorded: false } }, now),
    { text: 'NO COPY of this store is recorded.', detail: '' });
  // `recorded` must be exactly true: a word is not a copy.
  assert.equal(backupAlert({ backup: { recorded: 'true', last: 'copied=2026-10-11T01:00:00Z dest=/x' } }, now)?.text,
    'NO COPY of this store is recorded.');
  // A recent copy is the only silence; the boundary is seven whole days.
  assert.equal(backupAlert(receipt('2026-10-11T01:00:00Z'), now), null);
  assert.equal(backupAlert(receipt('2026-10-11T12:00:00Z'), now), null);
  assert.equal(backupAlert(receipt('2026-10-04T12:00:00Z'), now), null);
  assert.equal(backupAlert(receipt('2026-10-03T12:00:01Z'), now), null);
  assert.deepEqual(backupAlert(receipt('2026-10-03T12:00:00Z'), now),
    { text: 'The last copy of this store is 8 days old.',
      detail: 'copied=2026-10-03T12:00:00Z dest=/Volumes/second raw_bytes=1 store_bytes=2 by=op' });
  assert.equal(backupAlert(receipt('2025-10-11T12:00Z'), now)?.text, 'The last copy of this store is 365 days old.');
  // A date after now, a word that is no date, no `copied=` word, a clock that is no number.
  assert.equal(backupAlert(receipt('2026-10-11T12:00:01Z'), now)?.text, 'The last copy of this store is dated after now.');
  for (const bad of ['yesterday', '', '2026-10-11', '2026-13-45T99:99:99Z', '11/10/2026', '2026-10-11T01:00:00+05:30']) {
    assert.equal(backupAlert(receipt(bad), now)?.text, 'The last copy of this store has a date that cannot be read.', bad);
  }
  assert.equal(backupAlert({ backup: { recorded: true, last: 'dest=/x' } }, now)?.text,
    'The last copy of this store has a date that cannot be read.');
  assert.equal(backupAlert({ backup: { recorded: true } }, now)?.text,
    'The last copy of this store has a date that cannot be read.');
  assert.equal(backupAlert(receipt('2026-10-11T01:00:00Z'), NaN)?.text,
    'The last copy of this store has a date that cannot be read.');
  // An answer that is not a /disk.json answer is not "a copy exists".
  for (const body of [undefined, null, {}, { backup: null }, { backup: 'yes' }, []]) {
    assert.equal(backupAlert(body, now)?.text, 'Whether a copy of this store exists is not known.');
  }
});

test('the layout mounts the standing copy line on every page, and a failed read is said (RA-163)', () => {
  const layout = readFileSync(new URL('../src/routes/+layout.svelte', import.meta.url), 'utf8');
  assert.match(layout, /import CopyAlert from '\$lib\/CopyAlert\.svelte'/);
  assert.equal((layout.match(/<CopyAlert \/>/g) ?? []).length, 1);
  // Inside <main>, before the page: every route renders it.
  assert.ok(layout.indexOf('<CopyAlert />') > layout.indexOf('<main class="main"'));
  assert.ok(layout.indexOf('<CopyAlert />') < layout.indexOf('{@render children()}'));
  const tile = readFileSync(new URL('../src/lib/CopyAlert.svelte', import.meta.url), 'utf8');
  assert.match(tile, /ask\('\/disk\.json', \{ cache: 'no-store', signal: stop\.signal \}\)/);
  assert.match(tile, /alert = backupAlert\(body, Date\.now\(\)\)/);
  assert.match(tile, /Whether a copy of this store exists was not read\./);
  assert.match(tile, /return \(\) => stop\.abort\(\)/);
  assert.match(tile, /role="alert" data-copy-alert/);
});
