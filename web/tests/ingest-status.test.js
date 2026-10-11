// CE-79, D-1787: /ingest dropped every refusal from /ingest/status.json and
// kept a stale flight and halt list after a failed read.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { emptyPilot, failedPilot, foldIngestStatus, pilotNotice } from '../src/lib/ingest-status.js';

const page = readFileSync(new URL('../src/routes/ingest/+page.svelte', import.meta.url), 'utf8');

const surveyed = {
  surveyed: true, state: 'running', blocked_by: 'the sweep is pulling NIFTY 2026-01',
  in_flight: { instrument: 'NSE-INDEX-NIFTY', month: '2026-01', index: 1, of: 9 },
  waiting_on: [{ feed: 'dhan', vendor: 'Dhan', halted: 'PERMANENT REFUSAL' }]
};

test('a surveyed answer carries the flight, the halts, surveyed and blocked_by', () => {
  const pilot = foldIngestStatus(200, surveyed, 42);
  assert.equal(pilot.at, 42);
  assert.equal(pilot.surveyed, true);
  assert.equal(pilot.blockedBy, 'the sweep is pulling NIFTY 2026-01');
  assert.equal(pilot.feeds.length, 1);
  assert.equal(pilot.inFlight?.month, '2026-01');
  assert.equal(pilot.error, null);
  assert.equal(pilotNotice(pilot), null);
});

test('surveyed:false is said in the server words, never folded to "nothing halted"', () => {
  const pilot = foldIngestStatus(200, { surveyed: false, state: 'waiting', blocked_by: 'not known here yet. No round has finished', in_flight: null, waiting_on: [] }, 1);
  assert.equal(pilot.surveyed, false);
  const notice = pilotNotice(pilot);
  assert.equal(notice?.tone, 'warn');
  assert.match(notice?.text ?? '', /^not known here yet\. No round has finished/);
  assert.match(notice?.head ?? '', /not nothing outstanding/);
});

test('the 503 poisoned-lock body is named and the previous answer does not survive', () => {
  const before = foldIngestStatus(200, surveyed, 1);
  assert.equal(before.feeds.length, 1);
  const after = foldIngestStatus(503, { surveyed: false, error: "the autopilot's status lock is poisoned ... Restart the server." }, 2);
  assert.equal(after.inFlight, null);
  assert.deepEqual(after.feeds, []);
  assert.equal(after.surveyed, null);
  assert.equal(after.error, "/ingest/status.json answered HTTP 503: the autopilot's status lock is poisoned ... Restart the server.");
  assert.equal(pilotNotice(after)?.tone, 'bad');
  assert.equal(foldIngestStatus(500, undefined, 3).error, '/ingest/status.json answered HTTP 500 and named no reason');
});

test('a 200 without `surveyed` or without a JSON object is a failure, not an empty survey', () => {
  assert.match(foldIngestStatus(200, { waiting_on: [] }, 1).error ?? '', /`surveyed` is missing/);
  assert.match(foldIngestStatus(200, [], 1).error ?? '', /not a JSON object/);
  assert.match(foldIngestStatus(200, undefined, 1).error ?? '', /not a JSON object/);
  assert.equal(failedPilot('x').feeds.length, 0);
  assert.equal(emptyPilot().surveyed, null);
});

test('the page folds through the module, renders the notice and keeps no stale spread on failure', () => {
  assert.match(page, /pilot = foldIngestStatus\(r\.status, body, Date\.now\(\)\)/);
  assert.match(page, /pilot = failedPilot\(/);
  assert.doesNotMatch(page, /pilot = \{ \.\.\.pilot, busy: false, error: String\(why\) \}/);
  assert.match(page, /\{#if pilotSays\}[\s\S]{0,200}role="alert"[\s\S]{0,300}\{pilotSays\.text\}/);
});

// ---------------------------------------------------------------------------
// WP-61: the cell in flight, its pace, the stall rule and the idle notice
// (DB-G14, DB-G20, DB-G21, DB-B29, DB-B39, DB-B53). Fixtures are invented.
// ---------------------------------------------------------------------------
import {
  STALL_FACTOR, STALL_FLOOR_MS, busyNotice, flightMonth, flightOf, flightPace, flightRow,
  progressKey, spanText, stallVerdict
} from '../src/lib/ingest-status.js';

const cell = {
  instrument: 'NSE-CASH-ABB', feed: 'zerodha', timeframe: '1min', driver: 'hand', month: '2020-11',
  window: { from: '2020-11-01', to: '2021-01-31' }, index: 3, of: 208,
  elapsed_ms: 4_000, run_elapsed_ms: 90_000, per_instrument_ms: 45_000, eta_ms: 9_270_000,
  eta_basis: 'the instruments left times the mean of those finished'
};

test('DB-G20: only the row the cell names is in flight: same feed, rung, spot segment and instrument', () => {
  const flight = flightOf(cell);
  assert.ok(flight);
  const row = { feed: 'zerodha', seg: 'spot', tf: '1min', sym: 'ABB', key: 'NSE-CASH-ABB', mkey: 'NSE-ABB' };
  assert.equal(flightRow(flight, row), true);
  assert.equal(flightRow(flight, { ...row, feed: 'gdfl' }), false, 'another feed');
  assert.equal(flightRow(flight, { ...row, tf: '1day' }), false, 'another rung');
  assert.equal(flightRow(flight, { ...row, seg: 'options' }), false, 'a derivative segment');
  assert.equal(flightRow(flight, { ...row, sym: 'ABBOTINDIA', key: 'NSE-CASH-ABBOTINDIA', mkey: 'NSE-ABBOTINDIA' }), false, 'a longer symbol');
  assert.equal(flightRow(flight, { ...row, feed: null }), false, 'no feed chosen');
  assert.equal(flightRow(null, row), false, 'no cell');
  // AN API OLDER THAN WP-34 NAMES NO FEED OR RUNG: it marks NO row, where it
  // used to mark every row of the symbol in every feed and rung.
  const old = flightOf({ instrument: 'NSE-CASH-ABB', month: '2020-11', index: 1, of: 9 });
  assert.ok(old);
  assert.equal(flightRow(old, row), false);
  assert.equal(flightPace(old)?.labelled, false);
  // The months the cell is fetching: its window, or its one month without one.
  assert.equal(flightMonth(flight, '2020-11'), true);
  assert.equal(flightMonth(flight, '2021-01'), true);
  assert.equal(flightMonth(flight, '2021-02'), false);
  assert.equal(flightMonth(old, '2020-11'), true);
  assert.equal(flightMonth(old, '2020-12'), false);
  assert.equal(flightMonth(null, '2020-11'), false);
});

test('the cell is refused, not guessed, when it names no instrument or is not an object', () => {
  for (const bad of [null, undefined, 7, 'NIFTY', [], {}, { instrument: '   ' }, { instrument: 9 }]) {
    assert.equal(flightOf(bad), null, JSON.stringify(bad));
  }
  const odd = flightOf({ instrument: 'NIFTY', index: -1, of: 2.5, eta_ms: '9', window: { from: '2020-11', to: null } });
  assert.deepEqual(
    [odd?.index, odd?.of, odd?.etaMs, odd?.from, odd?.to],
    [null, null, null, null, null],
    'a negative, fractional or textual count and a malformed day are unknown, never zero'
  );
});

test('DB-G21: the pace is the server\'s measurement and the ETA always says it is an extrapolation', () => {
  const pace = flightPace(flightOf(cell));
  assert.ok(pace);
  assert.equal(pace.done, 2, 'index 3 is being fetched, so two are finished');
  assert.equal(pace.of, 208);
  assert.ok(Math.abs((pace.share ?? 0) - 2 / 208) < 1e-12);
  assert.match(pace.where, /NSE-CASH-ABB — instrument 3 of 208/);
  assert.match(pace.pace ?? '', /45s per instrument, measured/);
  assert.match(pace.eta ?? '', /EXTRAPOLATED \(the instruments left times the mean of those finished\)/);
  assert.match(pace.eta ?? '', /2h 34m/);
  // No measured pace: no pace sentence and no ETA, never a zero.
  const first = flightPace(flightOf({ ...cell, index: 1, per_instrument_ms: null, eta_ms: null }));
  assert.equal(first?.pace, null);
  assert.equal(first?.eta, null);
  assert.equal(first?.share, 0);
  assert.equal(flightPace(null), null);
  // An ETA with no basis is still labelled.
  assert.match(flightPace(flightOf({ ...cell, eta_basis: '' }))?.eta ?? '', /EXTRAPOLATED, not measured/);
  assert.equal(spanText(-1), '—');
  assert.equal(spanText(Number.NaN), '—');
  assert.equal(spanText(0), '0s');
  assert.equal(spanText(59_400), '59s');
  assert.equal(spanText(200_000), '3m 20s');
  assert.equal(spanText(7_500_000), '2h 05m');
});

test('DB-B39: a healthy 45 s instrument is not "stalled" at 20 s; three measured instruments is', () => {
  const healthy = stallVerdict({ sinceProgressMs: 44_700, perInstrumentMs: 44_700 });
  assert.equal(healthy.stalled, false, 'the old rule called this stalled after 20 s');
  assert.equal(healthy.afterMs, Math.max(STALL_FLOOR_MS, 44_700 * STALL_FACTOR));
  assert.equal(stallVerdict({ sinceProgressMs: 134_099, perInstrumentMs: 44_700 }).stalled, false);
  assert.equal(stallVerdict({ sinceProgressMs: 134_100, perInstrumentMs: 44_700 }).stalled, true);
  // No pace measured yet: the floor alone, on its exact edge.
  assert.equal(stallVerdict({ sinceProgressMs: STALL_FLOOR_MS - 1, perInstrumentMs: null }).stalled, false);
  assert.equal(stallVerdict({ sinceProgressMs: STALL_FLOOR_MS, perInstrumentMs: null }).stalled, true);
  // A fast day rung never drops the threshold under the floor.
  assert.equal(stallVerdict({ sinceProgressMs: 30_000, perInstrumentMs: 300 }).stalled, false);
  assert.equal(stallVerdict({ sinceProgressMs: Number.NaN, perInstrumentMs: 300 }).stalled, false);
  assert.match(healthy.why, /Called stalled after/);
  // Progress is anything the server reports moving.
  const flight = flightOf(cell);
  const run = { feeds: [{ legsDone: 1 }, { legsDone: 0 }], rowsNow: 10, passes: 1 };
  const key = progressKey(flight, run);
  assert.notEqual(key, progressKey(flightOf({ ...cell, index: 4 }), run), 'the position moved');
  assert.notEqual(key, progressKey(flightOf({ ...cell, instrument: 'NSE-CASH-ACC' }), run), 'the instrument moved');
  assert.notEqual(key, progressKey(flight, { ...run, feeds: [{ legsDone: 2 }, { legsDone: 0 }] }), 'a leg finished');
  assert.notEqual(key, progressKey(flight, { ...run, rowsNow: 11 }), 'a row landed');
  assert.equal(key, progressKey(flightOf({ ...cell, elapsed_ms: 9_999 }), run), 'a clock alone is not progress');
  assert.equal(typeof progressKey(null, null), 'string');
});

test('DB-B29: an idle tab names who is pulling, what stops it, and whether its own Pull is blocked', () => {
  const idle = foldIngestStatus(200, { surveyed: true, state: 'idle', in_flight: null, waiting_on: [] }, 1);
  assert.equal(busyNotice(idle, { running: false }, 'zerodha'), null, 'nothing going: no notice');
  assert.equal(busyNotice(idle, null, 'zerodha'), null);
  const name = (/** @type {string} */ wire) => ({ zerodha: 'Zerodha', gdfl: 'Global Datafeeds' })[wire] ?? wire;
  // A hand run in /pull/run.json, between instruments.
  const run = busyNotice(idle, { running: true, asked: [{ vendor: 'zerodha' }], feeds: [{ vendor: 'zerodha' }] }, 'zerodha', name);
  assert.equal(run?.driver, 'run');
  assert.equal(run?.blocksPull, true);
  assert.match(run?.head ?? '', /hand run/);
  assert.match(run?.text ?? '', /Zerodha/);
  assert.match(run?.text ?? '', /between two instruments or legs/);
  assert.equal(busyNotice(idle, { running: true, asked: [{ vendor: 'zerodha' }] }, 'gdfl', name)?.blocksPull, false, 'another feed is not blocked');
  assert.equal(busyNotice(idle, { running: true }, 'gdfl', name)?.blocksPull, true, 'a run that names no feed blocks: unknown is not free');
  // A single hand pull, named by its cell.
  const hand = foldIngestStatus(200, { surveyed: true, state: 'running', in_flight: cell, waiting_on: [] }, 1);
  const single = busyNotice(hand, { running: false }, 'zerodha', name);
  assert.equal(single?.driver, 'hand');
  assert.equal(single?.blocksPull, true);
  assert.match(single?.text ?? '', /Zerodha · 1min · NSE-CASH-ABB — instrument 3 of 208/);
  assert.match(single?.text ?? '', /Stop on this page stops it at its next instrument/);
  // The autopilot: Stop here does not touch it, and the notice says so.
  const auto = foldIngestStatus(200, { surveyed: true, state: 'running', in_flight: { ...cell, driver: 'autopilot' }, waiting_on: [] }, 1);
  const pilot = busyNotice(auto, { running: false }, 'zerodha', name);
  assert.equal(pilot?.tone, 'warn');
  assert.match(pilot?.head ?? '', /autopilot is pulling/);
  assert.match(pilot?.text ?? '', /pause it at \/autopilot/);
  assert.match(pilot?.text ?? '', /Stop on this page does not touch it/);
  assert.equal(busyNotice(auto, { running: false }, 'gdfl', name)?.blocksPull, false);
  // The fold keeps the seat and the pause, and an empty pilot states neither.
  assert.equal(foldIngestStatus(200, { surveyed: true, state: 'idle', pull_seat: 'held', paused: true }, 1).seat, 'held');
  assert.equal(foldIngestStatus(200, { surveyed: true, state: 'idle', pull_seat: 'held', paused: true }, 1).paused, true);
  assert.equal(emptyPilot().seat, null);
  assert.equal(emptyPilot().paused, null);
});

test('the page wires them: no census on a timer, the exact row, the shared feed rule, the run\'s own ask', () => {
  const code = page.replace(/<!--[\s\S]*?-->/g, '').replace(/\/\*[\s\S]*?\*\//g, '').replace(/^\s*\/\/.*$/gm, '');
  // DB-B53: nothing re-reads the whole census on a clock.
  assert.doesNotMatch(code, /watchStore\s*\(/);
  // DB-G20: the exact row, and the verdict is never "retry".
  assert.match(code, /flightRow\(flight, \{ feed: feedNow, seg: s\.key, tf: r\.dir, sym: m\.symbol/);
  assert.doesNotMatch(code, /state === 'retry'/);
  // DB-G14: the picker is the shared held-or-ready rule over the offered feeds.
  assert.match(code, /rows=\{offeredFeeds\(feeds\.all\)\.map\(\(f\) => \{[\s\S]{0,400}feedPickerRow\(/);
  assert.doesNotMatch(code, /disabled: f\.ready !== true/);
  // DB-B39: the stall is the module's verdict.
  assert.match(code, /stallVerdict\(\{ sinceProgressMs, perInstrumentMs: flight\?\.perInstrumentMs \?\? null \}\)/);
  assert.doesNotMatch(code, /stalledSecs >= 20/);
  // DB-G21: the in-flight line with its pace and labelled ETA is rendered.
  assert.match(code, /data-run-flight="named"[\s\S]{0,900}\{pace\.where\}[\s\S]{0,300}\{pace\.eta\}/);
  // DB-B29: an idle tab polls and shows the notice; Pull is disabled by it.
  assert.match(code, /busyNotice\(pilot, serverRun, feeds\.active, feedName\)/);
  assert.match(code, /disabled=\{phase === 'running' \|\| pressing \|\| busy\?\.blocksPull === true\}/);
  // DB-B32: a resumed run adopts the run's own start, and names its feeds.
  assert.match(code, /startedAt = run\.startedAtMs \?\? Date\.now\(\)/);
  assert.match(code, /\{adoptedSentence\(adopted, feedName\)\}/);
  // DB-B38: a row's pull sends its unsettled spans.
  assert.match(code, /unsettledSpans\(row\.months, from, to, monthLastDay\)/);
  // DB-G22: the import panel is drawn from the journal's summary.
  assert.match(code, /<ImportPanel[\s\S]{0,200}summary=\{importSummary\}/);
});
