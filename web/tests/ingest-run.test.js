// WP-61: what /ingest says about a run it watches (DB-G21, DB-B28, DB-B30,
// DB-B31, DB-B32). Every fixture is invented; no journal record is copied.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { adoptRun, adoptedSentence, isAskTimeout, lastRun, stopSaid, waitedOut } from '../src/lib/ingest-run.js';

const leg = (/** @type {Record<string, unknown>} */ over = {}) => ({
  route: '/pull/spot', vendor: 'zerodha', dir: '1min', label: 'Spot · 1 minute',
  target: 'n50', from: '2018-01-01', to: '2026-09-30', members: 0, ...over
});

test('DB-B32: a resumed run is the run\'s own feed, rungs, window and start, never this tab\'s', () => {
  const run = adoptRun({ running: true, startedAtMs: 1_791_600_000_123, asked: [leg(), leg({ dir: '1day', label: 'Spot · 1 day' })] });
  assert.equal(run.known, true);
  assert.equal(run.startedAtMs, 1_791_600_000_123);
  assert.deepEqual(run.vendors, ['zerodha']);
  assert.deepEqual(run.rungs, ['1min', '1day']);
  assert.deepEqual([run.from, run.to, run.target, run.members, run.legs], ['2018-01-01', '2026-09-30', 'n50', 0, 2]);
  assert.deepEqual(run.mixed, []);
  const said = adoptedSentence(run, (wire) => (wire === 'zerodha' ? 'Zerodha' : wire));
  assert.match(said, /It asks Zerodha for 1min, 1day over 2018-01-01 to 2026-09-30, every member of n50, in 2 leg\(s\)/);
  assert.doesNotMatch(said, /Global Datafeeds/);
});

test('legs that disagree are said to differ, and an older API is said to name nothing', () => {
  const mixed = adoptRun({ startedAtMs: 5, asked: [leg(), leg({ from: '2020-01-01', members: 2, vendor: 'gdfl' })] });
  assert.equal(mixed.from, null);
  assert.equal(mixed.members, null);
  assert.deepEqual(mixed.mixed.sort(), ['from', 'members']);
  assert.deepEqual(mixed.vendors, ['zerodha', 'gdfl']);
  assert.match(adoptedSentence(mixed, (w) => w), /a window that differs between its legs \(from, members\)|a window that differs between its legs/);
  for (const doc of [null, undefined, {}, { asked: [] }, { asked: 'x' }, { asked: [null, 7] }]) {
    const old = adoptRun(doc);
    assert.equal(old.known, false, JSON.stringify(doc));
    assert.equal(old.legs, 0);
    assert.match(adoptedSentence(old, (w) => w), /does not say what it asked for/);
  }
  // A start that is zero, negative, fractional or text is unknown, never the epoch.
  for (const bad of [0, -1, 1.5, '1791600000123', Number.NaN]) {
    assert.equal(adoptRun({ startedAtMs: bad, asked: [leg()] }).startedAtMs, null, String(bad));
  }
  assert.match(adoptedSentence(adoptRun({ asked: [leg()] }), (w) => w), /at a time the server did not record/);
  // A named member count is stated as a count.
  assert.match(adoptedSentence(adoptRun({ startedAtMs: 9, asked: [leg({ members: 3 })] }), (w) => w), /3 named instrument\(s\) of n50/);
});

test('DB-B28, DB-B30: Stop says what it stopped, in the server\'s words, and never "nothing" for a hand pull', () => {
  const run = stopSaid({ stopping: true, what: 'run', when: 'the run\'s leg in flight stops at its next instrument' });
  assert.deepEqual([run.stopped, run.what], [true, 'run']);
  assert.match(run.text, /Stop was taken: the run's leg in flight stops at its next instrument/);
  const hand = stopSaid({ stopping: true, what: 'hand_pull', when: 'the single hand pull in flight stops at its next instrument' });
  assert.deepEqual([hand.stopped, hand.what], [true, 'hand_pull']);
  assert.match(hand.text, /single hand pull in flight/);
  const none = stopSaid({ stopping: false, what: 'nothing', when: 'no hand run or hand pull is in flight' });
  assert.deepEqual([none.stopped, none.what], [false, 'nothing']);
  assert.match(none.text, /Nothing was stopped: no hand run or hand pull is in flight/);
  // An API older than the answer: read by its flag, and said to be unlabelled.
  const old = stopSaid({ stopping: true });
  assert.deepEqual([old.stopped, old.what], [true, 'unlabelled']);
  assert.match(old.text, /did not say what it stopped/);
  assert.equal(stopSaid({ stopping: false }).what, 'unlabelled');
  assert.equal(stopSaid(null).stopped, false);
  assert.equal(stopSaid({ stopping: 'true', what: 'run' }).stopped, false, 'only a boolean true is a stop');
  assert.equal(stopSaid({ stopping: true, what: 'everything' }).what, 'unlabelled', 'an unknown word is not trusted');
});

test('DB-B31: the page stopped waiting; it never says the pull was cancelled', () => {
  assert.equal(isAskTimeout(new Error('The request timed out and was cancelled after 120000 ms')), true);
  assert.equal(isAskTimeout(new Error('Failed to fetch')), false);
  assert.equal(isAskTimeout('timed out and was cancelled'), false, 'a string is not ask\'s error');
  assert.equal(isAskTimeout(null), false);
  const said = waitedOut('/pull/spot', 120_000, 'Spot · 1 minute');
  assert.match(said, /stopped waiting for Spot · 1 minute after 120 s/);
  assert.match(said, /The pull was NOT cancelled/);
  assert.match(said, /\/pull\/spot/);
});

const rec = (/** @type {Record<string, unknown>} */ over = {}) => ({
  ordinal: 40, fault: null, kind: 'run', at: 1_800_006_815, took_micros: 0, outcome: 'FAILED',
  members: 12, bars_stored: 900, failures: 7, window: '2018-01-01..=2026-09-30',
  source: 'an invented source', note: 'an invented note', note_whole: null, ...over
});

test('DB-G21: the last run is named whatever follows it: open, closed with its opening, alone, none, unread', () => {
  assert.equal(lastRun(null).state, 'unread');
  assert.equal(lastRun({}).state, 'unread', 'an API older than last_runs');
  assert.match(lastRun({}).sentence, /does not list the newest runs/);
  const failed = lastRun({ last_runs: [], last_runs_error: 'the journal index is torn at record 9' });
  assert.equal(failed.state, 'unread');
  assert.equal(failed.tone, 'down');
  assert.match(failed.sentence, /torn at record 9/);
  const none = lastRun({ last_runs: [], last_runs_error: null, journal: { feed_scope: { runs: 0, holds: 'This feed is never pulled.' } } });
  assert.equal(none.state, 'none');
  assert.match(none.sentence, /No run of this feed is in the pull journal\. This feed is never pulled\./);

  // In flight, or stopped before its closing record.
  const open = lastRun({ last_runs: [{ members_after: 3, record: rec({ outcome: 'OPENED', ordinal: 50 }) }] });
  assert.deepEqual([open.state, open.tone, open.membersAfter, open.ordinal], ['open', 'warn', 3, 50]);
  assert.match(open.sentence, /OPENED at .* IST over 2018-01-01\.\.=2026-09-30 and has no closing record/);

  // A closing record with its opening just before it, over the same window.
  const closed = lastRun({
    last_runs: [
      { members_after: 0, record: rec({ ordinal: 60, at: 1_800_006_815, failures: 2_400_100, bars_stored: 153 }) },
      { members_after: 2_400_100, record: rec({ ordinal: 10, at: 1_800_000_000, outcome: 'OPENED' }) }
    ]
  });
  assert.equal(closed.state, 'closed');
  assert.equal(closed.tone, 'down');
  assert.equal(closed.outcome, 'FAILED');
  assert.equal(closed.startedAt, 1_800_000_000);
  assert.equal(closed.tookMs, 6_815_000);
  assert.equal(closed.membersAfter, 2_400_100, 'the member failures follow the OPENING record');
  assert.equal(closed.failures, 2_400_100);
  assert.match(closed.sentence, /ended FAILED at .* having opened at .* \(1h 53m\) over 2018-01-01\.\.=2026-09-30/);
  assert.match(closed.sentence, /24,00,100 failure\(s\) by its own count/);
  assert.match(closed.sentence, /Its note: an invented note/);

  // An OPENED record over ANOTHER window is not this run's opening.
  const alone = lastRun({
    last_runs: [
      { members_after: 4, record: rec({ outcome: 'STORED', failures: 0, took_micros: 2_000_000 }) },
      { members_after: 0, record: rec({ outcome: 'OPENED', window: '2015-01-01..=2015-12-31' }) }
    ]
  });
  assert.deepEqual([alone.state, alone.tone, alone.startedAt, alone.tookMs, alone.membersAfter], ['closed', 'up', null, 2_000, 4]);
  assert.equal(lastRun({ last_runs: [{ members_after: 0, record: rec({ outcome: 'STORED NOTHING' }) }] }).tone, 'warn');
  // A faulted record is skipped, never read as a run.
  assert.equal(lastRun({ last_runs: [{ members_after: 0, record: { fault: 'undecodable' } }] }).state, 'none');
  // The whole note outranks the cut one.
  assert.match(lastRun({ last_runs: [{ members_after: 0, record: rec({ note_whole: 'the whole invented note' }) }] }).sentence, /Its note: the whole invented note/);
});
